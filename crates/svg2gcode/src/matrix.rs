//! Port of WPF `System.Windows.Media.Matrix`, `Point`, `Vector` (WindowsBase) used by GCodeFromSVG.cs / BezierTools.cs.
//!
//! Keeps the WPF internal type flag (identity / translation / scaling / unknown) because
//! `MatrixUtil.MultiplyMatrix` and `MultiplyPoint` pick special-case code paths per flag.

/// `System.Windows.Point`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// `new Point(x, y)`. `x`,`y`: coordinates. Returns the point.
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}

/// `System.Windows.Vector`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

impl Vector {
    /// `new Vector(x, y)`. `x`,`y`: components. Returns the vector.
    pub fn new(x: f64, y: f64) -> Self {
        Vector { x, y }
    }
}

const IDENTITY: u8 = 0;
const TRANSLATION: u8 = 1;
const SCALING: u8 = 2;
const UNKNOWN: u8 = 4;
/// `DoubleUtil.DBL_EPSILON`.
const DBL_EPSILON: f64 = 2.220_446_049_250_313E-16;

/// Error for `Matrix.Invert` on a singular matrix (`InvalidOperationException`).
#[derive(Debug, PartialEq, Eq)]
pub struct NotInvertible;

/// WPF affine 2x3 matrix (row-vector convention).
#[derive(Clone, Copy, Debug)]
pub struct Matrix {
    m11: f64,
    m12: f64,
    m21: f64,
    m22: f64,
    off_x: f64,
    off_y: f64,
    ty: u8,
}

impl Default for Matrix {
    /// `new Matrix()` = distinguished identity (fields zero, type identity).
    fn default() -> Self {
        Matrix { m11: 0.0, m12: 0.0, m21: 0.0, m22: 0.0, off_x: 0.0, off_y: 0.0, ty: IDENTITY }
    }
}

impl Matrix {
    /// `new Matrix(m11, m12, m21, m22, offsetX, offsetY)`; derives the type flag like WPF.
    ///
    /// Args are the six matrix entries. Returns the matrix.
    pub fn new(m11: f64, m12: f64, m21: f64, m22: f64, off_x: f64, off_y: f64) -> Self {
        let mut m = Matrix { m11, m12, m21, m22, off_x, off_y, ty: UNKNOWN };
        m.derive_type();
        m
    }

    /// `DeriveMatrixType`: detect exact identity / scale / translate.
    fn derive_type(&mut self) {
        self.ty = IDENTITY;
        if self.m21 != 0.0 || self.m12 != 0.0 {
            self.ty = UNKNOWN;
            return;
        }
        if self.m11 != 1.0 || self.m22 != 1.0 {
            self.ty = SCALING;
        }
        if self.off_x != 0.0 || self.off_y != 0.0 {
            self.ty |= TRANSLATION;
        }
        if self.ty & (TRANSLATION | SCALING) == IDENTITY {
            self.ty = IDENTITY;
        }
    }

    /// Internal `SetMatrix` with explicit type.
    #[allow(clippy::too_many_arguments)]
    fn set(&mut self, m11: f64, m12: f64, m21: f64, m22: f64, off_x: f64, off_y: f64, ty: u8) {
        *self = Matrix { m11, m12, m21, m22, off_x, off_y, ty };
    }

    /// `SetIdentity()`.
    pub fn set_identity(&mut self) {
        self.ty = IDENTITY;
    }

    /// `IsIdentity`.
    pub fn is_identity(&self) -> bool {
        self.ty == IDENTITY
            || (self.m11 == 1.0 && self.m12 == 0.0 && self.m21 == 0.0 && self.m22 == 1.0 && self.off_x == 0.0 && self.off_y == 0.0)
    }

