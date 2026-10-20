//! Text overlays shared by both panels (C# `DrawOverlay`, `MeasureText`, `FormatCoord`).

use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Vec2};
use lg_dotnet::fmt_float_custom;

use crate::ui::widgets::graph::rounded_rect_points;

/// Overlay font (C# `Courier New` 10).
pub fn font() -> FontId {
    FontId::monospace(11.0)
}

/// C# `string.Format("{0,10:######0.000}", v)`.
///
/// `v`: coordinate. Returns right aligned text, width 10.
pub fn format_coord(v: f32) -> String {
    format!("{:>10}", fmt_float_custom(v, "######0.000"))
}

/// C# `{0,6:####0.##}` (feed).
///
/// `v`: feed. Returns right aligned text, width 6.
pub fn format_feed(v: f32) -> String {
    format!("{:>6}", fmt_float_custom(v, "####0.##"))
}

/// C# `{0,6:##0.##}` (spindle).
///
/// `v`: spindle. Returns right aligned text, width 6.
pub fn format_spindle(v: f32) -> String {
    format!("{:>6}", fmt_float_custom(v, "##0.##"))
}

/// Draw a rounded translucent label with text; returns its text size (C# `MeasureText`: +10, +5).
///
/// `painter`: target. `point`: text top-left. `text`: label text. `back`: back color. `alpha`: back alpha.
/// `fore`: text color. Returns the size used by the box (text size + (10, 5)).
pub fn label(painter: &Painter, point: Pos2, text: &str, back: Color32, alpha: u8, fore: Color32) -> Vec2 {
    let galley = painter.layout_no_wrap(text.to_owned(), font(), fore);
    let size = galley.size() + Vec2::new(10.0, 5.0);
    box_at(painter, point, size, back, alpha);
    painter.galley(point, galley, fore);
    size
}

/// Measure a label without drawing.
///
/// `painter`: for fonts. `text`: text. Returns size incl. padding.
pub fn measure(painter: &Painter, text: &str) -> Vec2 {
    painter.layout_no_wrap(text.to_owned(), font(), Color32::WHITE).size() + Vec2::new(10.0, 5.0)
}

/// C# `DrawOverlay`: rounded rect (radius 7) at `(point - 5, size)` with alpha.
///
/// `painter`: target. `point`: text origin. `size`: box size. `color`: base color. `alpha`: 0..255.
pub fn box_at(painter: &Painter, point: Pos2, size: Vec2, color: Color32, alpha: u8) {
    let c = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
    let r = Rect::from_min_size(point - Vec2::splat(5.0), size);
    painter.add(egui::Shape::convex_polygon(rounded_rect_points(r, 7.0), c, egui::Stroke::NONE));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coord_format_width() {
        assert_eq!(format_coord(1.5), "     1.500");
        assert_eq!(format_coord(-12.3456), "   -12.346");
        assert_eq!(format_feed(1000.0), "  1000");
        assert_eq!(format_spindle(255.0), "   255");
    }
}
