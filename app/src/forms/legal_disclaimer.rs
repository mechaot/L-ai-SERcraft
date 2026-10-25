// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `LegalDisclaimer.cs` (+ Designer): legal disclaimer shown on first start.

use eframe::egui::{self, Align, Layout, RichText, TextureHandle, Ui, Vec2};

use crate::forms::form_util::*;
use crate::ui::widgets::GrblButton;
use crate::ui::{i18n, tr};

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

/// `HttpUtility.UrlEncode(string)`: UTF-8, space as `+`, lowercase hex.
///
/// `s`: text. Returns the encoded text.
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'*' | b'(' | b')' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02x}")),
        }
    }
    out
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

    /// Google-translate URL of `LlTranslate_LinkClicked`.
    ///
    /// Returns the URL for the current UI culture (two-letter code).
    pub fn translate_url() -> String {
        let t = |c: &str| tr(&format!("{P}{c}.Text")).replace('\n', "\r\n");
        let text = url_encode(&format!("{}\r\n\r\n{}\r\n\r\n{}", t("LblNotToy"), t("LblFree"), t("CbIHaveRead")));
        let lng = i18n::catalog().culture().split('-').next().unwrap_or("en").to_owned();
        format!("https://translate.google.it/?sl=en&tl={lng}&text={text}&op=translate")
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
        // row 2: translate link left, "have fun" right
        ui.horizontal(|ui| {
            if ui.link(ctl_text(P, "LlTranslate")).clicked() {
                open_link(&Self::translate_url());
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encoding_matches_httputility() {
        assert_eq!(url_encode("a b\r\nä!"), "a+b%0d%0a%c3%a4!");
        assert_eq!(url_encode("x=1&y/z"), "x%3d1%26y%2fz");
    }

    #[test]
    fn translate_url_shape() {
        let u = LegalDisclaimer::translate_url();
        assert!(u.starts_with("https://translate.google.it/?sl=en&tl="));
        assert!(u.ends_with("&op=translate"));
        assert!(u.contains("Laser+engraver+are+not+toys!"));
    }
}