    /// `M11` getter.
    pub fn m11(&self) -> f64 {
        if self.ty == IDENTITY { 1.0 } else { self.m11 }
    }
    /// `M12` getter.
    pub fn m12(&self) -> f64 {
        if self.ty == IDENTITY { 0.0 } else { self.m12 }
    }
    /// `M21` getter.
    pub fn m21(&self) -> f64 {
        if self.ty == IDENTITY { 0.0 } else { self.m21 }
    }
    /// `M22` getter.
    pub fn m22(&self) -> f64 {
        if self.ty == IDENTITY { 1.0 } else { self.m22 }
    }
    /// `OffsetX` getter.
    pub fn offset_x(&self) -> f64 {
        if self.ty == IDENTITY { 0.0 } else { self.off_x }
    }
    /// `OffsetY` getter.
    pub fn offset_y(&self) -> f64 {
        if self.ty == IDENTITY { 0.0 } else { self.off_y }
    }

    /// `M11` setter. `v`: new value.
    pub fn set_m11(&mut self, v: f64) {
        if self.ty == IDENTITY {
            self.set(v, 0.0, 0.0, 1.0, 0.0, 0.0, SCALING);
        } else {
            self.m11 = v;
            if self.ty != UNKNOWN {
                self.ty |= SCALING;
            }
        }
    }
    /// `M12` setter. `v`: new value.
    pub fn set_m12(&mut self, v: f64) {
        if self.ty == IDENTITY {
            self.set(1.0, v, 0.0, 1.0, 0.0, 0.0, UNKNOWN);
        } else {
            self.m12 = v;
            self.ty = UNKNOWN;
        }
    }
    /// `M21` setter. `v`: new value.
    pub fn set_m21(&mut self, v: f64) {
        if self.ty == IDENTITY {
            self.set(1.0, 0.0, v, 1.0, 0.0, 0.0, UNKNOWN);
        } else {
            self.m21 = v;
            self.ty = UNKNOWN;
        }
    }
    /// `M22` setter. `v`: new value.
    pub fn set_m22(&mut self, v: f64) {
        if self.ty == IDENTITY {
            self.set(1.0, 0.0, 0.0, v, 0.0, 0.0, SCALING);
        } else {
            self.m22 = v;
            if self.ty != UNKNOWN {
                self.ty |= SCALING;
            }
        }
    }
    /// `OffsetX` setter. `v`: new value.
    pub fn set_offset_x(&mut self, v: f64) {
        if self.ty == IDENTITY {
            self.set(1.0, 0.0, 0.0, 1.0, v, 0.0, TRANSLATION);
        } else {
            self.off_x = v;
            if self.ty != UNKNOWN {
                self.ty |= TRANSLATION;
            }
        }
    }
    /// `OffsetY` setter. `v`: new value.
    pub fn set_offset_y(&mut self, v: f64) {
        if self.ty == IDENTITY {
            self.set(1.0, 0.0, 0.0, 1.0, 0.0, v, TRANSLATION);
        } else {
            self.off_y = v;
            if self.ty != UNKNOWN {
                self.ty |= TRANSLATION;
            }
        }
    }

