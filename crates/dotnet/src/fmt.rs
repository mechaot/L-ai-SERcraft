//! Standard + custom numeric format engine (port of .NET Fx `Number.Formatting`, InvariantCulture).

use crate::number::{round_number, Number};

/// Parsed standard specifier.
struct Spec {
    /// Format letter as given (case kept).
    letter: char,
    /// Digit count, -1 when absent.
    digits: i32,
}

/// Split a format into standard specifier or None when custom.
///
/// `fmt`: format string. Returns Some for "<letter>[digits]" (digits 0..=99).
fn parse_spec(fmt: &str) -> Option<Spec> {
    let mut it = fmt.chars();
    let c = it.next()?;
    if !c.is_ascii_alphabetic() {
        return None;
    }
    let rest: &str = &fmt[1..];
    if rest.is_empty() {
        return Some(Spec { letter: c, digits: -1 });
    }
    // only ascii digits, at most 9 chars (avoid overflow)
    if rest.len() > 9 || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(Spec { letter: c, digits: rest.parse().ok()? })
}

/// Which source type is being formatted (drives precision and G rules).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Double,
    Float,
    Int,
    Decimal,
}

/// Format a finite non-integer-kind number: double, float, decimal.
///
/// `num_for`: builds the digit buffer for a requested significant-digit count.
/// `rt`: true when a buffer parses back to the source value (for "R").
/// `fmt`: format string; empty means "G". `kind`: source type.
/// Returns the formatted string.
pub(crate) fn format_with(
    num_for: &dyn Fn(usize) -> Number,
    rt: &dyn Fn(&Number) -> bool,
    fmt: &str,
    kind: Kind,
) -> String {
    let base_sig: usize = match kind {
        Kind::Double => 15,
        Kind::Float => 7,
        Kind::Int | Kind::Decimal => 40,
    };
    let fmt = if fmt.is_empty() { "G" } else { fmt };
    let Some(spec) = parse_spec(fmt) else {
        // custom: fixed precision per type
        let n = num_for(base_sig);
        let chars: Vec<char> = fmt.chars().collect();
        return format_custom(n, &chars);
    };
    let up = spec.letter.to_ascii_uppercase();
    // wider generation for G/E with many digits (Fx: 15 -> 17)
    let mut sig = base_sig;
    let wide = match kind {
        Kind::Double => 17,
        Kind::Float => 9,
        _ => base_sig,
    };
    match up {
        'G' if spec.digits > base_sig as i32 => sig = wide,
        'E' if kind != Kind::Decimal && spec.digits > (base_sig as i32) - 1 => sig = wide,
        _ => {}
    }
    let mut n = num_for(sig);
    let mut digits = spec.digits;
    match up {
        'R' => {
            // Fx: 15 digits (7 float) if they round trip, else 17 (9)
            let mut p = num_for(base_sig);
            let mut d = base_sig as i32;
            if !rt(&p) {
                p = num_for(wide);
                d = wide as i32;
            }
            general_rounded(&mut p, d, false, spec.letter == 'r')
        }
        'G' => {
            let suppress = false;
            if digits < 1 {
                digits = n.precision;
                if kind == Kind::Decimal && spec.digits == -1 {
                    // decimal plain G: no rounding, never scientific
                    return general_no_round(&n);
                }
            }
            general_rounded(&mut n, digits, suppress, spec.letter == 'g')
        }
        'F' => {
            if digits < 0 {
                digits = 2;
            }
            let pos = n.scale + digits;
            round_number(&mut n, pos);
            let mut s = String::new();
            if n.neg {
                s.push('-');
            }
            fixed(&mut s, &n, digits, false);
            s
        }
        'N' => {
            if digits < 0 {
                digits = 2;
            }
            let pos = n.scale + digits;
            round_number(&mut n, pos);
            let mut s = String::new();
            if n.neg {
                s.push('-');
            }
            fixed(&mut s, &n, digits, true);
            s
        }
        'P' => {
            if digits < 0 {
                digits = 2;
            }
            n.scale += 2;
            let pos = n.scale + digits;
            round_number(&mut n, pos);
            let mut s = String::new();
            if n.neg {
                s.push('-');
            }
            fixed(&mut s, &n, digits, true);
            s.push_str(" %");
            s
        }
        'E' => {
            if digits < 0 {
                digits = 6;
            }
            digits += 1;
            round_number(&mut n, digits);
            let mut s = String::new();
            if n.neg {
                s.push('-');
            }
            scientific(&mut s, &n, digits, spec.letter);
            s
        }
        _ => panic!("lg-dotnet: unsupported standard format {fmt:?} for this type"),
    }
}

