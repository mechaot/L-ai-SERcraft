// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) László Domoszlai (Bezier2Biarc)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `BezierToBiarc/Vector2.cs`: f32 2D vector. Only members used by the port are ported.
//! C# `Tuple.cs` is replaced by Rust tuples.

use std::ops::{Add, Div, Mul, Neg, Sub};

/// Float 2D vector (`Vector2`, f32 like C#).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    /// Ctor `Vector2(x, y)`. Returns the vector.
    pub const fn new(x: f32, y: f32) -> Self {
        Vector2 { x, y }
    }

    /// `Length()`: sqrt in double, cast to float. Returns the length.
    pub fn length(&self) -> f32 {
        ((self.x as f64) * (self.x as f64) + (self.y as f64) * (self.y as f64)).sqrt() as f32
    }

    /// `Distance(a, b)`: float diff, double sum of squares. Returns distance.
    pub fn distance(a: Vector2, b: Vector2) -> f32 {
        let n1 = a.x - b.x;
        let n2 = a.y - b.y;
        ((n1 as f64) * (n1 as f64) + (n2 as f64) * (n2 as f64)).sqrt() as f32
    }
}

impl Add for Vector2 {
    type Output = Vector2;
    fn add(self, r: Vector2) -> Vector2 {
        Vector2::new(self.x + r.x, self.y + r.y)
    }
}

impl Sub for Vector2 {
    type Output = Vector2;
    fn sub(self, r: Vector2) -> Vector2 {
        Vector2::new(self.x - r.x, self.y - r.y)
    }
}

impl Mul<Vector2> for Vector2 {
    type Output = Vector2;
    fn mul(self, r: Vector2) -> Vector2 {
        Vector2::new(self.x * r.x, self.y * r.y)
    }
}

impl Mul<Vector2> for f32 {
    type Output = Vector2;
    fn mul(self, r: Vector2) -> Vector2 {
        Vector2::new(self, self) * r
    }
}

impl Mul<f32> for Vector2 {
    type Output = Vector2;
    fn mul(self, r: f32) -> Vector2 {
        self * Vector2::new(r, r)
    }
}

impl Div<f32> for Vector2 {
    type Output = Vector2;
    /// C# divides by multiplying with the reciprocal.
    fn div(self, d: f32) -> Vector2 {
        let num = 1.0f32 / d;
        Vector2::new(self.x * num, self.y * num)
    }
}

impl Neg for Vector2 {
    type Output = Vector2;
    fn neg(self) -> Vector2 {
        Vector2::default() - self
    }
}
