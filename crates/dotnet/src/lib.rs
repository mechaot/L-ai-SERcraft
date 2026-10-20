//! `lg-dotnet`: bit-exact .NET Framework 4.x number/time/char semantics for the LaserGRBL port.
//!
//! No C# file maps here directly; used wherever the original relies on `ToString`, `Parse`,
//! `Math.Round`, casts, `decimal`, `TimeSpan` or `Char.IsXxx`.

mod chars;
mod decimal;
mod fmt;
mod number;
mod parse;
mod timespan;
mod unicode_tables;

pub use chars::{is_digit, is_letter, is_number, is_white_space, split_keep_empty};
pub use decimal::{
    dec_add, dec_div, dec_mul, dec_rem, dec_sub, decimal_to_f32, decimal_to_f64, f32_to_decimal, f64_to_decimal, fmt_decimal, fmt_decimal_custom,
    round_decimal, round_decimal_away, try_f32_to_decimal, try_f64_to_decimal,
};
pub use parse::{parse_decimal, parse_double, parse_float, parse_int};
pub use rust_decimal::Decimal;
pub use timespan::TimeSpan;

use fmt::{format_integer, format_with, Kind};
use number::Number;

/// Shared special-value text for NaN / infinities.
///
/// `v`: value. Returns Some for non-finite.
fn non_finite(v: f64) -> Option<&'static str> {
    if v.is_nan() {
        Some("NaN")
    } else if v == f64::INFINITY {
        Some("Infinity")
    } else if v == f64::NEG_INFINITY {
        Some("-Infinity")
    } else {
        None
    }
}

/// True when digit buffer parses back to `v`.
///
/// `n`: buffer. `v`: source value. `float`: compare after narrowing to f32.
fn round_trips(n: &Number, v: f64, float: bool) -> bool {
    let text = format!("{}e{}", String::from_utf8_lossy(&n.digits), n.scale - n.digits.len() as i32);
    let Ok(p) = text.parse::<f64>() else {
        return false;
    };
    let p = if n.neg { -p } else { p };
    if float { (p as f32) as f64 == v } else { p == v }
}

/// `double.ToString(format, InvariantCulture)`; also standard formats (F2, N0, P1, G, E3, R).
///
/// `v`: value. `fmt`: .NET numeric format, "" = plain `ToString()`.
/// Returns text. Panics on unsupported standard letters (C# FormatException).
pub fn fmt_double_custom(v: f64, fmt: &str) -> String {
    if let Some(s) = non_finite(v) {
        return s.to_string();
    }
    let nb = move |sig: usize| Number::from_f64(v, sig, 15);
    format_with(&nb, &|n| round_trips(n, v, false), fmt, Kind::Double)
}

/// `double.ToString(InvariantCulture)`: 15 significant digits, `E+15` / `E-05` style.
///
/// `v`: value. Returns text; -0.0 prints "0".
pub fn fmt_double(v: f64) -> String {
    fmt_double_custom(v, "")
}

/// `float.ToString(format, InvariantCulture)`: digits generated with 7 significant digits.
///
/// `v`: value. `fmt`: .NET numeric format. Returns text.
pub fn fmt_float_custom(v: f32, fmt: &str) -> String {
    let d = v as f64;
    if let Some(s) = non_finite(d) {
        return s.to_string();
    }
    let nb = move |sig: usize| Number::from_f64(d, sig, 7);
    format_with(&nb, &|n| round_trips(n, d, true), fmt, Kind::Float)
}

/// `float.ToString(InvariantCulture)`: 7 significant digits.
///
/// `v`: value. Returns text.
pub fn fmt_float(v: f32) -> String {
    fmt_float_custom(v, "")
}

/// `double.ToString("F<decimals>", InvariantCulture)`.
///
/// `v`: value. `decimals`: digits after the point. Returns text.
pub fn fmt_fixed(v: f64, decimals: usize) -> String {
    fmt_double_custom(v, &format!("F{decimals}"))
}

