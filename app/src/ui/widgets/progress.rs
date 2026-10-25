// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `ColorProgressBar.cs`, `DoubleProgressBar.cs`, `WaitingProgressBar.cs`.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

use super::graph::lerp_color;
use crate::ui::theme::{named, Theme};

/// `FillStyles`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillStyle {
    Solid,
    Dashed,
}

/// C# `L` (tick width) and `S` (space).
const L: f32 = 6.0;
const S: f32 = 2.0;

/// `ColorProgressBar` state + painter. Fields mirror C# properties.
#[derive(Clone, Debug)]
pub struct ColorProgressBar {
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
    pub reverse: bool,
    pub draw_progress_string: bool,
    pub progress_string_decimals: usize,
    pub fill_style: FillStyle,
    pub fill_color: Color32,
    pub bar_color: Color32,
    /// Border = `ColorScheme.ControlsBorder` (C# DrawBorder); None = theme.
    pub border_color: Option<Color32>,
    pub size: Vec2,
}

impl Default for ColorProgressBar {
    fn default() -> Self {
        ColorProgressBar {
            value: 0.0,
            minimum: 0.0,
            maximum: 100.0,
            step: 10.0,
            reverse: false,
            draw_progress_string: false,
            progress_string_decimals: 0,
            fill_style: FillStyle::Dashed,
            fill_color: named::WHITE,
            bar_color: Color32::from_rgb(255, 128, 128),
            border_color: None,
            size: Vec2::new(150.0, 10.0),
        }
    }
}

impl ColorProgressBar {
    /// `PerformStep`.
    pub fn perform_step(&mut self) {
        self.value = if self.value < self.maximum { self.value + self.step } else { self.maximum };
    }
    /// `PerformStepBack`.
    pub fn perform_step_back(&mut self) {
        self.value = if self.value > self.minimum { self.value - self.step } else { self.minimum };
    }
    /// `Increment(value)`.
    pub fn increment(&mut self, v: f64) {
        self.value = if self.value < self.maximum { self.value + v } else { self.maximum };
    }
    /// `Decrement(value)`.
    pub fn decrement(&mut self, v: f64) {
        self.value = if self.value > self.minimum { self.value - v } else { self.minimum };
    }
    /// `Value` setter semantic of min/max: clamp helper (C# `Minimum`/`Maximum` setters clamp value).
    pub fn set_range(&mut self, min: f64, max: f64) {
        self.value = self.value.max(min).min(max);
        self.minimum = min;
        self.maximum = max;
    }

    /// Progress text, "{0:p<dec>}" (NaN -> empty).
    pub fn progress_text(&self) -> String {
        let p = (self.value - self.minimum) / (self.maximum - self.minimum);
        if p.is_nan() {
            String::new()
        } else {
            format!("{:.*}%", self.progress_string_decimals, p * 100.0)
        }
    }

    /// Width in px of the bar part (`DrawBar`): floor((W-3)*(min(V,max)-min)/(max-min)).
    pub fn bar_width(&self, w: f32, v: f64) -> f32 {
        if v.is_nan() || v.is_infinite() || (self.maximum - self.minimum) == 0.0 {
            return 0.0;
        }
        (((w as f64 - 3.0) * (v.min(self.maximum) - self.minimum)) / (self.maximum - self.minimum)).floor() as f32
    }

    /// Size after C# `SetBoundsCore` (min 20x6; dashed: width multiple of 8 plus 1).
    pub fn snapped_size(&self, requested: Vec2) -> Vec2 {
        let w = requested.x.max(20.0);
        let h = requested.y.max(6.0);
        let w = if self.fill_style == FillStyle::Dashed { (w / 8.0).floor() * 8.0 + 1.0 } else { w };
        Vec2::new(w, h)
    }

    /// Draw the bar (default fill painter).
    ///
    /// `ui`: parent. Returns response (hover only).
    pub fn show(&self, ui: &mut Ui) -> Response {
        self.show_with(ui, |_, _, _| {})
    }

