// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! Brightness histogram with draggable start / stop markers and the brightness to power curve (raster-image dock).
//!
//! Not part of the C# app (intentional deviation). Immediate mode: the caller owns the histogram and the two
//! marker values, the widget edits them and reports live changes and the end of a drag.

use eframe::egui::{self, Align2, Color32, CursorIcon, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use lg_gcode::PowerMap;

use crate::ui::theme::Theme;

/// A marker is picked when the press is at most this many px away from its line.
const PICK_PX: f32 = 8.0;
/// Height of the black to white strip under the bars.
const STRIP_H: f32 = 8.0;
/// Height of the "0" / "255" label row under the strip.
const LABEL_H: f32 = 12.0;
/// Width of the right-hand power scale column.
const SCALE_W: f32 = 40.0;
/// Accent of the start marker and the power curve.
const START_COLOR: Color32 = Color32::from_rgb(226, 96, 24);
/// Accent of the stop marker.
const STOP_COLOR: Color32 = Color32::from_rgb(30, 144, 255);

/// Which of the two markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// Brightness that burns with S-MAX.
    Start,
    /// Brightness that burns with S-MIN.
    Stop,
}

/// Drag state kept in egui memory between frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Drag {
    marker: Marker,
    /// A value changed during this drag.
    moved: bool,
}

/// Power range for the curve on the right-hand scale (`y = power / s_limit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerScale {
    /// S-MIN.
    pub s_min: i32,
    /// S-MAX.
    pub s_max: i32,
    /// Full scale value (machine max PWM).
    pub s_limit: i32,
}

/// What a frame of the widget did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HistogramOutput {
    /// A marker value changed this frame (live while dragging).
    pub changed: bool,
    /// A drag that changed a value ended this frame (commit point).
    pub committed: bool,
}

/// Brightness histogram widget: `BrightnessHistogram::new(&hist, &mut start, &mut stop).power(scale).show(ui)`.
pub struct BrightnessHistogram<'a> {
    hist: &'a [u32; 256],
    start: &'a mut u8,
    stop: &'a mut u8,
    power: Option<PowerScale>,
    height: f32,
    id: Option<Id>,
    labels: (String, String),
}

impl<'a> BrightnessHistogram<'a> {
    /// New widget.
    ///
    /// `hist`: pixel count per brightness 0..=255. `start`/`stop`: marker values, edited in place.
    pub fn new(hist: &'a [u32; 256], start: &'a mut u8, stop: &'a mut u8) -> Self {
        BrightnessHistogram { hist, start, stop, power: None, height: 110.0, id: None, labels: ("Start".into(), "Stop".into()) }
    }

    /// Draw the power curve on a right-hand scale (without it the curve is on/off).
    ///
    /// `p`: S-MIN, S-MAX and the full scale value.
    pub fn power(mut self, p: PowerScale) -> Self {
        self.power = Some(p);
        self
    }

    /// Total height in px (default 110).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    /// Explicit id (keeps the drag state stable when the layout above shifts).
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Hover captions of the two markers.
    ///
    /// `start`/`stop`: texts shown before the value.
    pub fn labels(mut self, start: &str, stop: &str) -> Self {
        self.labels = (start.to_owned(), stop.to_owned());
        self
    }

    /// Draw and handle input.
    ///
    /// `ui`: parent, uses the full available width. Returns [`HistogramOutput`].
    pub fn show(self, ui: &mut Ui) -> HistogramOutput {
        let th = Theme::current();
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        let width = ui.available_width().max(80.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, self.height.max(STRIP_H + LABEL_H + 20.0)), Sense::hover());
        let resp = ui.interact(rect, id, Sense::click_and_drag());
        // plot left, power scale right, strip and labels below
        let scale_w = if self.power.is_some() { SCALE_W } else { 0.0 };
        let plot = Rect::from_min_max(rect.min, Pos2::new(rect.right() - scale_w, rect.bottom() - STRIP_H - LABEL_H));
        let strip = Rect::from_min_max(Pos2::new(plot.left(), plot.bottom()), Pos2::new(plot.right(), plot.bottom() + STRIP_H));
        let (left, w) = (plot.left(), plot.width());

