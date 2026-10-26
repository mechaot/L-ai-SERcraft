// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `LicenseForm.cs` (+ Designer), reworked: "License & Credits" dialog (LaserGRBL credit, edition notes, AI note, GPL text). No donation link.

use eframe::egui::{self, Align, Color32, Layout, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::forms::form_util::*;
use crate::ui::theme::Theme;
use crate::ui::widgets::GrblButton;
use crate::ui::fonts::bold;
use crate::ui::{tr, tr_fmt};

/// Fallback when the i18n catalog lacks the license text: the shipped `LICENSE.md`.
const LICENSE_MD: &str = include_str!("../../../LICENSE.md");

/// Authors, contributors and third-party notices: the shipped `NOTICE.md`.
const NOTICE_MD: &str = include_str!("../../../NOTICE.md");

/// Max height of the notices box, points.
const NOTICE_BOX_H: f32 = 260.0;

/// Accent blue of the credit panel stroke/heading (#4d9bff).
const ACCENT: Color32 = Color32::from_rgb(0x4d, 0x9b, 0xff);

/// Max height of the scrolling body, points.
const BODY_MAX_H: f32 = 500.0;

/// License dialog state.
#[derive(Default)]
pub struct LicenseForm;

/// Text of an i18n key, with fallback.
///
/// `key`: i18n key. `fallback`: used when the key is missing. Returns the text.
fn rtb_text(key: &str, fallback: &str) -> String {
    let t = tr(key);
    if t == key { fallback.to_owned() } else { t.to_owned() }
}

/// Split a line into text and `http(s)://` link segments (RichTextBox `DetectUrls`).
///
/// `line`: source line. Returns `(is_link, text)` segments in order.
pub fn split_links(line: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut rest = line;
    while let Some(pos) = ["https://", "http://"].iter().filter_map(|p| rest.find(p)).min() {
        if pos > 0 {
            out.push((false, rest[..pos].to_owned()));
        }
        let tail = &rest[pos..];
        let end = tail.find(|c: char| c.is_whitespace() || c == '<' || c == '>').unwrap_or(tail.len());
        // trailing punctuation is not part of the link
        let url = tail[..end].trim_end_matches(['.', ',', ';', ':', ')', '"', '\'']);
        out.push((true, url.to_owned()));
        rest = &tail[url.len()..];
    }
    if !rest.is_empty() {
        out.push((false, rest.to_owned()));
    }
    out
}

/// Draw text lines, URLs as clickable links. No scroll, no fixed height.
///
/// `ui`: parent. `text`: content, split at newlines. `th`: theme.
fn rich_lines(ui: &mut Ui, text: &str, th: &Theme) {
    for line in text.lines() {
        let segs = split_links(line);
        if segs.iter().any(|s| s.0) {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (link, t) in segs {
                    if link {
                        if ui.link(&t).clicked() {
                            open_link(&t);
                        }
                    } else {
                        ui.label(RichText::new(t).color(th.form_fore_color));
                    }
                }
            });
        } else if line.is_empty() {
            // blank line = paragraph gap
            ui.add_space(6.0);
        } else {
            ui.label(RichText::new(line).color(th.form_fore_color));
        }
    }
}

/// Read-only scrollable rich text area with clickable links.
///
/// `ui`: parent. `id`: scroll id. `text`: content. `height`: visible height. `th`: theme.
fn rich_box(ui: &mut Ui, id: &str, text: &str, height: f32, th: &Theme) {
    egui::Frame::new().fill(th.log_back_color).stroke(Stroke::NONE).inner_margin(2.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ScrollArea::vertical().id_salt(id).max_height(height).auto_shrink([false, false]).show(ui, |ui| rich_lines(ui, text, th));
    });
}

