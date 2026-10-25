// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) Michal Brylka (ColorSlider)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports `UserControls/ColorSlider.cs`: themed gradient slider (raster params; also vertical).

use eframe::egui::{self, Color32, Key, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};

use super::graph::{fill_poly_mirror_gradient, rounded_rect_points};
use crate::ui::theme::{control_paint_dark, control_paint_light, named::*};

/// `ColorSlider.ColorSchemas`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderSchema {
    PerlBlueGreen,
    PerlRedCoral,
    PerlGold,
    PerlRoyalColors,
}

/// Slider orientation (C# `Orientation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    /// Min at top, max at bottom (as C#).
    Vertical,
}

/// The 8 slider colors (C# `ThumbOuterColor` ... `ElapsedInnerColor`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderColors {
    pub thumb_outer: Color32,
    pub thumb_inner: Color32,
    pub thumb_pen: Color32,
    pub bar_outer: Color32,
    pub bar_inner: Color32,
    pub bar_pen: Color32,
    pub elapsed_outer: Color32,
    pub elapsed_inner: Color32,
}

impl SliderColors {
    /// Table `aColorSchema`.
    ///
    /// `s`: schema. Returns its 8 colors.
    pub fn schema(s: SliderSchema) -> Self {
        let (bo, bi, bp, eo, ei) = match s {
            SliderSchema::PerlBlueGreen => (SKY_BLUE, DARK_SLATE_BLUE, GAINSBORO, DARK_GREEN, CHARTREUSE),
            SliderSchema::PerlRedCoral => (RED, DARK_RED, GAINSBORO, CORAL, LIGHT_CORAL),
            SliderSchema::PerlGold => (GREEN_YELLOW, YELLOW, GOLD, ORANGE, ORANGE_RED),
            SliderSchema::PerlRoyalColors => (RED, CRIMSON, GAINSBORO, DARK_VIOLET, VIOLET),
        };
        SliderColors {
            thumb_outer: WHITE,
            thumb_inner: GAINSBORO,
            thumb_pen: SILVER,
            bar_outer: bo,
            bar_inner: bi,
            bar_pen: bp,
            elapsed_outer: eo,
            elapsed_inner: ei,
        }
    }

    /// Apply `f` to every color.
    fn map(self, f: impl Fn(Color32) -> Color32) -> Self {
        SliderColors {
            thumb_outer: f(self.thumb_outer),
            thumb_inner: f(self.thumb_inner),
            thumb_pen: f(self.thumb_pen),
            bar_outer: f(self.bar_outer),
            bar_inner: f(self.bar_inner),
            bar_pen: f(self.bar_pen),
            elapsed_outer: f(self.elapsed_outer),
            elapsed_inner: f(self.elapsed_inner),
        }
    }

    /// `DesaturateColors` (NTSC weighted gray) used when disabled.
    pub fn desaturated(self) -> Self {
        self.map(|c| {
            let g = (c.r() as f64 * 0.3 + c.g() as f64 * 0.6 + c.b() as f64 * 0.1) as u8;
            Color32::from_rgb(g, g, g)
        })
    }

    /// `LightenColors` (`ControlPaint.Light`) used on mouse hover.
    pub fn lightened(self) -> Self {
        self.map(control_paint_light)
    }
}

impl Default for SliderColors {
    fn default() -> Self {
        Self::schema(SliderSchema::PerlBlueGreen)
    }
}

/// Themed slider. `ui.add(ColorSlider::new(&mut v, 0..=100))`; `response.changed()` fires on value change.
pub struct ColorSlider<'a> {
    value: &'a mut i32,
    min: i32,
    max: i32,
    orientation: Orientation,
    thumb_size: f32,
    thumb_round: f32,
    border_round: f32,
    small_change: u32,
    large_change: u32,
    wheel_partitions: i32,
    draw_focus_rect: bool,
    semitransparent_thumb: bool,
    mouse_effects: bool,
    colors: SliderColors,
    length: Option<f32>,
}