        // input
        let mut out = HistogramOutput::default();
        let mut drag: Option<Drag> = ui.data(|d| d.get_temp::<Option<Drag>>(id)).flatten();
        let down = resp.is_pointer_button_down_on();
        let pointer = resp.interact_pointer_pos().or_else(|| resp.hover_pos());
        if down && drag.is_none() {
            // press: pick the nearest marker
            let press = ui.input(|i| i.pointer.press_origin()).or(pointer);
            if let Some(m) = press.and_then(|p| pick_marker(p.x, value_to_x(left, w, *self.start), value_to_x(left, w, *self.stop), PICK_PX)) {
                drag = Some(Drag { marker: m, moved: false });
            }
        }
        if let (true, Some(d), Some(p)) = (down, drag.as_mut(), pointer) {
            let v = x_to_value(left, w, p.x);
            let slot = match d.marker {
                Marker::Start => &mut *self.start,
                Marker::Stop => &mut *self.stop,
            };
            if *slot != v {
                *slot = v;
                d.moved = true;
                out.changed = true;
            }
        }
        if !down {
            // release: commit when something moved
            if let Some(d) = drag.take() {
                out.committed = d.moved;
            }
        }
        ui.data_mut(|d| d.insert_temp(id, drag));
        let (start, stop) = (*self.start, *self.stop);

        // cursor and hover text
        if let Some(p) = resp.hover_pos() {
            let near = pick_marker(p.x, value_to_x(left, w, start), value_to_x(left, w, stop), PICK_PX);
            if near.is_some() || drag.is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if plot.contains(p) && drag.is_none() {
                let v = near.map_or_else(|| x_to_value(left, w, p.x), |m| if m == Marker::Start { start } else { stop });
                let caption = match near {
                    Some(Marker::Start) => format!("{}: {v}", self.labels.0),
                    Some(Marker::Stop) => format!("{}: {v}", self.labels.1),
                    None => format!("{v}: {}", self.hist[v as usize]),
                };
                resp.on_hover_text_at_pointer(caption);
            }
        }

