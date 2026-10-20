#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `Generator/ShakeTest.cs` (+ Designer): shake test generator form.

use eframe::egui::{self, Grid, TextureHandle, Ui};
use lg_gcode::{GResult, GrblFile};

use super::{GenFlow, GeneratorOutcome};
use crate::forms::form_util::*;
use crate::ui::widgets::flat_combo;
use crate::ui::{tr, tr_plain};

/// i18n prefix of the form (sub-folder form).
const P: &str = "Generator.ShakeTest.";

/// Arguments of `GrblFile::generate_shake_test` (C# `GenerateShakeTest`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShakeTestArgs {
    pub axis: String,
    pub flimit: i32,
    pub axislen: i32,
    pub cpower: i32,
    pub cspeed: i32,
}

impl ShakeTestArgs {
    /// Call the generator.
    ///
    /// `f`: target program. Returns generator result (`Ok(false)` = busy).
    pub fn run(&self, f: &mut GrblFile) -> GResult<bool> {
        f.generate_shake_test(&self.axis, self.flimit, self.axislen, self.cpower, self.cspeed)
    }
}

/// Shake test generator form state.
pub struct ShakeTest {
    file: SharedFile,
    limits: Limits,
    /// 0 = "X", 1 = "Y" (`CbAxis`).
    axis: Option<usize>,
    /// `CbLimit` items (ints) and selected index.
    limit_items: Vec<i32>,
    limit_sel: Option<usize>,
    axis_len: i32,
    axis_len_max: i32,
    cross_speed: i32,
    cross_power: i32,
    tex: Option<TextureHandle>,
    flow: GenFlow,
}

impl ShakeTest {
    /// `new ShakeTest(core)` + the Load event (`AssignMinMaxLimit`, `RestoreDefault`).
    ///
    /// `file`: shared program the Create button replaces (C# `core.LoadedFile`).
    pub fn new(file: SharedFile) -> Self {
        let mut f = ShakeTest {
            file,
            limits: Limits::from_config(),
            axis: None,
            limit_items: Vec::new(),
            limit_sel: None,
            axis_len: 100,
            axis_len_max: 400,
            cross_speed: 1000,
            cross_power: 100,
            tex: None,
            flow: GenFlow::default(),
        };
        f.restore_default();
        f
    }

    /// C# `RestoreDefault` (`SelectedIndex = 0` fires `CbAxis_SelectedIndexChanged`).
    pub fn restore_default(&mut self) {
        self.cross_speed = 1000;
        self.cross_power = 100;
        self.axis = Some(0);
        self.on_axis_changed();
    }

