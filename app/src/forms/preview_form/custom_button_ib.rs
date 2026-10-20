//! UI side of `CustomButton.cs` / `PreviewForm.CustomButtonIB`: click/press/release semantics,
//! icon choice, PNG image rendering and the drag-reorder helpers.

use std::collections::HashMap;

use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, Response, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use lg_services::custom_button::{ButtonContext, ButtonTypes, CustomButton};

use crate::ui::icons::icons;
use crate::ui::theme::{change_color_brightness, Theme};
use crate::ui::widgets::image_button::add_brightness;

/// `CustomButtonIB` size (`Size = new Size(49, 49)`).
pub const BUTTON_SIZE: f32 = 49.0;
/// `CustomButtonIB` margin (`Margin = new Padding(2)`).
pub const BUTTON_MARGIN: f32 = 2.0;
/// C# `CAPTION_HEIGHT` of `ImageButton`.
const CAPTION_H: f32 = 12.0;

/// Per-button runtime state (C# fields `on`, `BackColor`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IbState {
    /// `on`: two-state button is latched.
    pub on: bool,
    /// Push button is held down (`BackColor` = pressed colour).
    pub pushed: bool,
    /// Last frame the pointer was down on this button (edge detection for MouseDown/Up).
    pub was_down: bool,
}

impl IbState {
    /// True when the button shows `ColorScheme.PressedButtons` as back colour.
    pub fn back_pressed(&self, ty: ButtonTypes) -> bool {
        match ty {
            ButtonTypes::TwoStateButton => self.on,
            ButtonTypes::PushButton => self.pushed,
            ButtonTypes::Button => false,
        }
    }
}

/// C# `DrawDisabled` / `RefreshEnabled`: `!EnabledNow(core) || PositionUnlocked`.
///
/// `cb`: button. `ctx`: core facts. `unlocked`: position unlocked (reorder mode). Returns true when drawn disabled.
pub fn is_disabled(cb: &CustomButton, ctx: &ButtonContext, unlocked: bool) -> bool {
    !cb.enabled_now(ctx) || unlocked
}

/// C# `PerformClick` (left button): `Button` runs `GCode`; `TwoStateButton` toggles and runs `GCode` / `GCode2`.
///
/// `cb`: button. `st`: its state. `ctx`: core facts. `unlocked`: reorder mode. Returns the code to run, if any.
pub fn perform_click(cb: &CustomButton, st: &mut IbState, ctx: &ButtonContext, unlocked: bool) -> Option<String> {
    // mDrawDisabled || !EnabledNow
    if is_disabled(cb, ctx, unlocked) {
        return None;
    }
    match cb.button_type {
        ButtonTypes::Button => Some(cb.gcode.clone()),
        ButtonTypes::TwoStateButton => {
            st.on = !st.on;
            Some(if st.on { cb.gcode.clone() } else { cb.gcode2.clone() })
        }
        ButtonTypes::PushButton => None,
    }
}

/// C# `PerformMouseDown` (left button): `PushButton` runs `GCode` and latches the look.
///
/// Args as [`perform_click`]. Returns the code to run, if any.
pub fn perform_mouse_down(cb: &CustomButton, st: &mut IbState, ctx: &ButtonContext, unlocked: bool) -> Option<String> {
    if is_disabled(cb, ctx, unlocked) {
        return None;
    }
    if cb.button_type == ButtonTypes::PushButton {
        st.pushed = true;
        return Some(cb.gcode.clone());
    }
    None
}

/// C# `PerformMouseUp` (left button): `PushButton` runs `GCode2` and releases the look.
///
/// Args as [`perform_click`]. Returns the code to run, if any.
pub fn perform_mouse_up(cb: &CustomButton, st: &mut IbState, ctx: &ButtonContext, unlocked: bool) -> Option<String> {
    if is_disabled(cb, ctx, unlocked) {
        return None;
    }
    if cb.button_type == ButtonTypes::PushButton {
        st.pushed = false;
        return Some(cb.gcode2.clone());
    }
    None
}

/// Icon key the non-legacy `RefreshCustomButtons` picks from the caption.
///
/// `caption`: button caption. `legacy`: legacy icon set (no replacement). `has_icon`: icon-set lookup.
/// Returns the key or None (then the button PNG is used).
pub fn icon_key_for_caption(caption: &str, legacy: bool, has_icon: impl Fn(&str) -> bool) -> Option<String> {
    if legacy {
        return None;
    }
    // the five standard buttons first, then any mdi icon named like the caption
    let std = match caption {
        "Frame" => Some("custom-framing"),
        "Center" => Some("custom-center"),
        "Corner" => Some("custom-corner"),
        "Focus" => Some("custom-focus"),
        "Blink" => Some("custom-blink"),
        _ => None,
    };
    let mdi = format!("mdi-{caption}");
    // C# applies the mdi check after the switch, so it wins when both exist
    if has_icon(&mdi) {
        return Some(mdi);
    }
    std.map(str::to_string)
}

