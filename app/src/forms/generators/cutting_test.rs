// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `Generator/CuttingTest.cs` (+ Designer): cutting test generator form.

use eframe::egui::{self, Grid, TextureHandle, Ui};
use lg_gcode::{GResult, GrblFile};

use super::{GenFlow, GeneratorOutcome};
use crate::forms::form_util::*;
use crate::ui::widgets::{enum_combo, grbl_group_box, PlaceholderTextBox};
use crate::ui::{tr, tr_plain};

/// i18n prefix of the form (sub-folder form).
const P: &str = "Generator.CuttingTest.";

/// Arguments of `GrblFile::generate_cutting_test` (C# `GenerateCuttingTest`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuttingTestArgs {
    pub f_col: i32,
    pub f_start: i32,
    pub f_end: i32,
    pub p_start: i32,
    pub p_end: i32,
    pub s_fixed: i32,
    pub f_text: i32,
    pub s_text: i32,
    pub title: String,
    pub ton: String,
}

impl CuttingTestArgs {
    /// Call the generator.
    ///
    /// `f`: target program. Returns generator result (`Ok(false)` = busy).
    pub fn run(&self, f: &mut GrblFile) -> GResult<bool> {
        f.generate_cutting_test(self.f_col, self.f_start, self.f_end, self.p_start, self.p_end, self.s_fixed, self.f_text, self.s_text, &self.title, &self.ton)
    }
}

/// Which input raised a change event (C# `sender`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    PMin,
    PMax,
    FMin,
    FMax,
    FSteps,
}

/// Cutting test generator form state.
pub struct CuttingTest {
    file: SharedFile,
    limits: Limits,
    laser_on: usize,
    s_fixed: i32,
    p_min: i32,
    p_max: i32,
    f_min: i32,
    f_max: i32,
    f_steps: i32,
    f_step_size: i32,
    f_text: i32,
    s_text: i32,
    size_w: f32,
    size_h: f32,
    f_min_enabled: bool,
    title: String,
    tex: Option<TextureHandle>,
    flow: GenFlow,
}

impl CuttingTest {
    /// `new CuttingTest(core)` + the Load event (`AssignMinMaxLimit`, `RestoreDefault`).
    ///
    /// `file`: shared program the Create button replaces (C# `core.LoadedFile`).
    pub fn new(file: SharedFile) -> Self {
        let mut f = CuttingTest {
            file,
            limits: Limits::from_config(),
            laser_on: 0,
            s_fixed: 0,
            p_min: 0,
            p_max: 0,
            f_min: 0,
            f_max: 0,
            f_steps: 11,
            f_step_size: 0,
            f_text: 0,
            s_text: 0,
            size_w: 100.0,
            size_h: 100.0,
            f_min_enabled: true,
            title: String::new(),
            tex: None,
            flow: GenFlow::default(),
        };
        f.restore_default();
        f
    }

    /// C# `RestoreDefault`.
    pub fn restore_default(&mut self) {
        self.p_min = 1;
        self.p_max = 4;
        self.s_fixed = 1000;
        self.f_min = 200;
        self.f_max = 1000;
        self.f_steps = 5;
        self.f_text = 1000;
        self.s_text = 500;
        // recompute sizes
        self.on_f_changed(None, true);
    }

    /// Current form values as generator arguments (C# `BtnCreate_Click` mapping).
    ///
    /// Returns the argument set.
    pub fn args(&self) -> CuttingTestArgs {
        CuttingTestArgs {
            f_col: self.f_steps,
            f_start: self.f_min,
            f_end: self.f_max,
            p_start: self.p_min,
            p_end: self.p_max,
            s_fixed: self.s_fixed,
            f_text: self.f_text,
            s_text: self.s_text,
            title: self.title.clone(),
            ton: LASER_OPTIONS[self.laser_on].1.to_owned(),
        }
    }

    /// C# `Ii_PCurrentValueChanged`.
    ///
    /// `sender`: input that changed. `by_user`: user commit.
    fn on_p_changed(&mut self, sender: Field, by_user: bool) {
        if by_user {
            if sender == Field::PMin && self.p_min > self.p_max {
                self.p_max = self.p_min;
            }
            if sender == Field::PMax && self.p_max < self.p_min {
                self.p_min = self.p_max;
            }
            self.compute_size();
        }
    }