    /// `MatrixUtil.MultiplyMatrix(ref m1, ref m2)`: m1 = m1 * m2 with WPF special cases.
    fn mul_in_place(m1: &mut Matrix, m2: &Matrix) {
        let (t1, t2) = (m1.ty, m2.ty);
        // identity cases
        if t2 == IDENTITY {
            return;
        }
        if t1 == IDENTITY {
            *m1 = *m2;
            return;
        }
        // translate on the right
        if t2 == TRANSLATION {
            m1.off_x += m2.off_x;
            m1.off_y += m2.off_y;
            if t1 != UNKNOWN {
                m1.ty |= TRANSLATION;
            }
            return;
        }
        // translate on the left
        if t1 == TRANSLATION {
            let (ox, oy) = (m1.off_x, m1.off_y);
            *m1 = *m2;
            m1.off_x = ox * m2.m11 + oy * m2.m21 + m2.off_x;
            m1.off_y = ox * m2.m12 + oy * m2.m22 + m2.off_y;
            m1.ty = if t2 == UNKNOWN { UNKNOWN } else { SCALING | TRANSLATION };
            return;
        }
        // scale / scale+translate combinations
        const ST: u8 = SCALING | TRANSLATION;
        match (t1, t2) {
            (SCALING, SCALING) => {
                m1.m11 *= m2.m11;
                m1.m22 *= m2.m22;
            }
            (SCALING, ST) => {
                m1.m11 *= m2.m11;
                m1.m22 *= m2.m22;
                m1.off_x = m2.off_x;
                m1.off_y = m2.off_y;
                m1.ty = ST;
            }
            (ST, SCALING) => {
                m1.m11 *= m2.m11;
                m1.m22 *= m2.m22;
                m1.off_x *= m2.m11;
                m1.off_y *= m2.m22;
            }
            (ST, ST) => {
                m1.m11 *= m2.m11;
                m1.m22 *= m2.m22;
                m1.off_x = m1.off_x * m2.m11 + m2.off_x;
                m1.off_y = m1.off_y * m2.m22 + m2.off_y;
            }
            _ => {
                // general case (type UNKNOWN on either side)
                *m1 = Matrix {
                    m11: m1.m11 * m2.m11 + m1.m12 * m2.m21,
                    m12: m1.m11 * m2.m12 + m1.m12 * m2.m22,
                    m21: m1.m21 * m2.m11 + m1.m22 * m2.m21,
                    m22: m1.m21 * m2.m12 + m1.m22 * m2.m22,
                    off_x: m1.off_x * m2.m11 + m1.off_y * m2.m21 + m2.off_x,
                    off_y: m1.off_x * m2.m12 + m1.off_y * m2.m22 + m2.off_y,
                    ty: UNKNOWN,
                };
            }
        }
    }

    /// `Matrix.Multiply(trans1, trans2)` (= `trans1 * trans2`). Returns the product.
    pub fn multiply(trans1: Matrix, trans2: Matrix) -> Matrix {
        let mut m = trans1;
        Self::mul_in_place(&mut m, &trans2);
        m
    }

    /// `this *= other` helper for the Append-style ops.
    fn append(&mut self, other: &Matrix) {
        Self::mul_in_place(self, other);
    }

    /// `Matrix.CreateRotationRadians(angle, cx, cy)`.
    fn rotation(angle: f64, cx: f64, cy: f64) -> Matrix {
        let (sin, cos) = (angle.sin(), angle.cos());
        let dx = cx * (1.0 - cos) + cy * sin;
        let dy = cy * (1.0 - cos) - cx * sin;
        let mut m = Matrix::default();
        m.set(cos, sin, -sin, cos, dx, dy, UNKNOWN);
        m
    }

    /// `Rotate(angle)`: degrees, appended.
    pub fn rotate(&mut self, angle: f64) {
        let a = angle % 360.0;
        let r = Self::rotation(a * (std::f64::consts::PI / 180.0), 0.0, 0.0);
        self.append(&r);
    }

    /// `RotateAt(angle, cx, cy)`: degrees about center, appended.
    pub fn rotate_at(&mut self, angle: f64, cx: f64, cy: f64) {
        let a = angle % 360.0;
        let r = Self::rotation(a * (std::f64::consts::PI / 180.0), cx, cy);
        self.append(&r);
    }

    /// `Scale(sx, sy)`: appended.
    pub fn scale(&mut self, sx: f64, sy: f64) {
        let mut s = Matrix::default();
        s.set(sx, 0.0, 0.0, sy, 0.0, 0.0, SCALING);
        self.append(&s);
    }

    /// `ScaleAt(sx, sy, cx, cy)`: appended.
    pub fn scale_at(&mut self, sx: f64, sy: f64, cx: f64, cy: f64) {
        let mut s = Matrix::default();
        s.set(sx, 0.0, 0.0, sy, cx - sx * cx, cy - sy * cy, SCALING | TRANSLATION);
        self.append(&s);
    }