    /// Draw with a custom progress layer (`DrawProgres` override).
    ///
    /// `ui`: parent. `custom`: called with `(painter, rect, self)` instead of the default bar when
    /// it paints something (default bar is drawn first).
    pub fn show_with(&self, ui: &mut Ui, custom: impl FnOnce(&egui::Painter, Rect, &ColorProgressBar)) -> Response {
        let size = self.snapped_size(self.size);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let p = ui.painter_at(rect);
            self.paint_background(&p, rect);
            let enabled = ui.is_enabled();
            self.draw_bar(&p, rect, self.value, if enabled { self.bar_color } else { named::LIGHT_GRAY });
            custom(&p, rect, self);
            if self.fill_style == FillStyle::Dashed {
                self.draw_tick(&p, rect);
            }
            if self.draw_progress_string {
                let fore = Theme::current().form_fore_color;
                p.text(rect.center(), Align2::CENTER_CENTER, self.progress_text(), FontId::proportional(12.0), fore);
            }
            self.draw_border(&p, rect);
        }
        resp
    }

    /// Fill background.
    pub(crate) fn paint_background(&self, p: &egui::Painter, rect: Rect) {
        p.rect_filled(rect, 0.0, self.fill_color);
    }

    /// 1px themed border.
    pub(crate) fn draw_border(&self, p: &egui::Painter, rect: Rect) {
        let c = self.border_color.unwrap_or_else(|| Theme::current().controls_border);
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, c), StrokeKind::Inside);
    }

    /// `DrawBar`.
    pub(crate) fn draw_bar(&self, p: &egui::Painter, rect: Rect, v: f64, color: Color32) {
        let bw = self.bar_width(rect.width(), v);
        let bh = rect.height() - 4.0;
        if bw > 0.0 && bh > 0.0 {
            let x = if self.reverse { rect.right() - bw - 1.0 } else { rect.left() + 2.0 };
            p.rect_filled(Rect::from_min_size(Pos2::new(x, rect.top() + 2.0), Vec2::new(bw, bh)), 0.0, color);
        }
    }

    /// `DrawTick`: vertical gaps in fill color.
    fn draw_tick(&self, p: &egui::Painter, rect: Rect) {
        let mut cur = L + 3.0;
        while cur < rect.width() {
            let x = rect.left() + cur;
            p.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(S, self.fill_color));
            cur += S + L;
        }
    }
}

/// One bar of a [`DoubleProgressBar`].
#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub value: f64,
    pub color: Color32,
}

/// Triangle marker of a [`DoubleProgressBar`].
#[derive(Clone, Copy, Debug)]
pub struct Reference {
    pub value: f64,
    pub color: Color32,
    pub border: Color32,
    pub bottom: bool,
}

/// `DoubleProgressBar`: several overlapping bars + reference markers + custom text.
#[derive(Clone, Debug)]
pub struct DoubleProgressBar {
    pub base: ColorProgressBar,
    pub bars: Vec<Bar>,
    pub references: Vec<Reference>,
    /// `PercString` (drawn centered when `base.draw_progress_string`).
    pub perc_string: String,
}

impl Default for DoubleProgressBar {
    fn default() -> Self {
        DoubleProgressBar {
            base: ColorProgressBar { fill_style: FillStyle::Solid, ..Default::default() },
            bars: Vec::new(),
            references: Vec::new(),
            perc_string: String::new(),
        }
    }
}

impl DoubleProgressBar {
    /// Draw.
    ///
    /// `ui`: parent. Returns response.
    pub fn show(&self, ui: &mut Ui) -> Response {
        let b = &self.base;
        let size = b.snapped_size(b.size);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let p = ui.painter_at(rect);
            b.paint_background(&p, rect);
            for bar in &self.bars {
                b.draw_bar(&p, rect, bar.value, bar.color);
            }
            for r in &self.references {
                self.draw_ref(&p, rect, r);
            }
            if b.draw_progress_string {
                p.text(rect.center(), Align2::CENTER_CENTER, &self.perc_string, FontId::proportional(12.0), Theme::current().form_fore_color);
            }
            b.draw_border(&p, rect);
        }
        resp
    }

    /// `DrawRef`: triangle at value position.
    fn draw_ref(&self, p: &egui::Painter, rect: Rect, r: &Reference) {
        let b = &self.base;
        if r.value.is_nan() || r.value.is_infinite() || b.maximum - b.minimum == 0.0 {
            return;
        }
        let mut pos = b.bar_width(rect.width(), r.value);
        if pos == 0.0 {
            pos = 1.0;
        }
        let x = rect.left() + pos;
        let pts = if r.bottom {
            vec![Pos2::new(x - 3.0, rect.bottom()), Pos2::new(x + 3.0, rect.bottom()), Pos2::new(x, rect.bottom() - 6.0)]
        } else {
            vec![Pos2::new(x - 3.0, rect.top()), Pos2::new(x + 3.0, rect.top()), Pos2::new(x, rect.top() + 6.0)]
        };
        p.add(Shape::convex_polygon(pts, r.color, Stroke::new(1.0, r.border)));
    }
}

/// `WaitingProgressBar.BouncingModeEnum`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BouncingMode {
    PingPong,
    FilledUpDown,
    RollingStrip,
}

/// `WaitingProgressBar`: animated indeterminate bar. Call `show` every frame; it ticks itself.
#[derive(Clone, Debug)]
pub struct WaitingProgressBar {
    pub base: ColorProgressBar,
    pub mode: BouncingMode,
    /// `Interval` ms between steps.
    pub interval_ms: f64,
    running: bool,
    last_tick: f64,
}

impl Default for WaitingProgressBar {
    fn default() -> Self {
        WaitingProgressBar {
            base: ColorProgressBar { size: Vec2::new(129.0, 16.0), ..Default::default() },
            mode: BouncingMode::PingPong,
            interval_ms: 100.0,
            running: false,
            last_tick: 0.0,
        }
    }
}

