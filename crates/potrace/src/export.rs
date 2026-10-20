//! Port of `CsPotrace/CsPotraceExport.cs`: `Export2GDIPlus` rendered with tiny-skia.
//!
//! NOTE: GDI+ anti-aliasing (SmoothingMode.HighQuality) differs from tiny-skia, so the pixels are
//! similar but not identical. The commented-out SVG/Shrink helpers of the C# file are dead code
//! and not ported (SVG output is [`crate::Tracer::get_svg`]).

use crate::trace::{Curve, CurveKind};
use resvg::tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// `Export2GDIPlus`: draw the curve lists onto a pixmap.
///
/// `fig` curve lists; `pixmap` target; `fill` fill colour (None = no fill); `border` stroke colour
/// and width (None = no stroke); `inset` width of a white pen drawn over the outline (> 0).
pub fn export_to_gdiplus(
    fig: &[Vec<Curve>],
    pixmap: &mut Pixmap,
    fill: Option<Color>,
    border: Option<(Color, f32)>,
    inset: f64,
) {
    let mut pb = PathBuilder::new();
    for lc in fig {
        let mut last: Option<(f32, f32)> = None;
        for c in lc {
            let a = (c.a.x as f32, c.a.y as f32);
            // new figure starts at first point; later points join if they differ
            match last {
                None => pb.move_to(a.0, a.1),
                Some(l) if l != a => pb.line_to(a.0, a.1),
                _ => {}
            }
            let b = (c.b.x as f32, c.b.y as f32);
            if c.kind == CurveKind::Line {
                pb.line_to(b.0, b.1);
            } else {
                pb.cubic_to(
                    c.control_point_a.x as f32,
                    c.control_point_a.y as f32,
                    c.control_point_b.x as f32,
                    c.control_point_b.y as f32,
                    b.0,
                    b.1,
                );
            }
            last = Some(b);
        }
    }
    let Some(path) = pb.finish() else { return };

    if let Some(col) = fill {
        let mut paint = Paint::default();
        paint.set_color(col);
        paint.anti_alias = true;
        // GraphicsPath default FillMode.Alternate
        pixmap.fill_path(&path, &paint, FillRule::EvenOdd, Transform::identity(), None);
    }

    if let Some((col, width)) = border {
        let mut paint = Paint::default();
        paint.set_color(col);
        paint.anti_alias = true;
        let stroke = Stroke { width, ..Stroke::default() };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    if inset > 0.0 {
        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);
        paint.anti_alias = true;
        let stroke = Stroke { width: inset as f32, ..Stroke::default() };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}
