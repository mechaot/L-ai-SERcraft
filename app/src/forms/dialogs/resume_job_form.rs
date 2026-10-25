// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `ResumeJobForm.cs` + `ResumeJobForm.Designer.cs`: "resume interrupted job" dialog.
//!
//! The core decides (`GrblCore::run_program` -> `RunDecision::AskResume(info)`); this form collects the
//! start line, then the caller runs `run_program_from_start_user` or `continue_program`.

use eframe::egui::{self, Color32, Ui, Vec2};
use lasergrbl_core::{DetectedIssue, ResumeInfo};
use lg_gcode::config::GPoint;

use super::common::{ModalEvent, button_en, modal, trf};
use crate::ui::dialog::{DialogResult, MessageBox, MessageBoxButtons, MessageBoxIcon};
use crate::ui::icons::{Tint, icons};
use crate::ui::tr;
use crate::ui::widgets::NumericUpDown;

/// Result of the dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum ResumeOutcome {
    /// Cancel / close box (C# returned -1).
    Cancelled,
    /// OK. `position`: 0-based first line to run (0 = from the beginning).
    /// `homing`: "Redo Homing" ticked. `wco`: "Home @" ticked.
    Ok { position: i32, homing: bool, wco: bool },
}

/// Radio choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    None,
    FromBeginning,
    SomeLines,
    FromSent,
    Specific,
}

/// Default radio of the constructor, by detected issue.
///
/// `issue`: last detected issue. `some_lines_enabled`/`sent_enabled`: radio enabled flags.
/// Returns the preselected choice (`Choice::None` keeps OK disabled, like C#).
pub fn initial_choice(issue: DetectedIssue, some_lines_enabled: bool, sent_enabled: bool) -> Choice {
    use DetectedIssue::*;
    match issue {
        // grbl does not execute its buffers: restart some lines before the last planned command
        StopResponding | UnexpectedReset | ManualReset => {
            if some_lines_enabled { Choice::SomeLines } else { Choice::FromBeginning }
        }
        // sent lines could be already executed: restart from sent
        ManualDisconnect | UnexpectedDisconnect => {
            if sent_enabled { Choice::FromSent } else { Choice::Specific }
        }
        ManualAbort => Choice::FromBeginning,
        _ => Choice::None,
    }
}

/// `Position` getter.
///
/// `choice`: selected radio. `some_line`/`sent`: 1-based numbers shown in the labels.
/// `specific`: spinner value (1-based). Returns the 0-based line or -1 for no choice.
pub fn position_of(choice: Choice, some_line: i32, sent: i32, specific: i32) -> i32 {
    match choice {
        Choice::FromBeginning => 0,
        Choice::SomeLines => some_line - 1,
        Choice::FromSent => sent - 1,
        Choice::Specific => specific - 1,
        Choice::None => -1,
    }
}

/// Text of the "Home @" checkbox: `"{0} X{1} Y{2}"` plus `" Z{0}"` when Z is not 0.
///
/// `caption`: translated base text. `wco`: last known work offset. Returns the text.
pub fn wco_caption(caption: &str, wco: &GPoint) -> String {
    let mut s = format!("{caption} X{} Y{}", lg_dotnet::fmt_float(wco.x), lg_dotnet::fmt_float(wco.y));
    if wco.z != 0.0 {
        s.push_str(&format!(" Z{}", lg_dotnet::fmt_float(wco.z)));
    }
    s
}

