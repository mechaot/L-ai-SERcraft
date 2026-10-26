// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `Generator/PowerVsSpeedForm.cs` (+ Designer): power vs speed (greyscale) test generator form.

use eframe::egui::{self, Grid, TextureHandle, Ui};
use lg_gcode::{GResult, GrblFile};

use super::{GenFlow, GeneratorOutcome};
use crate::forms::form_util::*;
use crate::ui::widgets::{enum_combo, grbl_group_box, DecimalInputRanged, NumericUpDown, PlaceholderTextBox};
use crate::ui::{tr, tr_plain};

/// i18n prefix of the form (sub-folder form).
const P: &str = "Generator.PowerVsSpeedForm.";

/// Arguments of `GrblFile::generate_greyscale_test` (C# `GenerateGreyscaleTest`).
/// NOTE(upstream): `f_grid`/`s_grid` repeat `f_text`/`s_text` in the C# call; kept.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerVsSpeedArgs {
    pub f_row: i32,
    pub s_col: i32,
    pub f_start: i32,
    pub f_end: i32,
    pub s_start: i32,
    pub s_end: i32,
    pub x_size: i32,
    pub y_size: i32,
    pub resolution: f64,
    pub f_grid: i32,
    pub s_grid: i32,
    pub title: String,
    pub f_text: i32,
    pub s_text: i32,
    pub ton: String,
}

impl PowerVsSpeedArgs {
    /// Call the generator.
    ///
    /// `f`: target program. Returns generator result (`Ok(false)` = busy).
    pub fn run(&self, f: &mut GrblFile) -> GResult<bool> {
        f.generate_greyscale_test(
            self.f_row, self.s_col, self.f_start, self.f_end, self.s_start, self.s_end, self.x_size, self.y_size, self.resolution, self.f_grid, self.s_grid, &self.title, self.f_text, self.s_text, &self.ton,
        )
    }
}

/// Which input raised a change event (C# `sender`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    SMin,
    SMax,
    SSteps,
    FMin,
    FMax,
    FSteps,
}

/// Power vs speed generator form state.
pub struct PowerVsSpeedForm {
    file: SharedFile,
    limits: Limits,
    laser_on: usize,
    s_min: i32,
    s_max: i32,
    s_steps: i32,
    s_step_size: i32,
    s_min_enabled: bool,
    f_min: i32,
    f_max: i32,
    f_steps: i32,
    f_step_size: i32,
    f_min_enabled: bool,
    quality: f64,
    f_text: i32,
    s_text: i32,
    size_w: f32,
    size_h: f32,
    title: String,
    tex: Option<TextureHandle>,
    flow: GenFlow,
}

impl PowerVsSpeedForm {
    /// `new PowerVsSpeedForm(core)` + the Load event (`AssignMinMaxLimit`, `RestoreDefault`).
    ///
    /// `file`: shared program the Create button replaces (C# `core.LoadedFile`).
    pub fn new(file: SharedFile) -> Self {
        let mut f = PowerVsSpeedForm {
            file,
            limits: Limits::from_config(),
            // M4 is the default selection here
            laser_on: 1,
            s_min: 0,
            s_max: 0,
            s_steps: 11,
            s_step_size: 0,
            s_min_enabled: true,
            f_min: 0,
            f_max: 0,
            f_steps: 11,
            f_step_size: 0,
            f_min_enabled: true,
            quality: 3.0,
            f_text: 0,
            s_text: 0,
            size_w: 100.0,
            size_h: 100.0,
            title: String::new(),
            tex: None,
            flow: GenFlow::default(),
        };
        f.restore_default();
        f
    }

    /// C# `RestoreDefault`.
    pub fn restore_default(&mut self) {
        self.s_min = 100;
        self.s_max = 1000;
        self.f_min = 1000;
        self.f_max = 4000;
        self.quality = 8.0;
        self.s_steps = 10;
        self.f_steps = 7;
        self.f_text = 1000;
        self.s_text = 500;
        // recompute sizes
        self.on_s_changed(None, true);
        self.on_f_changed(None, true);
    }

