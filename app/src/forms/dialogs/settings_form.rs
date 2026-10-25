// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `SettingsForm.cs` + `SettingsForm.Designer.cs`: tabbed settings dialog.
//!
//! `SettingsData` is the plain model (load/save of every setting key, unit-testable); `SettingsForm` is
//! the modal egui form around it.

use eframe::egui::{self, RichText, TextEdit, Ui, Vec2};
use lasergrbl_core::com::WrapperType;
use lasergrbl_core::{GrblCore, StreamingMode, ThreadingMode};
use lg_base::settings::{self, GraphicMode};
use lg_base::sound::EventId;
use lg_dotnet::TimeSpan;
use lg_gcode::config::{Firmware, GCODE_STD_FOOTER, GCODE_STD_HEADER, GCODE_STD_PASSES};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::forms::form_util::{load_texture, native_image};
use super::common::{ModalEvent, button, modal, open_link, setting_row, trf};
use crate::ui::dialog::{DialogResult, FileDialogOptions, MessageBox, MessageBoxButtons, PendingFile};
use crate::ui::widgets::{flat_combo, grbl_group_box};
use crate::ui::{icons, tr, tr_plain};

/// Sound events in the order of the form rows.
pub const SOUND_EVENTS: [EventId; 5] = [EventId::Success, EventId::Warning, EventId::Fatal, EventId::Connect, EventId::Disconnect];

/// Min auto cooling time (`MaxTs(TimeSpan.FromSeconds(10), ..)`).
const MIN_COOLING_SECONDS: f64 = 10.0;

/// One sound row: enabled flag + file path text (`xxxFullLabel`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundSetting {
    pub enabled: bool,
    pub file: String,
}

/// Every value edited by the form, same defaults as the C# constructor.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsData {
    pub firmware: Firmware,
    pub support_pwm: bool,
    pub protocol: WrapperType,
    pub streaming_mode: StreamingMode,
    pub unidirectional: bool,
    pub disable_g0_skip: bool,
    pub threading_mode: ThreadingMode,
    /// Shown value = NOT "Do not show Issue Detector".
    pub issue_detector: bool,
    pub soft_reset: bool,
    pub hard_reset: bool,
    pub query_di: bool,
    pub disable_bound_warn: bool,
    pub click_n_jog: bool,
    pub continuous_jog: bool,
    pub enable_z_jog: bool,
    pub hi_res: bool,
    pub header: String,
    pub passes: String,
    pub footer: String,
    pub sounds: [SoundSetting; 5],
    pub smart_bezier: bool,
    pub disable_safety_cd: bool,
    pub quiet_safety_cd: bool,
    pub legacy_icons: bool,
    pub graphic_mode: GraphicMode,
    pub auto_cooling: bool,
    pub on_min: i32,
    pub on_sec: i32,
    pub off_min: i32,
    pub off_sec: i32,
}

/// Default sound path text of the C# form: `Sound\<Id>.wav`.
///
/// `id`: event. Returns the default text.
pub fn default_sound_text(id: EventId) -> String {
    format!("Sound\\{}.wav", id.name())
}