/// Plain "G" for decimal: all digits kept, never scientific.
///
/// `n`: digit buffer. Returns text (caller handles trailing-zero preservation).
fn general_no_round(n: &Number) -> String {
    let mut s = String::new();
    if n.neg {
        s.push('-');
    }
    general_body(&mut s, n, 0, true, 'E');
    s
}

/// "G<digits>" path: round then render.
///
/// `n`: buffer. `digits`: max significant digits. `suppress`: never scientific.
/// `lower`: use `e` instead of `E`.
fn general_rounded(n: &mut Number, digits: i32, suppress: bool, lower: bool) -> String {
    round_number(n, digits);
    let mut s = String::new();
    if n.neg {
        s.push('-');
    }
    general_body(&mut s, n, digits, suppress, if lower { 'e' } else { 'E' });
    s
}

/// `FormatGeneral`.
///
/// `s`: output. `n`: rounded buffer. `max`: precision for scientific switch.
/// `suppress`: never scientific. `e`: exponent char.
fn general_body(s: &mut String, n: &Number, max: i32, suppress: bool, e: char) {
    let mut dig_pos = n.scale;
    let mut sci = false;
    if !suppress && (dig_pos > max || dig_pos < -3) {
        dig_pos = 1;
        sci = true;
    }
    let mut di = 0usize;
    if dig_pos > 0 {
        // integer part, zero padded
        while dig_pos > 0 {
            s.push(next_digit(&n.digits, &mut di));
            dig_pos -= 1;
        }
    } else {
        s.push('0');
    }
    if di < n.digits.len() || dig_pos < 0 {
        s.push('.');
        while dig_pos < 0 {
            s.push('0');
            dig_pos += 1;
        }
        while di < n.digits.len() {
            s.push(n.digits[di] as char);
            di += 1;
        }
    }
    if sci {
        exponent(s, n.scale - 1, e, 2, true);
    }
}

/// Next digit char or '0' when exhausted.
///
/// `d`: digits. `i`: cursor (advanced).
fn next_digit(d: &[u8], i: &mut usize) -> char {
    if *i < d.len() {
        let c = d[*i] as char;
        *i += 1;
        c
    } else {
        '0'
    }
}

/// `FormatExponent`.
///
/// `s`: output. `v`: exponent. `e`: 'E' or 'e'. `min`: min digit count.
/// `plus`: print '+' for non-negative.
fn exponent(s: &mut String, v: i32, e: char, min: usize, plus: bool) {
    s.push(e);
    if v < 0 {
        s.push('-');
    } else if plus {
        s.push('+');
    }
    s.push_str(&format!("{:0width$}", v.unsigned_abs(), width = min));
}

/// `FormatScientific` for "E" format.
///
/// `s`: output. `n`: rounded buffer. `digits`: total mantissa digits (precision+1).
/// `e`: 'E' or 'e'.
fn scientific(s: &mut String, n: &Number, digits: i32, e: char) {
    let mut di = 0usize;
    s.push(next_digit(&n.digits, &mut di));
    if digits != 1 {
        s.push('.');
    }
    let mut k = digits;
    loop {
        k -= 1;
        if k <= 0 {
            break;
        }
        s.push(next_digit(&n.digits, &mut di));
    }
    let ex = if n.digits.is_empty() { 0 } else { n.scale - 1 };
    exponent(s, ex, e, 3, true);
}

/// `FormatFixed` (F and N). Invariant: '.' decimal, ',' group by 3.
///
/// `s`: output. `n`: rounded buffer. `max_digits`: decimals. `group`: thousands sep.
fn fixed(s: &mut String, n: &Number, max_digits: i32, group: bool) {
    let mut dig_pos = n.scale;
    let mut di = 0usize;
    if dig_pos > 0 {
        let mut left = dig_pos;
        while left > 0 {
            s.push(next_digit(&n.digits, &mut di));
            if group && left > 1 && (left - 1) % 3 == 0 {
                s.push(',');
            }
            left -= 1;
        }
        dig_pos = 0;
    } else {
        s.push('0');
    }
    let mut md = max_digits;
    if md > 0 {
        s.push('.');
        if dig_pos < 0 {
            let z = (-dig_pos).min(md);
            for _ in 0..z {
                s.push('0');
            }
            md -= z;
        }
        while md > 0 {
            s.push(next_digit(&n.digits, &mut di));
            md -= 1;
        }
    }
}