impl<'a> ColorSlider<'a> {
    /// New slider (C# defaults: thumb 15, round 8, small 1, large 5, wheel partitions 10).
    ///
    /// `value`: bound value. `range`: inclusive min..=max (max > min enforced).
    pub fn new(value: &'a mut i32, range: std::ops::RangeInclusive<i32>) -> Self {
        let (min, mut max) = (*range.start(), *range.end());
        if max <= min {
            max = min + 1;
        }
        ColorSlider {
            value,
            min,
            max,
            orientation: Orientation::Horizontal,
            thumb_size: 15.0,
            thumb_round: 8.0,
            border_round: 8.0,
            small_change: 1,
            large_change: 5,
            wheel_partitions: 10,
            draw_focus_rect: true,
            semitransparent_thumb: true,
            mouse_effects: true,
            colors: SliderColors::default(),
            length: None,
        }
    }
    /// `Orientation`.
    pub fn orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }
    /// `ThumbSize` (px along the axis).
    pub fn thumb_size(mut self, s: f32) -> Self {
        self.thumb_size = s.max(1.0);
        self
    }
    /// `ThumbRoundRectSize` (single radius-ish value; C# size/2 is the corner radius).
    pub fn thumb_round(mut self, s: f32) -> Self {
        self.thumb_round = s.max(1.0);
        self
    }
    /// `BorderRoundRectSize`.
    pub fn border_round(mut self, s: f32) -> Self {
        self.border_round = s.max(1.0);
        self
    }
    /// `SmallChange` (arrow keys).
    pub fn small_change(mut self, v: u32) -> Self {
        self.small_change = v;
        self
    }
    /// `LargeChange` (PageUp/PageDown).
    pub fn large_change(mut self, v: u32) -> Self {
        self.large_change = v;
        self
    }
    /// `MouseWheelBarPartitions`.
    pub fn wheel_partitions(mut self, v: i32) -> Self {
        self.wheel_partitions = v.max(1);
        self
    }
    /// `DrawFocusRectangle`.
    pub fn draw_focus_rect(mut self, v: bool) -> Self {
        self.draw_focus_rect = v;
        self
    }
    /// `DrawSemitransparentThumb`.
    pub fn semitransparent_thumb(mut self, v: bool) -> Self {
        self.semitransparent_thumb = v;
        self
    }
    /// `MouseEffects`.
    pub fn mouse_effects(mut self, v: bool) -> Self {
        self.mouse_effects = v;
        self
    }
    /// Explicit colors.
    pub fn colors(mut self, c: SliderColors) -> Self {
        self.colors = c;
        self
    }
    /// `ColorSchema` shortcut.
    pub fn schema(mut self, s: SliderSchema) -> Self {
        self.colors = SliderColors::schema(s);
        self
    }
    /// Length along the axis in points (default: available width / 100 for vertical).
    pub fn length(mut self, l: f32) -> Self {
        self.length = Some(l);
        self
    }

    /// Set value clamped to range; returns true if changed.
    fn set(&mut self, v: i32) -> bool {
        let v = v.clamp(self.min, self.max);
        let ch = v != *self.value;
        *self.value = v;
        ch
    }
}

/// Logical-frame helper: horizontal frame (a along axis, b across), mapped to screen.
struct Frame {
    origin: Pos2,
    vertical: bool,
}

impl Frame {
    /// Logical point to screen.
    fn p(&self, a: f32, b: f32) -> Pos2 {
        if self.vertical {
            Pos2::new(self.origin.x + b, self.origin.y + a)
        } else {
            Pos2::new(self.origin.x + a, self.origin.y + b)
        }
    }
}

