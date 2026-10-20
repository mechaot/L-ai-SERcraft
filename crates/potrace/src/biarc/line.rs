//! Port of `BezierToBiarc/Line.cs`: line in point-slope form, f32.

use super::vector2::Vector2;

/// Line `y - y1 = m * (x - x1)`; vertical: m = NaN; horizontal: m = 0.
#[derive(Debug, Clone, Copy)]
pub struct Line {
    /// Slope.
    pub m: f32,
    /// Point on the line.
    pub p: Vector2,
}

/// `Slope(P1, P2)`: NaN for vertical. Returns the slope.
fn slope(p1: Vector2, p2: Vector2) -> f32 {
    if p2.x == p1.x { f32::NAN } else { (p2.y - p1.y) / (p2.x - p1.x) }
}

impl Line {
    /// Ctor `Line(P1, P2)`: line through two points. Returns the line.
    pub fn from_points(p1: Vector2, p2: Vector2) -> Self {
        Line { p: p1, m: slope(p1, p2) }
    }

    /// Ctor `Line(P, m)`: point and slope. Returns the line.
    pub const fn new(p: Vector2, m: f32) -> Self {
        Line { p, m }
    }

    /// `Intersection(l)`: crossing point with another line. Returns the point.
    pub fn intersection(&self, l: Line) -> Vector2 {
        if self.m.is_nan() {
            vertical_intersection(*self, l)
        } else if l.m.is_nan() {
            vertical_intersection(l, *self)
        } else {
            let x = (self.m * self.p.x - l.m * l.p.x - self.p.y + l.p.y) / (self.m - l.m);
            let y = self.m * x - self.m * self.p.x + self.p.y;
            Vector2::new(x, y)
        }
    }

    /// `CreatePerpendicularAt(P, P1)`: perpendicular to P-P1 through P. Returns the line.
    pub fn create_perpendicular_at(p: Vector2, p1: Vector2) -> Line {
        let m = slope(p, p1);
        if m == 0.0 {
            Line::new(p, f32::NAN)
        } else if m.is_nan() {
            Line::new(p, 0.0)
        } else {
            Line::new(p, -1.0f32 / m)
        }
    }
}

/// `VerticalIntersection`: `vl` is vertical, `l` is not. Returns the crossing.
fn vertical_intersection(vl: Line, l: Line) -> Vector2 {
    let x = vl.p.x;
    let y = l.m * (x - l.p.x) + l.p.y;
    Vector2::new(x, y)
}
