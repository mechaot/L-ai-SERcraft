#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Shared helpers of the WP-U4a forms. No C# counterpart: replaces WinForms boilerplate
//! (ShowDialog, ComboboxItem, tooltips, `GrblCore.Configuration` limit lookups).

use eframe::egui::{self, Align, Key, Layout, Modifiers, TextureHandle, Ui, Vec2};
use lg_gcode::{GResult, GrblFile};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

use crate::ui::dialog::{MessageBox, MessageBoxIcon};
use crate::ui::widgets::numeric::NumericOutput;
use crate::ui::widgets::{GrblButton, ImageButton, IntegerInputRanged};
use crate::ui::{tr, tr_plain};

/// Shared file handle used by every form that edits the loaded program.
pub type SharedFile = Arc<RwLock<GrblFile>>;

/// C# `ComboboxItem[] LaserOptions`: (text, value). Same table in every generator / import form.
pub const LASER_OPTIONS: [(&str, &str); 2] = [("M3 - Constant Power", "M3"), ("M4 - Dynamic Power", "M4")];

/// Result of a modal frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalEvent<T> {
    /// Content closure produced a result.
    Done(T),
    /// Esc or the close box (WinForms `DialogResult.Cancel` via window close).
    Closed,
}

/// Show a fixed-width modal "form" with a title row (and optional close box).
///
/// `ctx`: egui context. `id`: unique form id. `title`: window caption. `width`: client width in points.
/// `closable`: draw a close box (forms without a Cancel button). `content`: body, returns `Some(result)` to close.
/// Returns `Some(event)` on the closing frame, else None. Esc closes only when no popup (combo) is open.
pub fn run_modal<T>(
    ctx: &egui::Context,
    id: impl Hash + Debug,
    title: &str,
    width: f32,
    closable: bool,
    content: impl FnOnce(&mut Ui) -> Option<T>,
) -> Option<ModalEvent<T>> {
    let modal = egui::Modal::new(egui::Id::unique(id));
    let r = modal.show(ctx, |ui| {
        ui.set_width(width);
        // title row: bold caption, close box at the right edge
        let mut closed = false;
        ui.horizontal(|ui| {
            ui.label(crate::ui::fonts::bold(title));
            if closable {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(ImageButton::new("mdi-close").size(Vec2::splat(16.0))).clicked() {
                        closed = true;
                    }
                });
            }
        });
        ui.separator();
        if closed {
            return (None, true);
        }
        (content(ui), false)
    });
    let (inner, closed) = r.inner;
    if let Some(v) = inner {
        return Some(ModalEvent::Done(v));
    }
    // Esc handling, same rule as egui `ModalResponse::should_close` minus backdrop click
    let esc = r.is_top_modal && !r.any_popup_open && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
    if closed || esc { Some(ModalEvent::Closed) } else { None }
}

/// Bottom button row: right aligned `[Cancel] [OK]` with the C# icons.
///
/// `ui`: parent. `prefix`: form key prefix incl. trailing dot, e.g. "Generator.CuttingTest.".
/// `ok_ctl`: control name of the OK button. `cancel_ctl`: control name of Cancel (None = no Cancel).
/// Returns `Some(true)` on OK, `Some(false)` on Cancel.
pub fn ok_cancel_row(ui: &mut Ui, prefix: &str, ok_ctl: &str, cancel_ctl: Option<&str>) -> Option<bool> {
    let mut out = None;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        // right to left: OK first so it sits at the right edge
        let ok = GrblButton::new(ctl_text(prefix, ok_ctl)).icon("mdi-checkbox-marked").min_size(Vec2::new(74.0, 27.0));
        if ui.add(ok).clicked() {
            out = Some(true);
        }
        if let Some(c) = cancel_ctl {
            let cancel = GrblButton::new(ctl_text(prefix, c)).icon("mdi-close-box").min_size(Vec2::new(74.0, 27.0));
            if ui.add(cancel).clicked() {
                out = Some(false);
            }
        }
    });
    out
}

