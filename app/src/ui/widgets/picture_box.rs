//! Ports `UserControls/MyPictureBox.cs`: image on a "paper" with a draggable selection fence.
//! Adds optional wheel zoom (egui extra).

use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2};

use crate::ui::theme::{named, Theme};

/// Mouse interaction stage (`MouseStage`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Stage {
    #[default]
    NoAction,
    Outside,
    Fence,
    Handle,
}

/// Picture box state. Fence is in control-local px.
#[derive(Clone, Debug)]
pub struct PictureBox {
    /// Selection rectangle in control-local coordinates (width 0 = none).
    pub fence: Rect,
    /// Wheel zoom factor (1 = fit).
    pub zoom: f32,
    stage: Stage,
    down_pt: Pos2,
    down_offset: Vec2,
}

/// `CalculateResizeToFit`.
///
/// `image`: image size. `boxs`: box size. Returns rounded fitted size.
pub fn resize_to_fit(image: Vec2, boxs: Vec2) -> Vec2 {
    let s = (boxs.x / image.x).min(boxs.y / image.y);
    Vec2::new((image.x * s).round(), (image.y * s).round())
}

impl PictureBox {
    /// New box without fence.
    pub fn new() -> Self {
        PictureBox { fence: Rect::from_min_size(Pos2::ZERO, Vec2::ZERO), zoom: 1.0, stage: Stage::NoAction, down_pt: Pos2::ZERO, down_offset: Vec2::ZERO }
    }

    /// `PaperRectangle` in local coords: fitted image centered, inflated by -10.
    ///
    /// `image`: image size if any. `size`: control size. Returns paper rect.
    pub fn paper_rect(&self, image: Option<Vec2>, size: Vec2) -> Rect {
        let mut r = Rect::from_min_size(Pos2::ZERO, size);
        if let Some(img) = image {
            let s = (resize_to_fit(img, size) * self.zoom.max(0.01)).round();
            r = Rect::from_min_size(Pos2::new(((size.x - s.x) / 2.0).floor(), ((size.y - s.y) / 2.0).floor()), s);
        }
        r.expand(-10.0)
    }

    /// Handle rect (bottom-right corner, 10x10).
    fn handle(&self) -> Rect {
        let h = 5.0;
        Rect::from_min_size(Pos2::new(self.fence.right() - h, self.fence.bottom() - h), Vec2::splat(2.0 * h))
    }

    /// `GetMouseOverType`.
    fn over(&self, p: Pos2) -> Stage {
        if self.handle().contains(p) {
            Stage::Handle
        } else if self.fence.contains(p) {
            Stage::Fence
        } else {
            Stage::Outside
        }
    }

    /// `GetFenceRect`: normalized rect from origin + signed size.
    fn make_fence(loc: Pos2, sz: Vec2) -> Rect {
        let (mut x, mut y) = (loc.x, loc.y);
        let (mut w, mut h) = (sz.x, sz.y);
        if w < 0.0 {
            w = w.abs();
            x -= w;
        }
        if h < 0.0 {
            h = h.abs();
            y -= h;
        }
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
    }

    /// Draw and handle input.
    ///
    /// `ui`: parent. `texture`: image (None = empty paper). `size`: control size.
    /// Returns response; `changed()` while fence modified.
    pub fn show(&mut self, ui: &mut Ui, texture: Option<&TextureHandle>, size: Vec2) -> Response {
        if self.zoom <= 0.0 {
            self.zoom = 1.0;
        }
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let origin = rect.min.to_vec2();
        let img_size = texture.map(|t| t.size_vec2());
        // wheel zoom
        if resp.hovered() {
            let n = super::graph::wheel_notches(ui);
            if n != 0 {
                self.zoom = (self.zoom * 1.1_f32.powi(n)).clamp(0.2, 4.0);
            }
        }
        // mouse
        let local = resp.interact_pointer_pos().or(resp.hover_pos()).map(|p| p - origin);
        if resp.drag_started() {
            if let Some(p) = local {
                self.stage = self.over(p);
                match self.stage {
                    Stage::Outside => self.down_pt = p,
                    Stage::Fence | Stage::Handle => {
                        self.down_pt = self.fence.min;
                        self.down_offset = p - self.fence.min;
                    }
                    Stage::NoAction => {}
                }
            }
        }
        if resp.dragged() && self.stage != Stage::NoAction {
            if let Some(p) = local {
                match self.stage {
                    Stage::Outside => self.fence = Self::make_fence(self.down_pt, p - self.down_pt),
                    Stage::Fence => self.fence = Rect::from_min_size(p - self.down_offset, self.fence.size()),
                    Stage::Handle => {
                        self.fence = Rect::from_min_size(self.fence.min, Vec2::new((self.fence.left() - p.x).abs(), (self.fence.top() - p.y).abs()))
                    }
                    Stage::NoAction => {}
                }
                resp.mark_changed();
            }
        } else if let Some(p) = local {
            match self.over(p) {
                Stage::Handle => ui.ctx().set_cursor_icon(CursorIcon::ResizeNeSw),
                Stage::Fence if self.fence.width() > 0.0 => ui.ctx().set_cursor_icon(CursorIcon::PointingHand),
                _ => {}
            }
        }
        if resp.drag_stopped() {
            self.stage = Stage::NoAction;
        }

        if ui.is_rect_visible(rect) {
            let th = Theme::current();
            let p = ui.painter_at(rect);
            p.rect_filled(rect, 0.0, th.form_back_color);
            let paper = self.paper_rect(img_size, size).translate(origin);
            p.rect_filled(paper.translate(Vec2::splat(5.0)), 0.0, named::GRAY); // shadow
            p.rect_filled(paper, 0.0, Color32::WHITE);
            p.rect_stroke(paper, 0.0, Stroke::new(1.0, named::LIME_GREEN), StrokeKind::Inside);
            if let Some(t) = texture {
                let img = Rect::from_min_max(paper.min + Vec2::splat(1.0), paper.max);
                p.image(t.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
            if self.fence.width() > 0.0 {
                let f = self.fence.translate(origin);
                p.extend(egui::Shape::dashed_line_with_offset(&[f.left_top(), f.right_top(), f.right_bottom(), f.left_bottom(), f.left_top()], Stroke::new(1.0, Color32::RED), &[6.0], &[3.0], 0.0));
                p.rect_stroke(self.handle().translate(origin), 0.0, Stroke::new(2.0, named::LIME_GREEN), StrokeKind::Inside);
            }
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit() {
        assert_eq!(resize_to_fit(Vec2::new(200.0, 100.0), Vec2::new(100.0, 100.0)), Vec2::new(100.0, 50.0));
    }

    #[test]
    fn fence_normalizes() {
        let r = PictureBox::make_fence(Pos2::new(50.0, 50.0), Vec2::new(-20.0, -10.0));
        assert_eq!(r.min, Pos2::new(30.0, 40.0));
        assert_eq!(r.size(), Vec2::new(20.0, 10.0));
    }
}