    /// Axis name of the current selection.
    fn axis_name(&self) -> &'static str {
        if self.axis == Some(1) { "Y" } else { "X" }
    }

    /// Current form values as generator arguments (C# `BtnCreate_Click` mapping).
    ///
    /// Returns the argument set.
    pub fn args(&self) -> ShakeTestArgs {
        ShakeTestArgs {
            axis: self.axis_name().to_owned(),
            flimit: self.limit_sel.and_then(|i| self.limit_items.get(i)).copied().unwrap_or(0),
            axislen: self.axis_len,
            cpower: self.cross_power,
            cspeed: self.cross_speed,
        }
    }

    /// C# `CbAxis_SelectedIndexChanged`: refresh axis length and speed-limit list.
    fn on_axis_changed(&mut self) {
        let is_x = self.axis_name() == "X";
        let maxrate_a = if is_x { self.limits.max_rate_x } else { self.limits.max_rate_y };
        let maxlen_a = if is_x { self.limits.table_width } else { self.limits.table_height };
        self.axis_len_max = maxlen_a;
        self.axis_len = maxlen_a;

        // NOTE(upstream): C# keeps the old selection only if `SelectedItem as string` matches, items are ints so
        // that never happens and the selection always resets to index 0.
        self.limit_items.clear();
        self.limit_items.push(maxrate_a);
        let mut i = maxrate_a / 1000 * 1000;
        while i > 0 {
            if !self.limit_items.contains(&i) {
                self.limit_items.push(i);
            }
            i -= 1000;
        }
        self.limit_sel = Some(0);
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome once the form closed.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<GeneratorOutcome> {
        if self.tex.is_none() {
            self.tex = load_texture(ctx, "shaketest", include_bytes!("../../../../resources/grafica/shaketest.jpg"));
        }
        // message box of a finished create blocks the form until dismissed
        if let Some(r) = self.flow.poll(ctx) {
            return r;
        }
        let title = tr_plain(&format!("{P}$this.Text"));
        match run_modal(ctx, "shake-test", &title, 905.0, true, |ui| self.ui(ui))? {
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
                ui.set_width(600.0);
                ui.add(egui::Label::new(tr(&format!("{P}label1.Text"))).wrap());
                ui.add_space(6.0);
                self.parameters(ui);
                ui.add_space(6.0);
                result = ok_cancel_row(ui, P, "BtnCreate", Some("BtnCancel"));
            });
        });
        result
    }

    /// Parameter table (tableLayoutPanel6).
    fn parameters(&mut self, ui: &mut Ui) {
        let lim = self.limits;
        let axis_items: Vec<String> = vec![tr(&format!("{P}CbAxis.Items")).to_owned(), tr(&format!("{P}CbAxis.Items1")).to_owned()];
        let limit_texts: Vec<String> = self.limit_items.iter().map(|v| v.to_string()).collect();
        Grid::new("shake-params").num_columns(7).spacing([6.0, 6.0]).show(ui, |ui| {
            // row 0: test type
            label(ui, P, "label2");
            label(ui, P, "label7");
            let before = self.axis;
            flat_combo(ui, "shake-axis", &mut self.axis, &axis_items, 60.0);
            if self.axis != before {
                self.on_axis_changed();
            }
            label(ui, P, "label8");
            let max = self.axis_len_max;
            int_input(ui, "shake-axislen", &mut self.axis_len, 50, max, false, 60.0);
            label(ui, P, "label4");
            flat_combo(ui, "shake-limit", &mut self.limit_sel, &limit_texts, 146.0);
            ui.end_row();

            // row 1: cross engraving
            label(ui, P, "label3");
            label(ui, P, "label5");
            int_input(ui, "shake-speed", &mut self.cross_speed, 10, lim.max_rate_x.min(lim.max_rate_y), false, 60.0);
            label(ui, P, "label6");
            int_input(ui, "shake-power", &mut self.cross_power, 1, lim.max_pwm, false, 60.0);
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
        let f = ShakeTest::new(Arc::new(RwLock::new(GrblFile::new())));
        assert_eq!(f.limit_items, vec![4000, 3000, 2000, 1000]);
        assert_eq!((f.axis_len, f.cross_speed, f.cross_power), (300, 1000, 100));
        assert_eq!(f.args(), ShakeTestArgs { axis: "X".into(), flimit: 4000, axislen: 300, cpower: 100, cspeed: 1000 });
    }

    #[test]
    fn y_axis_uses_table_height() {
        let mut f = ShakeTest::new(Arc::new(RwLock::new(GrblFile::new())));
        f.axis = Some(1);
        f.on_axis_changed();
        assert_eq!(f.args().axislen, 200);
        assert_eq!(f.args().axis, "Y");
    }

    #[test]
    fn same_gcode_as_direct_call() {
        let form = ShakeTest::new(Arc::new(RwLock::new(GrblFile::new())));
        let mut a = GrblFile::new();
        assert!(form.args().run(&mut a).unwrap());
        let mut b = GrblFile::new();
        assert!(b.generate_shake_test("X", 4000, 300, 100, 1000).unwrap());
        assert!(a.count() > 10);
        assert_eq!(texts(&a), texts(&b));
    }
}
