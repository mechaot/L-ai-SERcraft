//! Gallery tab "Dialogs2": buttons opening every WP-U4b dialog against a core on the built-in emulator.

use eframe::egui::{self, Ui};
use lasergrbl_core::com::{ComConfig, WrapperType};
use lasergrbl_core::{CoreEvent, DetectedIssue, GrblCore, ResumeInfo, RunFromPositionInfo};
use lg_gcode::config::{Firmware, GPoint};
use lg_services::hotkeys::HotKeysManager;
use std::sync::Arc;

use crate::forms::form_util::{ShotSeq, ShotStep};
use super::discovery_form::DiscoveryForm;
use super::emulator_ui::EmulatorWindow;
use super::flash_grbl::FlashGrbl;
use super::grbl_config::GrblConfig;
use super::hotkey_manager_form::HotkeyManagerForm;
use super::issue_detector_form::IssueDetectorForm;
use super::laser_life_edit::LaserLifeEdit;
use super::laser_selector::LaserSelector;
use super::laser_usage::LaserUsage;
use super::ortur_wifi_config::OrturWiFiConfig;
use super::ortur_wifi_config_prompt::OrturWiFiConfigPrompt;
use super::resume_job_form::ResumeJobForm;
use super::run_from_position_form::RunFromPositionForm;
use super::safety_countdown::SafetyCountdown;
use super::settings_form::SettingsForm;

/// The one dialog currently open.
enum Open {
    Settings(SettingsForm),
    Config(GrblConfig),
    Hotkeys(HotkeyManagerForm),
    Resume(ResumeJobForm),
    RunFrom(RunFromPositionForm),
    Safety(SafetyCountdown),
    Usage(LaserUsage),
    LifeEdit(LaserLifeEdit),
    Selector(LaserSelector),
    Issue(IssueDetectorForm),
    Flash(FlashGrbl),
    Discovery(DiscoveryForm),
    WifiConfig(OrturWiFiConfig),
    WifiPrompt(OrturWiFiConfigPrompt),
}

/// Form names in screenshot order (see [`Dialogs2Tab::open_by_index`]).
const SHOT_NAMES: [&str; 22] = [
    "settings", "settings_raster", "settings_vector", "settings_jog", "settings_cooling", "settings_gcode", "settings_notification", "settings_options", "grbl_config", "hotkey_manager", "resume_job", "run_from_position", "safety_countdown", "laser_usage", "laser_life_edit", "laser_selector",
    "issue_detector", "flash_grbl", "discovery", "ortur_wifi_config", "ortur_wifi_config_prompt", "emulator_ui",
];

/// Gallery tab state.
#[derive(Default)]
pub struct Dialogs2Tab {
    shot: Option<ShotSeq>,
    shot_init: bool,
    core: Option<(Arc<GrblCore>, crossbeam_channel::Receiver<CoreEvent>)>,
    emu: Option<Arc<EmulatorWindow>>,
    open: Option<Open>,
    log: String,
    /// Frames the current dialog was drawn (tests).
    pub shown_frames: u32,
}

impl Dialogs2Tab {
    /// Core on the emulator, created and connected on first use.
    fn core(&mut self) -> Arc<GrblCore> {
        if self.core.is_none() {
            let (tx, rx) = crossbeam_channel::unbounded();
            let core = GrblCore::new(Firmware::Grbl, tx);
            core.configure(WrapperType::Emulator, &ComConfig::None);
            let _ = core.open_com();
            self.core = Some((core, rx));
        }
        self.core.as_ref().map(|c| c.0.clone()).expect("core")
    }

    /// Draw the tab.
    ///
    /// `ui`: target ui.
    pub fn ui(&mut self, ui: &mut Ui) {
        let core = self.core();
        if let Some((_, rx)) = &self.core {
            while rx.try_recv().is_ok() {}
        }
        ui.label("Core runs on the built-in emulator. Saving writes the real settings file.");
        let names = ["SettingsForm", "GrblConfig", "HotkeyManagerForm", "ResumeJobForm", "RunFromPositionForm", "SafetyCountdown", "LaserUsage", "LaserLifeEdit", "LaserSelector", "IssueDetectorForm", "FlashGrbl", "DiscoveryForm", "OrturWiFiConfig", "OrturWiFiConfigPrompt", "EmulatorUI"];
        ui.horizontal_wrapped(|ui| {
            for (i, n) in names.iter().enumerate() {
                if ui.button(*n).clicked() {
                    self.open_by_index(i, &core);
                }
            }
        });
        ui.label(&self.log);
    }