/// Drag-reorder insert index (C# `OnDragOver`).
///
/// `rects`: button rects in order. `pos`: pointer. Returns the index in `0..=rects.len()` where the dragged
/// button would be inserted (`len` = after the last), None when there is no button.
pub fn drop_index(rects: &[Rect], pos: Pos2) -> Option<usize> {
    let hit = |p: Pos2| rects.iter().position(|r| r.contains(p));
    // between two buttons: move 6 px to the left and retry, then take the nearest one
    let i = hit(pos).or_else(|| hit(Pos2::new(pos.x - 6.0, pos.y))).or_else(|| {
        rects
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.distance_sq_to_pos(pos).total_cmp(&b.1.distance_sq_to_pos(pos)))
            .map(|(i, _)| i)
    })?;
    let r = rects[i];
    let pct = (pos.x - r.left()) / r.width() * 100.0;
    // right half: insert after
    Some(if pct > 50.0 { i + 1 } else { i })
}

/// Texture cache for button PNGs (C# `Image` objects), keyed by guid.
#[derive(Default)]
pub struct ImageCache {
    map: HashMap<String, (usize, Option<TextureHandle>)>,
}

impl ImageCache {
    /// Forget all textures (after the list changed).
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Texture of the button image.
    ///
    /// `ctx`: egui context. `cb`: button. Returns the texture or None when the button has no decodable image.
    pub fn get(&mut self, ctx: &egui::Context, cb: &CustomButton) -> Option<TextureHandle> {
        let len = cb.image.as_ref().map_or(0, Vec::len);
        // rebuild when the image bytes changed size (edit dialog)
        if self.map.get(&cb.guid).is_none_or(|(l, _)| *l != len) {
            let tex = cb.image_rgba().map(|img| {
                let ci = ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
                ctx.load_texture(format!("custombutton:{}", cb.guid), ci, TextureOptions::LINEAR)
            });
            self.map.insert(cb.guid.clone(), (len, tex));
        }
        self.map.get(&cb.guid).and_then(|(_, t)| t.clone())
    }
}

/// Look of a PNG button (the `ImageButton.OnPaint` port for raw images).
pub struct PngButton<'a> {
    /// Image texture, None = blank button.
    pub tex: Option<&'a TextureHandle>,
    /// Caption under the image, empty = none.
    pub caption: &'a str,
    /// Draw in disabled look.
    pub disabled: bool,
    /// `BackColor` override (pressed look).
    pub back: Option<Color32>,
    /// Click sense.
    pub sense: Sense,
}

impl PngButton<'_> {
    /// Paint and interact.
    ///
    /// `ui`: parent ui. Returns the response of the 49x49 rect.
    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(BUTTON_SIZE), self.sense);
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let th = Theme::current();
        let hot = resp.hovered() && !self.disabled;
        let down = hot && resp.is_pointer_button_down_on();
        let painter = ui.painter_at(rect);
        let mut inner = Rect::from_min_size(rect.min, rect.size() - Vec2::splat(1.0));
        if down {
            inner = inner.shrink(1.0);
        }
        if icons().is_legacy() {
            // legacy: plain image, caption squeezes it, 1px border (BorderStyle.FixedSingle)
            let mut img = inner;
            if !self.caption.is_empty() {
                img = Rect::from_min_size(inner.min, inner.size() - Vec2::splat(CAPTION_H));
                img = img.translate(Vec2::new((inner.width() - img.width()) / 2.0, 0.0));
            }
            if let Some(b) = self.back {
                painter.rect_filled(rect, 0.0, b);
            }
            if let Some(t) = self.tex {
                let tint = if self.disabled { Color32::from_gray(110) } else if hot { Color32::from_gray(255) } else { Color32::from_gray(235) };
                painter.image(t.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
            }
            painter.rect_stroke(inner, 0.0, Stroke::new(1.0, th.controls_border), egui::StrokeKind::Inside);
        } else {
            let dir = if th.is_dark { 1.0 } else { -1.0 };
            let mut border = th.controls_border;
            let mut tint = Color32::WHITE;
            if self.disabled {
                border = th.disabled_buttons;
                tint = th.disabled_buttons;
            } else if hot {
                let k = if down { 0.3 } else { 0.2 };
                border = change_color_brightness(border, k * dir);
                tint = add_brightness(tint, k * dir);
            }
            painter.rect_filled(rect, 0.0, th.form_back_color);
            painter.rect_filled(inner, 7.0, border);
            let mut body = inner.shrink(3.0);
            painter.rect_filled(body, 5.0, self.back.unwrap_or(th.form_back_color));
            body = body.shrink(1.0);
            if !self.caption.is_empty() {
                body.max.y -= CAPTION_H;
            }
            if let Some(t) = self.tex {
                let side = body.width().min(body.height());
                let r = Rect::from_center_size(body.center(), Vec2::splat(side));
                painter.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
            }
        }
        if !self.caption.is_empty() {
            let y = rect.bottom() - CAPTION_H - 4.0;
            let r = Rect::from_min_max(Pos2::new(rect.left(), y), rect.right_bottom());
            painter.text(r.center(), egui::Align2::CENTER_CENTER, self.caption, egui::FontId::proportional(9.0), th.form_fore_color);
        }
        resp
    }
}

