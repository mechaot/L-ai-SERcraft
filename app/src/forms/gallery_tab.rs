#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Gallery tab "Forms" (WP-U4a): opens each form with a throwaway `GrblFile` and logs the outcome.

use eframe::egui::{self, Ui};
use std::sync::{Arc, RwLock};

use super::form_util::{ShotSeq, ShotStep, SharedFile};
use super::generators::cutting_test::CuttingTest;
use super::generators::power_vs_speed::PowerVsSpeedForm;
use super::generators::shake_test::ShakeTest;
use super::legal_disclaimer::LegalDisclaimer;
use super::license::LicenseForm;
use super::save_options::SaveOptionForm;
use super::splash::SplashScreen;
use super::svg_mode::SvgModeForm;

/// Accuracy test SVG shipped in `resources/generator` (dev gallery runs from the workspace).
const DEMO_SVG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/generator/LaserGRBL-accuracy-test-file.svg");

/// State of the "Forms" tab.
pub struct FormsTab {
    file: SharedFile,
    svg_mode: Option<SvgModeForm>,
    cutting: Option<CuttingTest>,
    power: Option<PowerVsSpeedForm>,
    shake: Option<ShakeTest>,
    save: Option<SaveOptionForm>,
    license: Option<LicenseForm>,
    legal: Option<LegalDisclaimer>,
    splash: Option<SplashScreen>,
    log: Vec<String>,
    shot: Option<ShotSeq>,
}

/// Form names in screenshot order.
const SHOT_NAMES: [&str; 8] = ["svg_mode", "cutting_test", "power_vs_speed", "shake_test", "save_options", "license", "legal_disclaimer", "splash"];

impl Default for FormsTab {
    fn default() -> Self {
        FormsTab {
            file: Arc::new(RwLock::new(lg_gcode::GrblFile::new())),
            svg_mode: None,
            cutting: None,
            power: None,
            shake: None,
            save: None,
            license: None,
            legal: None,
            splash: None,
            log: Vec::new(),
            shot: ShotSeq::from_env("LASERGRBL_FORMS_SHOT", &SHOT_NAMES),
        }
    }
}

impl FormsTab {
    /// Number of commands in the throwaway program.
    fn count(&self) -> usize {
        self.file.read().map(|f| f.count()).unwrap_or(0)
    }

