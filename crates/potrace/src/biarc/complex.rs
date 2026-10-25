// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) László Domoszlai (Bezier2Biarc)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `BezierToBiarc/Complex.cs` (arithmetic, Abs, Sqrt and friends; formatting and the
//! implicit conversions are not needed in Rust).

use std::ops::{Add, Div, Mul, Neg, Sub};

/// Complex number of two f64 (`Complex`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Complex {
    pub real: f64,
    pub imaginary: f64,
}

impl Complex {
    pub const ZERO: Complex = Complex { real: 0.0, imaginary: 0.0 };
    pub const ONE: Complex = Complex { real: 1.0, imaginary: 0.0 };
    pub const IMAGINARY_ONE: Complex = Complex { real: 0.0, imaginary: 1.0 };

    /// Ctor `Complex(real, imaginary)`. Returns the number.
    pub const fn new(real: f64, imaginary: f64) -> Self {
        Complex { real, imaginary }
    }

    /// Implicit conversion from double. `v` is the real part.
    pub const fn from_real(v: f64) -> Self {
        Complex { real: v, imaginary: 0.0 }
    }

    /// `Magnitude`: `Abs(self)`.
    pub fn magnitude(&self) -> f64 {
        Self::abs(*self)
    }

    /// `Phase`: atan2(im, re).
    pub fn phase(&self) -> f64 {
        self.imaginary.atan2(self.real)
    }

    /// `FromPolarCoordinates`. `magnitude` radius, `phase` angle. Returns the number.
    pub fn from_polar_coordinates(magnitude: f64, phase: f64) -> Self {
        Complex::new(magnitude * phase.cos(), magnitude * phase.sin())
    }

    /// `Abs` with the overflow-safe scaling of the original. `v` is the number.
    pub fn abs(v: Complex) -> f64 {
        if v.real.is_infinite() || v.imaginary.is_infinite() {
            return f64::INFINITY;
        }
        let n1 = v.real.abs();
        let n2 = v.imaginary.abs();
        if n1 > n2 {
            let n3 = n2 / n1;
            return n1 * (1.0 + n3 * n3).sqrt();
        }
        if n2 == 0.0 {
            return n1;
        }
        let n4 = n1 / n2;
        n2 * (1.0 + n4 * n4).sqrt()
    }

    /// `Conjugate`. `v` is the number.
    pub fn conjugate(v: Complex) -> Complex {
        Complex::new(v.real, -v.imaginary)
    }

    /// `Reciprocal`. `v` is the number. Zero maps to zero.
    pub fn reciprocal(v: Complex) -> Complex {
        if v.real == 0.0 && v.imaginary == 0.0 {
            return Complex::ZERO;
        }
        Complex::ONE / v
    }

    /// `Sqrt` via polar form. `v` is the number.
    pub fn sqrt(v: Complex) -> Complex {
        Complex::from_polar_coordinates(v.magnitude().sqrt(), v.phase() / 2.0)
    }

    /// `Exp`. `v` is the exponent.
    pub fn exp(v: Complex) -> Complex {
        let n = v.real.exp();
        Complex::new(n * v.imaginary.cos(), n * v.imaginary.sin())
    }

    /// `Log` (natural). `v` is the number.
    pub fn log(v: Complex) -> Complex {
        Complex::new(Complex::abs(v).ln(), v.imaginary.atan2(v.real))
    }
}

impl Neg for Complex {
    type Output = Complex;
    fn neg(self) -> Complex {
        Complex::new(-self.real, -self.imaginary)
    }
}

impl Add for Complex {
    type Output = Complex;
    fn add(self, r: Complex) -> Complex {
        Complex::new(self.real + r.real, self.imaginary + r.imaginary)
    }
}

impl Sub for Complex {
    type Output = Complex;
    fn sub(self, r: Complex) -> Complex {
        Complex::new(self.real - r.real, self.imaginary - r.imaginary)
    }
}

impl Mul for Complex {
    type Output = Complex;
    fn mul(self, r: Complex) -> Complex {
        Complex::new(
            self.real * r.real - self.imaginary * r.imaginary,
            self.imaginary * r.real + self.real * r.imaginary,
        )
    }
}

impl Div for Complex {
    type Output = Complex;
    fn div(self, r: Complex) -> Complex {
        let (re1, im1, re2, im2) = (self.real, self.imaginary, r.real, r.imaginary);
        if im2.abs() < re2.abs() {
            let num = im2 / re2;
            return Complex::new((re1 + im1 * num) / (re2 + im2 * num), (im1 - re1 * num) / (re2 + im2 * num));
        }
        let num1 = re2 / im2;
        Complex::new((im1 + re1 * num1) / (im2 + re2 * num1), (-re1 + im1 * num1) / (im2 + re2 * num1))
    }
}
