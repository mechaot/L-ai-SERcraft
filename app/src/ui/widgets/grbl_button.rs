//! Ports `UserControls/GrblButton.cs`: flat themed button, optional icon at left.

use eframe::egui::{self, Align2, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use crate::ui::icons::{icons, Icons, Tint};
use crate::ui::theme::{change_color_brightness, Theme};

/// Themed button. `ui.add(GrblButton::new("Connect").icon("mdi-power-plug"))`.
pub struct GrblButton {
    text: String,
    icon: Option<String>,
    min_size: Vec2,
}

impl GrblButton {
    /// New button.
    ///
    /// `text`: caption (strip mnemonics with `i18n::tr_plain`).
    pub fn new(text: impl Into<String>) -> Self {
        GrblButton { text: text.into(), icon: None, min_size: Vec2::new(60.0, 23.0) }
    }
    /// `Image`: icon key; drawn at (4,4) with edge `height-8`.
    pub fn icon(mut self, key: impl Into<String>) -> Self {
        self.icon = Some(key.into());
        self
    }
    /// Minimum size (`Size`).
    pub fn min_size(mut self, s: Vec2) -> Self {
        self.min_size = s;
        self
    }
}

impl Widget for GrblButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let th = Theme::current();
        let font = FontId::proportional(13.0);
        let galley = ui.painter().layout_no_wrap(self.text.clone(), font, th.form_fore_color);
        let h = self.min_size.y;
        let icon_w = if self.icon.is_some() { h - 8.0 } else { 0.0 };
        let w = (galley.size().x + icon_w + 16.0).max(self.min_size.x);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        if ui.is_rect_visible(rect) {
            let enabled = ui.is_enabled();
            let mut back = th.log_back_color;
            if resp.hovered() && enabled {
                back = change_color_brightness(back, -0.15);
            }
            if resp.is_pointer_button_down_on() && enabled {
                back = change_color_brightness(back, -0.15);
            }
            let p = ui.painter();
            p.rect_filled(rect, 0.0, th.form_back_color);
            let br = Rect::from_min_max(rect.min, rect.max - Vec2::splat(1.0));
            p.rect_filled(br, 5.0, back);
            p.rect_stroke(br, 5.0, Stroke::new(1.0, th.controls_border), StrokeKind::Inside);
            let mut start_x = rect.left();
            if let Some(k) = &self.icon {
                let side = h - 8.0;
                let r = Rect::from_min_size(rect.min + Vec2::splat(4.0), Vec2::splat(side));
                let col = if enabled { Icons::color_for(k, th.is_dark) } else { th.disabled_buttons };
                icons().image_tinted(ui.ctx(), k, side, Tint::Color(col)).paint_at(ui, r);
                start_x = rect.left() + h - 8.0;
            }
            let cx = start_x + (rect.right() - start_x) / 2.0;
            let col = if enabled { th.form_fore_color } else { th.disabled_buttons };
            p.text(egui::pos2(cx, rect.center().y), Align2::CENTER_CENTER, &self.text, FontId::proportional(13.0), col);
            let _ = Color32::TRANSPARENT;
        }
        resp
    }
}
