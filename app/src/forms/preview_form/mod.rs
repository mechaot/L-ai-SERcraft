//! Ports `PreviewForm.cs` + Designer: right-hand panel of the main window. Job preview on top,
//! toolbar below (reset, homing, unlock, zero, custom buttons flow, resume, stop).
//! Custom button editor: [`custom_button_form`]; button behaviour helpers: [`custom_button_ib`].
//!
//! The C# form never calls back into `MainForm` (dialogs of the custom buttons are its own), so
//! [`PreviewFormAction`] only reports state changes the host may want to observe.

pub mod custom_button_form;
pub mod custom_button_ib;
pub mod gallery_demo;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eframe::egui::{self, Align, Color32, Frame, Layout, Rect, Sense, Ui, Vec2};
use lasergrbl_core::{CoreEvent, GrblCore, MacStatus, PointF};
use lg_gcode::config::Firmware;
use lg_services::custom_button::{ButtonContext, CustomButton, CustomButtons, ImportPlan};
use lg_services::hotkeys::{Actions, HotKeysManager};

use crate::ui::dialog::{filters, DialogResult, FileDialogOptions, MessageBox, MessageBoxButtons, MessageBoxHost, MessageBoxIcon, PendingFile};
use crate::ui::icons::icons;
use crate::ui::preview::{Backend, Hotkeys, MachineStatus, PreviewEvent, PreviewInput, PreviewOptions, PreviewPanel};
use crate::ui::theme::Theme;
use crate::ui::tr;
use crate::ui::widgets::ImageButton;
use custom_button_form::{CustomButtonForm, FormResult};
use custom_button_ib::{
    drop_index, icon_key_for_caption, is_disabled, paint_unlocked_overlay, perform_click, perform_mouse_down, perform_mouse_up, IbState, ImageCache, PngButton, BUTTON_MARGIN, BUTTON_SIZE,
};

/// Unique ids for editor dialogs.
static FORM_ID: AtomicU64 = AtomicU64::new(0);

/// Next dialog id.
///
/// Returns a process-unique number.
pub(crate) fn next_form_id() -> u64 {
    FORM_ID.fetch_add(1, Ordering::Relaxed)
}

/// Things the host (main window) may want to react to.
#[derive(Debug, Clone, PartialEq)]
pub enum PreviewFormAction {
    /// The custom button list changed (add / edit / remove / reorder / import); already saved.
    CustomButtonsChanged,
    /// GL preview failed, the CPU renderer is used now.
    PreviewFellBackToCpu { reason: String },
    /// Camera moved: world window.
    ViewChanged { left: f64, right: f64, bottom: f64, top: f64 },
}

/// Message box tags of this form.
#[derive(Debug, Clone)]
enum Tag {
    /// `RemoveButton_Click` question (guid).
    Delete(String),
    /// `Import`: remove actual buttons? (YesNoCancel).
    ImportClear,
    /// `Import`: import button n? (YesNo).
    ImportItem(usize),
    /// `Import`: decode error.
    ImportError,
}

/// Running import (C# `CustomButtons.Import`, message boxes inverted).
struct ImportFlow {
    plan: ImportPlan,
    clear: bool,
    accepted: Vec<bool>,
}

/// File dialog purpose.
#[derive(Clone, Copy)]
enum FileKind {
    Import,
    Export,
}

/// Command picked from a context menu, executed after the layout pass.
enum Cmd {
    Add,
    Edit(String),
    Remove(String),
    ToggleLock,
    Export,
    Import,
}

/// Machine status as the preview panel knows it.
///
/// `s`: core status. Returns the preview enum (`Tool` of grblHAL counts as hold).
pub fn map_status(s: MacStatus) -> MachineStatus {
    match s {
        MacStatus::Disconnected => MachineStatus::Disconnected,
        MacStatus::Connecting => MachineStatus::Connecting,
        MacStatus::Idle => MachineStatus::Idle,
        MacStatus::Run => MachineStatus::Run,
        MacStatus::Hold | MacStatus::Tool => MachineStatus::Hold,
        MacStatus::Door => MachineStatus::Door,
        MacStatus::Home => MachineStatus::Home,
        MacStatus::Alarm => MachineStatus::Alarm,
        MacStatus::Check => MachineStatus::Check,
        MacStatus::Jog => MachineStatus::Jog,
        MacStatus::Queue => MachineStatus::Queue,
        MacStatus::Cooling => MachineStatus::Cooling,
        MacStatus::AutoHold => MachineStatus::AutoHold,
    }
}

