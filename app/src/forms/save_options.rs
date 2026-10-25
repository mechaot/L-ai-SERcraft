// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `SaveOptionForm.cs` (+ Designer): "Save G-code options" dialog.

use eframe::egui::{self, Grid, Ui, Vec2};

use crate::forms::form_util::*;
use crate::ui::widgets::{grbl_group_box, GrblButton, NumericUpDown};
use crate::ui::{tr, tr_plain};

/// i18n prefix of the form (root form).
const P: &str = "SaveOptionForm.";

/// Choices handed to `GrblCore.SaveProgram(parent, header, footer, between, cycles, lf)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveOptions {
    /// `CBHeader.Checked`.
    pub header: bool,
    /// `CBFooter.Checked`.
    pub footer: bool,
    /// `CBBetween.Checked`.
    pub between: bool,
    /// `(int)UDCount.Value`.
    pub cycles: i32,
    /// `CBLFLineEndings.Checked`.
    pub use_lf_line_endings: bool,
}

/// How the form ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOptionOutcome {
    /// Save pressed (`DialogResult.OK`): the caller saves the program with these options.
    Save(SaveOptions),
    /// Esc / close box.
    Cancel,
}

/// Save options dialog state.
pub struct SaveOptionForm {
    header: bool,
    footer: bool,
    between: bool,
    count: f64,
    lf: bool,
}

impl Default for SaveOptionForm {
    /// Designer defaults: header/footer/between checked, 1 pass, LF line endings off.
    fn default() -> Self {
        SaveOptionForm { header: true, footer: true, between: true, count: 1.0, lf: false }
    }
}

impl SaveOptionForm {
    /// `new SaveOptionForm()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current choices as [`SaveOptions`].
    ///
    /// Returns the option set.
    pub fn options(&self) -> SaveOptions {
        SaveOptions { header: self.header, footer: self.footer, between: self.between, cycles: self.count as i32, use_lf_line_endings: self.lf }
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome once the form closed.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<SaveOptionOutcome> {
        let title = tr_plain(&format!("{P}$this.Text"));
        match run_modal(ctx, "save-options", &title, 473.0, true, |ui| self.ui(ui))? {
            ModalEvent::Done(()) => Some(SaveOptionOutcome::Save(self.options())),
            ModalEvent::Closed => Some(SaveOptionOutcome::Cancel),
        }
    }

    /// Form body. Returns `Some(())` when Save was pressed.
    fn ui(&mut self, ui: &mut Ui) -> Option<()> {
        grbl_group_box(ui, &ctl_text(P, "groupBox1"), |ui| {
            ui.set_width(461.0);
            Grid::new("save-opts").num_columns(3).spacing([8.0, 8.0]).show(ui, |ui| {
                ui.checkbox(&mut self.header, ctl_text(P, "CBHeader"));
                ui.end_row();
                label(ui, P, "label1");
                ui.add(NumericUpDown::new(&mut self.count).range(1.0, 100.0).increment(1.0).decimal_places(0).width(81.0).id(egui::Id::unique("save-count")));
                ui.checkbox(&mut self.between, ctl_text(P, "CBBetween"));
                ui.end_row();
                ui.checkbox(&mut self.footer, ctl_text(P, "CBFooter"));
                ui.end_row();
                ui.checkbox(&mut self.lf, ctl_text(P, "CBLFLineEndings"));
                ui.end_row();
            });
        });
        ui.add_space(6.0);
        ui.add(egui::Label::new(tr(&format!("{P}label2.Text"))).wrap());
        ui.add_space(6.0);
        let mut out = None;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let b = GrblButton::new(ctl_text(P, "BtnSave")).icon("mdi-content-save").min_size(Vec2::new(143.0, 52.0));
            if ui.add(b).clicked() {
                out = Some(());
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_designer() {
        let o = SaveOptionForm::new().options();
        assert_eq!(o, SaveOptions { header: true, footer: true, between: true, cycles: 1, use_lf_line_endings: false });
    }

    #[test]
    fn options_follow_state() {
        let mut f = SaveOptionForm::new();
        f.count = 3.0;
        f.lf = true;
        f.between = false;
        let o = f.options();
        assert_eq!((o.cycles, o.use_lf_line_endings, o.between), (3, true, false));
    }
}