impl Widget for ColorSlider<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let vertical = self.orientation == Orientation::Vertical;
        let enabled = ui.is_enabled();
        let thick = 24.0;
        let size = match (vertical, self.length) {
            (false, l) => Vec2::new(l.unwrap_or_else(|| ui.available_width().clamp(60.0, 300.0)), thick),
            (true, l) => Vec2::new(thick, l.unwrap_or(100.0)),
        };
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let len = if vertical { rect.height() } else { rect.width() };
        let tk = if vertical { rect.width() } else { rect.height() };
        let thumb = self.thumb_size.min(len - 1.0).max(1.0);
        let range = (self.max - self.min) as f32;
        let mut changed = false;

        // mouse down/drag -> value from pointer
        if enabled && (resp.is_pointer_button_down_on() || resp.dragged()) {
            if let Some(pt) = resp.interact_pointer_pos() {
                let a = if vertical { pt.y - rect.top() } else { pt.x - rect.left() };
                let margin = (thumb / 2.0).floor();
                let coef = range / (len - 2.0 * margin).max(1.0);
                changed |= self.set(((a - margin) * coef + self.min as f32) as i32);
            }
            resp.request_focus();
        }
        // mouse wheel
        if enabled && resp.hovered() {
            let notches = super::graph::wheel_notches(ui);
            if notches != 0 {
                let step = (self.max - self.min) / self.wheel_partitions;
                // C#: value up with wheel up for horizontal; same sign for vertical
                changed |= self.set(*self.value + notches * step);
            }
        }
        // keyboard
        if enabled && resp.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    resp.id,
                    egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() },
                )
            });
            let (sc, lc) = (self.small_change as i32, self.large_change as i32);
            let cur = *self.value;
            let nv = ui.input(|i| {
                if i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::ArrowLeft) {
                    Some(cur - sc)
                } else if i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::ArrowRight) {
                    Some(cur + sc)
                } else if i.key_pressed(Key::Home) {
                    Some(self.min)
                } else if i.key_pressed(Key::End) {
                    Some(self.max)
                } else if i.key_pressed(Key::PageDown) {
                    Some(cur - lc)
                } else if i.key_pressed(Key::PageUp) {
                    Some(cur + lc)
                } else {
                    None
                }
            });
            if let Some(v) = nv {
                changed |= self.set(v);
            }
        }
        if changed {
            resp.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            self.paint(ui, rect, &resp, enabled, len, tk, thumb);
        }
        // tooltip = value (C# RiseChanged)
        resp.on_hover_text(self.value.to_string())
    }
}