/// Facts `CustomButton::enabled_now` reads.
///
/// `core`: the engine. Returns the context.
pub fn button_context(core: &GrblCore) -> ButtonContext {
    let st = core.machine_status();
    ButtonContext { is_connected: core.is_connected(), status_idle: st == MacStatus::Idle, status_run: st == MacStatus::Run, has_program: core.has_program() }
}

/// Enable/visibility of the fixed toolbar buttons (C# `TimerUpdate` + `SetCore`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolbarState {
    pub reset: bool,
    pub homing_visible: bool,
    pub homing: bool,
    pub unlock_visible: bool,
    pub unlock: bool,
    pub stop: bool,
    pub resume: bool,
    pub zeroing: bool,
}

impl ToolbarState {
    /// Read the rules from the core.
    ///
    /// `core`: the engine. Returns the state.
    pub fn from_core(core: &GrblCore) -> Self {
        Self {
            reset: core.can_reset_grbl(),
            homing_visible: GrblCore::configuration().homing_enabled(),
            homing: core.can_do_homing(),
            unlock_visible: core.firmware() == Firmware::Grbl,
            unlock: core.can_unlock(),
            stop: core.can_feed_hold(),
            resume: core.can_resume_hold(),
            zeroing: core.can_do_zeroing(),
        }
    }
}

/// The preview form.
pub struct PreviewForm {
    panel: PreviewPanel,
    /// Bumped on `CoreEvent::FileLoaded`.
    file_revision: u64,
    /// `MnLockButton.Checked` inverted: true = drag reorder mode.
    unlocked: bool,
    states: HashMap<String, IbState>,
    images: ImageCache,
    /// Hotkey driven presses, executed in the next `show` (needs core + list).
    pending_hotkeys: Vec<(usize, bool)>,
    /// `ImportButton(filename)` request: Some(None) = ask with a dialog.
    pending_import: Option<Option<PathBuf>>,
    button_count: usize,
    editor: Option<CustomButtonForm>,
    msgs: MessageBoxHost<Tag>,
    file: Option<(FileKind, PendingFile)>,
    /// File dialog to open at the next `poll_file` (needs a ctx).
    want_dialog: Option<FileKind>,
    import: Option<ImportFlow>,
    /// Dragged button index.
    drag: Option<usize>,
    /// Flow area height of the last frame (for the right-click area under the buttons).
    flow_height: f32,
}

impl Default for PreviewForm {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewForm {
    /// Ctor (`CreatePanel`: GDI setting picks the CPU renderer, else GL with CPU fallback).
    ///
    /// Returns the form; the core arrives with the first `show`.
    pub fn new() -> Self {
        Self::with_backend(Backend::Auto)
    }

    /// Ctor with a forced renderer (gallery / tests).
    ///
    /// `backend`: renderer choice. Returns the form.
    pub fn with_backend(backend: Backend) -> Self {
        Self {
            panel: PreviewPanel::new(backend),
            file_revision: 0,
            unlocked: false,
            states: HashMap::new(),
            images: ImageCache::default(),
            pending_hotkeys: Vec::new(),
            pending_import: None,
            button_count: 0,
            editor: None,
            msgs: MessageBoxHost::default(),
            file: None,
            want_dialog: None,
            import: None,
            drag: None,
            flow_height: 0.0,
        }
    }

    /// `OnColorChange`: call after a colour scheme switch.
    #[allow(dead_code)] // API for the main window
    pub fn on_color_change(&mut self) {
        self.panel.on_color_change();
    }

    /// Feed one drained core event (call for every `CoreEvent` of the frame).
    ///
    /// `ev`: the event. `core`: the engine (program end clears the executed marks of the loaded file).
    pub fn on_core_event(&mut self, ev: &CoreEvent, core: &GrblCore) {
        match ev {
            CoreEvent::FileLoaded { .. } => self.file_revision += 1,
            CoreEvent::AutoSizeDrawing => self.panel.auto_size_drawing(),
            CoreEvent::ZoomInDrawing => self.panel.zoom_in(),
            CoreEvent::ZoomOutDrawing => self.panel.zoom_out(),
            CoreEvent::ProgramEnded => PreviewPanel::on_program_ended(&core.loaded_file()),
            _ => {}
        }
    }