    /// Current form values as generator arguments (C# `BtnCreate_Click` mapping).
    ///
    /// Returns the argument set.
    pub fn args(&self) -> PowerVsSpeedArgs {
        PowerVsSpeedArgs {
            f_row: self.f_steps,
            s_col: self.s_steps,
            f_start: self.f_min,
            f_end: self.f_max,
            s_start: self.s_min,
            s_end: self.s_max,
            // (int) cast of the float size boxes
            x_size: self.size_w as i32,
            y_size: self.size_h as i32,
            resolution: self.quality,
            f_grid: self.f_text,
            s_grid: self.s_text,
            title: self.title.clone(),
            f_text: self.f_text,
            s_text: self.s_text,
            ton: LASER_OPTIONS[self.laser_on].1.to_owned(),
        }
    }

    /// C# `IiS_CurrentValueChanged`.
    ///
    /// `sender`: input that changed (None = C# `null`). `by_user`: user commit.
    fn on_s_changed(&mut self, sender: Option<Field>, by_user: bool) {
        if by_user {
            // recompute step size
            if self.s_steps == 1 {
                self.s_min_enabled = false;
                self.s_min = self.s_max;
                self.s_step_size = 0;
            } else {
                if !self.s_min_enabled {
                    self.s_min = 0;
                    self.s_min_enabled = true;
                }
                self.s_step_size = (self.s_max - self.s_min) / (self.s_steps - 1);
            }
        }
        if sender == Some(Field::SSteps) || sender.is_none() {
            self.size_w = (self.s_steps * 10) as f32;
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
            self.size_h = (self.f_steps * 10) as f32;
        }
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome once the form closed.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<GeneratorOutcome> {
        if self.tex.is_none() {
            self.tex = load_texture(ctx, "powervsspeed", include_bytes!("../../../../resources/grafica/powervsspeed.png"));
        }
        // message box of a finished create blocks the form until dismissed
        if let Some(r) = self.flow.poll(ctx) {
            return r;
        }
        let title = tr_plain(&format!("{P}$this.Text"));
        match run_modal(ctx, "power-vs-speed", &title, 790.0, true, |ui| self.ui(ui))? {
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
        Grid::new("pvs-params").num_columns(7).spacing([6.0, 6.0]).show(ui, |ui| {
            // row 0: power (S)
            label(ui, P, "label2");
            label(ui, P, "label3");
            let en = self.s_min_enabled;
            ui.add_enabled_ui(en, |ui| {
                if int_input(ui, "pvs-smin", &mut self.s_min, 0, lim.max_pwm, false, 49.0).is_some() {
                    self.on_s_changed(Some(Field::SMin), true);
                }
            });
            label(ui, P, "label4");
            if int_input(ui, "pvs-smax", &mut self.s_max, 0, lim.max_pwm, false, 49.0).is_some() {
                self.on_s_changed(Some(Field::SMax), true);
            }
            label(ui, P, "label5");
            // steps and step size share the last column (title box below uses the same width)
            ui.horizontal(|ui| {
                if int_input(ui, "pvs-ssteps", &mut self.s_steps, 1, 20, true, 28.0).is_some() {
                    self.on_s_changed(Some(Field::SSteps), true);
                }
                label(ui, P, "label6");
                ui.add_enabled_ui(false, |ui| {
                    int_input(ui, "pvs-sstepsize", &mut self.s_step_size, 0, 1000, false, 49.0);
                });
            });
            ui.end_row();

            // row 1: speed (F)
            label(ui, P, "label7");
            label(ui, P, "label8");
            let en = self.f_min_enabled;
            ui.add_enabled_ui(en, |ui| {
                if int_input(ui, "pvs-fmin", &mut self.f_min, 0, lim.max_rate_x, false, 49.0).is_some() {
                    self.on_f_changed(Some(Field::FMin), true);
                }
            });
            label(ui, P, "label9");
            if int_input(ui, "pvs-fmax", &mut self.f_max, 0, lim.max_rate_x, false, 49.0).is_some() {
                self.on_f_changed(Some(Field::FMax), true);
            }
            label(ui, P, "label10");
            // steps and step size share the last column (title box below uses the same width)
            ui.horizontal(|ui| {
                if int_input(ui, "pvs-fsteps", &mut self.f_steps, 1, 20, true, 28.0).is_some() {
                    self.on_f_changed(Some(Field::FSteps), true);
                }
                label(ui, P, "label11");
                ui.add_enabled_ui(false, |ui| {
                    int_input(ui, "pvs-fstepsize", &mut self.f_step_size, 0, 1000, false, 49.0);
                });
            });
            ui.end_row();

            // row 3: laser mode
            label(ui, P, "LblLaserMode");
            ui.label("");
            let items: Vec<usize> = vec![0, 1];
            let mut sel = self.laser_on;
            enum_combo(ui, "pvs-laser", &mut sel, &items, |i| LASER_OPTIONS[*i].0.to_owned(), false, 141.0);
            self.laser_on = sel;
            ui.label("");
            if info_button(ui, "") {
                open_link("https://lasergrbl.com/usage/raster-image-import/target-image-size-and-laser-options/#laser-modes");
            }
            ui.end_row();

            // row 4: quality
            label(ui, P, "label13");
            ui.label("");
            ui.add(NumericUpDown::new(&mut self.quality).range(1.0, 50.0).increment(1.0).decimal_places(3).width(55.0).id(egui::Id::unique("pvs-quality")));
            label(ui, P, "label14");
            ui.end_row();

            // row 6: text
            label(ui, P, "label12");
            label(ui, P, "label18");
            int_input(ui, "pvs-ftext", &mut self.f_text, 0, lim.max_rate_x, false, 49.0);
            label(ui, P, "label19");
            int_input(ui, "pvs-stext", &mut self.s_text, 0, lim.max_pwm, false, 49.0);
            ui.label("");
            ui.add(PlaceholderTextBox::new(&mut self.title, tr(&format!("{P}PhTbLabel.WaterMark"))).width(137.0).id(egui::Id::unique("pvs-title")));
            ui.end_row();

            // row 8: size (read only)
            label(ui, P, "label15");
            label(ui, P, "label16");
            ui.add_enabled(false, DecimalInputRanged::new(&mut self.size_w.clone()).range(0.0, 1000.0).decimals(1).width(49.0).id(egui::Id::unique("pvs-sizew")));
            label(ui, P, "label17");
            ui.add_enabled(false, DecimalInputRanged::new(&mut self.size_h.clone()).range(0.0, 1000.0).decimals(1).width(49.0).id(egui::Id::unique("pvs-sizeh")));
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
    fn defaults() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let f = PowerVsSpeedForm::new(Arc::new(RwLock::new(GrblFile::new())));
        assert_eq!((f.s_steps, f.s_min, f.s_max, f.s_step_size), (10, 100, 1000, 100));
        assert_eq!((f.f_steps, f.f_min, f.f_max, f.f_step_size), (7, 1000, 4000, 500));
        assert_eq!((f.size_w, f.size_h), (100.0, 70.0));
        assert_eq!(f.laser_on, 1);
        assert_eq!(f.quality, 8.0);
    }

    #[test]
    fn same_gcode_as_direct_call() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let form = PowerVsSpeedForm::new(Arc::new(RwLock::new(GrblFile::new())));
        let mut a = GrblFile::new();
        assert!(form.args().run(&mut a).unwrap());
        let mut b = GrblFile::new();
        // note: Ftext/Stext are passed twice (grid + text), exactly like C#
        assert!(b.generate_greyscale_test(7, 10, 1000, 4000, 100, 1000, 100, 70, 8.0, 1000, 500, "", 1000, 500, "M4").unwrap());
        assert!(a.count() > 100);
        assert_eq!(texts(&a), texts(&b));
    }

    #[test]
    fn single_step_disables_min() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let mut f = PowerVsSpeedForm::new(Arc::new(RwLock::new(GrblFile::new())));
        f.s_steps = 1;
        f.on_s_changed(Some(Field::SSteps), true);
        assert!(!f.s_min_enabled);
        assert_eq!((f.s_min, f.s_step_size, f.size_w), (1000, 0, 10.0));
    }
}
