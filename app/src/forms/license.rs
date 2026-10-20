#![allow(dead_code, clippy::collapsible_if, clippy::too_many_arguments, clippy::needless_borrow)]
//! Ports `LicenseForm.cs` (+ Designer): "LaserGRBL License" dialog (GPL text + donate link).

use eframe::egui::{self, Align, Layout, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::forms::form_util::*;
use crate::ui::theme::Theme;
use crate::ui::widgets::GrblButton;
use crate::ui::tr;

/// Fallback when the i18n catalog lacks the license text: the shipped `LICENSE.md`.
const LICENSE_MD: &str = include_str!("../../../LICENSE.md");

/// Window caption (hard coded in the designer).
const TITLE: &str = "LaserGRBL License";

/// License dialog state.
#[derive(Default)]
pub struct LicenseForm;

/// Text of `richTextBox1` (GPL) or `richTextBox2` (project notice).
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

/// Read-only rich text area with clickable links.
///
/// `ui`: parent. `id`: scroll id. `text`: content. `height`: visible height. `th`: theme.
fn rich_box(ui: &mut Ui, id: &str, text: &str, height: f32, th: &Theme) {
    egui::Frame::new().fill(th.log_back_color).stroke(Stroke::NONE).inner_margin(2.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ScrollArea::vertical().id_salt(id).max_height(height).auto_shrink([false, false]).show(ui, |ui| {
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
                } else {
                    ui.label(RichText::new(line).color(th.form_fore_color));
                }
            }
        });
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
        run_modal(ctx, "license", TITLE, 569.0, true, |ui| self.ui(ui)).map(|_| ())
    }

    /// Form body. Returns `Some(())` when OK was pressed.
    fn ui(&mut self, ui: &mut Ui) -> Option<()> {
        let th = Theme::current();
        let notice = rtb_text("LicenseForm.richTextBox2.Text", "");
        let gpl = rtb_text("LicenseForm.richTextBox1.Text", LICENSE_MD);
        rich_box(ui, "license-notice", &notice, 85.0, &th);
        ui.add_space(4.0);
        rich_box(ui, "license-gpl", &gpl, 205.0, &th);
        ui.add_space(4.0);
        let mut out = None;
        ui.horizontal(|ui| {
            if ui.add(GrblButton::new("Donate").icon("mdi-heart").min_size(Vec2::new(94.0, 36.0))).clicked() {
                open_link("https://lasergrbl.com/donate");
            }
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
        assert!(rtb_text("LicenseForm.richTextBox1.Text", LICENSE_MD).contains("GNU GENERAL PUBLIC LICENSE"));
        assert!(!rtb_text("LicenseForm.richTextBox2.Text", "").is_empty());
        assert!(LICENSE_MD.contains("GENERAL PUBLIC LICENSE"));
    }
}