    /// `ImportButton(filename)`: import custom buttons (`.zbn` double click, or menu).
    ///
    /// `filename`: file to import, None opens a file dialog first. Runs on the next `show`.
    #[allow(dead_code)] // API for the main window
    pub fn import_button(&mut self, filename: Option<PathBuf>) {
        self.pending_import = Some(filename);
    }

    /// `CustomImageButtons.Count` (for `HotKeyContext`).
    ///
    /// Returns the number of custom buttons shown.
    #[allow(dead_code)] // API for the main window
    pub fn custom_button_count(&self) -> usize {
        self.button_count
    }

    /// Hotkey effect `CustomButtonDown(i)`: `EmulateMouseInside = true; PerformMouseDown`.
    ///
    /// `i`: button index. Takes effect on the next `show`.
    pub fn custom_button_down(&mut self, i: usize) {
        self.pending_hotkeys.push((i, true));
    }

    /// Hotkey effect `CustomButtonUp(i)`: `PerformMouseUp; PerformClick`.
    ///
    /// `i`: button index. Takes effect on the next `show`.
    pub fn custom_button_up(&mut self, i: usize) {
        self.pending_hotkeys.push((i, false));
    }

    /// `RefreshCustomButtons`: rebuild per-button state (C# creates new controls, `on` is lost).
    fn refresh_custom_buttons(&mut self) {
        self.states.clear();
        self.images.clear();
    }

    /// Draw the form into the remaining space of `ui`.
    ///
    /// `ui`: target ui. `core`: engine. `buttons`: custom button list (edited by the form, saved on change).
    /// `hotkeys`: shortcut texts for the preview tips. Returns actions for the host.
    pub fn show(&mut self, ui: &mut Ui, core: &Arc<GrblCore>, buttons: &mut CustomButtons, hotkeys: &HotKeysManager) -> Vec<PreviewFormAction> {
        let mut actions = Vec::new();
        self.button_count = buttons.count();
        let bctx = button_context(core);
        self.run_pending(core, buttons, &bctx);
        self.poll_file(ui.ctx(), buttons);
        // toolbar below, preview takes the rest (table rows: Percent / AutoSize)
        let th = Theme::current();
        let mut cmds = Vec::new();
        egui::Panel::bottom("preview_form_toolbar")
            .frame(Frame::NONE.fill(th.form_back_color))
            .show_separator_line(false)
            .show(ui, |ui| self.toolbar(ui, core, buttons, &bctx, &mut cmds, &mut actions));
        let input = self.build_input(core, hotkeys);
        for ev in self.panel.show(ui, &input) {
            match ev {
                PreviewEvent::JogToPosition { x, y, fast } => core.jog_to_position_fast(PointF::new(x, y), fast),
                PreviewEvent::FellBackToCpu { reason } => actions.push(PreviewFormAction::PreviewFellBackToCpu { reason }),
                PreviewEvent::ViewChanged { left, right, bottom, top } => actions.push(PreviewFormAction::ViewChanged { left, right, bottom, top }),
            }
        }
        self.run_cmds(ui.ctx(), cmds, buttons);
        self.dialogs(ui.ctx(), buttons, &mut actions);
        actions
    }

    /// Fill `PreviewInput` from the core (C# panel reads these in `TimerUpdate`).
    fn build_input(&self, core: &GrblCore, hotkeys: &HotKeysManager) -> PreviewInput {
        let o = {
            let v = core.view_options();
            PreviewOptions {
                show_laser_off_movements: *v.show_laser_off_movements.value(),
                show_executed_commands: *v.show_executed_commands.value(),
                show_performance_diagnostic: *v.show_performance_diagnostic.value(),
                show_bounding_box: *v.show_bounding_box.value(),
                cross_cursor: *v.cross_cursor.value(),
                preview_line_size: *v.preview_line_size.value(),
                auto_size_on_drawing: *v.auto_size_on_drawing.value(),
            }
        };
        let hk = |a: Actions| hotkeys.get_hotkey_string(a).unwrap_or_default();
        PreviewInput {
            file: Some(core.loaded_file()),
            file_revision: self.file_revision,
            machine_position: core.machine_position(),
            work_position: core.work_position(),
            working_offset: core.working_offset(),
            current_f: core.current_f(),
            current_s: core.current_s(),
            machine_status: map_status(core.machine_status()),
            connected: core.is_connected(),
            jog_enabled: core.jog_enabled(),
            in_program: core.in_program(),
            options: o,
            hotkeys: Hotkeys { auto_size: hk(Actions::AutoSizeDrawing), zoom_in: hk(Actions::ZoomInDrawing), zoom_out: hk(Actions::ZoomOutDrawing) },
            ..PreviewInput::from_settings()
        }
    }