/// Caption of a control: `tr("<prefix><ctl>.Text")` without mnemonics.
///
/// `prefix`: form key prefix incl. trailing dot. `ctl`: control name. Returns plain text.
pub fn ctl_text(prefix: &str, ctl: &str) -> String {
    tr_plain(&format!("{prefix}{ctl}.Text"))
}

/// Tooltip of a control: `tr("<prefix><ctl>.ToolTip")`.
///
/// `prefix`: form key prefix incl. trailing dot. `ctl`: control name. Returns text.
pub fn ctl_tip(prefix: &str, ctl: &str) -> String {
    tr(&format!("{prefix}{ctl}.ToolTip")).to_owned()
}

/// Plain label for a control caption.
///
/// `ui`: parent. `prefix`/`ctl`: see [`ctl_text`].
pub fn label(ui: &mut Ui, prefix: &str, ctl: &str) {
    ui.label(ctl_text(prefix, ctl));
}

/// The 17x17 "information" image button (`mdi-information-slab-box`, hand cursor, tooltip).
///
/// `ui`: parent. `tip`: tooltip text. Returns true when clicked.
pub fn info_button(ui: &mut Ui, tip: &str) -> bool {
    let r = ui.add(ImageButton::new("mdi-information-slab-box").size(Vec2::splat(17.0)));
    // empty tooltip = control has none in the designer
    if tip.is_empty() { r.clicked() } else { r.on_hover_text(tip).clicked() }
}

/// Themed integer input (C# `IntegerInputRanged`).
///
/// `ui`: parent. `id`: unique id. `v`: bound value. `min`/`max`: limits. `force`: `ForceMinMax`.
/// `width`: control width. Returns `Some((old, new))` when the user committed a new value.
pub fn int_input(ui: &mut Ui, id: impl Hash + Debug, v: &mut i32, min: i32, max: i32, force: bool, width: f32) -> Option<(i32, i32)> {
    let o: NumericOutput<i32> = IntegerInputRanged::new(v).range(min, max).force_min_max(force).width(width).id(egui::Id::unique(id)).show(ui);
    o.committed
}

/// Decode an embedded image into an egui texture.
///
/// `ctx`: egui context. `name`: texture name. `bytes`: PNG/JPEG data. Returns None when decoding fails.
pub fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(name, ci, egui::TextureOptions::LINEAR))
}

/// Draw a texture at its native pixel size (WinForms `PictureBoxSizeMode.AutoSize`).
///
/// `ui`: parent. `tex`: texture or None (blank frame is skipped).
pub fn native_image(ui: &mut Ui, tex: Option<&TextureHandle>) {
    if let Some(t) = tex {
        ui.add(egui::Image::new(t).fit_to_original_size(1.0));
    }
}

/// `GrblCore.Configuration` values the forms need, truncated to C# `int`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_rate_x: i32,
    pub max_rate_y: i32,
    pub max_pwm: i32,
    pub table_width: i32,
    pub table_height: i32,
    pub laser_mode: bool,
}

impl Limits {
    /// Read the stored configuration (`lg_gcode::config::configuration()`).
    ///
    /// Returns limits; missing values fall back to the config defaults (4000/1000/300/200).
    pub fn from_config() -> Self {
        let c = lg_gcode::config::configuration::configuration();
        // (int)decimal truncates toward zero
        let i = |d: lg_dotnet::Decimal| lg_dotnet::f64_to_i32(lg_dotnet::decimal_to_f64(d));
        Limits {
            max_rate_x: i(c.max_rate_x()),
            max_rate_y: i(c.max_rate_y()),
            max_pwm: i(c.max_pwm()),
            table_width: i(c.table_width()),
            table_height: i(c.table_height()),
            laser_mode: c.laser_mode(),
        }
    }
}

/// Outcome of running a generator against the shared file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenStatus {
    /// Program replaced.
    Done,
    /// File busy (C# `CheckInUse` message then return).
    Busy,
    /// Generator error text (C# would throw to the UI thread).
    Failed(String),
}

