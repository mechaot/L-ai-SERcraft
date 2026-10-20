//! Ports `Tools/Graph.cs` (rounded rect) plus small GDI+-like drawing helpers shared by widgets.

use eframe::egui::{self, Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, Ui};

/// GDI+ `Graph.RoundedRect`: filled/stroked rounded rectangle path as polygon points.
///
/// `bounds`: rectangle. `radius`: corner radius (0 = plain rect). Returns closed outline points (clockwise).
pub fn rounded_rect_points(bounds: Rect, radius: f32) -> Vec<Pos2> {
    if radius <= 0.0 {
        return vec![bounds.left_top(), bounds.right_top(), bounds.right_bottom(), bounds.left_bottom()];
    }
    let r = radius.min(bounds.width() / 2.0).min(bounds.height() / 2.0);
    let mut pts = Vec::new();
    // (corner center, start angle) in screen space, clockwise from top-left
    let corners = [
        (Pos2::new(bounds.left() + r, bounds.top() + r), 180.0_f32),
        (Pos2::new(bounds.right() - r, bounds.top() + r), 270.0),
        (Pos2::new(bounds.right() - r, bounds.bottom() - r), 0.0),
        (Pos2::new(bounds.left() + r, bounds.bottom() - r), 90.0),
    ];
    for (c, start) in corners {
        for i in 0..=6 {
            let a = (start + 15.0 * i as f32).to_radians();
            pts.push(Pos2::new(c.x + r * a.cos(), c.y + r * a.sin()));
        }
    }
    pts
}

/// Fill a rounded rect (egui native rounding).
///
/// `p`: painter. `bounds`: rect. `radius`: corner radius. `color`: fill.
pub fn fill_rounded(p: &Painter, bounds: Rect, radius: f32, color: Color32) {
    p.rect_filled(bounds, radius, color);
}

/// Stroke a rounded rect outline.
///
/// `p`: painter. `bounds`: rect. `radius`: corner radius. `stroke`: pen.
pub fn stroke_rounded(p: &Painter, bounds: Rect, radius: f32, stroke: Stroke) {
    p.rect_stroke(bounds, radius, stroke, egui::StrokeKind::Inside);
}

/// Linear color blend (no gamma).
///
/// `a`,`b`: ends. `t`: 0..1. Returns blended color (alpha blended too).
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// Clip convex polygon by horizontal line (Sutherland-Hodgman, one edge).
///
/// `pts`: polygon. `y`: line. `keep_above`: keep part with smaller y if true. Returns clipped polygon.
pub fn clip_poly_y(pts: &[Pos2], y: f32, keep_above: bool) -> Vec<Pos2> {
    let inside = |p: Pos2| if keep_above { p.y <= y } else { p.y >= y };
    let mut out = Vec::new();
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        match (inside(a), inside(b)) {
            (true, true) => out.push(b),
            (true, false) | (false, true) => {
                let t = (y - a.y) / (b.y - a.y);
                out.push(Pos2::new(a.x + (b.x - a.x) * t, y));
                if inside(b) {
                    out.push(b);
                }
            }
            (false, false) => {}
        }
    }
    out
}

/// Fill convex polygon with a linear gradient along y: `c0` at `y0`, `c1` at `y1`.
///
/// `p`: painter. `pts`: convex polygon. `y0`,`y1`: gradient span. `c0`,`c1`: end colors.
pub fn fill_poly_gradient_y(p: &Painter, pts: &[Pos2], y0: f32, y1: f32, c0: Color32, c1: Color32) {
    if pts.len() < 3 {
        return;
    }
    let span = if (y1 - y0).abs() < 1e-3 { 1.0 } else { y1 - y0 };
    let mut mesh = Mesh::default();
    for q in pts {
        mesh.colored_vertex(*q, lerp_color(c0, c1, (q.y - y0) / span));
    }
    for i in 1..(pts.len() as u32 - 1) {
        mesh.add_triangle(0, i, i + 1);
    }
    p.add(Shape::mesh(mesh));
}

/// GDI+ `LinearGradientBrush(half-rect, outer, inner)` with `WrapMode.TileFlipXY` over a full shape:
/// top->middle goes outer->inner, middle->bottom goes inner->outer.
///
/// `p`: painter. `pts`: convex polygon. `top`,`height`: logical extent of the whole shape. `outer`,`inner`: colors.
pub fn fill_poly_mirror_gradient(p: &Painter, pts: &[Pos2], top: f32, height: f32, outer: Color32, inner: Color32) {
    let mid = top + height / 2.0;
    let half = (height / 2.0).max(1.0);
    fill_poly_gradient_y(p, &clip_poly_y(pts, mid, true), top, top + half, outer, inner);
    fill_poly_gradient_y(p, &clip_poly_y(pts, mid, false), mid, mid + half, inner, outer);
}

/// Mouse wheel notches this frame (+ = up/away). Line unit = 1 per line, point unit = 50 px per notch,
/// page unit = 3 notches; every wheel event counts at least one notch.
///
/// `ui`: ui to read input from. Returns signed notch count.
pub fn wheel_notches(ui: &Ui) -> i32 {
    ui.input(|i| {
        i.raw
            .events
            .iter()
            .map(|e| match e {
                egui::Event::MouseWheel { unit, delta, .. } => {
                    let n = match unit {
                        egui::MouseWheelUnit::Line => delta.y,
                        egui::MouseWheelUnit::Point => delta.y / 50.0,
                        egui::MouseWheelUnit::Page => delta.y * 3.0,
                    };
                    if n == 0.0 { 0 } else { (n.abs().round().max(1.0) as i32) * n.signum() as i32 }
                }
                _ => 0,
            })
            .sum()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_points_inside_bounds() {
        let r = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(40.0, 20.0));
        for p in rounded_rect_points(r, 5.0) {
            assert!(r.expand(0.01).contains(p));
        }
        assert_eq!(rounded_rect_points(r, 0.0).len(), 4);
    }

    #[test]
    fn clip_halves() {
        let sq = [Pos2::new(0.0, 0.0), Pos2::new(10.0, 0.0), Pos2::new(10.0, 10.0), Pos2::new(0.0, 10.0)];
        let up = clip_poly_y(&sq, 5.0, true);
        assert!(up.iter().all(|p| p.y <= 5.0));
        assert_eq!(up.len(), 4);
    }
}
