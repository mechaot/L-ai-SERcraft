//! Number parsing (.NET Fx `Number.ParseNumber` + `NumberToDouble/Decimal`), InvariantCulture.

use rust_decimal::Decimal;

/// NumberStyles subset.
#[derive(Clone, Copy)]
struct Styles {
    lead_white: bool,
    trail_white: bool,
    lead_sign: bool,
    trail_sign: bool,
    point: bool,
    thousands: bool,
    exponent: bool,
}

/// NumberStyles.Float | AllowThousands (double/float default here).
const FLOAT: Styles = Styles {
    lead_white: true,
    trail_white: true,
    lead_sign: true,
    trail_sign: false,
    point: true,
    thousands: true,
    exponent: true,
};
/// NumberStyles.Integer.
const INTEGER: Styles = Styles {
    lead_white: true,
    trail_white: true,
    lead_sign: true,
    trail_sign: false,
    point: false,
    thousands: false,
    exponent: false,
};
/// NumberStyles.Number.
const NUMBER: Styles = Styles {
    lead_white: true,
    trail_white: true,
    lead_sign: true,
    trail_sign: true,
    point: true,
    thousands: true,
    exponent: false,
};
/// double.Parse(s, InvariantCulture) default = Float | AllowThousands (same as FLOAT).
const DOUBLE_DEFAULT: Styles = FLOAT;

/// Raw scan result.
struct Scanned {
    neg: bool,
    /// All digits seen (both sides of the point, leading zeros kept).
    digits: Vec<u8>,
    /// Digits before the point.
    point: i64,
    /// Explicit exponent.
    exp: i64,
}

/// .NET whitespace for number parsing (0x09..=0x0D, 0x20).
///
/// `c`: char to test.
fn is_white(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | ' ')
}

/// Scan `s` per `st`. None when the text is not a valid number.
///
/// `s`: input text. `st`: allowed syntax.
fn scan(s: &str, st: Styles) -> Option<Scanned> {
    let c: Vec<char> = s.chars().collect();
    let at = |i: usize| c.get(i).copied().unwrap_or('\0');
    let mut p = 0usize;
    // leading white
    while st.lead_white && p < c.len() && is_white(c[p]) {
        p += 1;
    }
    // leading sign
    let mut neg = false;
    let mut sign_seen = false;
    if st.lead_sign && (at(p) == '+' || at(p) == '-') {
        neg = at(p) == '-';
        sign_seen = true;
        p += 1;
    }
    let mut digits = Vec::new();
    let mut point: Option<usize> = None;
    let mut any = false;
    loop {
        let ch = at(p);
        if ch.is_ascii_digit() {
            digits.push(ch as u8);
            any = true;
        } else if ch == '.' && st.point && point.is_none() {
            point = Some(digits.len());
        } else if ch == ',' && st.thousands && any && point.is_none() {
            // group separator skipped
        } else {
            break;
        }
        p += 1;
    }
    if !any {
        return None;
    }
    let point_i = point.unwrap_or(digits.len()) as i64;
    // exponent
    let mut exp: i64 = 0;
    if st.exponent && (at(p) == 'e' || at(p) == 'E') {
        let mut q = p + 1;
        let mut eneg = false;
        if at(q) == '+' || at(q) == '-' {
            eneg = at(q) == '-';
            q += 1;
        }
        if at(q).is_ascii_digit() {
            let mut e: i64 = 0;
            while at(q).is_ascii_digit() {
                e = (e * 10 + (at(q) as i64 - '0' as i64)).min(100_000);
                q += 1;
            }
            exp = if eneg { -e } else { e };
            p = q;
        }
    }
    // trailing white / sign in any order
    loop {
        let ch = at(p);
        if st.trail_white && p < c.len() && is_white(ch) {
            p += 1;
        } else if st.trail_sign && !sign_seen && (ch == '+' || ch == '-') {
            neg = ch == '-';
            sign_seen = true;
            p += 1;
        } else {
            break;
        }
    }
    // trailing NULs are ignored by .NET
    while p < c.len() && c[p] == '\0' {
        p += 1;
    }
    if p != c.len() {
        return None;
    }
    Some(Scanned { neg, digits, point: point_i, exp })
}

/// Digit scan to f64 (correctly rounded). None on overflow to infinity.
///
/// `sc`: scanned number.
fn scanned_to_f64(sc: &Scanned) -> Option<f64> {
    let lead = sc.digits.iter().take_while(|&&d| d == b'0').count();
    if lead == sc.digits.len() {
        // .NET Fx: "-0" parses to +0
        return Some(0.0);
    }
    let mut ds: &[u8] = &sc.digits[lead..];
    while ds.last() == Some(&b'0') {
        ds = &ds[..ds.len() - 1];
    }
    // value = 0.ds * 10^(point - lead + exp) -> ds * 10^(that - len)
    let e10 = sc.point - lead as i64 + sc.exp - ds.len() as i64;
    let text = format!("{}e{}", std::str::from_utf8(ds).ok()?, e10.clamp(-1_000_000, 1_000_000));
    let v: f64 = text.parse().ok()?;
    if v.is_infinite() {
        return None;
    }
    Some(if sc.neg { -v } else { v })
}

