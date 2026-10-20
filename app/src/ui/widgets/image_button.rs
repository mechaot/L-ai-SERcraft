//! Ports `UserControls/ImageButton.cs`: icon button with themed border, hover/pressed look, caption.

use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::ui::icons::{icons, Icons, Tint};
use crate::ui::theme::{change_color_brightness, Theme};

/// C# `CLICK_SCALE_IN_PIXEL`.
const CLICK_PX: f32 = 1.0;
/// C# `CAPTION_HEIGHT`.
const CAPTION_H: f32 = 12.0;

/// `ColorMatrix` brightness add (`ImageTransform.Brightness`): add `v*255` to each RGB channel.
///
/// `c`: color. `v`: -1..1. Returns shifted color.
pub fn add_brightness(c: Color32, v: f32) -> Color32 {
    let d = (v * 255.0) as i32;
    let f = |x: u8| (x as i32 + d).clamp(0, 255) as u8;
    Color32::from_rgba_unmultiplied(f(c.r()), f(c.g()), f(c.b()), c.a())
}

/// Icon button. `ui.add(ImageButton::new("mdi-play").size(vec2(32.,32.)))`; `.clicked()` as usual.
pub struct ImageButton<'a> {
    icon: &'a str,
    alt_icon: Option<&'a str>,
    use_alt: bool,
    caption: Option<&'a str>,
    size: Vec2,
    rounded_borders: bool,
    back_color: Option<Color32>,
    forced_disabled: bool,
}

impl<'a> ImageButton<'a> {
    /// New button.
    ///
    /// `icon`: icon key (e.g. "mdi-power-plug", "custom-home").
    pub fn new(icon: &'a str) -> Self {
        ImageButton {
            icon,
            alt_icon: None,
            use_alt: false,
            caption: None,
            size: Vec2::splat(32.0),
            rounded_borders: false,
            back_color: None,
            forced_disabled: false,
        }
    }
    /// `AltImage` key plus `UseAltImage` switch.
    pub fn alt(mut self, alt_icon: &'a str, use_alt: bool) -> Self {
        self.alt_icon = Some(alt_icon);
        self.use_alt = use_alt;
        self
    }
    /// `Caption` (small text under the icon).
    pub fn caption(mut self, c: &'a str) -> Self {
        self.caption = Some(c);
        self
    }
    /// Control size in points (`Size`).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// `RoundedBorders`.
    pub fn rounded_borders(mut self, v: bool) -> Self {
        self.rounded_borders = v;
        self
    }
    /// `BackColor` (default transparent).
    pub fn back_color(mut self, c: Color32) -> Self {
        self.back_color = Some(c);
        self
    }
    /// Force disabled look without disabling input (`DrawDisabled()` override, used by C# subclasses).
    pub fn draw_disabled(mut self, v: bool) -> Self {
        self.forced_disabled = v;
        self
    }
}

impl Widget for ImageButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let th = Theme::current();
        let ic = icons();
        let disabled = !ui.is_enabled() || self.forced_disabled;
        let hot = resp.hovered() && !disabled;
        let down = hot && resp.is_pointer_button_down_on();
        let key = if self.use_alt { self.alt_icon.unwrap_or(self.icon) } else { self.icon };
        let painter = ui.painter_at(rect);
        let mut pos = rect.min;
        let mut size = rect.size() - Vec2::splat(1.0); // C# Width-1 / Height-1
        if down {
            pos += Vec2::splat(CLICK_PX);
            size -= Vec2::splat(CLICK_PX * 2.0);
        }

        if ic.is_legacy() {
            // legacy: no tint; caption squeezes the image
            let mut img_rect = Rect::from_min_size(pos, size);
            if self.caption.is_some() {
                img_rect = Rect::from_min_size(pos, size - Vec2::splat(CAPTION_H));
                img_rect = img_rect.translate(Vec2::new((size.x - img_rect.width()) / 2.0, 0.0));
            }
            let tint = if disabled {
                Color32::from_gray(90)
            } else if down {
                Color32::from_gray(255)
            } else {
                Color32::WHITE
            };
            let mut img = ic.image_plain(ui.ctx(), key, img_rect.width().min(img_rect.height()));
            img = img.tint(tint);
            img.paint_at(ui, img_rect);
            if hot && !down {
                painter.rect_filled(img_rect, 0.0, Color32::from_white_alpha(18));
            }
            if let Some(c) = self.caption {
                paint_caption(ui, rect, c, th.form_fore_color);
            }
            return resp;
        }

        // SVG mode
        let dir = if th.is_dark { 1.0 } else { -1.0 };
        let base = Icons::color_for(key, th.is_dark);
        let mut border = base;
        let mut tint = base;
        if disabled {
            tint = th.disabled_buttons;
            border = th.disabled_buttons;
        } else if hot {
            let k = if down { 0.3 } else { 0.2 };
            border = change_color_brightness(border, k * dir);
            tint = add_brightness(tint, k * dir);
        }
        // clear with form back color (C# Graphics.Clear)
        painter.rect_filled(rect, 0.0, th.form_back_color);
        let mut inner = Rect::from_min_size(pos, size);
        if self.rounded_borders {
            painter.rect_filled(inner, 7.0, border);
            inner = inner.shrink(3.0);
            let back = self.back_color.unwrap_or(th.form_back_color);
            painter.rect_filled(inner, 5.0, back);
            inner = inner.shrink(1.0);
        } else if let Some(bc) = self.back_color {
            painter.rect_filled(inner, 5.0, bc);
        }
        let side = inner.width().min(inner.height());
        let c = inner.center();
        let img_rect = Rect::from_center_size(c, Vec2::splat(side));
        ic.image_tinted(ui.ctx(), key, side, Tint::Color(tint)).paint_at(ui, img_rect);
        if let Some(cap) = self.caption {
            paint_caption(ui, rect, cap, th.form_fore_color);
        }
        let _ = (Pos2::ZERO, Stroke::NONE);
        resp
    }
}

/// Draw the small caption at the bottom of the control.
fn paint_caption(ui: &Ui, rect: Rect, text: &str, color: Color32) {
    let y = rect.bottom() - CAPTION_H - 4.0;
    let r = Rect::from_min_max(Pos2::new(rect.left(), y), rect.right_bottom());
    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(9.0), color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_add() {
        assert_eq!(add_brightness(Color32::from_rgb(10, 250, 100), 0.2), Color32::from_rgb(61, 255, 151));
    }
}