/// `int.ToString(format, InvariantCulture)`; supports D, X, x, N, F, G, E, P and custom.
///
/// `v`: value. `fmt`: format ("" = plain). Returns text. Hex of negatives is 32-bit two's complement.
pub fn fmt_i32(v: i32, fmt: &str) -> String {
    format_integer(v < 0, v.unsigned_abs() as u64, v as u32 as u64, 10, fmt)
}

/// `long.ToString(format, InvariantCulture)`.
///
/// `v`: value. `fmt`: format. Returns text. Hex of negatives is 64-bit two's complement.
pub fn fmt_i64(v: i64, fmt: &str) -> String {
    format_integer(v < 0, v.unsigned_abs(), v as u64, 19, fmt)
}

/// `uint.ToString(format, InvariantCulture)` (also use for `byte`/`ushort`).
///
/// `v`: value. `fmt`: format. Returns text.
pub fn fmt_u32(v: u32, fmt: &str) -> String {
    format_integer(false, v as u64, v as u64, 10, fmt)
}

/// `ulong.ToString(format, InvariantCulture)`.
///
/// `v`: value. `fmt`: format. Returns text.
pub fn fmt_u64(v: u64, fmt: &str) -> String {
    format_integer(false, v, v, 20, fmt)
}

/// Powers of ten 1e0..1e15 (`Math.roundPower10Double`).
const ROUND_POW10: [f64; 16] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15,
];

/// `Math.Round(double)`: round half to even.
///
/// `v`: value. Returns rounded value.
pub fn round_half_even(v: f64) -> f64 {
    v.round_ties_even()
}

/// `Math.Round(double, digits)` = .NET Fx `InternalRound(ToEven)`:
/// scale by 10^digits, banker's round, unscale. Binary midpoints round on the scaled product,
/// so `round_double(2.675, 2) == 2.67`.
///
/// `v`: value. `digits`: 0..=15 (C# throws ArgumentOutOfRange; this panics).
pub fn round_double(v: f64, digits: i32) -> f64 {
    assert!((0..=15).contains(&digits), "Math.Round digits out of range");
    if v.abs() < 1e16 {
        let p = ROUND_POW10[digits as usize];
        let mut x = v * p;
        x = x.round_ties_even();
        return x / p;
    }
    v
}

/// `Math.Round(double, digits, MidpointRounding.AwayFromZero)` = .NET Fx `InternalRound(AwayFromZero)`:
/// scale, split off fraction (modf), add sign when |fraction| >= 0.5, unscale.
///
/// `v`: value. `digits`: 0..=15 (panics otherwise).
pub fn round_double_away(v: f64, digits: i32) -> f64 {
    assert!((0..=15).contains(&digits), "Math.Round digits out of range");
    if v.abs() < 1e16 {
        let p = ROUND_POW10[digits as usize];
        let mut x = v * p;
        // modf: integer part stays in x, fraction returned
        let ip = x.trunc();
        let fraction = x - ip;
        x = ip;
        if fraction.abs() >= 0.5 {
            x += if fraction > 0.0 { 1.0 } else { -1.0 };
        }
        return x / p;
    }
    v
}

/// C# unchecked `(int)double` on x86/x64: truncate; NaN/out of range give `i32::MIN`.
///
/// `v`: value. Returns the cast result.
pub fn f64_to_i32(v: f64) -> i32 {
    if v > -2_147_483_649.0 && v < 2_147_483_648.0 {
        v as i32
    } else {
        i32::MIN
    }
}

/// C# unchecked `(int)float`.
///
/// `v`: value. Returns the cast result.
pub fn f32_to_i32(v: f32) -> i32 {
    f64_to_i32(v as f64)
}

/// C# unchecked `(long)double`: truncate; NaN/out of range give `i64::MIN`.
///
/// `v`: value. Returns the cast result.
pub fn f64_to_i64(v: f64) -> i64 {
    if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&v) {
        v as i64
    } else {
        i64::MIN
    }
}

/// C# unchecked `(long)float`.
///
/// `v`: value. Returns the cast result.
pub fn f32_to_i64(v: f32) -> i64 {
    f64_to_i64(v as f64)
}