/// The resume dialog.
pub struct ResumeJobForm {
    info: ResumeInfo,
    some_line: i32,
    sent: i32,
    choice: Choice,
    specific: f64,
    homing: bool,
    wco: bool,
    confirm: Option<MessageBox>,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl ResumeJobForm {
    /// Create the dialog (C# private ctor).
    ///
    /// `info`: data from `RunDecision::AskResume` (`suggest_setwco` is both `allowWCO` and `suggestWCO`).
    pub fn open(info: ResumeInfo) -> Self {
        let some_line = (info.executed - 17).max(0) + 1;
        let sent = info.sent + 1;
        let some_enabled = some_line > 1;
        let choice = initial_choice(info.last_issue, some_enabled, true);
        ResumeJobForm {
            some_line,
            sent,
            choice,
            specific: sent as f64,
            homing: info.homing_enabled && info.suggest_homing,
            wco: info.suggest_setwco,
            info,
            confirm: None,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// `Position` of the current choice.
    pub fn position(&self) -> i32 {
        position_of(self.choice, self.some_line, self.sent, self.specific as i32)
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<ResumeOutcome> {
        // "homing required" confirmation (OKCancel, default Cancel)
        if let Some(mb) = &mut self.confirm {
            if let Some(r) = mb.show(ctx) {
                self.confirm = None;
                if r == DialogResult::Ok {
                    return Some(self.ok());
                }
            }
            return None;
        }
        let title = trf("ResumeJobForm.$this.Text", "Resume Job");
        let id = ("resumejob", self.id);
        match modal(ctx, id, &title, 560.0, true, |ui| self.body(ui)) {
            Some(ModalEvent::Done(true)) => self.on_ok_click(),
            Some(ModalEvent::Done(false)) | Some(ModalEvent::Closed) => Some(ResumeOutcome::Cancelled),
            None => None,
        }
    }

    /// Build the OK result.
    fn ok(&self) -> ResumeOutcome {
        ResumeOutcome::Ok { position: self.position(), homing: self.homing && self.info.homing_enabled, wco: self.wco && self.info.suggest_setwco }
    }

    /// `BtnOK_Click`: warn when resuming mid-job without homing though the position is unknown.
    fn on_ok_click(&mut self) -> Option<ResumeOutcome> {
        let homing = self.homing && self.info.homing_enabled;
        if self.position() <= 0 || !self.info.suggest_homing || homing {
            return Some(self.ok());
        }
        let mb = MessageBox::new(tr("Strings.ResumeJobHomingRequired"), tr("Strings.ResumeJobHomingRequiredTitle"))
            .buttons(MessageBoxButtons::OkCancel)
            .icon(MessageBoxIcon::Warning)
            .default_button(1);
        self.confirm = Some(mb);
        None
    }

    /// Form body. Returns Some(true)=OK, Some(false)=cancel.
    fn body(&mut self, ui: &mut Ui) -> Option<bool> {
        let t = |c: &str| tr(&format!("ResumeJobForm.{c}.Text")).to_owned();
        ui.horizontal_top(|ui| {
            // PBWarn
            ui.add(icons().image_tinted(ui.ctx(), "mdi-alert", 32.0, Tint::Color(Color32::from_rgb(242, 165, 1))));
            ui.vertical(|ui| {
                ui.add(egui::Label::new(t("LblDetected")).wrap());
                ui.horizontal(|ui| {
                    ui.label(t("LblCause"));
                    // TxtCause is a plain Label
                    ui.label(format!("{:?}", self.info.last_issue));
                });
            });
        });
        ui.add(egui::Label::new(t("LblBrief")).wrap());
        ui.add_space(6.0);
        let some_enabled = self.some_line > 1;
        let sent_enabled = self.info.sent < self.info.target;
        egui::Grid::new(("resume-grid", self.id)).num_columns(2).min_col_width(300.0).show(ui, |ui| {
            // header row: LblOptions / LblLineNumber
            ui.label(t("LblOptions"));
            ui.label(t("LblLineNumber"));
            ui.end_row();
            ui.radio_value(&mut self.choice, Choice::FromBeginning, t("RbFromBeginning"));
            ui.label(t("LblBegin"));
            ui.end_row();
            ui.add_enabled_ui(some_enabled, |ui| ui.radio_value(&mut self.choice, Choice::SomeLines, t("RbSomeLines")));
            ui.add_enabled(some_enabled, egui::Label::new(self.some_line.to_string()));
            ui.end_row();
            ui.radio_value(&mut self.choice, Choice::FromSent, t("RbFromSent"));
            ui.add_enabled(sent_enabled, egui::Label::new(self.sent.to_string()));
            ui.end_row();
            ui.radio_value(&mut self.choice, Choice::Specific, t("RbFromSpecific"));
            ui.add_enabled_ui(self.choice == Choice::Specific, |ui| {
                ui.add(NumericUpDown::new(&mut self.specific).range(1.0, self.sent as f64).width(70.0).id(egui::Id::unique("resume-specific")));
            });
            ui.end_row();
        });
        ui.add_space(8.0);
        let mut out = None;
        ui.horizontal(|ui| {
            if self.info.homing_enabled {
                ui.checkbox(&mut self.homing, t("CbRedoHoming"));
            }
            if self.info.suggest_setwco {
                let cap = wco_caption(&t("CbRestoreWCO"), &self.info.last_known_wco);
                ui.checkbox(&mut self.wco, cap);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let sz = Vec2::new(80.0, 27.0);
                if button_en(ui, self.position() >= 0, &t("BtnOK"), None, sz) {
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
    fn initial_radio_by_issue() {
        use DetectedIssue::*;
        assert_eq!(initial_choice(StopResponding, true, true), Choice::SomeLines);
        assert_eq!(initial_choice(UnexpectedReset, false, true), Choice::FromBeginning);
        assert_eq!(initial_choice(ManualDisconnect, true, true), Choice::FromSent);
        assert_eq!(initial_choice(UnexpectedDisconnect, true, false), Choice::Specific);
        assert_eq!(initial_choice(ManualAbort, true, true), Choice::FromBeginning);
        assert_eq!(initial_choice(MachineAlarm, true, true), Choice::None);
        assert_eq!(initial_choice(Unknown, true, true), Choice::None);
    }

    #[test]
    fn positions() {
        assert_eq!(position_of(Choice::FromBeginning, 5, 9, 3), 0);
        assert_eq!(position_of(Choice::SomeLines, 5, 9, 3), 4);
        assert_eq!(position_of(Choice::FromSent, 5, 9, 3), 8);
        assert_eq!(position_of(Choice::Specific, 5, 9, 3), 2);
        assert_eq!(position_of(Choice::None, 5, 9, 3), -1);
    }

    #[test]
    fn wco_text() {
        assert_eq!(wco_caption("Home @", &GPoint::new(1.5, 2.0, 0.0)), "Home @ X1.5 Y2");
        assert_eq!(wco_caption("Home @", &GPoint::new(0.0, 0.0, -3.25)), "Home @ X0 Y0 Z-3.25");
    }

    #[test]
    fn some_line_arithmetic() {
        let info = ResumeInfo {
            executed: 40,
            sent: 60,
            target: 100,
            last_issue: DetectedIssue::StopResponding,
            homing_enabled: true,
            suggest_homing: true,
            suggest_setwco: false,
            last_known_wco: GPoint::new(0.0, 0.0, 0.0),
        };
        let f = ResumeJobForm::open(info);
        // max(0, exec-17)+1 and sent+1
        assert_eq!((f.some_line, f.sent), (24, 61));
        assert_eq!(f.position(), 23);
        assert!(f.homing && !f.wco);
    }
}
