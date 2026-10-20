//! Port of the basic value types of `Clipper/Clipper.cs`: `IntPoint`, `IntRect`, `DoublePoint`,
//! `Int128` and the enums.

/// Clipper integer coordinate (`cInt`, 64 bit because `use_int32` is off).
pub type CInt = i64;
/// One polygon or polyline.
pub type Path = Vec<IntPoint>;
/// A set of paths.
pub type Paths = Vec<Path>;

/// Max abs coordinate before 128 bit math is needed (`ClipperBase.loRange`).
pub const LO_RANGE: CInt = 0x3FFF_FFFF;
/// Max abs coordinate allowed at all (`ClipperBase.hiRange`).
pub const HI_RANGE: CInt = 0x3FFF_FFFF_FFFF_FFFF;

/// Error thrown by the C# lib (`ClipperException`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipperException(pub String);

impl std::fmt::Display for ClipperException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ClipperException {}

/// C# `(long)double` unchecked cast: truncate, NaN / out of range gives `i64::MIN` (x64 cvttsd2si).
///
/// `v` is the double to convert. Returns the truncated integer.
pub fn f64_to_cint(v: f64) -> CInt {
    if v.is_nan() || v >= 9_223_372_036_854_775_808.0 || v < -9_223_372_036_854_775_808.0 {
        i64::MIN
    } else {
        v as i64
    }
}

/// `Clipper.Round`: round half away from zero, then C# cast to `cInt`.
///
/// `value` is the double to round. Returns the rounded coordinate.
pub fn round(value: f64) -> CInt {
    if value < 0.0 {
        f64_to_cint(value - 0.5)
    } else {
        f64_to_cint(value + 0.5)
    }
}

/// Integer point (`IntPoint`, no Z).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct IntPoint {
    pub x: CInt,
    pub y: CInt,
}

impl IntPoint {
    /// Ctor `IntPoint(cInt X, cInt Y)`. `x`, `y` are the coordinates.
    pub const fn new(x: CInt, y: CInt) -> Self {
        IntPoint { x, y }
    }

    /// Ctor `IntPoint(double x, double y)`: truncating cast (not rounding!).
    ///
    /// `x`, `y` are doubles, truncated toward zero. Returns the point.
    pub fn from_f64(x: f64, y: f64) -> Self {
        IntPoint { x: f64_to_cint(x), y: f64_to_cint(y) }
    }
}

/// Double point (`DoublePoint`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DoublePoint {
    pub x: f64,
    pub y: f64,
}

impl DoublePoint {
    /// Ctor `DoublePoint(double x, double y)`. `x`, `y` are the coordinates.
    pub const fn new(x: f64, y: f64) -> Self {
        DoublePoint { x, y }
    }

    /// Ctor `DoublePoint(IntPoint)`. `ip` is converted to double. Returns the point.
    pub fn from_int_point(ip: IntPoint) -> Self {
        DoublePoint { x: ip.x as f64, y: ip.y as f64 }
    }
}

/// Bounding rectangle (`IntRect`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IntRect {
    pub left: CInt,
    pub top: CInt,
    pub right: CInt,
    pub bottom: CInt,
}