        if ui.is_rect_visible(rect) {
            self.paint(ui, &th, plot, strip, rect, (start, stop), drag.map(|d| d.marker));
        }
        out
    }

    /// Paint all layers.
    ///
    /// `ui`: parent. `th`: theme. `plot`/`strip`/`rect`: areas (bars, gradient strip, whole widget).
    /// `(start, stop)`: marker values. `active`: marker being dragged.
    fn paint(&self, ui: &Ui, th: &Theme, plot: Rect, strip: Rect, rect: Rect, (start, stop): (u8, u8), active: Option<Marker>) {
        let p = ui.painter_at(rect);
        let (left, w) = (plot.left(), plot.width());
        // ink contrasts with the plot background in every scheme
        let ink = contrast(th.log_back_color);
        p.rect_filled(plot, 0.0, th.log_back_color);

        // bars, log scale
        let max = self.hist.iter().copied().max().unwrap_or(0);
        // opaque mix (no alpha): neighbouring bars may overlap by a fraction of a px without stripes
        let bar_col = super::graph::lerp_color(th.log_back_color, ink, 0.5);
        for (i, c) in self.hist.iter().enumerate() {
            let h = bar_fraction(*c, max) * plot.height();
            if h < 0.5 {
                continue;
            }
            let x0 = left + i as f32 / 256.0 * w;
            let x1 = left + (i + 1) as f32 / 256.0 * w;
            p.rect_filled(Rect::from_min_max(Pos2::new(x0, plot.bottom() - h), Pos2::new(x1 + 0.6, plot.bottom())), 0.0, bar_col);
        }

        // brightness strip
        for i in 0..256 {
            let x0 = left + i as f32 / 256.0 * w;
            let x1 = left + (i + 1) as f32 / 256.0 * w;
            p.rect_filled(Rect::from_min_max(Pos2::new(x0, strip.top()), Pos2::new(x1 + 0.5, strip.bottom())), 0.0, Color32::from_gray(i as u8));
        }

        // outside [min, max]: no power, dimmed
        let (lo, hi) = (start.min(stop), start.max(stop));
        let dim = th.log_back_color.gamma_multiply(0.72);
        let x_lo = left + lo as f32 / 256.0 * w;
        let x_hi = left + (hi as f32 + 1.0) / 256.0 * w;
        for (a, b) in [(left, x_lo), (x_hi, left + w)] {
            if b > a {
                p.rect_filled(Rect::from_min_max(Pos2::new(a, plot.top()), Pos2::new(b, strip.bottom())), 0.0, dim);
            }
        }
        let border = Stroke::new(1.0, th.controls_border);
        p.rect_stroke(plot, 0.0, border, egui::StrokeKind::Inside);
        p.rect_stroke(strip, 0.0, border, egui::StrokeKind::Inside);

        // power curve: S-MAX at start, S-MIN at stop, zero outside
        let map = PowerMap { start, stop };
        let pts: Vec<Pos2> = (0..=255u8)
            .map(|b| {
                let y = match self.power {
                    Some(s) => (map.power(b, s.s_min, s.s_max) as f32 / s.s_limit.max(1) as f32).clamp(0.0, 1.0),
                    None => f32::from(u8::from(map.in_range(b))),
                };
                Pos2::new(value_to_x(left, w, b), plot.bottom() - 1.0 - y * (plot.height() - 2.0))
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(1.6, START_COLOR)));

        // markers: line + handle at the top
        for (m, v, col) in [(Marker::Stop, stop, STOP_COLOR), (Marker::Start, start, START_COLOR)] {
            let x = value_to_x(left, w, v);
            let on = active == Some(m);
            p.line_segment([Pos2::new(x, plot.top()), Pos2::new(x, strip.bottom())], Stroke::new(if on { 2.0 } else { 1.0 }, col));
            let outline = Stroke::new(1.0, th.log_back_color);
            let r = if on { 5.5 } else { 4.5 };
            match m {
                // start: triangle pointing down
                Marker::Start => {
                    let tri = vec![Pos2::new(x - r, plot.top()), Pos2::new(x + r, plot.top()), Pos2::new(x, plot.top() + r * 1.6)];
                    p.add(Shape::convex_polygon(tri, col, outline));
                }
                // stop: circle
                Marker::Stop => {
                    p.circle(Pos2::new(x, plot.top() + r), r, col, outline);
                }
            }
        }

        // labels under the strip and power scale on the right
        let font = FontId::proportional(10.0);
        let fg = th.form_fore_color;
        let ly = strip.bottom() + 1.0;
        p.text(Pos2::new(left, ly), Align2::LEFT_TOP, "0", font.clone(), fg);
        p.text(Pos2::new(left + w, ly), Align2::RIGHT_TOP, "255", font.clone(), fg);
        if let Some(s) = self.power {
            let x = plot.right() + 8.0;
            p.text(Pos2::new(x, plot.top()), Align2::LEFT_TOP, format!("S{}", s.s_limit), font.clone(), fg);
            p.text(Pos2::new(x, plot.bottom()), Align2::LEFT_BOTTOM, "0", font, fg);
        }
    }
}

/// x position (px) of the middle of the bin of brightness `v`.
///
/// `left`: plot left edge. `width`: plot width. `v`: brightness 0..=255. Returns the x coordinate.
pub fn value_to_x(left: f32, width: f32, v: u8) -> f32 {
    left + (f32::from(v) + 0.5) / 256.0 * width
}

/// Brightness bin under an x position.
///
/// `left`: plot left edge. `width`: plot width. `x`: pointer x. Returns 0..=255 (clamped at the edges).
pub fn x_to_value(left: f32, width: f32, x: f32) -> u8 {
    ((x - left) / width.max(1.0) * 256.0).floor().clamp(0.0, 255.0) as u8
}

/// Marker to grab for a press.
///
/// `px`: press x. `x_start`/`x_stop`: marker x positions. `tol`: max distance in px.
/// Returns the nearest marker within `tol`; on a tie (markers on top of each other) the press side decides,
/// so a drag to the right takes `Stop` and to the left `Start`; None when both are too far.
pub fn pick_marker(px: f32, x_start: f32, x_stop: f32, tol: f32) -> Option<Marker> {
    let (ds, dt) = ((px - x_start).abs(), (px - x_stop).abs());
    if ds > tol && dt > tol {
        return None;
    }
    if (ds - dt).abs() < 0.5 {
        return Some(if px >= x_start { Marker::Stop } else { Marker::Start });
    }
    Some(if ds < dt { Marker::Start } else { Marker::Stop })
}