    /// Tab body: one button per form and the outcome log.
    ///
    /// `ui`: target ui.
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label(format!("Throwaway program: {} commands", self.count()));
        ui.horizontal_wrapped(|ui| {
            if ui.button("SVG mode (unused)").clicked() {
                self.svg_mode = Some(SvgModeForm::new(DEMO_SVG));
            }
            if ui.button("Cutting test").clicked() {
                self.cutting = Some(CuttingTest::new(self.file.clone()));
            }
            if ui.button("Power vs speed").clicked() {
                self.power = Some(PowerVsSpeedForm::new(self.file.clone()));
            }
            if ui.button("Shake test").clicked() {
                self.shake = Some(ShakeTest::new(self.file.clone()));
            }
            if ui.button("Save options").clicked() {
                self.save = Some(SaveOptionForm::new());
            }
            if ui.button("License").clicked() {
                self.license = Some(LicenseForm::new());
            }
            if ui.button("Legal disclaimer").clicked() {
                self.legal = Some(LegalDisclaimer::new());
            }
            if ui.button("Splash").clicked() {
                self.splash = Some(SplashScreen::new());
            }
        });
        ui.separator();
        for l in self.log.iter().rev().take(20) {
            ui.label(l);
        }
    }

    /// Open form number `i` of [`SHOT_NAMES`].
    ///
    /// `i`: index.
    fn open_by_index(&mut self, i: usize) {
        let f = self.file.clone();
        match i {
            0 => self.svg_mode = Some(SvgModeForm::new(DEMO_SVG)),
            1 => self.cutting = Some(CuttingTest::new(f)),
            2 => self.power = Some(PowerVsSpeedForm::new(f)),
            3 => self.shake = Some(ShakeTest::new(f)),
            4 => self.save = Some(SaveOptionForm::new()),
            5 => self.license = Some(LicenseForm::new()),
            6 => self.legal = Some(LegalDisclaimer::new()),
            _ => self.splash = Some(SplashScreen::new()),
        }
    }

    /// Dev helper: step the screenshot sequence.
    ///
    /// `ctx`: egui context.
    fn debug_shots(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.shot.take() else { return };
        match s.step(ctx) {
            ShotStep::Open(i) => self.open_by_index(i),
            ShotStep::CloseAll => {
                self.svg_mode = None;
                self.cutting = None;
                self.power = None;
                self.shake = None;
                self.save = None;
                self.license = None;
                self.legal = None;
                self.splash = None;
            }
            ShotStep::Idle => {}
        }
        self.shot = Some(s);
    }

    /// Draw every open form (call each frame from the gallery's dialog pass).
    ///
    /// `ctx`: egui context.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        self.debug_shots(ctx);
        if let Some(f) = &mut self.svg_mode {
            if let Some(o) = f.show(ctx) {
                self.log.push(format!("SVG mode: {o:?}"));
                self.svg_mode = None;
            }
        }
        if let Some(f) = &mut self.cutting {
            if let Some(o) = f.show(ctx) {
                self.log.push(format!("Cutting test: {o:?}"));
                self.cutting = None;
            }
        }
        if let Some(f) = &mut self.power {
            if let Some(o) = f.show(ctx) {
                self.log.push(format!("Power vs speed: {o:?}"));
                self.power = None;
            }
        }
        if let Some(f) = &mut self.shake {
            if let Some(o) = f.show(ctx) {
                self.log.push(format!("Shake test: {o:?}"));
                self.shake = None;
            }
        }
        if let Some(f) = &mut self.save {
            if let Some(o) = f.show(ctx) {
                self.log.push(format!("Save options: {o:?}"));
                self.save = None;
            }
        }
        if let Some(f) = &mut self.license {
            if f.show(ctx).is_some() {
                self.log.push("License: closed".into());
                self.license = None;
            }
        }
        if let Some(f) = &mut self.legal {
            if let Some(o) = f.show(ctx) {
                self.log.push(format!("Legal disclaimer: {o:?}"));
                self.legal = None;
            }
        }
        if let Some(f) = &mut self.splash {
            if f.show(ctx) {
                self.log.push("Splash: closed".into());
                self.splash = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{icons, tr};

    /// Every i18n key the forms read must exist in the catalog.
    #[test]
    fn all_keys_exist() {
        let p = "SvgConverter.ConvertSizeAndOptionForm.";
        let mut keys: Vec<String> = ["$this.Text", "BtnCancel.Text", "BtnCreate.Text", "BtnColorFilter.ToolTip", "BtnModulationInfo.ToolTip", "BtnOnOffInfo.ToolTip", "BtnPSHelper.ToolTip", "GbLaser.Text", "GbSpeed.Text", "LblBorderTracing.Text", "LblBorderTracingmm.Text", "LblSmax.Text", "LblSmin.Text", "gbFilter.Text", "label18.Text", "labelFilter.Text"]
            .iter()
            .map(|k| format!("{p}{k}"))
            .collect();
        let cut = (2..=19).filter(|n| ![5, 6, 14].contains(n)).map(|n| format!("label{n}.Text")).chain(["$this.Text", "BtnCancel.Text", "BtnCreate.Text", "GbParameters.Text", "LblLaserMode.Text", "PhTbLabel.WaterMark", "label1.Text"].map(String::from));
        keys.extend(cut.map(|k| format!("Generator.CuttingTest.{k}")));
        let pvs = (2..=19).map(|n| format!("label{n}.Text")).chain(["$this.Text", "BtnCancel.Text", "BtnCreate.Text", "GbParameters.Text", "LblLaserMode.Text", "PhTbLabel.WaterMark"].map(String::from));
        keys.extend(pvs.map(|k| format!("Generator.PowerVsSpeedForm.{k}")));
        for k in ["$this.Text", "BtnCancel.Text", "BtnCreate.Text", "CbAxis.Items", "CbAxis.Items1", "label1.Text", "label2.Text", "label3.Text", "label4.Text", "label5.Text", "label6.Text", "label7.Text", "label8.Text"] {
            keys.push(format!("Generator.ShakeTest.{k}"));
        }
        for k in ["$this.Text", "BtnSave.Text", "CBBetween.Text", "CBFooter.Text", "CBHeader.Text", "CBLFLineEndings.Text", "groupBox1.Text", "label1.Text", "label2.Text"] {
            keys.push(format!("SaveOptionForm.{k}"));
        }
        for k in ["$this.Text", "BtnAccept.Text", "CbIHaveRead.Text", "LblFree.Text", "LblHaveFun.Text", "LblNotToy.Text", "LlTranslate.Text"] {
            keys.push(format!("LegalDisclaimer.{k}"));
        }
        keys.extend(["Title", "Material", "MaterialNone", "Modified", "EditMaterials", "Apply", "NoSource", "Generating", "Passes", "ApplyFailed"].map(|k| format!("ParamsDock.{k}")));
        keys.extend(["Strings.AlreadyLoading", "Strings.WarnWrongLaserMode", "Strings.WarnWrongLaserModeTitle", "LicenseForm.richTextBox1.Text", "LicenseForm.richTextBox2.Text"].map(String::from));
        let missing: Vec<&String> = keys.iter().filter(|k| tr(k) == k.as_str()).collect();
        assert!(missing.is_empty(), "missing keys: {missing:?}");
    }

    /// Icons used by the forms exist.
    #[test]
    fn icon_keys_exist() {
        let i = icons::Icons::new(false);
        for k in ["mdi-close", "mdi-close-box", "mdi-checkbox-marked", "mdi-information-slab-box", "mdi-content-save", "mdi-heart", "mdi-database"] {
            assert!(i.contains(k), "{k}");
        }
    }

    /// Every form renders a few frames headless without panicking.
    #[test]
    fn all_forms_render() {
        let ctx = egui::Context::default();
        let mut tab = FormsTab::default();
        tab.svg_mode = Some(SvgModeForm::new(DEMO_SVG));
        tab.cutting = Some(CuttingTest::new(tab.file.clone()));
        tab.power = Some(PowerVsSpeedForm::new(tab.file.clone()));
        tab.shake = Some(ShakeTest::new(tab.file.clone()));
        tab.save = Some(SaveOptionForm::new());
        tab.license = Some(LicenseForm::new());
        tab.legal = Some(LegalDisclaimer::new());
        // one form at a time: modals stack and only the top one takes input
        for _ in 0..3 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                tab.ui(ui);
                tab.dialogs(&ui.ctx().clone());
            });
            out.textures_delta.clear();
        }
    }
}
