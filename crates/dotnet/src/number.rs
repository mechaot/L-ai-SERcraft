// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Digit-string number model shared by all formatters (mirrors .NET Fx `NumberBuffer`).

/// Decimal digit buffer: value = 0.DIGITS * 10^scale.
#[derive(Debug, Clone)]
pub(crate) struct Number {
    /// Negative flag. Always false when `digits` is empty.
    pub neg: bool,
    /// ASCII digits, first is nonzero, no trailing zeros. Empty means zero.
    pub digits: Vec<u8>,
    /// Decimal point position relative to the first digit.
    pub scale: i32,
    /// Default significant digits for "G" without count (15 double, 7 float, ...).
    pub precision: i32,
}

impl Number {
    /// Zero value.
    ///
    /// `precision`: default "G" precision of the source type.
    pub fn zero(precision: i32) -> Self {
        Number { neg: false, digits: Vec::new(), scale: 0, precision }
    }

    /// Build from raw digit bytes and point position; normalizes zeros.
    ///
    /// `neg`: sign. `digits`: ASCII digits, may have leading/trailing zeros.
    /// `scale`: point position relative to first digit of `digits`.
    /// `precision`: default "G" precision.
    pub fn from_digits(neg: bool, mut digits: Vec<u8>, mut scale: i32, precision: i32) -> Self {
        // drop leading zeros, shift point
        let lead = digits.iter().take_while(|&&d| d == b'0').count();
        if lead > 0 {
            digits.drain(..lead);
            scale -= lead as i32;
        }
        // drop trailing zeros
        while digits.last() == Some(&b'0') {
            digits.pop();
        }
        if digits.is_empty() {
            return Number::zero(precision);
        }
        Number { neg, digits, scale, precision }
    }

    /// Finite double (or widened float) rounded to `sig` significant digits.
    ///
    /// `v`: finite value. `sig`: significant digits (15 / 7 / 17 / 9).
    /// `precision`: default "G" precision to store.
    pub fn from_f64(v: f64, sig: usize, precision: i32) -> Self {
        let neg = v.is_sign_negative();
        let a = v.abs();
        if a == 0.0 {
            return Number::zero(precision);
        }
        // correctly rounded sig+1 digits to detect possible midpoint
        let (mut digs, mut exp) = sci_digits(a, sig);
        let ext = sci_digits(a, sig + 1);
        if ext.0.last() == Some(&b'5') {
            // maybe exact tie: use exact expansion, round half up like native ecvt path
            let (full, fexp) = sci_digits(a, 800);
            let mut cut: Vec<u8> = full[..sig].to_vec();
            let mut e = fexp;
            if full[sig] >= b'5' {
                let mut i = sig;
                loop {
                    if i == 0 {
                        cut.insert(0, b'1');
                        cut.truncate(sig);
                        e += 1;
                        break;
                    }
                    i -= 1;
                    if cut[i] == b'9' {
                        cut[i] = b'0';
                    } else {
                        cut[i] += 1;
                        break;
                    }
                }
            }
            digs = cut;
            exp = e;
        }
        Number::from_digits(neg, digs, exp + 1, precision)
    }

    /// Integer magnitude with sign.
    ///
    /// `neg`: sign. `mag`: absolute value. `precision`: default "G" precision.
    pub fn from_u64(neg: bool, mag: u64, precision: i32) -> Self {
        let s = mag.to_string().into_bytes();
        let len = s.len() as i32;
        Number::from_digits(neg && mag != 0, s, len, precision)
    }

    /// Exact decimal.
    ///
    /// `d`: value (scale/trailing zeros are irrelevant for formatting).
    pub fn from_decimal(d: rust_decimal::Decimal) -> Self {
        let m = d.mantissa();
        let s = m.unsigned_abs().to_string().into_bytes();
        let len = s.len() as i32;
        Number::from_digits(m < 0, s, len - d.scale() as i32, 29)
    }
}

/// `sig` significant digits (rounded half-even by Rust, exact) and decimal exponent.
///
/// `a`: positive finite. `sig`: digit count.
/// Returns (ASCII digits of length `sig`, exponent of first digit).
fn sci_digits(a: f64, sig: usize) -> (Vec<u8>, i32) {
    let s = format!("{:.*e}", sig - 1, a);
    let (mant, exp) = s.split_once('e').expect("exp");
    let digits: Vec<u8> = mant.bytes().filter(|b| b.is_ascii_digit()).collect();
    (digits, exp.parse().expect("exp int"))
}

/// Round digit buffer at `pos` digits (half up on digit string). `Number::RoundNumber`.
///
/// `n`: buffer to round. `pos`: digits to keep (may be <= 0).
/// Zero result clears sign (Mono / .NET Fx observed: "-0.00" never printed for doubles).
pub(crate) fn round_number(n: &mut Number, pos: i32) {
    let len = n.digits.len() as i32;
    let mut i: i32 = 0;
    // walk to pos or end of digits
    while i < pos && i < len {
        i += 1;
    }
    if i == pos && i < len && n.digits[i as usize] >= b'5' {
        // round up with carry
        while i > 0 && n.digits[(i - 1) as usize] == b'9' {
            i -= 1;
        }
        if i > 0 {
            n.digits[(i - 1) as usize] += 1;
        } else {
            n.scale += 1;
            n.digits.clear();
            n.digits.push(b'1');
            i = 1;
        }
    } else {
        // plain truncate, strip trailing zeros
        while i > 0 && n.digits[(i - 1) as usize] == b'0' {
            i -= 1;
        }
    }
    n.digits.truncate(i.max(0) as usize);
    if n.digits.is_empty() {
        n.scale = 0;
        n.neg = false;
    }
}
