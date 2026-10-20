//! Port of `BezierToBiarc/Arc.cs` and `BiArc.cs`.

use super::line::Line;
use super::vector2::Vector2;

/// Circular arc with redundant info (`Arc`, f32 fields).
#[derive(Debug, Clone, Copy)]
pub struct Arc {
    /// Center.
    pub c: Vector2,
    /// Radius.
    pub r: f32,
    /// Start angle (rad).
    pub start_angle: f32,
    /// Sweep angle (rad).
    pub sweep_angle: f32,
    /// Start point.
    pub p1: Vector2,
    /// End point.
    pub p2: Vector2,
}

impl Arc {
    /// Ctor `Arc(C, r, startAngle, sweepAngle, P1, P2)`. Args map 1:1 to the fields.
    pub const fn new(c: Vector2, r: f32, start_angle: f32, sweep_angle: f32, p1: Vector2, p2: Vector2) -> Self {
        Arc { c, r, start_angle, sweep_angle, p1, p2 }
    }

    /// `IsClockwise`: sweep > 0.
    pub fn is_clockwise(&self) -> bool {
        self.sweep_angle > 0.0
    }

    /// `PointAt(t)`: point on the arc, `t` in 0..1.
    pub fn point_at(&self, t: f32) -> Vector2 {
        // angle sum is f32 arithmetic in C#, trig in double
        let ang = (self.start_angle + t * self.sweep_angle) as f64;
        let x = self.c.x as f64 + self.r as f64 * ang.cos();
        let y = self.c.y as f64 + self.r as f64 * ang.sin();
        Vector2::new(x as f32, y as f32)
    }

    /// `Length`: arc length (f32).
    pub fn length(&self) -> f32 {
        self.r * self.sweep_angle.abs()
    }

    /// `LinearLength`: chord length.
    /// NOTE(upstream): `dY = P2.Y - P2.Y` is always 0 (typo kept), so this is |dX| only.
    #[allow(clippy::eq_op)]
    pub fn linear_length(&self) -> f32 {
        let dx = self.p2.x - self.p1.x;
        let dy = self.p2.y - self.p2.y;
        ((dx * dx + dy * dy) as f64).sqrt() as f32
    }
}

/// Two arcs joined at a transition point (`BiArc`).
#[derive(Debug, Clone, Copy)]
pub struct BiArc {
    pub a1: Arc,
    pub a2: Arc,
}

impl BiArc {
    /// Ctor `BiArc(P1, T1, P2, T2, T)`: `p1` start, `t1` tangent at p1, `p2` end, `t2` tangent
    /// at p2, `t` transition point. Returns the biarc.
    pub fn new(p1: Vector2, t1: Vector2, p2: Vector2, t2: Vector2, t: Vector2) -> Self {
        // orientation
        let mut sum = 0.0f64;
        sum += ((t.x - p1.x) * (t.y + p1.y)) as f64;
        sum += ((p2.x - t.x) * (p2.y + t.y)) as f64;
        sum += ((p1.x - p2.x) * (p1.y + p2.y)) as f64;
        let cw = sum < 0.0;

        // perpendiculars to the tangents at P1 and P2
        let tl1 = Line::create_perpendicular_at(p1, p1 + t1);
        let tl2 = Line::create_perpendicular_at(p2, p2 + t2);

        // perpendicular bisectors of P1T and P2T
        let p1t2 = (p1 + t) / 2.0;
        let pb_p1t = Line::create_perpendicular_at(p1t2, t);

        let p2t2 = (p2 + t) / 2.0;
        let pb_p2t = Line::create_perpendicular_at(p2t2, t);

        // circle origins
        let c1 = tl1.intersection(pb_p1t);
        let c2 = tl2.intersection(pb_p2t);

        let r1 = (c1 - p1).length();
        let r2 = (c2 - p2).length();

        let sv1 = p1 - c1;
        let ev1 = t - c1;
        let start_angle1 = (sv1.y as f64).atan2(sv1.x as f64);
        let mut sweep_angle1 = (ev1.y as f64).atan2(ev1.x as f64) - start_angle1;

        let sv2 = t - c2;
        let ev2 = p2 - c2;
        let start_angle2 = (sv2.y as f64).atan2(sv2.x as f64);
        let mut sweep_angle2 = (ev2.y as f64).atan2(ev2.x as f64) - start_angle2;

        // adjust angles to orientation
        let two_pi = 2.0 * std::f64::consts::PI;
        if cw && sweep_angle1 < 0.0 {
            sweep_angle1 += two_pi;
        }
        if !cw && sweep_angle1 > 0.0 {
            sweep_angle1 -= two_pi;
        }
        if cw && sweep_angle2 < 0.0 {
            sweep_angle2 += two_pi;
        }
        if !cw && sweep_angle2 > 0.0 {
            sweep_angle2 -= two_pi;
        }

        BiArc {
            a1: Arc::new(c1, r1, start_angle1 as f32, sweep_angle1 as f32, p1, t),
            a2: Arc::new(c2, r2, start_angle2 as f32, sweep_angle2 as f32, t, p2),
        }
    }

    /// `PointAt(t)`: point on the biarc, `t` in 0..1.
    pub fn point_at(&self, t: f32) -> Vector2 {
        let s = self.a1.length() / (self.a1.length() + self.a2.length());
        if t <= s { self.a1.point_at(t / s) } else { self.a2.point_at((t - s) / (1.0 - s)) }
    }

    /// `Length`: total length.
    pub fn length(&self) -> f32 {
        self.a1.length() + self.a2.length()
    }
}