/// WinForms multiline text uses CRLF; egui uses LF.
///
/// `s`: stored text. Returns text for the editor.
pub fn to_editor_text(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// Inverse of [`to_editor_text`] plus `Trim()` (done in `BtnSave_Click`).
///
/// `s`: editor text. Returns the text stored in settings.
pub fn from_editor_text(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\n', "\r\n").trim().to_owned()
}

/// `Path.GetFileName` for a Windows or Unix path text.
///
/// `p`: path text. Returns the last component.
pub fn file_name_of(p: &str) -> String {
    p.rsplit(['\\', '/']).next().unwrap_or(p).to_owned()
}

/// `MaxTs(TimeSpan.FromSeconds(10), new TimeSpan(0, min, sec))`.
///
/// `min`,`sec`: selected combo values. Returns the span to store.
pub fn cooling_span(min: i32, sec: i32) -> TimeSpan {
    let t = TimeSpan::from_seconds((min * 60 + sec) as f64);
    let floor = TimeSpan::from_seconds(MIN_COOLING_SECONDS);
    if t.ticks() < floor.ticks() { floor } else { t }
}

impl SettingsData {
    /// Constructor part of `SettingsForm`: read every key with its default.
    pub fn load() -> Self {
        let get_ts = |k: &str, d: TimeSpan| lasergrbl_core::util::get_timespan_setting(k, d);
        let on = get_ts("AutoCooling TOn", TimeSpan::from_minutes(10.0));
        let off = get_ts("AutoCooling TOff", TimeSpan::from_minutes(1.0));
        let sounds = SOUND_EVENTS.map(|id| SoundSetting {
            enabled: settings::get(&format!("Sound.{}.Enabled", id.name()), true),
            file: settings::get(&format!("Sound.{}", id.name()), default_sound_text(id)),
        });
        SettingsData {
            firmware: settings::get("Firmware Type", Firmware::Grbl),
            support_pwm: settings::get("Support Hardware PWM", true),
            protocol: settings::get("ComWrapper Protocol", WrapperType::UsbSerial),
            streaming_mode: settings::get("Streaming Mode", StreamingMode::Buffered),
            unidirectional: settings::get("Unidirectional Engraving", false),
            disable_g0_skip: settings::get("Disable G0 fast skip", false),
            threading_mode: settings::get("Threading Mode", ThreadingMode::UltraFast),
            issue_detector: !settings::get("Do not show Issue Detector", false),
            soft_reset: settings::get("Reset Grbl On Connect", true),
            hard_reset: settings::get("HardReset Grbl On Connect", false),
            query_di: settings::get("Query MachineInfo ($I) at connect", true),
            disable_bound_warn: settings::get("DisableBoundaryWarning", false),
            click_n_jog: settings::get("Click N Jog", true),
            continuous_jog: settings::get("Enable Continuous Jog", false),
            // NOTE(upstream): key is misspelled "Enale" in C#
            enable_z_jog: settings::get("Enale Z Jog Control", false),
            hi_res: settings::get("Raster Hi-Res", false),
            header: to_editor_text(&settings::get("GCode.CustomHeader", GCODE_STD_HEADER.to_string())),
            passes: to_editor_text(&settings::get("GCode.CustomPasses", GCODE_STD_PASSES.to_string())),
            footer: to_editor_text(&settings::get("GCode.CustomFooter", GCODE_STD_FOOTER.to_string())),
            sounds,
            smart_bezier: settings::get("Vector.UseSmartBezier", true),
            disable_safety_cd: settings::get("DisableSafetyCountdown", false),
            quiet_safety_cd: settings::get("QuietSafetyCountdown", false),
            legacy_icons: settings::get("LegacyIcons", false),
            graphic_mode: settings::configured_graphic_mode(),
            auto_cooling: settings::get("AutoCooling", false),
            // out-of-range value leaves the C# combo empty; clamp to the item range instead
            on_min: on.minutes().clamp(0, 60),
            on_sec: on.seconds().clamp(0, 59),
            off_min: off.minutes().clamp(0, 10),
            off_sec: off.seconds().clamp(0, 59),
        }
    }

    /// `BtnSave_Click` settings part (without the restart questions).
    pub fn save(&self) {
        settings::set("Firmware Type", self.firmware);
        settings::set("Support Hardware PWM", self.support_pwm);
        settings::set("ComWrapper Protocol", self.protocol);
        settings::set("Streaming Mode", self.streaming_mode);
        settings::set("Unidirectional Engraving", self.unidirectional);
        settings::set("Disable G0 fast skip", self.disable_g0_skip);
        settings::set("Threading Mode", self.threading_mode);
        settings::set("Do not show Issue Detector", !self.issue_detector);
        settings::set("Reset Grbl On Connect", self.soft_reset);
        settings::set("HardReset Grbl On Connect", self.hard_reset);
        settings::set("Query MachineInfo ($I) at connect", self.query_di);
        settings::set("Enable Continuous Jog", self.continuous_jog);
        settings::set("Enale Z Jog Control", self.enable_z_jog);
        settings::set("DisableBoundaryWarning", self.disable_bound_warn);
        settings::set("Click N Jog", self.click_n_jog);

        settings::set("AutoCooling", self.auto_cooling);
        settings::set("AutoCooling TOn", cooling_span(self.on_min, self.on_sec).to_string());
        settings::set("AutoCooling TOff", cooling_span(self.off_min, self.off_sec).to_string());

        settings::set("GCode.CustomHeader", from_editor_text(&self.header));
        settings::set("GCode.CustomPasses", from_editor_text(&self.passes));
        settings::set("GCode.CustomFooter", from_editor_text(&self.footer));

        for (id, s) in SOUND_EVENTS.iter().zip(&self.sounds) {
            let key = format!("Sound.{}", id.name());
            let text = s.file.trim();
            // Windows default text would break the Linux path join: keep the key unset instead
            if !(text == default_sound_text(*id) && !settings::exists(&key)) {
                settings::set(&key, text);
            }
            settings::set(&format!("{key}.Enabled"), s.enabled);
        }


        settings::set("Raster Hi-Res", self.hi_res);
        settings::set("Vector.UseSmartBezier", self.smart_bezier);
        settings::set("DisableSafetyCountdown", self.disable_safety_cd);
        settings::set("QuietSafetyCountdown", self.quiet_safety_cd);
        settings::set("LegacyIcons", self.legacy_icons);
        settings::set_configured_graphic_mode(self.graphic_mode);
    }
}

/// Result of the settings dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOutcome {
    /// Cancel / close box.
    Cancelled,
    /// Saved (C# `SettingsChanged` event fired). `restart`: the user confirmed one of the
    /// "restart now?" questions, caller must restart the application.
    Saved { restart: bool },
}