impl IntRect {
    /// Ctor `IntRect(l, t, r, b)`. Args are the four edges. Returns the rect.
    pub const fn new(left: CInt, top: CInt, right: CInt, bottom: CInt) -> Self {
        IntRect { left, top, right, bottom }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipType {
    Intersection,
    Union,
    Difference,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PolyType {
    #[default]
    Subject,
    Clip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyFillType {
    EvenOdd,
    NonZero,
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Square,
    Round,
    Miter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndType {
    ClosedPolygon,
    ClosedLine,
    OpenButt,
    OpenSquare,
    OpenRound,
}

/// Signed 128 bit value (`Int128` struct): hi signed, lo unsigned. Only used for exact products.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Int128 {
    hi: i64,
    lo: u64,
}

impl Int128 {
    /// Ctor `Int128(Int64 hi, UInt64 lo)`. Returns the raw pair.
    pub(crate) const fn from_parts(hi: i64, lo: u64) -> Self {
        Int128 { hi, lo }
    }

    /// Ctor `Int128(Int64)`: sign extend. `v` is the value.
    #[allow(dead_code)]
    pub(crate) fn from_i64(v: i64) -> Self {
        Int128 { lo: v as u64, hi: if v < 0 { -1 } else { 0 } }
    }

    /// `IsNegative`. Returns true when the sign bit is set.
    #[allow(dead_code)]
    pub(crate) fn is_negative(self) -> bool {
        self.hi < 0
    }

    /// `operator +` with wrap. `rhs` is added. Returns the sum.
    pub(crate) fn add(self, rhs: Int128) -> Int128 {
        let mut r = self;
        r.hi = r.hi.wrapping_add(rhs.hi);
        r.lo = r.lo.wrapping_add(rhs.lo);
        // carry
        if r.lo < rhs.lo {
            r.hi = r.hi.wrapping_add(1);
        }
        r
    }

    /// Unary `operator -`. Returns the two's complement negation.
    pub(crate) fn neg(self) -> Int128 {
        if self.lo == 0 {
            Int128 { hi: self.hi.wrapping_neg(), lo: 0 }
        } else {
            Int128 { hi: !self.hi, lo: (!self.lo).wrapping_add(1) }
        }
    }

    /// `operator -`. `rhs` is subtracted. Returns the difference.
    #[allow(dead_code)]
    pub(crate) fn sub(self, rhs: Int128) -> Int128 {
        self.add(rhs.neg())
    }

    /// `explicit operator double`. Returns the value as f64.
    #[allow(dead_code)]
    pub(crate) fn to_f64(self) -> f64 {
        const SHIFT64: f64 = 18446744073709551616.0;
        if self.hi < 0 {
            if self.lo == 0 {
                (self.hi as f64) * SHIFT64
            } else {
                -((!self.lo) as f64 + (!self.hi) as f64 * SHIFT64)
            }
        } else {
            self.lo as f64 + self.hi as f64 * SHIFT64
        }
    }

    /// `Int128Mul`: exact 64x64 -> 128 signed product. `lhs`, `rhs` are the factors.
    pub(crate) fn mul(lhs: i64, rhs: i64) -> Int128 {
        let negate = (lhs < 0) != (rhs < 0);
        let lhs = if lhs < 0 { lhs.wrapping_neg() } else { lhs };
        let rhs = if rhs < 0 { rhs.wrapping_neg() } else { rhs };
        let int1_hi = (lhs as u64) >> 32;
        let int1_lo = (lhs as u64) & 0xFFFF_FFFF;
        let int2_hi = (rhs as u64) >> 32;
        let int2_lo = (rhs as u64) & 0xFFFF_FFFF;

        let a = int1_hi.wrapping_mul(int2_hi);
        let b = int1_lo.wrapping_mul(int2_lo);
        let c = int1_hi
            .wrapping_mul(int2_lo)
            .wrapping_add(int1_lo.wrapping_mul(int2_hi));

        let mut hi = a.wrapping_add(c >> 32) as i64;
        let lo = (c << 32).wrapping_add(b);
        if lo < b {
            hi = hi.wrapping_add(1);
        }
        let result = Int128::from_parts(hi, lo);
        if negate { result.neg() } else { result }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int128_mul_matches_i128() {
        let vals = [0i64, 1, -1, 7, -7, 1 << 40, -(1 << 40), i64::MAX, i64::MIN + 1, 123456789012345];
        for &a in &vals {
            for &b in &vals {
                let p = (a as i128) * (b as i128);
                let m = Int128::mul(a, b);
                let want = Int128::from_parts((p >> 64) as i64, p as u64);
                assert_eq!(m, want, "{a} * {b}");
            }
        }
    }

    #[test]
    fn round_and_cast() {
        assert_eq!(round(0.5), 1);
        assert_eq!(round(-0.5), -1);
        assert_eq!(round(2.4999), 2);
        assert_eq!(f64_to_cint(f64::NAN), i64::MIN);
        assert_eq!(f64_to_cint(-3.9), -3);
        assert_eq!(IntPoint::from_f64(1.9, -1.9), IntPoint::new(1, -1));
    }
}
