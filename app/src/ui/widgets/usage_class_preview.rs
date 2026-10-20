//! Ports `UserControls/UsageClassPreview.cs`: 10-bar histogram of laser usage classes.

use eframe::egui::{Color32, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, Widget};

use crate::ui::theme::{change_color_brightness, named};

/// Bar colors (C# `mCol`).
const COLORS: [Color32; 10] = [
    Color32::from_rgb(0x22, 0x3D, 0xFA),
    Color32::from_rgb(0x6B, 0xB0, 0xFC),
    Color32::from_rgb(0x90, 0xFF, 0xE4),
    Color32::from_rgb(0x90, 0xFF, 0x43),
    Color32::from_rgb(0xC3, 0xFF, 0x3C),
    Color32::from_rgb(0xF8, 0xFF, 0x3C),
    Color32::from_rgb(0xED, 0xB6, 0x2B),
    Color32::from_rgb(0xE1, 0x70, 0x1A),
    Color32::from_rgb(0xDD, 0x3D, 0x07),
    Color32::from_rgb(0xDB, 0x00, 0x00),
];

/// Usage histogram. `ui.add(UsageClassPreview::new(&hours))`.
pub struct UsageClassPreview<'a> {
    /// Hours per class (`LaserLifeCounter.Classes[i].TotalHours`); empty = no counter.
    hours: &'a [f64],
    size: Vec2,
}

impl<'a> UsageClassPreview<'a> {
    /// New preview.
    ///
    /// `hours`: total hours of each of the 10 classes (0-10%, ... 90-100% power).
    pub fn new(hours: &'a [f64]) -> Self {
        UsageClassPreview { hours, size: Vec2::new(200.0, 100.0) }
    }
    /// Control size.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
}

/// Bar rectangles in control-local px (`ComputeRect`). Empty if no data.
///
/// `hours`: class hours. `w`,`h`: control size. Returns one rect per class (local coordinates).
pub fn compute_rects(hours: &[f64], w: f32, h: f32) -> Vec<Rect> {
    let max = hours.iter().cloned().fold(0.0, f64::max) * 1.05;
    if hours.is_empty() || max <= 0.0 {
        return Vec::new();
    }
    let cw = (w as f64 - 1.0) / hours.len() as f64;
    hours
        .iter()
        .enumerate()
        .map(|(i, hr)| {
            let bh = f32::max(2.0, (hr / max * h as f64) as f32);
            Rect::from_min_size(Pos2::new((i as f64 * cw) as f32, h - bh - 1.0), Vec2::new(cw as f32, bh))
        })
        .collect()
}

impl Widget for UsageClassPreview<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let p = ui.painter_at(rect);
            // dash-dot guide lines every 10%
            for i in 1..10 {
                let y = rect.top() + (rect.height() - 1.0) / 10.0 * i as f32;
                p.extend(Shape::dashed_line(&[Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(1.0, named::DARK_GRAY), 6.0, 3.0));
            }
            let rects = compute_rects(self.hours, rect.width(), rect.height());
            for (i, r) in rects.iter().enumerate() {
                let r = r.translate(rect.min.to_vec2());
                let c = COLORS[i % 10];
                p.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 200));
                p.rect_stroke(r, 0.0, Stroke::new(1.0, change_color_brightness(c, -0.3)), StrokeKind::Inside);
            }
            p.rect_stroke(rect, 0.0, Stroke::new(1.0, named::DARK_GRAY), StrokeKind::Inside);
            // tooltip text of the hovered bar
            if let Some(pos) = resp.hover_pos() {
                let local = pos - rect.min.to_vec2();
                if let Some(i) = rects.iter().position(|r| r.contains(local)) {
                    return resp.on_hover_text(format!("{}-{}%  {:.1} h", i * 10, (i + 1) * 10, self.hours[i]));
                }
            }
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rects() {
        let h = [1.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let r = compute_rects(&h, 101.0, 100.0);
        assert_eq!(r.len(), 10);
        assert!(r[1].height() > r[0].height());
        assert_eq!(r[2].height(), 2.0);
        assert!(compute_rects(&[0.0; 10], 100.0, 100.0).is_empty());
    }
}