    /// Hotkey presses and import requests queued since the last frame.
    fn run_pending(&mut self, core: &GrblCore, buttons: &CustomButtons, bctx: &ButtonContext) {
        for (i, down) in std::mem::take(&mut self.pending_hotkeys) {
            let Some(cb) = buttons.get_button(i) else { continue };
            let st = self.states.entry(cb.guid.clone()).or_default();
            let mut codes = Vec::new();
            if down {
                codes.extend(perform_mouse_down(cb, st, bctx, self.unlocked));
            } else {
                // EmulateCustomButtonUp: PerformMouseUp then PerformClick
                codes.extend(perform_mouse_up(cb, st, bctx, self.unlocked));
                codes.extend(perform_click(cb, st, bctx, self.unlocked));
            }
            for c in codes {
                core.execute_custom_code(&c);
            }
        }
        if let Some(req) = self.pending_import.take() {
            match req {
                None => self.want_dialog = Some(FileKind::Import),
                Some(path) => self.begin_import(buttons, &path),
            }
        }
    }

    /// Open wanted file dialogs and handle finished ones (Export / Import menu).
    fn poll_file(&mut self, ctx: &egui::Context, buttons: &mut CustomButtons) {
        if let Some(kind) = self.want_dialog.take() {
            let pending = match kind {
                FileKind::Import => PendingFile::open(ctx, FileDialogOptions { filter: Some(filters::ZIPPED_BUTTON_OPEN.into()), ..Default::default() }),
                FileKind::Export => PendingFile::save(ctx, FileDialogOptions { filter: Some(filters::ZIPPED_BUTTON_SAVE.into()), file_name: Some("CustomButtons.zbn".into()), ..Default::default() }),
            };
            self.file = Some((kind, pending));
        }
        let Some((kind, pending)) = self.file.as_mut() else { return };
        let Some(res) = pending.poll() else { return };
        let kind = *kind;
        self.file = None;
        let Some(mut path) = res else { return };
        match kind {
            FileKind::Export => {
                // AddExtension = true
                if path.extension().is_none() {
                    path.set_extension("zbn");
                }
                if let Err(e) = buttons.export_zbn(&path) {
                    lg_base::logger::log_exception("CustomButtons", &e);
                }
            }
            FileKind::Import => self.begin_import(buttons, &path),
        }
    }

    /// First half of `CustomButtons.Import`: decode and ask the questions.
    fn begin_import(&mut self, buttons: &CustomButtons, path: &std::path::Path) {
        match buttons.import_plan(path) {
            // missing file or no button inside: C# returns false
            Ok(None) => {}
            Err(e) => self.msgs.push(Tag::ImportError, MessageBox::new(e, "")),
            Ok(Some(plan)) => {
                let ask = plan.ask_clear;
                self.import = Some(ImportFlow { plan, clear: false, accepted: Vec::new() });
                if ask {
                    self.msgs.push(Tag::ImportClear, MessageBox::new(tr("Strings.BoxImportCustomButtonClearText"), tr("Strings.BoxImportCustomButtonCaption")).buttons(MessageBoxButtons::YesNoCancel));
                } else {
                    self.ask_import_item(0);
                }
            }
        }
    }

    /// Queue the `Import "{name}"?` question of candidate `n`.
    fn ask_import_item(&mut self, n: usize) {
        let Some(flow) = &self.import else { return };
        let name = flow.plan.candidates[n].display_name.clone();
        self.msgs.push(Tag::ImportItem(n), MessageBox::new(format!("Import \"{name}\"?"), tr("Strings.BoxImportCustomButtonCaption")).buttons(MessageBoxButtons::YesNo));
    }

