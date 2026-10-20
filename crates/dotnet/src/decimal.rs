//! `System.Decimal` formatting, conversions and rounding (.NET Fx semantics).

use rust_decimal::{Decimal, RoundingStrategy};

use crate::fmt::{format_with, Kind};
use crate::number::Number;

/// `decimal.ToString()`: keeps scale, no sign for zero.
///
/// `v`: value. Returns e.g. "1000.000".
pub fn fmt_decimal(v: Decimal) -> String {
    let m = v.mantissa();
    let scale = v.scale() as usize;
    let mut digits = m.unsigned_abs().to_string();
    // pad so there is at least one integer digit
    if digits.len() <= scale {
        digits = format!("{}{}", "0".repeat(scale + 1 - digits.len()), digits);
    }
    let split = digits.len() - scale;
    let mut s = String::new();
    if m < 0 {
        s.push('-');
    }
    s.push_str(&digits[..split]);
    if scale > 0 {
        s.push('.');
        s.push_str(&digits[split..]);
    }
    s
}

/// `decimal.ToString(format, InvariantCulture)`.
///
/// `v`: value. `fmt`: standard or custom format; "" and "G" = `fmt_decimal`.
/// Returns text. Panics on unsupported standard letters (C# FormatException).
pub fn fmt_decimal_custom(v: Decimal, fmt: &str) -> String {
    if fmt.is_empty() || fmt == "G" || fmt == "g" {
        return fmt_decimal(v);
    }
    let nb = move |_sig: usize| Number::from_decimal(v);
    format_with(&nb, &|_| true, fmt, Kind::Decimal)
}

/// Powers of ten as doubles (Fx `VarR8FromDec` table).
const POW10: [f64; 29] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22, 1e23, 1e24, 1e25, 1e26, 1e27, 1e28,
];

/// `(double)decimal`: (lo64 + hi32 * 2^64) / 10^scale, like native `VarR8FromDec`.
///
/// `d`: value. Returns nearest-ish double (not always correctly rounded, same as .NET).
pub fn decimal_to_f64(d: Decimal) -> f64 {
    let m = d.mantissa().unsigned_abs();
    let lo64 = (m & u64::MAX as u128) as u64;
    let hi32 = (m >> 64) as u32;
    let mut dbl = (lo64 as f64 + hi32 as f64 * 18_446_744_073_709_551_616.0) / POW10[d.scale() as usize];
    if d.is_sign_negative() {
        dbl = -dbl;
    }
    dbl
}

/// `(float)decimal`.
///
/// `d`: value.
pub fn decimal_to_f32(d: Decimal) -> f32 {
    decimal_to_f64(d) as f32
}

/// Port of oleaut `VarDecFromR8/R4` (used by `(decimal)double` / `(decimal)float`).
///
/// `input`: finite value (float widened to double). `float`: 7-digit variant, else 15.
/// Returns None on overflow. Tiny values give zero (scale 0).
fn var_dec_from_r(input: f64, float: bool) -> Option<Decimal> {
    let bits = input.to_bits();
    let neg = bits >> 63 != 0;
    let exp = ((bits >> 52) & 0x7FF) as i32 - 1022;
    // 2^-94 is the smallest value that can reach 0.5e-28
    if exp < -94 {
        return Some(Decimal::ZERO);
    }
    if exp > 96 {
        return None;
    }
    let mut dbl = input.abs();
    let digits_ = if float { 6 } else { 14 };
    let mut power = digits_ - ((exp * 19728) >> 16);
    if power >= 0 {
        // fewer digits than wanted: scale up
        if power > 28 {
            power = 28;
        }
        dbl *= POW10[power as usize];
    } else if power != -1 || dbl >= if float { 1e7 } else { 1e15 } {
        dbl /= POW10[(-power) as usize];
    } else {
        power = 0;
    }
    if dbl < if float { 1e6 } else { 1e14 } && power < 28 {
        dbl *= 10.0;
        power += 1;
    }
    // round to integer, ties to even
    let mut mant = dbl as u64;
    dbl -= mant as f64;
    if dbl > 0.5 || (dbl == 0.5 && mant & 1 != 0) {
        mant += 1;
    }
    if mant == 0 {
        return Some(Decimal::ZERO);
    }
    let mut m = mant as u128;
    if power < 0 {
        // add -power factors of ten
        m *= 10u128.pow((-power) as u32);
        power = 0;
    } else {
        // strip trailing zeros while scale stays >= 0
        while power > 0 && m.is_multiple_of(10) {
            m /= 10;
            power -= 1;
        }
    }
    let mm = m as i128;
    Some(Decimal::from_i128_with_scale(if neg { -mm } else { mm }, power as u32))
}

/// `(decimal)double`: oleaut algorithm (15 digits). None for NaN/Inf/overflow (C# throws).
///
/// `v`: source value.
pub fn try_f64_to_decimal(v: f64) -> Option<Decimal> {
    if !v.is_finite() {
        return None;
    }
    var_dec_from_r(v, false)
}

/// `(decimal)double`. Overflow/NaN (C# OverflowException) saturates: NaN -> 0, +-big -> +-MAX.
///
/// `v`: source value.
pub fn f64_to_decimal(v: f64) -> Decimal {
    match try_f64_to_decimal(v) {
        Some(d) => d,
        None if v.is_nan() => Decimal::ZERO,
        None if v < 0.0 => Decimal::MIN,
        None => Decimal::MAX,
    }
}