/// `OnPaint` overlay of the unlocked (reorder) mode: translucent outline plus diagonal hatch.
///
/// `ui`: parent ui. `rect`: button rect.
pub fn paint_unlocked_overlay(ui: &Ui, rect: Rect) {
    let p = ui.painter_at(rect);
    let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(135, 206, 250, 150));
    let r = Rect::from_min_size(rect.min, rect.size() - Vec2::splat(1.0));
    p.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Inside);
    // DrawLine(i, 0, 0, i) for i in 0..2*max step 5
    let max = 2.0 * r.width().max(r.height());
    let mut i = 0.0;
    while i < max {
        p.line_segment([rect.min + Vec2::new(i, 0.0), rect.min + Vec2::new(0.0, i)], stroke);
        i += 5.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lg_services::custom_button::EnableStyles;

    fn ctx(connected: bool) -> ButtonContext {
        ButtonContext { is_connected: connected, ..Default::default() }
    }

    fn button(ty: ButtonTypes) -> CustomButton {
        let mut b = CustomButton::with_style(EnableStyles::Always);
        b.button_type = ty;
        b.gcode = "ON".into();
        b.gcode2 = "OFF".into();
        b
    }

    #[test]
    fn plain_button_runs_gcode_on_click() {
        let b = button(ButtonTypes::Button);
        let mut st = IbState::default();
        assert_eq!(perform_click(&b, &mut st, &ctx(false), false).as_deref(), Some("ON"));
        // press/release do nothing for a plain button
        assert_eq!(perform_mouse_down(&b, &mut st, &ctx(false), false), None);
        assert_eq!(perform_mouse_up(&b, &mut st, &ctx(false), false), None);
    }

    #[test]
    fn two_state_toggles() {
        let b = button(ButtonTypes::TwoStateButton);
        let mut st = IbState::default();
        assert_eq!(perform_click(&b, &mut st, &ctx(false), false).as_deref(), Some("ON"));
        assert!(st.back_pressed(b.button_type));
        assert_eq!(perform_click(&b, &mut st, &ctx(false), false).as_deref(), Some("OFF"));
        assert!(!st.back_pressed(b.button_type));
    }

    #[test]
    fn push_button_down_up() {
        let b = button(ButtonTypes::PushButton);
        let mut st = IbState::default();
        assert_eq!(perform_mouse_down(&b, &mut st, &ctx(false), false).as_deref(), Some("ON"));
        assert!(st.back_pressed(b.button_type));
        assert_eq!(perform_mouse_up(&b, &mut st, &ctx(false), false).as_deref(), Some("OFF"));
        assert!(!st.back_pressed(b.button_type));
        // click itself does nothing
        assert_eq!(perform_click(&b, &mut st, &ctx(false), false), None);
    }

    #[test]
    fn disabled_or_unlocked_blocks_everything() {
        let mut b = button(ButtonTypes::Button);
        b.set_enable_style(EnableStyles::Connected);
        let mut st = IbState::default();
        assert_eq!(perform_click(&b, &mut st, &ctx(false), false), None);
        assert!(is_disabled(&b, &ctx(false), false));
        assert_eq!(perform_click(&b, &mut st, &ctx(true), false).as_deref(), Some("ON"));
        // reorder mode: drawn disabled, nothing runs
        assert!(is_disabled(&b, &ctx(true), true));
        assert_eq!(perform_click(&b, &mut st, &ctx(true), true), None);
    }

    #[test]
    fn icon_choice() {
        let has = |k: &str| k == "mdi-flash";
        assert_eq!(icon_key_for_caption("Center", false, has).as_deref(), Some("custom-center"));
        assert_eq!(icon_key_for_caption("Frame", false, has).as_deref(), Some("custom-framing"));
        assert_eq!(icon_key_for_caption("flash", false, has).as_deref(), Some("mdi-flash"));
        assert_eq!(icon_key_for_caption("other", false, has), None);
        // legacy set keeps the PNG
        assert_eq!(icon_key_for_caption("Center", true, has), None);
    }

    #[test]
    fn drop_index_halves() {
        let rects: Vec<Rect> = (0..3).map(|i| Rect::from_min_size(Pos2::new(i as f32 * 53.0, 0.0), Vec2::splat(49.0))).collect();
        assert_eq!(drop_index(&rects, Pos2::new(10.0, 10.0)), Some(0));
        assert_eq!(drop_index(&rects, Pos2::new(40.0, 10.0)), Some(1));
        assert_eq!(drop_index(&rects, Pos2::new(53.0 + 10.0, 10.0)), Some(1));
        // right half of the last button: after the end
        assert_eq!(drop_index(&rects, Pos2::new(106.0 + 40.0, 10.0)), Some(3));
        // gap between buttons: shifted 6 px left lands in the previous one
        assert_eq!(drop_index(&rects, Pos2::new(51.0, 10.0)), Some(1));
        assert_eq!(drop_index(&[], Pos2::ZERO), None);
    }
}