    /// Execute context menu commands.
    fn run_cmds(&mut self, ctx: &egui::Context, cmds: Vec<Cmd>, buttons: &mut CustomButtons) {
        let _ = ctx;
        for c in cmds {
            match c {
                Cmd::Add => self.editor = Some(CustomButtonForm::new_create()),
                Cmd::Edit(g) => {
                    if let Some(cb) = buttons.buttons().iter().find(|b| b.guid == g) {
                        self.editor = Some(CustomButtonForm::new_edit(cb));
                    }
                }
                Cmd::Remove(g) => self.msgs.push(
                    Tag::Delete(g),
                    // default button = No (Button2)
                    MessageBox::new(tr("Strings.BoxDeleteCustomButtonText"), tr("Strings.BoxDeleteCustomButtonTitle")).buttons(MessageBoxButtons::YesNo).icon(MessageBoxIcon::Question).default_button(1),
                ),
                // MnLockButton: Checked toggles, ButtonPositionUnlocked = !Checked
                Cmd::ToggleLock => self.unlocked = !self.unlocked,
                Cmd::Export => self.want_dialog = Some(FileKind::Export),
                Cmd::Import => self.want_dialog = Some(FileKind::Import),
            }
        }
    }

    /// Editor dialog and message boxes.
    fn dialogs(&mut self, ctx: &egui::Context, buttons: &mut CustomButtons, actions: &mut Vec<PreviewFormAction>) {
        if let Some(ed) = self.editor.as_mut() {
            match ed.show(ctx, buttons) {
                FormResult::Open => {}
                r => {
                    self.editor = None;
                    // RefreshCustomButtons after the dialog, saved or not
                    self.refresh_custom_buttons();
                    if r == FormResult::Saved {
                        actions.push(PreviewFormAction::CustomButtonsChanged);
                    }
                }
            }
        }
        let Some((tag, res)) = self.msgs.show(ctx) else { return };
        match tag {
            Tag::Delete(g) => {
                if res == DialogResult::Yes {
                    buttons.remove(&g);
                    buttons.save_file();
                    self.refresh_custom_buttons();
                    actions.push(PreviewFormAction::CustomButtonsChanged);
                }
            }
            Tag::ImportError => {}
            Tag::ImportClear => match res {
                DialogResult::Yes | DialogResult::No => {
                    if let Some(f) = self.import.as_mut() {
                        f.clear = res == DialogResult::Yes;
                    }
                    self.ask_import_item(0);
                }
                // Cancel: import aborted
                _ => self.import = None,
            },
            Tag::ImportItem(n) => {
                let Some(f) = self.import.as_mut() else { return };
                f.accepted.push(res == DialogResult::Yes);
                if n + 1 < f.plan.candidates.len() {
                    self.ask_import_item(n + 1);
                } else if let Some(f) = self.import.take() {
                    // apply, then PreviewForm refreshes and saves
                    buttons.apply_import(&f.plan, f.clear, &f.accepted);
                    buttons.save_file();
                    self.refresh_custom_buttons();
                    actions.push(PreviewFormAction::CustomButtonsChanged);
                }
            }
        }
    }

