// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports the `StatusBar` of `MainForm` (labels, buffer bar, overrides) and the status part of `TimerUpdate`.

use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use lasergrbl_core::{GrblCore, MacStatus};
use lg_base::tools::utils::{time_span_to_string, TimePrecision};
use lg_dotnet::fmt_double_custom;

use crate::ui::icons::icons;
use crate::ui::theme::{named, Theme};
use crate::ui::widgets::{LabelTb, OverrideKind};
use crate::ui::{tr, tr_plain};

/// Request from the status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusAction {
    /// `BtnUnlockFromStuck_Click`: confirmation, then `UnlockFromBufferStuck(false)`.
    UnlockFromStuck,
    /// `TTlaserLife_Click`: `LaserUsage.CreateAndShowDialog`.
    LaserUsage,
}

/// Status bar state.
pub struct StatusBar {
    /// `TTTLines.Text` (set when a file is loaded).
    lines: String,
    /// Override sliders while the popup is open (Rapid, Linear, Power).
    ov: Option<[LabelTb; 3]>,
}

/// Colours of the status label for a machine status (`TimerUpdate` switch).
///
/// `s`: status. `th`: theme. Returns (background, foreground).
pub fn status_colors(s: MacStatus, th: &Theme) -> (Color32, Color32) {
    match s {
        MacStatus::Alarm => (named::RED, named::WHITE),
        MacStatus::Door | MacStatus::Hold | MacStatus::Cooling | MacStatus::AutoHold => (named::DARK_ORANGE, named::BLACK),
        MacStatus::Jog | MacStatus::Run | MacStatus::Check => (Color32::from_rgb(144, 238, 144), named::BLACK),
        _ => (th.form_back_color, th.form_fore_color),
    }
}

/// Back/fore colours of an override label (`RefreshOverride`).
///
/// `v`: override percent. `th`: theme. Returns (background, foreground).
pub fn override_colors(v: i32, th: &Theme) -> (Color32, Color32) {
    let bg = if v > 100 { named::LIGHT_PINK } else if v < 100 { named::LIGHT_BLUE } else { th.form_back_color };
    let fg = if v != 100 { named::BLACK } else { th.form_fore_color };
    (bg, fg)
}

/// Text of an override label, `"G0 [1.00x]"`.
///
/// `name`: "G0"/"G1"/"S". `v`: override percent. Returns the label text.
pub fn override_text(name: &str, v: i32) -> String {
    format!("{name} [{}x]", fmt_double_custom(v as f64 / 100.0, "0.00"))
}

/// Draw one status label with the etched border.
///
/// `ui`: parent. `text`: caption. `bg`: background (None = transparent). `fg`: text colour. `border`: draw the
/// etched frame. Returns the response (click sense).
fn label(ui: &mut Ui, text: &str, bg: Option<Color32>, fg: Color32, border: bool) -> egui::Response {
    let font = egui::FontId::proportional(12.0);
    let g = ui.painter().layout_no_wrap(text.to_owned(), font.clone(), fg);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 8.0, 20.0), Sense::click());
    if let Some(b) = bg {
        ui.painter().rect_filled(rect, 0.0, b);
    }
    if border {
        let th = Theme::current();
        ui.painter().rect_stroke(rect.shrink(0.5), 0.0, Stroke::new(1.0, th.controls_border), StrokeKind::Inside);
    }
    ui.painter().galley(Pos2::new(rect.left() + 4.0, rect.center().y - g.size().y / 2.0), g, fg);
    resp
}

impl Default for StatusBar {
    /// Designer default: `TTTLines.Text = "0"`.
    fn default() -> Self {
        StatusBar { lines: "0".to_owned(), ov: None }
    }
}

impl StatusBar {
    /// `OnFileLoaded`: update the line count.
    ///
    /// `core`: the engine.
    pub fn on_file_loaded(&mut self, core: &GrblCore) {
        let n = core.loaded_file().read().map(|f| f.count()).unwrap_or(0);
        self.lines = n.to_string();
    }

