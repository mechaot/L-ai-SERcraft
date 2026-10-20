//! Themed port of the WinForms `TrackBar` (used by `JogForm` speed and `LabelTB` overrides) with ticks.

use eframe::egui::{self, Key, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

use super::color_slider::Orientation;
use crate::ui::theme::Theme;

/// `TickStyle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickStyle {
    None,
    TopLeft,
    BottomRight,
    Both,
}

/// TrackBar: `ui.add(TrackBar::new(&mut v, 10..=200).small_change(5).large_change(10).tick_frequency(10))`.
pub struct TrackBar<'a> {
    value: &'a mut i32,
    min: i32,
    max: i32,
    small: i32,
    large: i32,
    tick_freq: i32,
    tick_style: TickStyle,
    orientation: Orientation,
    length: Option<f32>,
}

impl<'a> TrackBar<'a> {
    /// New track bar.
    ///
    /// `value`: bound value. `range`: inclusive min..=max.
    pub fn new(value: &'a mut i32, range: std::ops::RangeInclusive<i32>) -> Self {
        TrackBar {
            value,
            min: *range.start(),
            max: (*range.end()).max(*range.start() + 1),
            small: 1,
            large: 5,
            tick_freq: 1,
            tick_style: TickStyle::BottomRight,
            orientation: Orientation::Horizontal,
            length: None,
        }
    }
    /// `SmallChange`.
    pub fn small_change(mut self, v: i32) -> Self {
        self.small = v;
        self
    }
    /// `LargeChange`.
    pub fn large_change(mut self, v: i32) -> Self {
        self.large = v;
        self
    }
    /// `TickFrequency`.
    pub fn tick_frequency(mut self, v: i32) -> Self {
        self.tick_freq = v.max(1);
        self
    }
    /// `TickStyle`.
    pub fn tick_style(mut self, s: TickStyle) -> Self {
        self.tick_style = s;
        self
    }
    /// `Orientation` (vertical: max at top, as WinForms TrackBar).
    pub fn orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }
    /// Length along axis.
    pub fn length(mut self, l: f32) -> Self {
        self.length = Some(l);
        self
    }
}

impl Widget for TrackBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let vertical = self.orientation == Orientation::Vertical;
        let th = Theme::current();
        let enabled = ui.is_enabled();
        let size = if vertical {
            Vec2::new(45.0, self.length.unwrap_or(100.0))
        } else {
            Vec2::new(self.length.unwrap_or_else(|| ui.available_width().clamp(60.0, 400.0)), 40.0)
        };
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let pad = 10.0;
        let (a0, a1) = if vertical { (rect.top() + pad, rect.bottom() - pad) } else { (rect.left() + pad, rect.right() - pad) };
        let range = (self.max - self.min) as f32;
        // value<->axis position; vertical has max at top
        let pos_of = |v: i32| {
            let t = (v - self.min) as f32 / range;
            if vertical { a1 - t * (a1 - a0) } else { a0 + t * (a1 - a0) }
        };
        let mut changed = false;
        let mut set = |this: &mut i32, v: i32| {
            let v = v.clamp(self.min, self.max);
            if v != *this {
                *this = v;
                changed = true;
            }
        };
        if enabled && (resp.dragged() || resp.is_pointer_button_down_on()) {
            if let Some(p) = resp.interact_pointer_pos() {
                let a = if vertical { p.y } else { p.x };
                let t = ((a - a0) / (a1 - a0)).clamp(0.0, 1.0);
                let t = if vertical { 1.0 - t } else { t };
                set(self.value, self.min + (t * range).round() as i32);
            }
            resp.request_focus();
        }
        if enabled && resp.hovered() {
            let n = super::graph::wheel_notches(ui);
            if n != 0 {
                // WinForms: wheel moves by 3 * small change (system scroll lines)
                set(self.value, *self.value + n * self.small * 3);
            }
        }
        if enabled && resp.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(resp.id, egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() })
            });
            let cur = *self.value;
            let nv = ui.input(|i| {
                if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowDown) {
                    Some(cur - self.small)
                } else if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowUp) {
                    Some(cur + self.small)
                } else if i.key_pressed(Key::PageDown) {
                    Some(cur - self.large)
                } else if i.key_pressed(Key::PageUp) {
                    Some(cur + self.large)
                } else if i.key_pressed(Key::Home) {
                    Some(self.min)
                } else if i.key_pressed(Key::End) {
                    Some(self.max)
                } else {
                    None
                }
            });
            if let Some(v) = nv {
                set(self.value, v);
            }
        }
        if changed {
            resp.mark_changed();
        }
        if ui.is_rect_visible(rect) {
            let p = ui.painter_at(rect);
            let col = if enabled { th.form_fore_color } else { th.disabled_buttons };
            let c = rect.center();
            let to = |a: f32, b: f32| if vertical { Pos2::new(b, a) } else { Pos2::new(a, b) };
            let cb = if vertical { c.x } else { c.y };
            // groove
            let groove = Rect::from_two_pos(to(a0, cb - 2.0), to(a1, cb + 2.0));
            p.rect_filled(groove, 1.0, th.log_back_color);
            p.rect_stroke(groove, 1.0, Stroke::new(1.0, th.controls_border), egui::StrokeKind::Inside);
            // ticks
            let mut v = self.min;
            while v <= self.max {
                let a = pos_of(v);
                if matches!(self.tick_style, TickStyle::TopLeft | TickStyle::Both) {
                    p.line_segment([to(a, cb - 9.0), to(a, cb - 6.0)], Stroke::new(1.0, col));
                }
                if matches!(self.tick_style, TickStyle::BottomRight | TickStyle::Both) {
                    p.line_segment([to(a, cb + 6.0), to(a, cb + 9.0)], Stroke::new(1.0, col));
                }
                v += self.tick_freq;
            }
            // thumb
            let a = pos_of(*self.value);
            let thumb = Rect::from_center_size(to(a, cb), if vertical { Vec2::new(18.0, 9.0) } else { Vec2::new(9.0, 18.0) });
            let fill = if resp.is_pointer_button_down_on() { th.pressed_buttons } else if resp.hovered() { th.menu_highlight_color() } else { th.form_buttons_color() };
            p.rect_filled(thumb, 2.0, fill);
            p.rect_stroke(thumb, 2.0, Stroke::new(1.0, if resp.has_focus() { th.link_color } else { col }), egui::StrokeKind::Inside);
        }
        resp.on_hover_text(self.value.to_string())
    }
}