/// Special strings accepted by double.Parse in Fx (case sensitive, trimmed).
///
/// `s`: input. Returns the value when `s` is Infinity / -Infinity / NaN.
fn special(s: &str) -> Option<f64> {
    match s.trim() {
        "Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        // .NET double.NaN has the sign bit set
        "NaN" => Some(f64::from_bits(0xFFF8_0000_0000_0000)),
        _ => None,
    }
}

/// `double.Parse(s, InvariantCulture)` (NumberStyles.Float | AllowThousands).
///
/// `s`: text. Returns None where .NET throws (format or overflow).
pub fn parse_double(s: &str) -> Option<f64> {
    match scan(s, DOUBLE_DEFAULT) {
        Some(sc) => scanned_to_f64(&sc),
        None => special(s),
    }
}

/// `float.Parse(s, InvariantCulture)`. .NET Fx parses as double then narrows.
///
/// `s`: text. Returns None where .NET throws.
pub fn parse_float(s: &str) -> Option<f32> {
    match scan(s, FLOAT) {
        Some(sc) => {
            let d = scanned_to_f64(&sc)?;
            let f = d as f32;
            if f.is_infinite() { None } else { Some(f) }
        }
        None => special(s).map(|d| if d.is_nan() { f32::from_bits(0xFFC0_0000) } else { d as f32 }),
    }
}

/// `int.Parse(s, InvariantCulture)` (NumberStyles.Integer).
///
/// `s`: text. Returns None on format error or overflow.
pub fn parse_int(s: &str) -> Option<i32> {
    let sc = scan(s, INTEGER)?;
    let mut v: i64 = 0;
    for &d in &sc.digits {
        v = v * 10 + (d - b'0') as i64;
        if v > 2_147_483_648 {
            return None;
        }
    }
    let v = if sc.neg { -v } else { v };
    i32::try_from(v).ok()
}

/// `decimal.Parse(s, InvariantCulture)` (NumberStyles.Number).
///
/// `s`: text. Returns None on format error or overflow. Keeps trailing zeros as scale.
pub fn parse_decimal(s: &str) -> Option<Decimal> {
    let sc = scan(s, NUMBER)?;
    // split leading zeros, compute point position like NumberBuffer.scale
    let lead = sc.digits.iter().take_while(|&&d| d == b'0').count();
    let digits: Vec<u8> = sc.digits[lead..].iter().take(50).copied().collect();
    let mut e: i64 = sc.point - lead as i64;
    const MAX96: u128 = (1u128 << 96) - 1;
    const M10: u128 = MAX96 / 10;
    let mut value: u128 = 0;
    if digits.is_empty() {
        if e > 0 {
            e = 0;
        }
    } else {
        if e > 29 {
            return None;
        }
        if e < -28 {
            // all digits far below 1e-28: Mono gives 0 (scale 28), no rounding
            return Some(Decimal::from_i128_with_scale(0, 28));
        }
        let mut p = 0usize;
        // accumulate while it fits (port of NumberToDecimal loop)
        loop {
            let d = if p < digits.len() { (digits[p] - b'0') as u128 } else { 0 };
            let more = e > 0 || (p < digits.len() && e > -28);
            let fits = value < M10 || (value == M10 && d <= 5);
            if !(more && fits) {
                break;
            }
            value *= 10;
            if p < digits.len() {
                value += d;
                p += 1;
            }
            e -= 1;
        }
        // round on next digit, half to even when exactly half
        let next = digits.get(p).copied().unwrap_or(0);
        let cur_exists = p < digits.len();
        p += 1;
        if cur_exists && next >= b'5' {
            let mut round = true;
            if next == b'5' && (p < 2 || (digits[p - 2] - b'0').is_multiple_of(2)) {
                // exactly half only if all remaining digits are zero
                let mut count = 20;
                while p < digits.len() && digits[p] == b'0' && count != 0 {
                    p += 1;
                    count -= 1;
                }
                if p >= digits.len() || count == 0 {
                    round = false;
                }
            }
            if round {
                value += 1;
                if value > MAX96 {
                    value = 0x1999_9999_9999_9999_9999_999A;
                    e += 1;
                }
            }
        }
    }
    // pad integer zeros
    while e > 0 {
        value *= 10;
        if value > MAX96 {
            return None;
        }
        e -= 1;
    }
    // zero (or fully underflowed) values keep at most scale 28
    let scale = ((-e) as u32).min(28);
    let m = value as i128;
    let mut d = Decimal::from_i128_with_scale(m, scale);
    // C# decimal keeps the sign bit on zero (-0.00m)
    d.set_sign_negative(sc.neg);
    Some(d)
}