    /// The toolbar row (`tableLayoutPanel8`).
    fn toolbar(&mut self, ui: &mut Ui, core: &Arc<GrblCore>, buttons: &mut CustomButtons, bctx: &ButtonContext, cmds: &mut Vec<Cmd>, actions: &mut Vec<PreviewFormAction>) {
        let _ = actions;
        let st = ToolbarState::from_core(core);
        let cell = BUTTON_SIZE + 2.0 * BUTTON_MARGIN;
        let gap = 5.0;
        let fixed_left = [st.reset, st.homing_visible, st.unlock_visible].iter().filter(|v| **v).count() as f32 * cell + gap + cell;
        let fixed_right = gap + 2.0 * cell;
        let mid = (ui.available_width() - fixed_left - fixed_right).max(cell);
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.set_min_height(cell);
        ui.horizontal_top(|ui| {
            // columns 0-2
            if tool_button(ui, "custom-reset", "PreviewForm.BtnReset.ToolTip", st.reset) {
                core.grbl_reset();
            }
            if st.homing_visible && tool_button(ui, "custom-homing", "PreviewForm.BtnHoming.ToolTip", st.homing) {
                core.grbl_homing();
            }
            if st.unlock_visible && tool_button(ui, "custom-unlock", "PreviewForm.BtnUnlock.ToolTip", st.unlock) {
                core.grbl_unlock();
            }
            ui.add_space(gap);
            if tool_button(ui, "custom-zeroing", "PreviewForm.BtnZeroing.ToolTip", st.zeroing) {
                core.set_new_zero();
            }
            // column 5: the flow of custom buttons
            let origin = ui.cursor().min;
            let area = Rect::from_min_size(origin, Vec2::new(mid, self.flow_height.max(cell)));
            // area menu first, so the buttons on top win the clicks
            let area_resp = ui.interact(area, ui.scope_id().with("cb_area"), Sense::click());
            let inner = ui.allocate_ui_with_layout(Vec2::new(mid, 0.0), Layout::left_to_right(Align::TOP).with_main_wrap(true), |ui| {
                ui.set_min_size(Vec2::new(mid, cell));
                ui.spacing_mut().item_spacing = Vec2::splat(2.0 * BUTTON_MARGIN);
                self.flow(ui, core, buttons, bctx, cmds)
            });
            self.flow_height = inner.response.rect.height();
            if buttons.count() == 0 {
                let p = ui.painter().with_clip_rect(area);
                p.text(area.center(), egui::Align2::CENTER_CENTER, tr("Strings.AddCustomButtonsHint"), egui::FontId::proportional(12.0), Color32::DARK_GRAY);
            }
            area_resp.context_menu(|ui| area_menu(ui, self.unlocked, buttons.count() > 0, cmds));
            // columns 7-8
            ui.add_space(gap);
            if tool_button(ui, "custom-resume", "PreviewForm.BtnResume.ToolTip", st.resume) {
                core.cycle_start_resume(false);
            }
            if tool_button(ui, "custom-stop", "PreviewForm.BtnStop.ToolTip", st.stop) {
                core.feed_hold(false);
            }
        });
    }

    /// Custom button flow (`CustomButtonArea`): buttons, drag reorder, per-button menu.
    fn flow(&mut self, ui: &mut Ui, core: &Arc<GrblCore>, buttons: &mut CustomButtons, bctx: &ButtonContext, cmds: &mut Vec<Cmd>) {
        let th = Theme::current();
        let legacy = icons().is_legacy();
        let list: Vec<CustomButton> = buttons.buttons().to_vec();
        let mut rects: Vec<Rect> = Vec::with_capacity(list.len());
        let mut drag_started = None;
        for (idx, cb) in list.iter().enumerate() {
            let disabled = is_disabled(cb, bctx, self.unlocked);
            let st = self.states.entry(cb.guid.clone()).or_default();
            let back = st.back_pressed(cb.button_type).then_some(th.pressed_buttons);
            let sense = if self.unlocked { Sense::click_and_drag() } else { Sense::click() };
            // non-legacy: icon by caption, else the PNG
            let icon = icon_key_for_caption(&cb.caption, legacy, |k| icons().contains(k));
            let resp = match &icon {
                Some(key) => {
                    let mut b = ImageButton::new(key).size(Vec2::splat(BUTTON_SIZE)).rounded_borders(true).caption(&cb.caption).draw_disabled(disabled);
                    if let Some(c) = back {
                        b = b.back_color(c);
                    }
                    let r = ui.add(b);
                    // reorder mode needs drag: a second interact over the same rect
                    if self.unlocked { ui.interact(r.rect, r.id.with("drag"), sense) } else { r }
                }
                None => {
                    let tex = self.images.get(ui.ctx(), cb);
                    PngButton { tex: tex.as_ref(), caption: &cb.caption, disabled, back, sense }.show(ui)
                }
            };
            rects.push(resp.rect);
            let resp = resp.on_hover_text(&cb.tooltip);
            if self.unlocked {
                paint_unlocked_overlay(ui, resp.rect);
                if resp.drag_started() {
                    drag_started = Some(idx);
                }
            } else {
                // MouseDown / MouseUp edges, then Click (WinForms order)
                let down = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down());
                let mut codes = Vec::new();
                if down && !st.was_down {
                    codes.extend(perform_mouse_down(cb, st, bctx, self.unlocked));
                }
                if !down && st.was_down {
                    codes.extend(perform_mouse_up(cb, st, bctx, self.unlocked));
                }
                if resp.clicked_by(egui::PointerButton::Primary) {
                    codes.extend(perform_click(cb, st, bctx, self.unlocked));
                }
                st.was_down = down;
                for c in codes {
                    core.execute_custom_code(&c);
                }
            }
            let guid = cb.guid.clone();
            resp.context_menu(|ui| {
                if ui.button(tr("Strings.CustomButtonRemove")).clicked() {
                    cmds.push(Cmd::Remove(guid.clone()));
                    ui.close();
                }
                if ui.button(tr("Strings.CustomButtonEdit")).clicked() {
                    cmds.push(Cmd::Edit(guid.clone()));
                    ui.close();
                }
            });
        }
        if drag_started.is_some() {
            self.drag = drag_started;
        }
        self.drag_drop(ui, &rects, buttons);
    }