/// Tabs in designer order.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Hardware,
    Raster,
    Vector,
    Jog,
    Cooling,
    GCode,
    Sound,
    Options,
}

/// Restart question kinds, asked in this order after save.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ask {
    Graphic,
    Firmware,
    Icons,
}

/// Dialog phase.
enum Phase {
    Editing,
    Asking(VecDeque<(Ask, MessageBox)>),
}

/// The settings dialog.
pub struct SettingsForm {
    core: Arc<GrblCore>,
    data: SettingsData,
    tab: Tab,
    prev_graphic: GraphicMode,
    phase: Phase,
    warn_ortur_ac: bool,
    file_pick: Option<(usize, PendingFile)>,
    info: Option<MessageBox>,
    id: u64,
    cool_tex: Option<Option<egui::TextureHandle>>,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Graphic mode combo entries: caption and value.
const GRAPHIC_MODES: [(&str, GraphicMode); 4] = [
    ("Auto", GraphicMode::Auto),
    ("Hardware Acceleration", GraphicMode::Fbo),
    ("Software Rendering", GraphicMode::Dib),
    ("Legacy", GraphicMode::Gdi),
];

const FIRMWARES: [Firmware; 4] = [Firmware::Grbl, Firmware::Smoothie, Firmware::Marlin, Firmware::VigoWork];
const PROTOCOLS: [WrapperType; 6] = [
    WrapperType::UsbSerial,
    WrapperType::UsbSerial2,
    WrapperType::RJCPSerial,
    WrapperType::Telnet,
    WrapperType::LaserWebESP8266,
    WrapperType::Emulator,
];
const STREAMING: [StreamingMode; 3] = [StreamingMode::Buffered, StreamingMode::Synchronous, StreamingMode::RepeatOnError];
const THREADING: [ThreadingMode; 5] = [ThreadingMode::Insane, ThreadingMode::UltraFast, ThreadingMode::Fast, ThreadingMode::Quiet, ThreadingMode::Slow];

/// Control column width of the settings rows.
const CTL_W: f32 = 170.0;

/// Key text of a designer label of this form.
///
/// `ctl`: control name. Returns the translated `.Text`.
fn t(ctl: &str) -> String {
    tr(&format!("SettingsForm.{ctl}.Text")).to_owned()
}

/// Combo over a slice of copyable enum values.
///
/// `ui`: parent. `id`: id salt. `items`: values. `names`: display names. `cur`: bound value.
fn enum_pick<T: Copy + PartialEq>(ui: &mut Ui, id: &str, items: &[T], names: &[String], cur: &mut T) {
    let mut sel = items.iter().position(|i| i == cur);
    if flat_combo(ui, id, &mut sel, names, 150.0).changed() {
        if let Some(i) = sel {
            *cur = items[i];
        }
    }
}

impl SettingsForm {
    /// Create the dialog (C# ctor): loads all settings.
    ///
    /// `core`: engine, used for the Ortur warning and the firmware restart question.
    pub fn open(core: &Arc<GrblCore>) -> Self {
        let warn = !core.grbl_version().is_some_and(|v| v.is_ortur() && v.ortur_fw_version_number() >= 140);
        SettingsForm {
            core: core.clone(),
            data: SettingsData::load(),
            tab: Tab::Hardware,
            prev_graphic: settings::configured_graphic_mode(),
            phase: Phase::Editing,
            warn_ortur_ac: warn,
            file_pick: None,
            info: None,
            cool_tex: None,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Select a tab by designer index (dev screenshots).
    ///
    /// `i`: tab index 0..=7, others are ignored.
    pub fn select_tab(&mut self, i: usize) {
        const ALL: [Tab; 8] = [Tab::Hardware, Tab::Raster, Tab::Vector, Tab::Jog, Tab::Cooling, Tab::GCode, Tab::Sound, Tab::Options];
        if let Some(t) = ALL.get(i) {
            self.tab = *t;
        }
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame, else None.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<SettingsOutcome> {
        // post-save restart questions are separate message boxes
        if let Phase::Asking(q) = &mut self.phase {
            return Self::run_asking(ctx, q);
        }
        if let Some(mb) = &mut self.info {
            if mb.show(ctx).is_some() {
                self.info = None;
            }
            return None;
        }
        self.poll_file_pick(ctx);
        let title = trf("SettingsForm.$this.Text", "Settings");
        let id = ("settings", self.id);
        let ev = modal(ctx, id, &title, 906.0, true, |ui| self.body(ui));
        match ev {
            Some(ModalEvent::Done(true)) => self.on_save(),
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) => Some(SettingsOutcome::Cancelled),
            None => None,
        }
    }

    /// Show the restart questions one by one.
    ///
    /// `ctx`: egui context. `q`: pending questions. Returns the outcome when done.
    fn run_asking(ctx: &egui::Context, q: &mut VecDeque<(Ask, MessageBox)>) -> Option<SettingsOutcome> {
        let Some((_, mb)) = q.front_mut() else { return Some(SettingsOutcome::Saved { restart: false }) };
        match mb.show(ctx) {
            Some(DialogResult::Ok) => Some(SettingsOutcome::Saved { restart: true }),
            Some(_) => {
                q.pop_front();
                if q.is_empty() { Some(SettingsOutcome::Saved { restart: false }) } else { None }
            }
            None => None,
        }
    }

    /// `BtnSave_Click`: store settings and queue the restart questions.
    fn on_save(&mut self) -> Option<SettingsOutcome> {
        self.data.save();
        let q = Self::restart_questions(self.prev_graphic, self.core.firmware(), icons::icons().is_legacy());
        if q.is_empty() {
            return Some(SettingsOutcome::Saved { restart: false });
        }
        self.phase = Phase::Asking(q);
        None
    }

    /// The three "restart now?" checks of `BtnSave_Click` (read from the just saved settings).
    ///
    /// `prev_graphic`: graphic mode at dialog open. `core_fw`: firmware of the running core.
    /// `legacy_now`: legacy icon flag of the running process. Returns questions in C# order.
    fn restart_questions(prev_graphic: GraphicMode, core_fw: Firmware, legacy_now: bool) -> VecDeque<(Ask, MessageBox)> {
        let mut q = VecDeque::new();
        let title = tr("Strings.FirmwareRequireRestart");
        let ask = |key: &str| MessageBox::new(tr(key), title).buttons(MessageBoxButtons::OkCancel);
        if prev_graphic != settings::configured_graphic_mode() {
            q.push_back((Ask::Graphic, ask("Strings.PreviewChangesRequiresRestart")));
        }
        if core_fw != settings::get("Firmware Type", Firmware::Grbl) {
            q.push_back((Ask::Firmware, ask("Strings.FirmwareRequireRestartNow")));
        }
        if legacy_now != settings::get("LegacyIcons", false) {
            q.push_back((Ask::Icons, ask("Strings.IconsChangesRequiresRestart")));
        }
        q
    }

    /// Collect a finished sound file dialog.
    fn poll_file_pick(&mut self, _ctx: &egui::Context) {
        if let Some((idx, pf)) = &mut self.file_pick {
            if let Some(res) = pf.poll() {
                if let Some(p) = res {
                    self.data.sounds[*idx].file = p.display().to_string();
                }
                self.file_pick = None;
            }
        }
    }

    /// Form body: tab strip, tab page, save/cancel row. Returns Some(true)=save, Some(false)=cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        ui.horizontal(|ui| {
            let tabs = [
                (Tab::Hardware, "TpHardware"),
                (Tab::Raster, "TpRasterImport"),
                (Tab::Vector, "TpVectorImport"),
                (Tab::Jog, "TpJogControl"),
                (Tab::Cooling, "TpAutoCooling"),
                (Tab::GCode, "TpGCodeSettings"),
                (Tab::Sound, "TpSoundSettings"),
                (Tab::Options, "TpOptions"),
            ];
            for (tab, ctl) in tabs {
                ui.selectable_value(&mut self.tab, tab, tr_plain(&format!("SettingsForm.{ctl}.Text")));
            }
        });
        ui.separator();
        // TabControl client area is fixed for every tab (394px in WinForms; egui text is taller)
        egui::ScrollArea::vertical().min_scrolled_height(440.0).max_height(440.0).auto_shrink([false, false]).show(ui, |ui| match self.tab {
            Tab::Hardware => self.tab_hardware(ui),
            Tab::Raster => self.tab_raster(ui),
            Tab::Vector => self.tab_vector(ui),
            Tab::Jog => self.tab_jog(ui),
            Tab::Cooling => self.tab_cooling(ui),
            Tab::GCode => self.tab_gcode(ui),
            Tab::Sound => self.tab_sound(ui),
            Tab::Options => self.tab_options(ui),
        });
        ui.separator();
        let mut out = None;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let sz = Vec2::new(90.0, 27.0);
            if button(ui, &tr_plain("SettingsForm.BtnSave.Text"), Some("mdi-checkbox-marked"), sz) {
                out = Some(true);
            }
            if button(ui, &tr_plain("SettingsForm.BtnCancel.Text"), Some("mdi-close-box"), sz) {
                out = Some(false);
            }
        });
        out
    }

    /// Protocol tab (`TpHardware`).
    fn tab_hardware(&mut self, ui: &mut Ui) {
        let d = &mut self.data;
        let base = "https://lasergrbl.com/configuration/";
        let names: Vec<String> = FIRMWARES.iter().map(|f| format!("{f:?}")).collect();
        if setting_row(ui, Some(""), CTL_W, |ui| enum_pick(ui, "cbcore", &FIRMWARES, &names, &mut d.firmware), &t("label9")) {
            open_link(&format!("{base}#firmware-type"));
        }
        let names: Vec<String> = PROTOCOLS.iter().map(|f| f.name().to_owned()).collect();
        if setting_row(ui, Some(""), CTL_W, |ui| enum_pick(ui, "cbprotocol", &PROTOCOLS, &names, &mut d.protocol), &t("label3")) {
            open_link(&format!("{base}#protocol"));
        }
        let names: Vec<String> = STREAMING.iter().map(|f| format!("{f:?}")).collect();
        if setting_row(ui, Some(""), CTL_W, |ui| enum_pick(ui, "cbstreaming", &STREAMING, &names, &mut d.streaming_mode), &t("label4")) {
            open_link(&format!("{base}#streaming-mode"));
        }
        let names: Vec<String> = THREADING.iter().map(|f| format!("{f:?}")).collect();
        if setting_row(ui, Some(""), CTL_W, |ui| enum_pick(ui, "cbthreading", &THREADING, &names, &mut d.threading_mode), &t("label6")) {
            open_link(&format!("{base}#threading-mode"));
        }
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.issue_detector, t("CbIssueDetector")); }, &t("label7"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.soft_reset, t("CbSoftReset")); }, &t("label2"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.hard_reset, t("CbHardReset")); }, &t("label8"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.query_di, t("CbQueryDI")); }, &t("label46"));
    }

    /// Raster import tab (`TpRasterImport`).
    fn tab_raster(&mut self, ui: &mut Ui) {
        let d = &mut self.data;
        if setting_row(ui, Some(""), CTL_W, |ui| { ui.checkbox(&mut d.support_pwm, t("CBSupportPWM")); }, &t("label1")) {
            open_link("https://lasergrbl.com/configuration/#pwm-support");
        }
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.unidirectional, t("CbUnidirectional")); }, &t("label5"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.hi_res, t("CbHiRes")); }, &t("label22"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.disable_g0_skip, t("CbDisableSkip")); }, &t("label39"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.disable_bound_warn, t("CbDisableBoundWarn")); }, &t("label40"));
    }

    /// Vector import tab (`TpVectorImport`).
    fn tab_vector(&mut self, ui: &mut Ui) {
        let d = &mut self.data;
        // imageButton1 is Visible=false in the designer: no info button
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.smart_bezier, t("CbSmartBezier")); }, &t("label43"));
    }

    /// Jog control tab (`TpJogControl`).
    fn tab_jog(&mut self, ui: &mut Ui) {
        let d = &mut self.data;
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.continuous_jog, t("CbContinuosJog")); }, &t("label10"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.enable_z_jog, t("CbEnableZJog")); }, &t("label11"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.click_n_jog, t("CbClickNJog")); }, &t("label41"));
    }

    /// Automatic cooling tab (`TpAutoCooling`).
    fn tab_cooling(&mut self, ui: &mut Ui) {
        let warn = self.warn_ortur_ac;
        let d = &mut self.data;
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.auto_cooling, t("CbAutoCooling")); }, &t("label12"));
        let mins_on: Vec<String> = (0..=60).map(|i| i.to_string()).collect();
        let mins_off: Vec<String> = (0..=10).map(|i| i.to_string()).collect();
        let secs: Vec<String> = (0..=59).map(|i| i.to_string()).collect();
        // "On" row
        setting_row(
            ui,
            None,
            CTL_W + 170.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(t("label14"));
                    let mut s = Some(d.on_min as usize);
                    if flat_combo(ui, "onmin", &mut s, &mins_on, 50.0).changed() {
                        d.on_min = s.unwrap_or(0) as i32;
                    }
                    ui.label(t("label15"));
                    let mut s = Some(d.on_sec as usize);
                    if flat_combo(ui, "onsec", &mut s, &secs, 50.0).changed() {
                        d.on_sec = s.unwrap_or(0) as i32;
                    }
                    ui.label(t("label16"));
                });
            },
            &t("label13"),
        );
        // "Off" row
        setting_row(
            ui,
            None,
            CTL_W + 170.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(t("label18"));
                    let mut s = Some(d.off_min as usize);
                    if flat_combo(ui, "offmin", &mut s, &mins_off, 50.0).changed() {
                        d.off_min = s.unwrap_or(0) as i32;
                    }
                    ui.label(t("label17"));
                    let mut s = Some(d.off_sec as usize);
                    if flat_combo(ui, "offsec", &mut s, &secs, 50.0).changed() {
                        d.off_sec = s.unwrap_or(0) as i32;
                    }
                    ui.label(t("label19"));
                });
            },
            &t("label20"),
        );
        // pictureBox1: 32x32 illustration, right aligned in the control column
        let tex = self.cool_tex.get_or_insert_with(|| load_texture(ui.ctx(), "settings-cooling", include_bytes!("../../../../resources/grafica/settings_cooling.png"))).clone();
        setting_row(ui, None, CTL_W + 170.0, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| native_image(ui, tex.as_ref()));
        }, &t("label21"));
        if warn {
            // same column as the other descriptions (cell 2,4)
            ui.horizontal_top(|ui| {
                ui.add_space(26.0 + CTL_W + 170.0 + 12.0);
                let w = ui.available_width();
                ui.allocate_ui(Vec2::new(w, 0.0), |ui| {
                    ui.set_width(w);
                    ui.add(egui::Label::new(RichText::new(t("LblWarnOrturAC")).color(egui::Color32::RED)).wrap());
                });
            });
        }
    }

    /// GCode tab (`TpGCodeSettings`).
    fn tab_gcode(&mut self, ui: &mut Ui) {
        let d = &mut self.data;
        let rows: [(&str, &mut String, &str); 3] = [("groupBox1", &mut d.header, "LblHeader"), ("groupBox3", &mut d.passes, "LblPasses"), ("groupBox2", &mut d.footer, "LblFooter")];
        for (gb, text, lbl) in rows {
            ui.horizontal_top(|ui| {
                ui.allocate_ui(Vec2::new(204.0, 0.0), |ui| {
                    ui.set_width(204.0);
                    grbl_group_box(ui, &t(gb), |ui| {
                        // fixed 194x61 text box like the designer, scrolls when longer
                        egui::ScrollArea::vertical().id_salt(lbl).max_height(61.0).show(ui, |ui| {
                            ui.add(TextEdit::multiline(text).desired_rows(3).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                        });
                    });
                });
                let w = ui.available_width();
                ui.allocate_ui(Vec2::new(w, 0.0), |ui| {
                    ui.set_width(w);
                    ui.add(egui::Label::new(t(lbl)).wrap());
                });
            });
        }
    }

    /// Notification tab (`TpSoundSettings`).
    fn tab_sound(&mut self, ui: &mut Ui) {
        // (title label, "Sound File" label, description label, change button) per event
        let ctl: [(&str, &str, &str); 5] = [
            ("LblSuccessSound", "label25", "label23"),
            ("label26", "label27", "label24"),
            ("label29", "label30", "label28"),
            ("label34", "label35", "label32"),
            ("label37", "label38", "label36"),
        ];
        let mut pick: Option<usize> = None;
        for (i, (title, file_lbl, desc)) in ctl.iter().enumerate() {
            let s = &mut self.data.sounds[i];
            ui.horizontal_top(|ui| {
                // CbPlayXxx has no caption in the designer
                ui.checkbox(&mut s.enabled, "");
                ui.allocate_ui(Vec2::new(380.0, 0.0), |ui| {
                    ui.set_width(380.0);
                    // 2x2 table: [title][Sound File] / [Change 220px][file name]
                    egui::Grid::new(("snd-grid", i)).num_columns(2).spacing([6.0, 2.0]).show(ui, |ui| {
                        ui.label(crate::ui::fonts::bold(t(title)).size(15.0));
                        ui.label(crate::ui::fonts::bold(t(file_lbl)));
                        ui.end_row();
                        if button(ui, &t("changeSucBtn"), None, Vec2::new(220.0, 23.0)) {
                            pick = Some(i);
                        }
                        ui.label(file_name_of(&s.file)).on_hover_text(&s.file);
                        ui.end_row();
                    });
                });
                let w = ui.available_width();
                ui.allocate_ui(Vec2::new(w, 0.0), |ui| {
                    ui.set_width(w);
                    ui.add(egui::Label::new(t(desc)).wrap());
                });
            });
            ui.add_space(4.0);
        }
        if let Some(i) = pick {
            // SoundBrowserDialog.Filter = "Waveform Audio File|*.wav"
            let filter = trf("SettingsForm.SoundBrowserDialog.Filter", "Waveform Audio File|*.wav");
            self.file_pick = Some((i, PendingFile::open(ui.ctx(), FileDialogOptions { filter: Some(filter), ..Default::default() })));
        }
    }

    /// Options tab (`TpOptions`).
    fn tab_options(&mut self, ui: &mut Ui) {
        let d = &mut self.data;
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.quiet_safety_cd, t("CbQuietSafetyCB")); }, &t("label48"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.disable_safety_cd, t("CbDisableSafetyCD")); }, &t("label47"));
        setting_row(ui, None, CTL_W, |ui| { ui.checkbox(&mut d.legacy_icons, t("CbLegacyIcons")); }, &t("label49"));
        let names: Vec<String> = GRAPHIC_MODES.iter().map(|g| g.0.to_owned()).collect();
        let modes: Vec<GraphicMode> = GRAPHIC_MODES.iter().map(|g| g.1).collect();
        if setting_row(ui, Some(""), CTL_W, |ui| enum_pick(ui, "cbgraphic", &modes, &names, &mut d.graphic_mode), &t("label50")) {
            open_link("https://lasergrbl.com/configuration/#rendering-mode");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::common::test_guard as settings_guard;

    #[test]
    fn defaults_match_csharp() {
        let _g = settings_guard();
        let d = SettingsData::load();
        assert_eq!(d.firmware, Firmware::Grbl);
        assert!(d.support_pwm && d.soft_reset && d.query_di && d.click_n_jog && d.smart_bezier && d.issue_detector);
        assert_eq!(d.protocol, WrapperType::UsbSerial);
        assert_eq!(d.streaming_mode, StreamingMode::Buffered);
        assert_eq!(d.threading_mode, ThreadingMode::UltraFast);
        assert!(!d.hard_reset && !d.unidirectional && !d.disable_g0_skip && !d.continuous_jog && !d.legacy_icons);
        assert_eq!((d.on_min, d.on_sec, d.off_min, d.off_sec), (10, 0, 1, 0));
        assert_eq!(d.header, GCODE_STD_HEADER);
        assert_eq!(d.sounds[0].file, "Sound\\Success.wav");
        assert!(d.sounds.iter().all(|s| s.enabled));
    }

    #[test]
    fn round_trip() {
        let _g = settings_guard();
        let mut d = SettingsData::load();
        d.firmware = Firmware::Marlin;
        d.protocol = WrapperType::Telnet;
        d.streaming_mode = StreamingMode::RepeatOnError;
        d.threading_mode = ThreadingMode::Slow;
        d.issue_detector = false;
        d.hard_reset = true;
        d.enable_z_jog = true;
        d.auto_cooling = true;
        d.on_min = 3;
        d.on_sec = 30;
        d.off_min = 0;
        d.off_sec = 5; // below 10 s: stored as 10 s
        d.header = "G21\nM8 ".into();
        d.sounds[2].file = "/tmp/x.wav".into();
        d.sounds[2].enabled = false;
        d.graphic_mode = GraphicMode::Dib;
        d.save();
        let r = SettingsData::load();
        assert_eq!(r.firmware, Firmware::Marlin);
        assert_eq!(r.protocol, WrapperType::Telnet);
        assert_eq!(r.streaming_mode, StreamingMode::RepeatOnError);
        assert_eq!(r.threading_mode, ThreadingMode::Slow);
        assert!(!r.issue_detector && r.hard_reset && r.enable_z_jog && r.auto_cooling);
        assert_eq!((r.on_min, r.on_sec, r.off_min, r.off_sec), (3, 30, 0, 10));
        assert_eq!(r.header, "G21\nM8");
        assert_eq!(r.sounds[2], SoundSetting { enabled: false, file: "/tmp/x.wav".into() });
        assert_eq!(r.graphic_mode, GraphicMode::Dib);
        // raw keys exactly as C#
        assert!(settings::get("Do not show Issue Detector", false));
        assert_eq!(settings::get("AutoCooling TOn", String::new()), "00:03:30");
        assert_eq!(settings::get("AutoCooling TOff", String::new()), "00:00:10");
        assert_eq!(settings::get("GCode.CustomHeader", String::new()), "G21\r\nM8");
        // untouched default sound text must not become a stored key
        assert!(!settings::exists("Sound.Success"));
    }

    #[test]
    fn passes_default_keeps_crlf() {
        let _g = settings_guard();
        let d = SettingsData::load();
        assert!(!d.passes.contains('\r'));
        d.save();
        assert_eq!(settings::get("GCode.CustomPasses", String::new()), GCODE_STD_PASSES);
    }

    #[test]
    fn cooling_floor_and_helpers() {
        assert_eq!(cooling_span(0, 3), TimeSpan::from_seconds(10.0));
        assert_eq!(cooling_span(2, 1), TimeSpan::from_seconds(121.0));
        assert_eq!(file_name_of("Sound\\success.wav"), "success.wav");
        assert_eq!(file_name_of("/a/b/c.wav"), "c.wav");
    }

    #[test]
    fn restart_questions_follow_changes() {
        let _g = settings_guard();
        settings::set("LegacyIcons", true);
        settings::set("Firmware Type", Firmware::Smoothie);
        settings::set_configured_graphic_mode(GraphicMode::Gdi);
        let q = SettingsForm::restart_questions(GraphicMode::Auto, Firmware::Grbl, false);
        assert_eq!(q.len(), 3);
        assert_eq!(q[0].0, Ask::Graphic);
        assert_eq!(q[1].0, Ask::Firmware);
        assert_eq!(q[2].0, Ask::Icons);
        assert!(SettingsForm::restart_questions(GraphicMode::Gdi, Firmware::Smoothie, true).is_empty());
    }
}
