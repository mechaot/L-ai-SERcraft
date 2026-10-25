// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `IssueDetectorForm.cs` + `IssueDetectorForm.Designer.cs`: "Problem detected!" box.
//!
//! Wire to `CoreEvent::IssueDetected(issue)`; honour `"Do not show Issue Detector"` before opening
//! (see [`IssueDetectorForm::enabled`]).

use eframe::egui::{self, Color32, Ui, Vec2};
use lasergrbl_core::DetectedIssue;

use super::common::{ModalEvent, button_en, modal, open_link, trf};
use crate::ui::icons::{Tint, icons};
use crate::ui::tr;

/// Result of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueOutcome {
    /// OK pressed (a ticked "Do not show again" was stored).
    Ok,
    /// Close box / Esc (nothing stored, like C#).
    Closed,
}

/// The dialog.
pub struct IssueDetectorForm {
    cause: String,
    do_not_show: bool,
    id: u64,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl IssueDetectorForm {
    /// `Settings "Do not show Issue Detector"` is false: the detector may pop up.
    ///
    /// Returns true when the dialog should be shown for an issue.
    pub fn enabled() -> bool {
        !lg_base::settings::get("Do not show Issue Detector", false)
    }

    /// Create the dialog (`CreateAndShowDialog`).
    ///
    /// `issue`: detected issue, shown as its enum name.
    pub fn open(issue: DetectedIssue) -> Self {
        IssueDetectorForm { cause: format!("{issue:?}"), do_not_show: false, id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    /// Draw the dialog; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome on the closing frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<IssueOutcome> {
        let title = trf("IssueDetectorForm.$this.Text", "Problem detected!");
        let id = ("issue", self.id);
        // CancelButton = BtnOK: Esc acts as OK (saves "do not show again"); the close box does not
        let esc = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let ev = if esc { Some(ModalEvent::Done(())) } else { modal(ctx, id, &title, 427.0, true, |ui| self.body(ui)) };
        match ev {
            Some(ModalEvent::Done(())) => {
                // BtnOK_Click
                if self.do_not_show {
                    lg_base::settings::set("Do not show Issue Detector", true);
                }
                Some(IssueOutcome::Ok)
            }
            Some(ModalEvent::Closed) => Some(IssueOutcome::Closed),
            None => None,
        }
    }

    /// Form body. Returns Some(()) on OK.
    fn body(&mut self, ui: &mut Ui) -> Option<()> {
        let t = |c: &str| tr(&format!("IssueDetectorForm.{c}.Text")).to_owned();
        ui.horizontal_top(|ui| {
            ui.add(icons().image_tinted(ui.ctx(), "mdi-alert", 32.0, Tint::Color(Color32::from_rgb(242, 165, 1))));
            ui.vertical(|ui| {
                ui.label(t("LblDetected"));
                ui.horizontal(|ui| {
                    ui.label(t("LblCause"));
                    // TxtCause is a plain Label
                    ui.label(&self.cause);
                });
            });
        });
        ui.add(egui::Label::new(t("LblBrief")).wrap());
        // LL link label
        if ui.link(t("LL")).clicked() {
            open_link("https://lasergrbl.com/faq#issues");
        }
        ui.add_space(6.0);
        let mut out = None;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.do_not_show, t("CbDoNotShow"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button_en(ui, true, &t("BtnOK"), None, Vec2::new(80.0, 27.0)) {
                    out = Some(());
                }
            });
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::common::test_guard as settings_guard;

    #[test]
    fn cause_text_and_setting() {
        let _g = settings_guard();
        let f = IssueDetectorForm::open(DetectedIssue::StopResponding);
        assert_eq!(f.cause, "StopResponding");
        assert!(IssueDetectorForm::enabled());
        lg_base::settings::set("Do not show Issue Detector", true);
        assert!(!IssueDetectorForm::enabled());
    }
}
