// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) László Domoszlai (Bezier2Biarc)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `BezierToBiarc/CubicBezier.cs`.

use super::complex::Complex;
use super::vector2::Vector2;

/// Cubic bezier with f32 points.
#[derive(Debug, Clone, Copy)]
pub struct CubicBezier {
    /// Start point.
    pub p1: Vector2,
    /// End point.
    pub p2: Vector2,
    /// First control point.
    pub c1: Vector2,
    /// Second control point.
    pub c2: Vector2,
}

impl CubicBezier {
    /// Ctor `CubicBezier(P1, C1, C2, P2)`. Returns the curve.
    pub const fn new(p1: Vector2, c1: Vector2, c2: Vector2, p2: Vector2) -> Self {
        CubicBezier { p1, c1, p2, c2 }
    }

    /// `PointAt(t)`: parametric point, `t` in 0..1 (powers computed in double like C#).
    pub fn point_at(&self, t: f32) -> Vector2 {
        let omt = 1.0f32 - t; // float `1 - t`
        let td = t as f64;
        let omtd = omt as f64;
        let w0 = omtd.powf(3.0) as f32;
        let w1 = (3.0 * omtd.powf(2.0) * td) as f32;
        let w2 = ((3.0f32 * omt) as f64 * td.powf(2.0)) as f32;
        let w3 = td.powf(3.0) as f32;
        w0 * self.p1 + w1 * self.c1 + w2 * self.c2 + w3 * self.p2
    }

    /// `Split(t)`: split at `t` in 0..1. Returns both halves.
    pub fn split(&self, t: f32) -> (CubicBezier, CubicBezier) {
        let p0 = self.p1 + t * (self.c1 - self.p1);
        let p1 = self.c1 + t * (self.c2 - self.c1);
        let p2 = self.c2 + t * (self.p2 - self.c2);

        let p01 = p0 + t * (p1 - p0);
        let p12 = p1 + t * (p2 - p1);

        let dp = p01 + t * (p12 - p01);

        (CubicBezier::new(self.p1, p0, p01, dp), CubicBezier::new(dp, p12, p2, self.p2))
    }

    /// `InflexionPoints`: the two roots (may be complex). Returns `(t1, t2)`.
    pub fn inflexion_points(&self) -> (Complex, Complex) {
        let a = self.c1 - self.p1;
        let b = self.c2 - self.c1 - a;
        let c = self.p2 - self.c2 - a - 2.0f32 * b;

        let ca = Complex::new((b.x * c.y - b.y * c.x) as f64, 0.0);
        let cb = Complex::new((a.x * c.y - a.y * c.x) as f64, 0.0);
        let cc = Complex::new((a.x * b.y - a.y * b.x) as f64, 0.0);

        let four = Complex::from_real(4.0);
        let two = Complex::from_real(2.0);
        let disc = cb * cb - four * ca * cc;
        let t1 = (-cb + Complex::sqrt(disc)) / (two * ca);
        let t2 = (-cb - Complex::sqrt(cb * cb - four * ca * cc)) / (two * ca);
        (t1, t2)
    }
}