/// `(decimal)float`: oleaut algorithm (7 digits). None for NaN/Inf/overflow.
///
/// `v`: source value.
pub fn try_f32_to_decimal(v: f32) -> Option<Decimal> {
    if !v.is_finite() {
        return None;
    }
    var_dec_from_r(v as f64, true)
}

/// `(decimal)float`, saturating like `f64_to_decimal`.
///
/// `v`: source value.
pub fn f32_to_decimal(v: f32) -> Decimal {
    match try_f32_to_decimal(v) {
        Some(d) => d,
        None if v.is_nan() => Decimal::ZERO,
        None if v < 0.0 => Decimal::MIN,
        None => Decimal::MAX,
    }
}

/// `Math.Round(decimal, digits)`: banker's rounding.
///
/// `d`: value. `digits`: decimals (0..=28).
pub fn round_decimal(d: Decimal, digits: u32) -> Decimal {
    d.round_dp_with_strategy(digits, RoundingStrategy::MidpointNearestEven)
}

/// `Math.Round(decimal, digits, MidpointRounding.AwayFromZero)`.
///
/// `d`: value. `digits`: decimals (0..=28).
pub fn round_decimal_away(d: Decimal, digits: u32) -> Decimal {
    d.round_dp_with_strategy(digits, RoundingStrategy::MidpointAwayFromZero)
}

/// Fix zero-operand scale loss: C# keeps `max(scale)` for add/sub.
///
/// `r`: raw result. `a`/`b`: operands.
fn keep_add_scale(mut r: Decimal, a: Decimal, b: Decimal) -> Decimal {
    let want = a.scale().max(b.scale());
    if r.scale() < want {
        // widen scale; ignore failure (value too big to widen)
        let mut t = r;
        t.rescale(want);
        if t == r && t.scale() == want {
            r = t;
        }
    }
    r
}

/// `a + b` with C# scale rules. None on overflow.
///
/// `a`, `b`: operands.
pub fn dec_add(a: Decimal, b: Decimal) -> Option<Decimal> {
    a.checked_add(b).map(|r| keep_add_scale(r, a, b))
}

/// `a - b` with C# scale rules. None on overflow.
///
/// `a`, `b`: operands.
pub fn dec_sub(a: Decimal, b: Decimal) -> Option<Decimal> {
    a.checked_sub(b).map(|r| keep_add_scale(r, a, b))
}

/// `a * b` with C# scale rules (zero keeps `min(28, sa+sb)`). None on overflow.
///
/// `a`, `b`: operands.
pub fn dec_mul(a: Decimal, b: Decimal) -> Option<Decimal> {
    if a.is_zero() || b.is_zero() {
        // Empirical (Mono + Fx DecCalc): a zero product loses its scale when the other
        // operand's mantissa needs more than 64 bits.
        let other = if a.is_zero() { b } else { a };
        let big = !other.is_zero() && (other.mantissa().unsigned_abs() >> 64) != 0;
        let scale = if big { 0 } else { (a.scale() + b.scale()).min(28) };
        return Some(Decimal::from_i128_with_scale(0, scale));
    }
    a.checked_mul(b)
}

/// Raise scale of `r` up to `min_scale` when representable.
///
/// `r`: value. `min_scale`: wanted minimum scale (<= 28).
fn at_least_scale(r: Decimal, min_scale: u32) -> Decimal {
    if r.scale() >= min_scale {
        return r;
    }
    let mut t = r;
    t.rescale(min_scale);
    if t == r && t.scale() == min_scale { t } else { r }
}

/// `a / b` with C# scale rules: quotient (up to 28 decimals) with trailing zeros stripped, but an
/// exact quotient keeps at least scale `scale(a) - scale(b)`; a zero dividend gives zero with scale `max(0, scale(a) - scale(b))`.
/// None on overflow / divide by zero.
///
/// `a`: dividend. `b`: divisor.
pub fn dec_div(a: Decimal, b: Decimal) -> Option<Decimal> {
    let q = a.checked_div(b)?;
    if a.is_zero() {
        let pref = a.scale().saturating_sub(b.scale()).min(28);
        return Some(Decimal::from_i128_with_scale(0, pref));
    }
    if q.is_zero() {
        return Some(Decimal::ZERO);
    }
    let n = q.normalize();
    // exact quotient keeps preferred scale scale(a)-scale(b) (e.g. 1000.000 / 0.1 = 10000.00)
    let exact = q.checked_mul(b) == Some(a);
    if exact {
        let pref = a.scale().saturating_sub(b.scale()).min(28);
        return Some(at_least_scale(n, pref));
    }
    Some(n)
}

/// `a % b` (sign of dividend). `|a| < |b|` returns `a` unchanged, otherwise scale is
/// `max(scale(a), scale(b))`. None on divide by zero.
///
/// `a`: dividend. `b`: divisor.
pub fn dec_rem(a: Decimal, b: Decimal) -> Option<Decimal> {
    let r = a.checked_rem(b)?;
    if a.abs() < b.abs() {
        return Some(a);
    }
    Some(at_least_scale(r, a.scale().max(b.scale())))
}
