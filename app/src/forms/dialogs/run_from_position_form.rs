// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `RunFromPositionForm.cs` + `RunFromPositionForm.Designer.cs`: "run from line N" dialog.
//!
//! Data comes from `GrblCore::run_from_position_info()`; on OK the caller runs
//! `GrblCore::continue_program(position, homing, false)`.

use eframe::egui::{self, Color32, Ui, Vec2};
use lasergrbl_core::RunFromPositionInfo;

use super::common::{ModalEvent, button_en, modal, trf};
use crate::ui::icons::{Tint, icons};
use crate::ui::tr;
use crate::ui::widgets::NumericUpDown;

/// Result of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunFromPositionOutcome {
    /// Cancel / close box (C# returned -1).
    Cancelled,
    /// OK. `position`: 0-based first line. `homing`: "Do Homing" ticked.
    Ok { position: i32, homing: bool },
}

/// The dialog.
pub struct RunFromPositionForm {
    total: i32,
    allow_homing: bool,
    line: f64,
    homing: bool,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl RunFromPositionForm {
    /// Create the dialog (C# private ctor).
    ///
    /// `info`: line count and homing flag from the core.
    pub fn open(info: &RunFromPositionInfo) -> Self {
        Self::new(info.line_count as i32, info.homing_enabled)
    }

    /// Same with plain values (`total`, `allowHoming`).
    ///
    /// `total`: number of lines of the loaded file. `allow_homing`: show the homing checkbox.
    pub fn new(total: i32, allow_homing: bool) -> Self {
        RunFromPositionForm { total, allow_homing, line: 1.0, homing: false, id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    /// `Position` getter: spinner value - 1.
    pub fn position(&self) -> i32 {
        self.line as i32 - 1
    }

    /// Spinner maximum: `total + 1`.
    pub fn max_line(&self) -> i32 {
        self.total + 1
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<RunFromPositionOutcome> {
        let title = trf("RunFromPositionForm.$this.Text", "Run From Position");
        let id = ("runfrompos", self.id);
        match modal(ctx, id, &title, 520.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) => Some(RunFromPositionOutcome::Ok { position: self.position(), homing: self.homing && self.allow_homing }),
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) => Some(RunFromPositionOutcome::Cancelled),
            None => None,
        }
    }

    /// Form body. Returns Some(true)=OK, Some(false)=cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        let t = |c: &str| tr(&format!("RunFromPositionForm.{c}.Text")).to_owned();
        ui.horizontal_top(|ui| {
            ui.add(icons().image_tinted(ui.ctx(), "mdi-alert", 32.0, Tint::Color(Color32::from_rgb(242, 165, 1))));
            ui.add(egui::Label::new(t("LblBrief")).wrap());
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(crate::ui::fonts::bold(t("label1")));
            ui.add_space(200.0);
            ui.label(t("label2"));
        });
        ui.horizontal(|ui| {
            // single radio, always checked
            let mut on = true;
            ui.radio_value(&mut on, true, t("RbFromSpecific"));
            ui.add_space(90.0);
            let max = self.max_line() as f64;
            ui.add(NumericUpDown::new(&mut self.line).range(1.0, max).width(80.0).id(egui::Id::unique("runfrompos-line")));
        });
        ui.add_space(8.0);
        let mut out = None;
        ui.horizontal(|ui| {
            if self.allow_homing {
                ui.checkbox(&mut self.homing, t("CbRedoHoming"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let sz = Vec2::new(80.0, 27.0);
                if button_en(ui, true, &t("BtnOK"), None, sz) {
                    out = Some(true);
                }
                if button_en(ui, true, &t("BtnCancel"), None, sz) {
                    out = Some(false);
                }
            });
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_range() {
        let f = RunFromPositionForm::new(250, true);
        // spinner starts at 1 -> position 0, maximum is total+1
        assert_eq!(f.position(), 0);
        assert_eq!(f.max_line(), 251);
        assert!(!f.homing);
        let g = RunFromPositionForm::open(&RunFromPositionInfo { line_count: 10, homing_enabled: false });
        assert_eq!(g.max_line(), 11);
        assert!(!g.allow_homing);
    }
}
