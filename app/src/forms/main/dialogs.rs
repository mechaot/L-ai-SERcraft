//! Dialog host of the main window: one `Overlay` per modal form, shown one at a time (C# `ShowDialog`).

use eframe::egui;

use crate::forms::dialogs::discovery_form::{DiscoveryForm, DiscoveryOutcome};
use crate::forms::dialogs::flash_grbl::{FlashGrbl, FlashOutcome};
use crate::forms::dialogs::grbl_config::{GrblConfig, GrblConfigOutcome};
use crate::forms::dialogs::hotkey_manager_form::{HotkeyManagerForm, HotkeyOutcome};
use crate::forms::dialogs::issue_detector_form::{IssueDetectorForm, IssueOutcome};
use crate::forms::dialogs::laser_selector::{LaserSelector, LaserSelectorOutcome};
use crate::forms::dialogs::laser_usage::{LaserUsage, LaserUsageOutcome};
use crate::forms::dialogs::ortur_wifi_config::{OrturWiFiConfig, WifiConfigOutcome};
use crate::forms::dialogs::ortur_wifi_config_prompt::{OrturWiFiConfigPrompt, WifiPromptOutcome};
use crate::forms::dialogs::resume_job_form::{ResumeJobForm, ResumeOutcome};
use crate::forms::dialogs::run_from_position_form::{RunFromPositionForm, RunFromPositionOutcome};
use crate::forms::dialogs::safety_countdown::{SafetyCountdown, SafetyOutcome};
use crate::forms::dialogs::settings_form::{SettingsForm, SettingsOutcome};
use crate::forms::generators::cutting_test::CuttingTest;
use crate::forms::generators::power_vs_speed::PowerVsSpeedForm;
use crate::forms::generators::shake_test::ShakeTest;
use crate::forms::legal_disclaimer::{LegalDisclaimer, LegalOutcome};
use crate::forms::license::LicenseForm;
use crate::forms::main::exception::{ExceptionDialog, ExceptionResult};
use crate::forms::material::MaterialEditor;
use crate::forms::raster::{RasterImport, RasterOutcome};
use crate::forms::save_options::{SaveOptionForm, SaveOptionOutcome};
use crate::forms::splash::SplashScreen;
use crate::forms::svg_import::{SvgImport, SvgOutcome};

/// What to do once the safety countdown said go (order of `SafetyCountdown.CanGo` in C#).
#[derive(Debug, Clone, PartialEq)]
pub enum AfterSafety {
    /// Fresh `RunProgram`.
    Run,
    /// Resume dialog: from the beginning.
    ResumeFromStart { homing: bool },
    /// Resume dialog: from a known line.
    Continue { position: i32, homing: bool, setwco: bool },
    /// "Run from position" dialog.
    FromPosition { position: i32, homing: bool },
}

/// A modal form.
pub enum Overlay {
    Splash(SplashScreen),
    Legal(LegalDisclaimer),
    Settings(SettingsForm),
    GrblConfig(GrblConfig),
    Hotkeys(HotkeyManagerForm),
    Material(MaterialEditor),
    SaveOptions(SaveOptionForm),
    License(LicenseForm),
    Raster(Box<RasterImport>),
    Svg(Box<SvgImport>),
    CuttingTest(Box<CuttingTest>),
    PowerVsSpeed(Box<PowerVsSpeedForm>),
    ShakeTest(Box<ShakeTest>),
    Resume(ResumeJobForm),
    RunFromPosition(RunFromPositionForm),
    Safety(SafetyCountdown, AfterSafety),
    LaserSelector(LaserSelector),
    LaserUsage(LaserUsage),
    Issue(IssueDetectorForm),
    Flash(FlashGrbl),
    Discovery(DiscoveryForm),
    OrturWifi(OrturWiFiConfig),
    WifiPrompt(OrturWiFiConfigPrompt),
    Exception(ExceptionDialog),
}

/// Result of a closed overlay.
pub enum OverlayResult {
    /// Nothing to act on (cancelled / informational).
    Closed,
    SplashDone,
    Legal(LegalOutcome),
    Settings(SettingsOutcome),
    Hotkeys(HotkeyOutcome),
    SaveOptions(SaveOptionOutcome),
    Raster(RasterOutcome),
    Svg,
    Resume(ResumeOutcome),
    RunFromPosition(RunFromPositionOutcome),
    Safety(SafetyOutcome, AfterSafety),
    LaserSelector(LaserSelectorOutcome),
    Issue,
    Flash(FlashOutcome),
    Discovery(DiscoveryOutcome),
    OrturWifi(WifiConfigOutcome),
    WifiPrompt(WifiPromptOutcome),
    Exception(ExceptionResult),
}

impl Overlay {
    /// Draw the form.
    ///
    /// `ctx`: egui context. Returns the result on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<OverlayResult> {
        use OverlayResult as R;
        match self {
            Overlay::Splash(f) => f.show(ctx).then_some(R::SplashDone),
            Overlay::Legal(f) => f.show(ctx).map(R::Legal),
            Overlay::Settings(f) => f.show(ctx).map(R::Settings),
            Overlay::GrblConfig(f) => f.show(ctx).map(|GrblConfigOutcome::Closed| R::Closed),
            Overlay::Hotkeys(f) => f.show(ctx).map(R::Hotkeys),
            Overlay::Material(f) => f.show(ctx).then_some(R::Closed),
            Overlay::SaveOptions(f) => f.show(ctx).map(R::SaveOptions),
            Overlay::License(f) => f.show(ctx).map(|_| R::Closed),
            Overlay::Raster(f) => f.show(ctx).map(R::Raster),
            Overlay::Svg(f) => f.show(ctx).map(|_: SvgOutcome| R::Svg),
            Overlay::CuttingTest(f) => f.show(ctx).map(|_| R::Closed),
            Overlay::PowerVsSpeed(f) => f.show(ctx).map(|_| R::Closed),
            Overlay::ShakeTest(f) => f.show(ctx).map(|_| R::Closed),
            Overlay::Resume(f) => f.show(ctx).map(R::Resume),
            Overlay::RunFromPosition(f) => f.show(ctx).map(R::RunFromPosition),
            Overlay::Safety(f, next) => f.show(ctx).map(|o| R::Safety(o, next.clone())),
            Overlay::LaserSelector(f) => f.show(ctx).map(R::LaserSelector),
            Overlay::LaserUsage(f) => f.show(ctx).map(|LaserUsageOutcome::Closed| R::Closed),
            Overlay::Issue(f) => f.show(ctx).map(|_: IssueOutcome| R::Issue),
            Overlay::Flash(f) => f.show(ctx).map(R::Flash),
            Overlay::Discovery(f) => f.show(ctx).map(R::Discovery),
            Overlay::OrturWifi(f) => f.show(ctx).map(R::OrturWifi),
            Overlay::WifiPrompt(f) => f.show(ctx).map(R::WifiPrompt),
            Overlay::Exception(f) => f.show(ctx).map(R::Exception),
        }
    }
}