/// Bar height on the log scale.
///
/// `count`: pixels in the bin. `max`: largest bin. Returns `ln(1 + count) / ln(1 + max)` in 0..=1 (0 when `max` is 0).
pub fn bar_fraction(count: u32, max: u32) -> f32 {
    if max == 0 {
        return 0.0;
    }
    ((1.0 + f64::from(count)).ln() / (1.0 + f64::from(max)).ln()) as f32
}

/// Black or white, whichever reads better on `bg`.
///
/// `bg`: background color. Returns the contrasting ink color.
fn contrast(bg: Color32) -> Color32 {
    let luma = 0.299 * f32::from(bg.r()) + 0.587 * f32::from(bg.g()) + 0.114 * f32::from(bg.b());
    if luma > 140.0 { Color32::BLACK } else { Color32::WHITE }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_value_roundtrip() {
        for (left, w) in [(10.0f32, 256.0f32), (0.0, 200.0), (5.5, 733.0)] {
            for v in 0..=255u8 {
                assert_eq!(x_to_value(left, w, value_to_x(left, w, v)), v, "left {left} w {w} v {v}");
            }
        }
    }

    #[test]
    fn x_to_value_clamps() {
        assert_eq!(x_to_value(10.0, 100.0, -50.0), 0);
        assert_eq!(x_to_value(10.0, 100.0, 10_000.0), 255);
        // degenerate width never divides by zero
        assert_eq!(x_to_value(0.0, 0.0, 5.0), 255);
    }

    #[test]
    fn pick_nearest_within_tolerance() {
        assert_eq!(pick_marker(52.0, 50.0, 120.0, 8.0), Some(Marker::Start));
        assert_eq!(pick_marker(118.0, 50.0, 120.0, 8.0), Some(Marker::Stop));
        assert_eq!(pick_marker(85.0, 50.0, 120.0, 8.0), None);
        // exactly at the tolerance
        assert_eq!(pick_marker(58.0, 50.0, 120.0, 8.0), Some(Marker::Start));
        assert_eq!(pick_marker(58.5, 50.0, 120.0, 8.0), None);
        // closer marker wins when both are in reach
        assert_eq!(pick_marker(60.0, 58.0, 66.0, 8.0), Some(Marker::Start));
        assert_eq!(pick_marker(64.5, 58.0, 66.0, 8.0), Some(Marker::Stop));
    }

    #[test]
    fn pick_tie_uses_press_side() {
        // markers on top of each other
        assert_eq!(pick_marker(103.0, 100.0, 100.0, 8.0), Some(Marker::Stop));
        assert_eq!(pick_marker(97.0, 100.0, 100.0, 8.0), Some(Marker::Start));
    }

    #[test]
    fn bar_fraction_is_log_scaled() {
        assert_eq!(bar_fraction(0, 0), 0.0);
        assert_eq!(bar_fraction(0, 1000), 0.0);
        assert!((bar_fraction(1000, 1000) - 1.0).abs() < 1e-6);
        // 31 of 1023: ln(32) / ln(1024) = 0.5
        assert!((bar_fraction(31, 1023) - 0.5).abs() < 1e-6);
        // a tiny bin stays visible next to a huge one
        assert!(bar_fraction(1, 1_000_000) > 0.04);
    }

    #[test]
    fn contrast_follows_background() {
        assert_eq!(contrast(Color32::WHITE), Color32::BLACK);
        assert_eq!(contrast(Color32::from_rgb(70, 79, 92)), Color32::WHITE);
        assert_eq!(contrast(Color32::from_rgb(220, 220, 220)), Color32::BLACK);
    }

    /// One frame in a fresh context. `hist`: data. `start`/`stop`: marker values. `events`: input.
    fn frame(ctx: &egui::Context, hist: &[u32; 256], start: &mut u8, stop: &mut u8, events: Vec<egui::Event>) -> HistogramOutput {
        let ri = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 200.0))), events, ..Default::default() };
        let mut out = HistogramOutput::default();
        let _ = ctx.run_ui(ri, |ui| {
            out = BrightnessHistogram::new(hist, start, stop).id(Id::unique("hist-test")).power(PowerScale { s_min: 0, s_max: 1000, s_limit: 1000 }).show(ui);
        });
        out
    }

    #[test]
    fn drag_moves_marker_and_commits_on_release() {
        let ctx = egui::Context::default();
        let mut hist = [0u32; 256];
        hist[10] = 5;
        hist[200] = 50;
        let (mut start, mut stop) = (20u8, 200u8);
        // plot geometry of the 300 px wide test screen: ui starts at the screen origin plus the default margin
        frame(&ctx, &hist, &mut start, &mut stop, vec![]);
        let r = ctx.read_response(Id::unique("hist-test")).expect("widget drawn").rect;
        let (left, w) = (r.left(), r.width() - SCALE_W);
        let y = r.top() + 30.0;
        let x_start = value_to_x(left, w, 20);
        let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        // press on the start marker
        let o1 = frame(&ctx, &hist, &mut start, &mut stop, vec![egui::Event::PointerMoved(Pos2::new(x_start, y)), btn(Pos2::new(x_start, y), true)]);
        assert!(!o1.changed && !o1.committed);
        // drag to brightness 100
        let x_new = value_to_x(left, w, 100);
        let o2 = frame(&ctx, &hist, &mut start, &mut stop, vec![egui::Event::PointerMoved(Pos2::new(x_new, y))]);
        assert!(o2.changed && !o2.committed);
        assert_eq!((start, stop), (100, 200));
        // release: commit
        let o3 = frame(&ctx, &hist, &mut start, &mut stop, vec![btn(Pos2::new(x_new, y), false)]);
        assert!(o3.committed);
        assert_eq!((start, stop), (100, 200));
        // next frame: nothing pending
        let o4 = frame(&ctx, &hist, &mut start, &mut stop, vec![]);
        assert_eq!(o4, HistogramOutput::default());
    }

    #[test]
    fn press_away_from_markers_changes_nothing() {
        let ctx = egui::Context::default();
        let hist = [1u32; 256];
        let (mut start, mut stop) = (0u8, 254u8);
        frame(&ctx, &hist, &mut start, &mut stop, vec![]);
        let r = ctx.read_response(Id::unique("hist-test")).unwrap().rect;
        let (left, w) = (r.left(), r.width() - SCALE_W);
        let pos = Pos2::new(value_to_x(left, w, 120), r.top() + 30.0);
        let btn = |pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        frame(&ctx, &hist, &mut start, &mut stop, vec![egui::Event::PointerMoved(pos), btn(true)]);
        let o = frame(&ctx, &hist, &mut start, &mut stop, vec![egui::Event::PointerMoved(Pos2::new(pos.x + 20.0, pos.y))]);
        assert!(!o.changed);
        let o = frame(&ctx, &hist, &mut start, &mut stop, vec![btn(false)]);
        assert!(!o.committed);
        assert_eq!((start, stop), (0, 254));
    }

    #[test]
    fn renders_in_light_and_dark_scheme() {
        use crate::ui::theme::Scheme;
        let _g = crate::forms::form_util::test_guard();
        let before = Theme::current_scheme();
        let mut hist = [0u32; 256];
        for (i, h) in hist.iter_mut().enumerate() {
            *h = (i as u32 * 37) % 500;
        }
        for s in Scheme::ALL {
            Theme::set_current(s);
            let ctx = egui::Context::default();
            let (mut a, mut b) = (16u8, 210u8);
            frame(&ctx, &hist, &mut a, &mut b, vec![]);
            // inverted map and equal markers must not panic either
            let (mut a, mut b) = (210u8, 16u8);
            frame(&ctx, &hist, &mut a, &mut b, vec![]);
            let (mut a, mut b) = (100u8, 100u8);
            frame(&ctx, &hist, &mut a, &mut b, vec![]);
        }
        Theme::set_current(before);
    }
}