/// Run a generator synchronously under the write lock (C# generators are not threaded).
///
/// `file`: shared program. `f`: closure calling the `lg_gcode` generator. Returns [`GenStatus`].
pub fn run_generator(file: &SharedFile, f: impl FnOnce(&mut GrblFile) -> GResult<bool>) -> GenStatus {
    let mut g = file.write().unwrap_or_else(|e| e.into_inner());
    match f(&mut g) {
        Ok(true) => GenStatus::Done,
        Ok(false) => GenStatus::Busy,
        Err(e) => {
            lg_base::logger::log_exception("Generator", &e);
            GenStatus::Failed(e.to_string())
        }
    }
}

/// `MessageBox.Show(Strings.AlreadyLoading)`.
///
/// Returns the box to push into a host.
pub fn already_loading_box() -> MessageBox {
    MessageBox::new(tr("Strings.AlreadyLoading"), "").icon(MessageBoxIcon::None)
}

/// `Tools.Utils.OpenLink(url)` (usage id placeholder left empty).
///
/// `url`: http(s) link, `|` separated list allowed.
pub fn open_link(url: &str) {
    lg_base::tools::utils::open_link(url);
}

/// Guard of [`test_guard`]; resets the settings store again when dropped so other test modules
/// (which share the mutex) never see leftovers.
#[cfg(test)]
pub struct TestGuard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

/// Point the settings store at a fresh unique file (init_at flushes the old store into the OLD path, so path must change).
#[cfg(test)]
fn reset_store() {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    lg_base::settings::init_at(std::env::temp_dir().join(format!("lg-forms-settings-{}-{n}.json", std::process::id())));
}

#[cfg(test)]
impl Drop for TestGuard {
    fn drop(&mut self) {
        reset_store();
    }
}

/// Test helper: serialize settings tests (one shared mutex) and start from an empty store.
///
/// Returns the guard; keep it alive for the whole test.
#[cfg(test)]
pub fn test_guard() -> TestGuard {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let g = M.lock().unwrap_or_else(|e| e.into_inner());
    reset_store();
    TestGuard(g)
}

/// Alias of [`test_guard`] kept for older call sites.
#[cfg(test)]
pub fn settings_guard() -> TestGuard {
    test_guard()
}

/// Dev screenshot sequencer: opens form after form, saves `<dir>/<name>.png`, then closes the app.
pub struct ShotSeq {
    dir: std::path::PathBuf,
    names: &'static [&'static str],
    idx: usize,
    frames: u32,
    saved: Arc<std::sync::atomic::AtomicBool>,
}

/// What the host has to do after [`ShotSeq::step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotStep {
    /// Nothing.
    Idle,
    /// Open the form with this index.
    Open(usize),
    /// Close all forms (next frame starts the next one).
    CloseAll,
}

impl ShotSeq {
    /// Sequencer from env var `var` (directory for the PNGs); None when unset.
    ///
    /// `var`: env var name. `names`: file stem per form, in opening order.
    pub fn from_env(var: &str, names: &'static [&'static str]) -> Option<Self> {
        let dir: std::path::PathBuf = std::env::var_os(var)?.into();
        let _ = std::fs::create_dir_all(&dir);
        Some(ShotSeq { dir, names, idx: 0, frames: 0, saved: Arc::new(Default::default()) })
    }

    /// Advance one frame. Frame 1 opens, frame 25 grabs the screen, then closes and moves on.
    ///
    /// `ctx`: egui context. Returns the action for the host. Quits the app after the last form.
    pub fn step(&mut self, ctx: &egui::Context) -> ShotStep {
        use std::sync::atomic::Ordering::SeqCst;
        self.frames += 1;
        ctx.request_repaint();
        if self.frames == 1 {
            return ShotStep::Open(self.idx);
        }
        if self.frames == 25 {
            let path = self.dir.join(format!("{}.png", self.names[self.idx]));
            let saved = self.saved.clone();
            saved.store(false, SeqCst);
            ctx.request_screenshot(move |img| {
                let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buf) = image::RgbaImage::from_raw(img.width() as u32, img.height() as u32, raw) {
                    let _ = buf.save(&path);
                }
                saved.store(true, SeqCst);
            });
        }
        if self.frames > 25 && self.saved.load(SeqCst) {
            self.idx += 1;
            self.frames = 0;
            if self.idx >= self.names.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return ShotStep::CloseAll;
        }
        ShotStep::Idle
    }
}
