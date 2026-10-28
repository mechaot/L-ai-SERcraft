// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `LegalDisclaimer.cs` (+ Designer): legal disclaimer shown on first start.

use eframe::egui::{self, Align, Layout, RichText, TextureHandle, Ui, Vec2};

use crate::forms::form_util::*;
use crate::ui::widgets::GrblButton;
use crate::ui::tr;

/// i18n prefix of the form (root form).
const P: &str = "LegalDisclaimer.";

/// How the form ended. The caller stores setting `"IsDisclamerAccepted"` on `Accepted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegalOutcome {
    /// C# `accepted == true` (Accept pressed after ticking the check box).
    Accepted,
    /// Form closed without accepting (C# `accepted == false`).
    Declined,
}

/// Legal disclaimer dialog state.
#[derive(Default)]
pub struct LegalDisclaimer {
    have_read: bool,
    tex: Option<TextureHandle>,
}

impl LegalDisclaimer {
    /// `new LegalDisclaimer()` (Accept starts disabled).
    pub fn new() -> Self {
        Self::default()
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns the outcome once the form closed.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<LegalOutcome> {
        if self.tex.is_none() {
            self.tex = load_texture(ctx, "legaldisclaimer", include_bytes!("../../../resources/grafica/legaldisclaimer.png"));
        }
        let title = tr(&format!("{P}$this.Text")).to_owned();
        match run_modal(ctx, "legal-disclaimer", &title, 677.0, true, |ui| self.ui(ui))? {
            ModalEvent::Done(()) => Some(LegalOutcome::Accepted),
            ModalEvent::Closed => Some(LegalOutcome::Declined),
        }
    }

    /// Form body. Returns `Some(())` when Accept was pressed.
    fn ui(&mut self, ui: &mut Ui) -> Option<()> {
        // row 0: picture + "not a toy" text
        ui.horizontal(|ui| {
            native_image(ui, self.tex.as_ref());
            ui.add(egui::Label::new(RichText::new(tr(&format!("{P}LblNotToy.Text"))).size(13.0)).wrap());
        });
        ui.add_space(6.0);
        ui.add(egui::Label::new(RichText::new(tr(&format!("{P}LblFree.Text"))).size(13.0)).wrap());
        ui.add_space(6.0);
        // row 2: "have fun" right
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(tr(&format!("{P}LblHaveFun.Text"))).size(13.0));
            });
        });
        ui.add_space(6.0);
        // row 3: check box + Accept (enabled only when ticked)
        let mut out = None;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.have_read, crate::ui::fonts::bold(ctl_text(P, "CbIHaveRead")).size(13.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let b = GrblButton::new(ctl_text(P, "BtnAccept")).min_size(Vec2::new(96.0, 39.0));
                if ui.add_enabled(self.have_read, b).clicked() {
                    out = Some(());
                }
            });
        });
        out
    }
}