    /// `Translate(ox, oy)`: appended.
    pub fn translate(&mut self, ox: f64, oy: f64) {
        if self.ty == IDENTITY {
            self.set(1.0, 0.0, 0.0, 1.0, ox, oy, TRANSLATION);
        } else if self.ty == UNKNOWN {
            self.off_x += ox;
            self.off_y += oy;
        } else {
            self.off_x += ox;
            self.off_y += oy;
            self.ty |= TRANSLATION;
        }
    }

    /// `Skew(skewX, skewY)`: degrees, appended.
    pub fn skew(&mut self, skew_x: f64, skew_y: f64) {
        let sx = skew_x % 360.0;
        let sy = skew_y % 360.0;
        let mut s = Matrix::default();
        let k = std::f64::consts::PI / 180.0;
        s.set(1.0, (sy * k).tan(), (sx * k).tan(), 1.0, 0.0, 0.0, UNKNOWN);
        self.append(&s);
    }

    /// `Determinant`.
    pub fn determinant(&self) -> f64 {
        match self.ty {
            IDENTITY | TRANSLATION => 1.0,
            SCALING | 3 => self.m11 * self.m22,
            _ => self.m11 * self.m22 - self.m12 * self.m21,
        }
    }

    /// `HasInverse`.
    pub fn has_inverse(&self) -> bool {
        self.determinant().abs() >= 10.0 * DBL_EPSILON
    }

    /// `Invert()`. Returns Err when singular (WPF throws InvalidOperationException).
    pub fn invert(&mut self) -> Result<(), NotInvertible> {
        let det = self.determinant();
        if det.abs() < 10.0 * DBL_EPSILON {
            return Err(NotInvertible);
        }
        match self.ty {
            IDENTITY => {}
            3 => {
                self.m11 = 1.0 / self.m11;
                self.m22 = 1.0 / self.m22;
                self.off_x = -self.off_x * self.m11;
                self.off_y = -self.off_y * self.m22;
            }
            SCALING => {
                self.m11 = 1.0 / self.m11;
                self.m22 = 1.0 / self.m22;
            }
            TRANSLATION => {
                self.off_x = -self.off_x;
                self.off_y = -self.off_y;
            }
            _ => {
                let inv = 1.0 / det;
                let (a, b, c, d) = (self.m11, self.m12, self.m21, self.m22);
                let (ox, oy) = (self.off_x, self.off_y);
                self.set(d * inv, -b * inv, -c * inv, a * inv, (c * oy - ox * d) * inv, (ox * b - a * oy) * inv, UNKNOWN);
            }
        }
        Ok(())
    }

    /// `Transform(Point)`: `MatrixUtil.MultiplyPoint`. `p`: input. Returns the mapped point.
    pub fn transform(&self, p: Point) -> Point {
        let (mut x, mut y) = (p.x, p.y);
        match self.ty {
            IDENTITY => {}
            TRANSLATION => {
                x += self.off_x;
                y += self.off_y;
            }
            SCALING => {
                x *= self.m11;
                y *= self.m22;
            }
            3 => {
                x *= self.m11;
                x += self.off_x;
                y *= self.m22;
                y += self.off_y;
            }
            _ => {
                let xadd = y * self.m21 + self.off_x;
                let yadd = x * self.m12 + self.off_y;
                x *= self.m11;
                x += xadd;
                y *= self.m22;
                y += yadd;
            }
        }
        Point { x, y }
    }

    /// `Transform(Vector)`: no translation. `v`: input. Returns the mapped vector.
    pub fn transform_vector(&self, v: Vector) -> Vector {
        let (mut x, mut y) = (v.x, v.y);
        match self.ty {
            IDENTITY | TRANSLATION => {}
            SCALING | 3 => {
                x *= self.m11;
                y *= self.m22;
            }
            _ => {
                let xadd = y * self.m21;
                let yadd = x * self.m12;
                x *= self.m11;
                x += xadd;
                y *= self.m22;
                y += yadd;
            }
        }
        Vector { x, y }
    }
}