    /// Draw the bar.
    ///
    /// `ui`: bottom panel ui. `core`: the engine. `stuck`: `IsBufferStuck` (from `TimerUpdate`). Returns requests.
    pub fn show(&mut self, ui: &mut Ui, core: &GrblCore, stuck: bool) -> Vec<StatusAction> {
        let mut out = Vec::new();
        let th = Theme::current();
        let fg = th.form_fore_color;
        let status = core.machine_status();
        let in_program = core.in_program();
        let estimated = if in_program { core.projected_time() } else { core.loaded_file().read().map(|f| f.estimated_time()).unwrap_or_default() };
        let est_text = time_span_to_string(estimated, TimePrecision::Second, TimePrecision::Second, " ,", true);
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            // ---- left part ----
            label(ui, tr("MainForm.TTLLines.Text"), None, fg, false);
            label(ui, &self.lines, None, fg, false);
            label(ui, tr("MainForm.TTLBuffer.Text"), None, fg, false);
            // PbBuffer: continuous bar, red while stuck
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(70.0, 16.0), Sense::hover());
            let max = core.buffer_size().max(1) as f32;
            let used = core.used_buffer() as f32;
            ui.painter().rect_filled(rect, 0.0, th.log_back_color);
            let w = (rect.width() - 2.0) * (used / max).clamp(0.0, 1.0);
            let col = if stuck { Color32::from_rgb(218, 38, 38) } else { Color32::from_rgb(6, 176, 37) };
            ui.painter().rect_filled(Rect::from_min_size(rect.min + Vec2::splat(1.0), Vec2::new(w, rect.height() - 2.0)), 0.0, col);
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, th.controls_border), StrokeKind::Inside);
            resp.on_hover_text(format!("Buffer: {}/{} Free:{}", core.used_buffer(), core.buffer_size(), core.free_buffer()));
            // BtnUnlockFromStuck
            let b = ui.add_enabled(stuck, egui::Button::image(icons().image(ui.ctx(), "mdi-lock-open", 16.0)).frame(false));
            if b.clicked() {
                out.push(StatusAction::UnlockFromStuck);
            }
            label(ui, tr(if in_program { "Strings.MainFormProjectedTime" } else { "Strings.MainFormEstimatedTime" }), None, fg, true);
            label(ui, &est_text, None, fg, false);

            // ---- right part ----
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (bg, sfg) = status_colors(status, &th);
                let r = label(ui, crate::ui::tr(&format!("Strings.MacStatus{}", lasergrbl_core::NamedEnum::variant_name(&status))), Some(bg), sfg, false);
                if r.double_clicked() {
                    lg_base::tools::utils::open_link("https://lasergrbl.com/usage/machine-status/");
                }
                label(ui, tr("MainForm.TTLStatus.Text"), None, fg, true);
                if core.support_override() {
                    // order right to left: G0, G1, S
                    let items = [("G0", core.override_g0(), 0usize), ("G1", core.override_g1(), 1), ("S", core.override_s(), 2)];
                    let mut any_open = false;
                    for (name, v, idx) in items {
                        let (bg, f) = override_colors(v, &th);
                        let resp = label(ui, &override_text(name, v), Some(bg), f, true);
                        if resp.clicked() {
                            // fresh sliders from the current overrides (ctor of LabelTB)
                            self.ov = Some([
                                LabelTb::new(OverrideKind::Rapid, core.override_g0()),
                                LabelTb::new(OverrideKind::Linear, core.override_g1()),
                                LabelTb::new(OverrideKind::Power, core.override_s()),
                            ]);
                        }
                        let _ = idx;
                        let popup = egui::Popup::from_toggle_button_response(&resp)
                            .align(egui::RectAlign::TOP_START)
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
                        let shown = popup.show(|ui| {
                            ui.set_width(150.0);
                            if let Some(ov) = self.ov.as_mut() {
                                // menu order: Power, Linear, Rapid
                                for k in [2usize, 1, 0] {
                                    let r = ov[k].show(ui, 110.0);
                                    if r.changed {
                                        match ov[k].kind {
                                            OverrideKind::Rapid => core.set_t_override_g0(ov[k].percent()),
                                            OverrideKind::Linear => core.set_t_override_g1(ov[k].percent()),
                                            OverrideKind::Power => core.set_t_override_s(ov[k].percent()),
                                        }
                                    }
                                    if r.commit {
                                        core.manage_overrides();
                                    }
                                }
                            }
                        });
                        any_open |= shown.is_some();
                    }
                    if !any_open {
                        self.ov = None;
                    }
                }
                label(ui, "", None, fg, false);
                // laser life: total hours, click opens the usage dialog
                let hours = lasergrbl_core::laser_life::get_current_time().total_hours();
                let ll = label(ui, &format!("{}h", fmt_double_custom(hours, "0.0")), None, fg, true);
                if ll.on_hover_text(tr_plain("MainForm.TTlaserLife.ToolTipText")).clicked() {
                    out.push(StatusAction::LaserUsage);
                }
            });
        });
        let _ = RichText::new("");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_label_text() {
        assert_eq!(override_text("G0", 100), "G0 [1.00x]");
        assert_eq!(override_text("S", 125), "S [1.25x]");
    }

    #[test]
    fn override_label_colors() {
        let th = Theme::of(crate::ui::theme::Scheme::CADStyle);
        assert_eq!(override_colors(150, &th).0, named::LIGHT_PINK);
        assert_eq!(override_colors(50, &th).0, named::LIGHT_BLUE);
        assert_eq!(override_colors(100, &th).0, th.form_back_color);
    }

    #[test]
    fn status_label_colors() {
        let th = Theme::of(crate::ui::theme::Scheme::CADStyle);
        assert_eq!(status_colors(MacStatus::Alarm, &th), (named::RED, named::WHITE));
        assert_eq!(status_colors(MacStatus::Run, &th).1, named::BLACK);
        assert_eq!(status_colors(MacStatus::Idle, &th).0, th.form_back_color);
    }
}