impl WaitingProgressBar {
    /// New stopped bar.
    ///
    /// `mode`: bouncing mode.
    pub fn new(mode: BouncingMode) -> Self {
        WaitingProgressBar { mode, ..Default::default() }
    }
    /// `Running`.
    pub fn running(&self) -> bool {
        self.running
    }
    /// `Start`.
    pub fn start(&mut self) {
        self.running = true;
    }
    /// `Stop` (value reset to 0).
    pub fn stop(&mut self) {
        if self.running {
            self.running = false;
            self.base.value = 0.0;
        }
    }

    /// One timer tick (`TIM_Elapsed`).
    pub fn tick(&mut self) {
        let b = &mut self.base;
        if self.mode == BouncingMode::RollingStrip {
            if b.value == b.maximum {
                b.value = 0.0;
            } else {
                b.perform_step();
            }
        } else {
            if b.reverse {
                b.perform_step_back();
            } else {
                b.perform_step();
            }
            if b.value == b.maximum || b.value == b.minimum {
                b.reverse = !b.reverse;
            }
        }
    }

    /// Draw and advance animation by wall time.
    ///
    /// `ui`: parent. Returns response.
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        if self.running {
            let now = ui.input(|i| i.time);
            if now - self.last_tick >= self.interval_ms / 1000.0 {
                self.last_tick = now;
                self.tick();
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(self.interval_ms as u64));
        }
        let mode = self.mode;
        let fill_color = self.base.fill_color;
        let bar_color = self.base.bar_color;
        let reverse = self.base.reverse;
        if mode == BouncingMode::FilledUpDown {
            return self.base.show(ui);
        }
        // strip modes: no default bar, custom gradient strip
        let b = &self.base;
        let size = b.snapped_size(b.size);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let p = ui.painter_at(rect);
            b.paint_background(&p, rect);
            let (w, h) = (rect.width(), rect.height());
            let bar_h = h - 4.0;
            let bw = ((w - 3.0) / 3.0).floor();
            let range = b.maximum - b.minimum;
            let bp = (((w + bw + 4.0) as f64 * (b.value - b.minimum)) / range).floor() as f32;
            if bw > 0.0 && bar_h > 0.0 {
                let x = if mode == BouncingMode::PingPong && reverse { bp - bw - 1.0 } else { bp - bw };
                let strip = Rect::from_min_size(Pos2::new(rect.left() + x, rect.top() + 2.0), Vec2::new(bw, bar_h));
                paint_strip(&p, strip.intersect(rect.shrink(1.0)), fill_color, bar_color);
            }
            if b.fill_style == FillStyle::Dashed {
                let mut cur = L + 3.0;
                while cur < w {
                    let x = rect.left() + cur;
                    p.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(S, fill_color));
                    cur += S + L;
                }
            }
            b.draw_border(&p, rect);
        }
        resp
    }
}

/// Vertical-blend strip (C# `Blend` factors 0.1,1,1,1,1,0.85,0.1 at 0,.2,.5,.5,.5,.8,1).
fn paint_strip(p: &egui::Painter, r: Rect, fill: Color32, bar: Color32) {
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return;
    }
    const POS: [f32; 7] = [0.0, 0.2, 0.5, 0.5, 0.5, 0.8, 1.0];
    const FAC: [f32; 7] = [0.1, 1.0, 1.0, 1.0, 1.0, 0.85, 0.1];
    let mut mesh = egui::Mesh::default();
    for (i, (t, f)) in POS.iter().zip(FAC).enumerate() {
        // skip duplicate positions in the middle (same factor anyway)
        if i == 2 || i == 3 {
            continue;
        }
        let y = r.top() + r.height() * t;
        let c = lerp_color(fill, bar, f);
        mesh.colored_vertex(Pos2::new(r.left(), y), c);
        mesh.colored_vertex(Pos2::new(r.right(), y), c);
    }
    // 5 rows of vertices -> 4 quads
    for row in 0..4u32 {
        let i = row * 2;
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i + 1, i + 3, i + 2);
    }
    p.add(Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_math() {
        let b = ColorProgressBar::default();
        assert_eq!(b.bar_width(103.0, 50.0), 50.0);
        assert_eq!(b.bar_width(103.0, 500.0), 100.0);
        assert_eq!(b.snapped_size(Vec2::new(150.0, 10.0)), Vec2::new(145.0, 10.0));
    }

    #[test]
    fn waiting_pingpong_bounces() {
        let mut w = WaitingProgressBar::default();
        w.base.step = 50.0;
        w.tick();
        w.tick();
        assert_eq!(w.base.value, 100.0);
        assert!(w.base.reverse);
    }

    #[test]
    fn text() {
        let mut b = ColorProgressBar::default();
        b.value = 25.0;
        assert_eq!(b.progress_text(), "25%");
    }
}