    /// Open dialog number `i` (order of [`SHOT_NAMES`]).
    ///
    /// `i`: index. `core`: core on the emulator.
    fn open_by_index(&mut self, i: usize, core: &Arc<GrblCore>) {
        let core = core.clone();
        // settings tabs take the first 8 shot slots; the other dialogs follow
        if i < 8 {
            let mut f = SettingsForm::open(&core);
            f.select_tab(i);
            self.open = Some(Open::Settings(f));
            return;
        }
        let i = i - 7;
        match i {
            0 => self.open = Some(Open::Settings(SettingsForm::open(&core))),
            1 => self.open = Some(Open::Config(GrblConfig::open(&core))),
            2 => self.open = Some(Open::Hotkeys(HotkeyManagerForm::open(&HotKeysManager::with_defaults()))),
            3 => {
                let info = ResumeInfo { executed: 40, sent: 60, target: 100, last_issue: DetectedIssue::StopResponding, homing_enabled: true, suggest_homing: true, suggest_setwco: true, last_known_wco: GPoint::new(10.0, 20.0, 0.0) };
                self.open = Some(Open::Resume(ResumeJobForm::open(info)));
            }
            4 => self.open = Some(Open::RunFrom(RunFromPositionForm::open(&RunFromPositionInfo { line_count: 250, homing_enabled: true }))),
            5 => self.open = Some(Open::Safety(SafetyCountdown::open())),
            6 => self.open = Some(Open::Usage(LaserUsage::open())),
            7 => self.open = Some(Open::LifeEdit(LaserLifeEdit::open(&lasergrbl_core::laser_life::LaserLifeCounter::create_new()))),
            8 => self.open = Some(Open::Selector(LaserSelector::open())),
            9 => self.open = Some(Open::Issue(IssueDetectorForm::open(DetectedIssue::UnexpectedReset))),
            10 => self.open = Some(Open::Flash(FlashGrbl::open())),
            11 => self.open = Some(Open::Discovery(DiscoveryForm::open(&core))),
            12 => self.open = Some(Open::WifiConfig(OrturWiFiConfig::open(&core))),
            13 => self.open = Some(Open::WifiPrompt(OrturWiFiConfigPrompt::new("Ortur Laser Master 3"))),
            _ => {
                let e = self.emu.get_or_insert_with(EmulatorWindow::install).clone();
                lasergrbl_core::emulator::show_ui("Emulator started (gallery)");
                let _ = e;
            }
        }
    }

    /// Dev screenshot driver (env `LASERGRBL_DIALOGS_SHOT=<dir>`).
    ///
    /// `ctx`: egui context.
    fn debug_shots(&mut self, ctx: &egui::Context) {
        if !self.shot_init {
            self.shot_init = true;
            self.shot = ShotSeq::from_env("LASERGRBL_DIALOGS_SHOT", &SHOT_NAMES);
        }
        let Some(mut s) = self.shot.take() else { return };
        match s.step(ctx) {
            ShotStep::Open(i) => {
                let core = self.core();
                self.open_by_index(i, &core);
            }
            ShotStep::CloseAll => self.open = None,
            ShotStep::Idle => {}
        }
        self.shot = Some(s);
    }

    /// Draw open dialogs; call from the gallery's `dialogs`.
    ///
    /// `ctx`: egui context.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        self.debug_shots(ctx);
        if let Some(e) = &self.emu {
            e.show(ctx);
        }
        let Some(o) = &mut self.open else { return };
        self.shown_frames += 1;
        let (done, text) = match o {
            Open::Settings(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::Config(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::Hotkeys(f) => f.show(ctx).map(|r| (true, format!("{r:?}").chars().take(200).collect())).unwrap_or_default(),
            Open::Resume(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::RunFrom(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::Safety(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::Usage(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::LifeEdit(f) => f.show(ctx).map(|r| (true, format!("{r:?}").chars().take(200).collect())).unwrap_or_default(),
            Open::Selector(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::Issue(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::Flash(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::Discovery(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::WifiConfig(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
            Open::WifiPrompt(f) => f.show(ctx).map(|r| (true, format!("{r:?}"))).unwrap_or_default(),
        };
        if done {
            self.log = format!("last outcome: {text}");
            self.open = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::dialogs::common::test_guard;

    /// Smoke: draw every dialog a few frames headless.
    #[test]
    fn every_dialog_renders() {
        let _g = test_guard();
        let ctx = egui::Context::default();
        let mut tab = Dialogs2Tab::default();
        let (tx, _rx) = crossbeam_channel::unbounded();
        let core = GrblCore::new(Firmware::Grbl, tx);
        let info = ResumeInfo { executed: 40, sent: 60, target: 100, last_issue: DetectedIssue::StopResponding, homing_enabled: true, suggest_homing: true, suggest_setwco: true, last_known_wco: GPoint::new(1.0, 2.0, 3.0) };
        let makers: Vec<Box<dyn Fn() -> Open>> = vec![
            Box::new({ let c = core.clone(); move || Open::Settings(SettingsForm::open(&c)) }),
            Box::new({ let c = core.clone(); move || Open::Config(GrblConfig::open(&c)) }),
            Box::new(|| Open::Hotkeys(HotkeyManagerForm::open(&HotKeysManager::with_defaults()))),
            Box::new(move || Open::Resume(ResumeJobForm::open(info.clone()))),
            Box::new(|| Open::RunFrom(RunFromPositionForm::new(10, true))),
            Box::new(|| Open::Safety(SafetyCountdown::open())),
            Box::new(|| Open::Usage(LaserUsage::open())),
            Box::new(|| Open::LifeEdit(LaserLifeEdit::open(&lasergrbl_core::laser_life::LaserLifeCounter::create_new()))),
            Box::new(|| Open::Selector(LaserSelector::open())),
            Box::new(|| Open::Issue(IssueDetectorForm::open(DetectedIssue::ManualReset))),
            Box::new(|| Open::Flash(FlashGrbl::open())),
            Box::new({ let c = core.clone(); move || Open::Discovery(DiscoveryForm::open(&c)) }),
            Box::new({ let c = core.clone(); move || Open::WifiConfig(OrturWiFiConfig::open(&c)) }),
            Box::new(|| Open::WifiPrompt(OrturWiFiConfigPrompt::new("Machine"))),
        ];
        for mk in makers {
            tab.open = Some(mk());
            tab.shown_frames = 0;
            for _ in 0..3 {
                let mut out = ctx.run_ui(egui::RawInput::default(), |ui| tab.dialogs(&ui.ctx().clone()));
                out.textures_delta.clear();
            }
            assert!(tab.shown_frames >= 1);
            tab.open = None;
        }
    }
}