    /// C# `IiF_CurrentValueChanged`.
    ///
    /// `sender`: input that changed (None = C# `null`). `by_user`: user commit.
    fn on_f_changed(&mut self, sender: Option<Field>, by_user: bool) {
        if by_user {
            // recompute step size
            if self.f_steps == 1 {
                self.f_min_enabled = false;
                self.f_min = self.f_max;
                self.f_step_size = 0;
            } else {
                if !self.f_min_enabled {
                    self.f_min = 0;
                    self.f_min_enabled = true;
                }
                self.f_step_size = (self.f_max - self.f_min) / (self.f_steps - 1);
            }
        }
        if sender == Some(Field::FSteps) || sender.is_none() {
            self.compute_size();
        }
    }

    /// C# `ComputeSize`.
    fn compute_size(&mut self) {
        self.size_w = (self.f_steps * 14 - 4 + 3) as f32;
        self.size_h = ((self.p_max - self.p_min + 1) * 14 - 4 + 3 + 3 + 3) as f32;
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome once the form closed.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<GeneratorOutcome> {
        if self.tex.is_none() {
            self.tex = load_texture(ctx, "cuttingtest", include_bytes!("../../../../resources/grafica/cuttingtest.png"));
        }
        // message box of a finished create blocks the form until dismissed
        if let Some(r) = self.flow.poll(ctx) {
            return r;
        }
        let title = tr_plain(&format!("{P}$this.Text"));
        match run_modal(ctx, "cutting-test", &title, 790.0, true, |ui| self.ui(ui))? {
            ModalEvent::Done(true) => {
                // C# BtnCreate_Click
                let args = self.args();
                self.flow.run(&self.file, |f| args.run(f));
                ctx.request_repaint();
                None
            }
            ModalEvent::Done(false) | ModalEvent::Closed => Some(GeneratorOutcome::Cancelled),
        }
    }

    /// Form body. Returns `Some(true)` on Create, `Some(false)` on Cancel.
    fn ui(&mut self, ui: &mut Ui) -> Option<bool> {
        let mut result = None;
        ui.horizontal_top(|ui| {
            native_image(ui, self.tex.as_ref());
            ui.vertical(|ui| {
                ui.set_width(500.0);
                // intro label centered like the designer (Anchor None)
                ui.vertical_centered(|ui| ui.label(tr(&format!("{P}label1.Text"))));
                ui.add_space(4.0);
                grbl_group_box(ui, &ctl_text(P, "GbParameters"), |ui| self.parameters(ui));
                ui.add_space(4.0);
                result = ok_cancel_row(ui, P, "BtnCreate", Some("BtnCancel"));
            });
        });
        result
    }

    /// Parameters group box (tableLayoutPanel3).
    fn parameters(&mut self, ui: &mut Ui) {
        let lim = self.limits;
        let from = ctl_text(P, "label3");
        let to = ctl_text(P, "label4");
        Grid::new("cut-params").num_columns(7).spacing([6.0, 6.0]).show(ui, |ui| {
            // row 0: power (S)
            label(ui, P, "label13");
            ui.label("");
            int_input(ui, "cut-sfixed", &mut self.s_fixed, 1, 1000, false, 49.0);
            ui.end_row();

            // row 1: passes
            label(ui, P, "label2");
            ui.label(&from);
            if let Some(_c) = int_input(ui, "cut-pmin", &mut self.p_min, 1, 20, true, 49.0) {
                self.on_p_changed(Field::PMin, true);
            }
            ui.label(&to);
            if let Some(_c) = int_input(ui, "cut-pmax", &mut self.p_max, 1, 20, true, 49.0) {
                self.on_p_changed(Field::PMax, true);
            }
            ui.end_row();

            // row 2: speed (F)
            label(ui, P, "label7");
            label(ui, P, "label8");
            let en = self.f_min_enabled;
            ui.add_enabled_ui(en, |ui| {
                if int_input(ui, "cut-fmin", &mut self.f_min, 0, lim.max_rate_x, false, 49.0).is_some() {
                    self.on_f_changed(Some(Field::FMin), true);
                }
            });
            label(ui, P, "label9");
            if int_input(ui, "cut-fmax", &mut self.f_max, 0, lim.max_rate_x, false, 49.0).is_some() {
                self.on_f_changed(Some(Field::FMax), true);
            }
            label(ui, P, "label10");
            // steps and step size share the last column (title box below uses the same width)
            ui.horizontal(|ui| {
                if int_input(ui, "cut-fsteps", &mut self.f_steps, 1, 20, true, 28.0).is_some() {
                    self.on_f_changed(Some(Field::FSteps), true);
                }
                label(ui, P, "label11");
                ui.add_enabled_ui(false, |ui| {
                    int_input(ui, "cut-fstepsize", &mut self.f_step_size, 0, 1000, false, 49.0);
                });
            });
            ui.end_row();

            // row 4: laser mode
            label(ui, P, "LblLaserMode");
            ui.label("");
            let items: Vec<usize> = vec![0, 1];
            let mut sel = self.laser_on;
            enum_combo(ui, "cut-laser", &mut sel, &items, |i| LASER_OPTIONS[*i].0.to_owned(), false, 141.0);
            self.laser_on = sel;
            ui.label("");
            if info_button(ui, "") {
                open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#laser-modes");
            }
            ui.end_row();

            // row 6: text
            label(ui, P, "label12");
            label(ui, P, "label18");
            int_input(ui, "cut-ftext", &mut self.f_text, 0, lim.max_rate_x, false, 49.0);
            label(ui, P, "label19");
            int_input(ui, "cut-stext", &mut self.s_text, 0, lim.max_pwm, false, 49.0);
            ui.label("");
            ui.add(PlaceholderTextBox::new(&mut self.title, tr(&format!("{P}PhTbLabel.WaterMark"))).width(137.0).id(egui::Id::unique("cut-title")));
            ui.end_row();

            // row 8: size (read only)
            label(ui, P, "label15");
            label(ui, P, "label16");
            ui.add_enabled(false, crate::ui::widgets::DecimalInputRanged::new(&mut self.size_w.clone()).range(0.0, 1000.0).decimals(1).width(49.0).id(egui::Id::unique("cut-sizew")));
            label(ui, P, "label17");
            ui.add_enabled(false, crate::ui::widgets::DecimalInputRanged::new(&mut self.size_h.clone()).range(0.0, 1000.0).decimals(1).width(49.0).id(egui::Id::unique("cut-sizeh")));
            ui.end_row();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    fn texts(f: &GrblFile) -> Vec<String> {
        f.commands().iter().map(|c| c.command().to_owned()).collect()
    }

    #[test]
    fn defaults_and_sizes() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let f = CuttingTest::new(Arc::new(RwLock::new(GrblFile::new())));
        assert_eq!((f.f_steps, f.f_min, f.f_max, f.f_step_size), (5, 200, 1000, 200));
        assert_eq!((f.size_w, f.size_h), (69.0, 61.0));
        assert_eq!(f.laser_on, 0);
    }

    #[test]
    fn same_gcode_as_direct_call() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let form = CuttingTest::new(Arc::new(RwLock::new(GrblFile::new())));
        let mut a = GrblFile::new();
        assert!(form.args().run(&mut a).unwrap());
        let mut b = GrblFile::new();
        assert!(b.generate_cutting_test(5, 200, 1000, 1, 4, 1000, 1000, 500, "", "M3").unwrap());
        assert!(a.count() > 10);
        assert_eq!(texts(&a), texts(&b));
    }

    #[test]
    fn single_step_disables_fmin() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let mut f = CuttingTest::new(Arc::new(RwLock::new(GrblFile::new())));
        f.f_steps = 1;
        f.on_f_changed(Some(Field::FSteps), true);
        assert!(!f.f_min_enabled);
        assert_eq!((f.f_min, f.f_step_size), (1000, 0));
        f.f_steps = 3;
        f.on_f_changed(Some(Field::FSteps), true);
        assert!(f.f_min_enabled);
        assert_eq!((f.f_min, f.f_step_size), (0, 500));
    }

    #[test]
    fn passes_stay_ordered() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let mut f = CuttingTest::new(Arc::new(RwLock::new(GrblFile::new())));
        f.p_min = 6;
        f.on_p_changed(Field::PMin, true);
        assert_eq!(f.p_max, 6);
        f.p_max = 2;
        f.on_p_changed(Field::PMax, true);
        assert_eq!(f.p_min, 2);
        // one pass: 1 * 14 - 4 + 9
        assert_eq!(f.size_h, 19.0);
    }
}