    /// Insert marker while dragging and `OnDragDrop` on release.
    fn drag_drop(&mut self, ui: &mut Ui, rects: &[Rect], buttons: &mut CustomButtons) {
        let Some(old) = self.drag else { return };
        let Some(pos) = ui.input(|i| i.pointer.interact_pos()) else { return };
        let Some(ins) = drop_index(rects, pos) else { return };
        // CornflowerBlue bar left of the target (right of the last)
        let (x, h, top) = match rects.get(ins) {
            Some(r) => (r.left() - 3.0, r.height(), r.top()),
            None => {
                let r = rects[rects.len() - 1];
                (r.right() + 1.0, r.height(), r.top())
            }
        };
        ui.painter().rect_filled(Rect::from_min_size(egui::pos2(x, top + 2.0), Vec2::new(2.0, h)), 0.0, Color32::from_rgb(100, 149, 237));
        if ui.input(|i| i.pointer.any_released()) {
            self.drag = None;
            if buttons.reorder(old, ins as i32) {
                self.refresh_custom_buttons();
            }
        }
    }
}

/// Menu of the flow area (`MNAddCB`).
fn area_menu(ui: &mut Ui, unlocked: bool, has_buttons: bool, cmds: &mut Vec<Cmd>) {
    if ui.button(tr("PreviewForm.addCustomButtonToolStripMenuItem.Text")).clicked() {
        cmds.push(Cmd::Add);
        ui.close();
    }
    // MnLockButton is checked while locked
    if ui.checkbox(&mut !unlocked, tr("PreviewForm.MnLockButton.Text")).clicked() {
        cmds.push(Cmd::ToggleLock);
        ui.close();
    }
    ui.separator();
    if ui.add_enabled(has_buttons, egui::Button::new(tr("PreviewForm.exportCustomButtonsToolStripMenuItem.Text"))).clicked() {
        cmds.push(Cmd::Export);
        ui.close();
    }
    if ui.button(tr("PreviewForm.importCustomButtonsToolStripMenuItem.Text")).clicked() {
        cmds.push(Cmd::Import);
        ui.close();
    }
}

/// One fixed 49x49 toolbar button with 2px margin.
///
/// `ui`: horizontal parent. `icon`: icon key. `tip_key`: tooltip i18n key. `enabled`: `Enabled`.
/// Returns true when clicked.
fn tool_button(ui: &mut Ui, icon: &str, tip_key: &str, enabled: bool) -> bool {
    ui.add_space(BUTTON_MARGIN);
    let r = ui.add_enabled(enabled, ImageButton::new(icon).size(Vec2::splat(BUTTON_SIZE)).rounded_borders(!icons().is_legacy())).on_hover_text(tr(tip_key));
    ui.add_space(BUTTON_MARGIN);
    r.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping() {
        assert_eq!(map_status(MacStatus::Run), MachineStatus::Run);
        assert_eq!(map_status(MacStatus::Tool), MachineStatus::Hold);
        assert_eq!(map_status(MacStatus::Disconnected), MachineStatus::Disconnected);
    }

    #[test]
    fn form_ids_unique() {
        assert_ne!(next_form_id(), next_form_id());
    }
}