/// Find start index of section `section` (0..=2) in a custom format; 0 when absent/empty.
///
/// `f`: format chars. `section`: wanted section number.
fn find_section(f: &[char], mut section: i32) -> usize {
    if section == 0 {
        return 0;
    }
    let mut src = 0usize;
    loop {
        if src >= f.len() {
            return 0;
        }
        let ch = f[src];
        src += 1;
        match ch {
            '\'' | '"' => {
                // skip quoted literal
                while src < f.len() && f[src] != '\0' {
                    let c = f[src];
                    src += 1;
                    if c == ch {
                        break;
                    }
                }
            }
            '\\' => {
                if src < f.len() && f[src] != '\0' {
                    src += 1;
                }
            }
            ';' => {
                section -= 1;
                if section == 0 {
                    if src >= f.len() || f[src] == '\0' || f[src] == ';' {
                        return 0;
                    }
                    return src;
                }
            }
            _ => {}
        }
    }
}

/// Custom format engine (`NumberToStringFormat`).
///
/// `n`: digit buffer (consumed). `f`: format chars.
/// Returns formatted text. Supports 0 # . , % ‰ quotes \ E+0 and `;` sections.
pub(crate) fn format_custom(mut n: Number, f: &[char]) -> String {
    let mut section = find_section(f, if n.digits.is_empty() { 2 } else if n.neg { 1 } else { 0 });
    let mut digit_count: i32;
    let mut decimal_pos: i32;
    let mut first_digit: i32;
    let mut last_digit: i32;
    let mut thousand_seps: bool;
    let mut scientific_fmt: bool;
    loop {
        digit_count = 0;
        decimal_pos = -1;
        first_digit = i32::MAX;
        last_digit = 0;
        let mut scale_adjust = 0;
        let mut thousand_pos: i32 = -1;
        let mut thousand_count = 0;
        thousand_seps = false;
        scientific_fmt = false;
        // scan section: count placeholders
        let mut src = section;
        while src < f.len() {
            let ch = f[src];
            src += 1;
            if ch == ';' || ch == '\0' {
                break;
            }
            match ch {
                '#' => digit_count += 1,
                '0' => {
                    if first_digit == i32::MAX {
                        first_digit = digit_count;
                    }
                    digit_count += 1;
                    last_digit = digit_count;
                }
                '.' => {
                    if decimal_pos < 0 {
                        decimal_pos = digit_count;
                    }
                }
                ',' => {
                    if digit_count > 0 && decimal_pos < 0 {
                        if thousand_pos >= 0 {
                            if thousand_pos == digit_count {
                                thousand_count += 1;
                                continue;
                            }
                            thousand_seps = true;
                        }
                        thousand_pos = digit_count;
                        thousand_count = 1;
                    }
                }
                '%' => scale_adjust += 2,
                '\u{2030}' => scale_adjust += 3,
                '\'' | '"' => {
                    while src < f.len() && f[src] != '\0' {
                        let c = f[src];
                        src += 1;
                        if c == ch {
                            break;
                        }
                    }
                }
                '\\' => {
                    if src < f.len() && f[src] != '\0' {
                        src += 1;
                    }
                }
                'E' | 'e' => {
                    let at = |k: usize| f.get(k).copied().unwrap_or('\0');
                    if at(src) == '0' || ((at(src) == '+' || at(src) == '-') && at(src + 1) == '0') {
                        // skip sign and zeros
                        if at(src) != '0' {
                            src += 1;
                        }
                        while at(src) == '0' {
                            src += 1;
                        }
                        scientific_fmt = true;
                    }
                }
                _ => {}
            }
        }
        if decimal_pos < 0 {
            decimal_pos = digit_count;
        }
        if thousand_pos >= 0 {
            if thousand_pos == decimal_pos {
                // trailing commas before point: scale by 1000 each
                scale_adjust -= thousand_count * 3;
            } else {
                thousand_seps = true;
            }
        }
        if !n.digits.is_empty() {
            n.scale += scale_adjust;
            let pos = if scientific_fmt { digit_count } else { n.scale + digit_count - decimal_pos };
            round_number(&mut n, pos);
            if n.digits.is_empty() {
                // rounded to zero: switch to zero section when it differs
                let s = find_section(f, 2);
                if s != section {
                    section = s;
                    continue;
                }
            }
        } else {
            n.neg = false;
            n.scale = 0;
        }
        break;
    }
    let has_zero_decimals = last_digit > decimal_pos;
    let first_digit = if first_digit < decimal_pos { decimal_pos - first_digit } else { 0 };
    let (mut dig_pos, mut adjust) = if scientific_fmt {
        (decimal_pos, 0)
    } else {
        (n.scale.max(decimal_pos), n.scale - decimal_pos)
    };
    let mut out = String::new();
    // sign only for first section (others carry their own literals)
    if n.neg && section == 0 {
        out.push('-');
    }
    let mut di = 0usize;
    let mut idx = 0i32; // placeholder index
    let mut decimal_written = false;
    // push digit then group separator if needed
    let sep = |out: &mut String, dig_pos: i32| {
        if thousand_seps && dig_pos > 1 && (dig_pos - 1) % 3 == 0 {
            out.push(',');
        }
    };
    let mut src = section;
    while src < f.len() {
        let ch = f[src];
        src += 1;
        if ch == ';' || ch == '\0' {
            break;
        }
        if adjust > 0 && matches!(ch, '#' | '0' | '.') {
            // dump surplus integer digits at first placeholder
            while adjust > 0 {
                out.push(next_digit(&n.digits, &mut di));
                sep(&mut out, dig_pos);
                dig_pos -= 1;
                adjust -= 1;
            }
        }
        match ch {
            '#' | '0' => {
                let c: Option<char> = if adjust < 0 {
                    adjust += 1;
                    if dig_pos <= first_digit { Some('0') } else { None }
                } else if di < n.digits.len() {
                    di += 1;
                    Some(n.digits[di - 1] as char)
                } else if dig_pos >= 1 || idx < last_digit {
                    Some('0')
                } else {
                    None
                };
                if let Some(c) = c {
                    out.push(c);
                    sep(&mut out, dig_pos);
                }
                dig_pos -= 1;
                idx += 1;
            }
            '.' => {
                if dig_pos != 0 || decimal_written {
                    continue;
                }
                if has_zero_decimals || (decimal_pos < digit_count && di < n.digits.len()) {
                    out.push('.');
                    decimal_written = true;
                }
            }
            '\u{2030}' => out.push('\u{2030}'),
            '%' => out.push('%'),
            ',' => {}
            '\'' | '"' => {
                while src < f.len() && f[src] != '\0' && f[src] != ch {
                    out.push(f[src]);
                    src += 1;
                }
                if src < f.len() && f[src] != '\0' {
                    src += 1;
                }
            }
            '\\' => {
                if src < f.len() && f[src] != '\0' {
                    out.push(f[src]);
                    src += 1;
                }
            }
            'E' | 'e' => {
                let at = |k: usize| f.get(k).copied().unwrap_or('\0');
                if at(src) == '0' || ((at(src) == '+' || at(src) == '-') && at(src + 1) == '0') {
                    let plus = at(src) == '+';
                    if at(src) != '0' {
                        src += 1;
                    }
                    let start = src;
                    while at(src) == '0' {
                        src += 1;
                    }
                    let ex = if n.digits.is_empty() { 0 } else { n.scale - decimal_pos };
                    exponent(&mut out, ex, ch, src - start, plus);
                } else {
                    out.push(ch);
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Integer formatting incl. D / X.
///
/// `neg`/`mag`: sign and magnitude. `hex_value`: two's complement bits for X.
/// `fmt`: format string. Returns text.
pub(crate) fn format_integer(neg: bool, mag: u64, hex_value: u64, prec: i32, fmt: &str) -> String {
    let fmt = if fmt.is_empty() { "G" } else { fmt };
    if let Some(spec) = parse_spec(fmt) {
        match spec.letter {
            'D' | 'd' => {
                let w = spec.digits.max(0) as usize;
                let mut s = String::new();
                if neg && mag != 0 {
                    s.push('-');
                }
                s.push_str(&format!("{:0w$}", mag, w = w));
                return s;
            }
            'X' => return format!("{:0w$X}", hex_value, w = spec.digits.max(0) as usize),
            'x' => return format!("{:0w$x}", hex_value, w = spec.digits.max(0) as usize),
            'G' | 'g' if spec.digits <= 0 => {
                let mut s = String::new();
                if neg && mag != 0 {
                    s.push('-');
                }
                s.push_str(&mag.to_string());
                return s;
            }
            _ => {}
        }
    }
    let nb = move |_sig: usize| Number::from_u64(neg, mag, prec);
    format_with(&nb, &|_| true, fmt, Kind::Int)
}