/// Highlighted credit panel: accent stroke, slightly lighter fill, full text visible.
///
/// `ui`: parent. `title`: bold heading. `text`: body with links. `th`: theme.
fn credit_panel(ui: &mut Ui, title: &str, text: &str, th: &Theme) {
    // fill = background nudged towards white
    let fill = th.form_back_color.lerp_to_gamma(Color32::WHITE, 0.07);
    egui::Frame::new().fill(fill).stroke(Stroke::new(1.5, ACCENT)).corner_radius(4.0).inner_margin(10.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(bold(title).size(15.0).color(ACCENT));
        ui.add_space(4.0);
        rich_lines(ui, text, th);
    });
}

impl LicenseForm {
    /// `new LicenseForm()`.
    pub fn new() -> Self {
        LicenseForm
    }

    /// Draw the modal form; call every frame while open.
    ///
    /// `ctx`: egui context. Returns `Some(())` once the form closed (OK button, Esc or close box).
    pub fn show(&mut self, ctx: &egui::Context) -> Option<()> {
        run_modal(ctx, "license", tr("LicenseForm.Title"), 569.0, true, |ui| self.ui(ui)).map(|_| ())
    }

    /// Form body: header, credit panel, edition, AI note, notices, license + GPL box, OK button.
    ///
    /// `ui`: modal content area. Returns `Some(())` when OK was pressed.
    fn ui(&mut self, ui: &mut Ui) -> Option<()> {
        let th = Theme::current();
        let gpl = rtb_text("LicenseForm.richTextBox1.Text", LICENSE_MD);
        // body scrolls as a whole
        ScrollArea::vertical().id_salt("license-body").max_height(BODY_MAX_H).auto_shrink([false, true]).show(ui, |ui| {
            ui.set_width(ui.available_width());
            // header: name, version, copyright
            ui.horizontal(|ui| {
                ui.label(bold(lg_base::APP_NAME).size(24.0).color(th.form_fore_color));
                ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).size(14.0).color(th.form_fore_color));
            });
            ui.label(RichText::new("© 2026 Andreas Pösch (mechaot)").color(th.form_fore_color));
            ui.add_space(8.0);
            // credit text + third-party paragraph
            let credit = format!("{}\n\n{}", tr("LicenseForm.CreditText"), tr("LicenseForm.CreditThirdParty"));
            credit_panel(ui, tr("LicenseForm.CreditTitle"), &credit, &th);
            ui.add_space(8.0);
            ui.label(bold(tr("LicenseForm.EditionTitle")).color(th.form_fore_color));
            rich_lines(ui, tr("LicenseForm.EditionText"), &th);
            ui.add_space(6.0);
            rich_lines(ui, tr("LicenseForm.AiText"), &th);
            ui.add_space(8.0);
            // closed by default; NOTICE.md with clickable links
            egui::CollapsingHeader::new(bold(tr("LicenseForm.NoticesTitle")).color(th.form_fore_color))
                .id_salt("license-notices-head")
                .default_open(false)
                .show(ui, |ui| rich_box(ui, "license-notices", NOTICE_MD, NOTICE_BOX_H, &th));
            ui.add_space(8.0);
            rich_lines(ui, tr("LicenseForm.LicenseText"), &th);
            ui.add_space(6.0);
            rich_lines(ui, tr("LicenseForm.Independent"), &th);
            ui.add_space(6.0);
            rich_lines(ui, &tr_fmt("LicenseForm.SourceText", &[&env!("CARGO_PKG_VERSION")]), &th);
            ui.add_space(4.0);
            rich_box(ui, "license-gpl", &gpl, 205.0, &th);
        });
        ui.add_space(4.0);
        let mut out = None;
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(GrblButton::new("OK").icon("mdi-checkbox-marked").min_size(Vec2::new(94.0, 36.0))).clicked() {
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
    fn links_are_split() {
        let s = split_links("see https://github.com/arkypita/LaserGRBL, ok");
        assert_eq!(s, vec![(false, "see ".into()), (true, "https://github.com/arkypita/LaserGRBL".into()), (false, ", ok".into())]);
        assert_eq!(split_links("no link"), vec![(false, "no link".into())]);
    }

    #[test]
    fn license_text_available() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        assert!(rtb_text("LicenseForm.richTextBox1.Text", LICENSE_MD).contains("GNU GENERAL PUBLIC LICENSE"));
        assert!(LICENSE_MD.contains("GENERAL PUBLIC LICENSE"));
        // new texts exist and credit the original
        for k in ["LicenseForm.Title", "LicenseForm.CreditTitle", "LicenseForm.CreditText", "LicenseForm.CreditThirdParty", "LicenseForm.EditionTitle", "LicenseForm.EditionText", "LicenseForm.AiText", "LicenseForm.LicenseText", "LicenseForm.Independent", "LicenseForm.NoticesTitle", "LicenseForm.SourceText"] {
            assert_ne!(tr(k), k, "{k}");
        }
        assert!(tr("LicenseForm.CreditText").contains("Diego Settimi"));
        assert!(tr("LicenseForm.AiText").contains("Claude Opus") && tr("LicenseForm.AiText").contains("Claude Sonnet"));
        assert!(tr("LicenseForm.EditionText").contains("Andreas Pösch") && tr("LicenseForm.EditionText").contains("mechaot"));
    }

    #[test]
    fn legal_notices() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        // GPLv3 "Appropriate Legal Notices": warranty line, both names
        let lt = tr("LicenseForm.LicenseText");
        assert!(lt.contains("NO WARRANTY"));
        assert!(lt.contains("Diego Settimi") && lt.contains("Andreas Pösch"));
        let ind = tr("LicenseForm.Independent");
        assert!(ind.contains("not affiliated") && ind.contains("Diego Settimi"));
        assert!(tr("LicenseForm.AiText").ends_with("Andreas Pösch."));
        // third-party paragraph names the NOTICE.md authors
        for who in ["Sven Hasemann", "Peter Selinger", "Wolfgang Nagl", "Angus Johnson", "Michal Brylka", "Hershey", "Pictogrammers"] {
            assert!(tr("LicenseForm.CreditThirdParty").contains(who), "{who}");
        }
    }

    #[test]
    fn notice_included() {
        assert!(NOTICE_MD.contains("Andreas Pösch") && NOTICE_MD.contains("Diego Settimi"));
        assert!(NOTICE_MD.contains("Boost Software License"));
        // NOTICE links render as clickable segments
        assert!(NOTICE_MD.lines().flat_map(split_links).any(|s| s.0 && s.1 == "https://github.com/arkypita/LaserGRBL"));
        // LICENSE.md carries both copyright blocks
        assert!(LICENSE_MD.contains("COPYRIGHT NOTICE FOR L-ai-SERcraft") && LICENSE_MD.contains("COPYRIGHT NOTICE FOR LaserGRBL"));
    }

    #[test]
    fn source_text_has_version() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let s = tr_fmt("LicenseForm.SourceText", &[&"9.9.9"]);
        assert!(s.contains("L-ai-SERcraft-9.9.9-source.tar.gz"), "{s}");    }

    #[test]
    fn credit_links_clickable() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let links: Vec<String> = tr("LicenseForm.CreditText").lines().flat_map(split_links).filter(|s| s.0).map(|s| s.1).collect();
        assert_eq!(links, vec!["https://lasergrbl.com", "https://github.com/arkypita/LaserGRBL"]);
    }

    #[test]
    fn no_donate_button() {
        // needle built at runtime so this file does not match itself
        let needle = ["Don", "ate"].concat();
        let body = include_str!("license.rs").split("#[cfg(test)]").next().unwrap();
        assert!(!body.to_lowercase().contains(&needle.to_lowercase()));
    }

    #[test]
    fn renders_headless() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let ctx = egui::Context::default();
        let mut f = LicenseForm::new();
        // textures delta must be cleared or egui panics on drop
        for _ in 0..3 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                f.show(&ui.ctx().clone());
            });
            out.textures_delta.clear();
        }
    }
}