impl ColorSlider<'_> {
    /// Paint bar, elapsed part, thumb, focus rectangle.
    #[allow(clippy::too_many_arguments)]
    fn paint(&self, ui: &Ui, rect: Rect, resp: &Response, enabled: bool, len: f32, tk: f32, thumb: f32) {
        let vertical = self.orientation == Orientation::Vertical;
        let painter = ui.painter_at(rect);
        let dragging = resp.dragged() || resp.is_pointer_button_down_on();
        let colors = if !enabled {
            self.colors.desaturated()
        } else if self.mouse_effects && resp.hovered() {
            self.colors.lightened()
        } else {
            self.colors
        };
        let f = Frame { origin: rect.min, vertical };
        let range = (self.max - self.min) as f32;
        let track = ((*self.value - self.min) as f32 * (len - thumb) / range).floor();
        // logical rects: (x0,y0,x1,y1) in (axis, across)
        let thumb_r = (track, 1.0, track + thumb - 1.0, 1.0 + tk - 3.0);
        let bar_y0 = (tk / 3.0).floor();
        let bar = (1.0, bar_y0, len - 1.0, tk - bar_y0);
        let elapsed_end = (thumb_r.0 + (thumb / 2.0).floor()).min(bar.2);
        let quad = |x0: f32, y0: f32, x1: f32, y1: f32| -> Vec<Pos2> {
            vec![f.p(x0, y0), f.p(x1, y0), f.p(x1, y1), f.p(x0, y1)]
        };
        // gradient is across the axis; compute y in screen along "across" => for vertical, gradient runs in x.
        let grad = |pts: &[Pos2], t0: f32, h: f32, o: Color32, i: Color32| {
            if vertical {
                // swap to reuse y-gradient: rotate points (x<->y), paint via transposed coords then un-transpose
                let tp: Vec<Pos2> = pts.iter().map(|q| Pos2::new(q.y, q.x)).collect();
                paint_transposed(&painter, &tp, rect.left() + t0, h, o, i);
            } else {
                fill_poly_mirror_gradient(&painter, pts, rect.top() + t0, h, o, i);
            }
        };
        // bar
        let bar_h = bar.3 - bar.1;
        grad(&quad(bar.0, bar.1, bar.2, bar.3), bar.1, bar_h, colors.bar_outer, colors.bar_inner);
        // elapsed (excluding thumb while dragging semitransparent)
        let ex_end = if dragging && self.semitransparent_thumb { elapsed_end.min(thumb_r.0) } else { elapsed_end };
        if ex_end > bar.0 {
            grad(&quad(bar.0, bar.1, ex_end, bar.3), bar.1, bar_h, colors.elapsed_outer, colors.elapsed_inner);
        }
        // bar band
        let q = quad(bar.0, bar.1, bar.2, bar.3);
        painter.add(Shape::closed_line(q, Stroke::new(0.5, colors.bar_pen)));
        // thumb
        let r = (self.thumb_round / 2.0).max(1.0);
        let tr = Rect::from_min_max(f.p(thumb_r.0, thumb_r.1), f.p(thumb_r.2, thumb_r.3));
        let tr = Rect::from_two_pos(tr.min, tr.max);
        let pts = rounded_rect_points(tr, r);
        let (mut to, mut ti) = (colors.thumb_outer, colors.thumb_inner);
        if dragging && self.semitransparent_thumb {
            to = Color32::from_rgba_unmultiplied(to.r(), to.g(), to.b(), 175);
            ti = Color32::from_rgba_unmultiplied(ti.r(), ti.g(), ti.b(), 175);
        }
        let (t_top, t_h) = if vertical { (tr.left() - rect.left(), tr.width()) } else { (tr.top() - rect.top(), tr.height()) };
        grad(&pts, t_top, t_h, to, ti);
        let mut pen = colors.thumb_pen;
        let over_thumb = resp.hover_pos().is_some_and(|p| tr.contains(p));
        if self.mouse_effects && (dragging || over_thumb) {
            pen = control_paint_dark(pen);
        }
        painter.add(Shape::closed_line(pts, Stroke::new(1.0, pen)));
        // focus rectangle (dotted)
        if self.draw_focus_rect && resp.has_focus() && enabled {
            let fr = Rect::from_min_max(rect.min + Vec2::new(1.0, 0.0), rect.max - Vec2::new(1.0, 1.0));
            let mut pts = rounded_rect_points(fr, (self.border_round / 2.0).max(1.0));
            pts.push(pts[0]);
            let c = colors.bar_pen;
            painter.extend(Shape::dotted_line(&pts, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 200), 3.0, 0.6));
        }
    }
}

/// Mirror gradient where input points are transposed (gradient axis is screen x). Draw by
/// building the mesh in transposed space then swapping coordinates back.
fn paint_transposed(painter: &egui::Painter, tp: &[Pos2], top: f32, height: f32, outer: Color32, inner: Color32) {
    use super::graph::{clip_poly_y, lerp_color};
    let mid = top + height / 2.0;
    let half = (height / 2.0).max(1.0);
    let part = |poly: Vec<Pos2>, y0: f32, c0: Color32, c1: Color32| {
        if poly.len() < 3 {
            return;
        }
        let mut mesh = egui::Mesh::default();
        for q in &poly {
            mesh.colored_vertex(Pos2::new(q.y, q.x), lerp_color(c0, c1, (q.y - y0) / half));
        }
        for i in 1..(poly.len() as u32 - 1) {
            mesh.add_triangle(0, i, i + 1);
        }
        painter.add(Shape::mesh(mesh));
    };
    // here poly.y is the screen-x gradient axis
    let tp_swapped: Vec<Pos2> = tp.iter().map(|q| Pos2::new(q.x, q.y)).collect();
    part(clip_poly_y(&tp_swapped, mid, true), top, outer, inner);
    part(clip_poly_y(&tp_swapped, mid, false), mid, inner, outer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desaturate_is_gray() {
        let c = SliderColors::default().desaturated();
        assert_eq!(c.bar_outer.r(), c.bar_outer.g());
        assert_eq!(SliderColors::schema(SliderSchema::PerlRedCoral).bar_outer, RED);
    }
}
