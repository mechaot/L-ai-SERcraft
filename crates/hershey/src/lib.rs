//! Port of `Hershey/Hershey.cs`: Hershey single-stroke text as G-code lines.
//!
//! C# returns `List<GrblCommand>`; here we return the raw lines passed to `new GrblCommand(line)`.
//! The `GrblCommand` ctor trims and uppercases its input. Callers MUST do the same when wrapping.

mod data;

use data::{HOR, VER};
use lg_dotnet::{fmt_double, parse_double};

/// Space between glyphs (C# `spbwl`).
const SPBWL: f64 = 0.3;

/// Pick glyph table row for a printable ASCII unit.
fn charmap(c: u16, horiz: bool) -> &'static [&'static str] {
    let idx = (c - 32) as usize;
    if horiz { HOR[idx] } else { VER[idx] }
}

/// Find all matches of regex `[XY]-?\d+(\.\d+)?` (ASCII digits) in `s`, left to right, non-overlapping.
///
/// * `s` - G-code fragment to scan.
///
/// Returns the matched substrings.
fn find_coords(s: &str) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'X' || b[i] == b'Y' {
            let mut j = i + 1;
            // optional minus
            if j < b.len() && b[j] == b'-' {
                j += 1;
            }
            let ds = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > ds {
                // optional fraction, needs at least one digit after dot
                if j < b.len() && b[j] == b'.' {
                    let mut k = j + 1;
                    while k < b.len() && b[k].is_ascii_digit() {
                        k += 1;
                    }
                    if k > j + 1 {
                        j = k;
                    }
                }
                out.push(&s[i..j]);
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Shift all X/Y words of a glyph and measure its extent (C# `ApplyOffset`).
///
/// * `coordinates` - glyph command strings.
/// * `x_offset`, `y_offset` - added to every X / Y value.
///
/// Returns `(shifted lines, size_x, size_y)`; sizes are max-min with 0 included in the range.
fn apply_offset(coordinates: &[&str], x_offset: f64, y_offset: f64) -> (Vec<String>, f64, f64) {
    let (mut max_x, mut min_x, mut max_y, mut min_y) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut updated = Vec::with_capacity(coordinates.len());
    for line in coordinates {
        let mut upd = (*line).to_string();
        for coord in find_coords(line) {
            let axis = coord.as_bytes()[0] as char;
            // table values always parse
            let mut value = parse_double(&coord[1..]).unwrap_or(0.0);
            if axis == 'X' {
                max_x = value.max(max_x);
                min_x = value.min(min_x);
                value += x_offset;
            } else {
                max_y = value.max(max_y);
                min_y = value.min(min_y);
                value += y_offset;
            }
            // NOTE(upstream): replaces every occurrence, sequentially, like C# string.Replace
            upd = upd.replace(coord, &format!("{}{}", axis, fmt_double(value)));
        }
        updated.push(upd);
    }
    (updated, max_x - min_x, max_y - min_y)
}

/// Measure text extent (C# `MeasureString`).
///
/// * `text` - string to measure; only chars 32..=126 count.
/// * `horiz` - true for horizontal layout, false for vertical.
///
/// Returns `(size_x, size_y)`.
pub fn measure_string(text: &str, horiz: bool) -> (f64, f64) {
    let units: Vec<u16> = text.encode_utf16().collect();
    let (mut size_x, mut size_y) = (0.0f64, 0.0f64);
    for (i, &c) in units.iter().enumerate() {
        if (32..=126).contains(&c) {
            let (_, max_x, max_y) = apply_offset(charmap(c, horiz), 0.0, 0.0);
            let last = i + 1 >= units.len();
            if horiz {
                size_x += max_x;
                size_y = size_y.max(max_y);
                if !last {
                    size_x += SPBWL;
                }
            } else {
                size_y += max_y;
                size_x = size_x.max(max_x);
                if !last {
                    size_y += SPBWL;
                }
            }
        }
    }
    (size_x, size_y)
}

/// Build G-code lines for a text (C# `CreateString`).
///
/// * `text` - text to engrave; chars outside 32..=126 are skipped.
/// * `c_x`, `c_y` - center of the text box.
/// * `f_speed` - feed rate for the leading `M5 S.. F..` line.
/// * `s_pow` - spindle/laser power for the leading line.
/// * `horiz` - horizontal (true) or vertical (false) layout.
/// * `ton` - laser-on command replacing `M3` in glyph data (e.g. `"M4"`).
///
/// Returns raw lines, NOT trimmed/uppercased (the `GrblCommand` ctor does that; callers must too).
pub fn create_string(text: &str, c_x: f64, c_y: f64, f_speed: i32, s_pow: i32, horiz: bool, ton: &str) -> Vec<String> {
    let mut rv = Vec::new();
    let (mut size_x, mut size_y) = measure_string(text, horiz);
    if horiz {
        size_y = 2.0;
    } else {
        size_x = 2.0;
    }
    // set power and speed (off)
    rv.push(format!("M5 S{s_pow} F{f_speed}"));
    let mut o_x = c_x - size_x / 2.0;
    let mut o_y = c_y - size_y / 2.0;
    for c in text.encode_utf16() {
        if (32..=126).contains(&c) {
            let (lines, max_x, max_y) = apply_offset(charmap(c, horiz), o_x, o_y);
            // the char itself
            for part in lines {
                rv.push(part.replace("M3", ton));
            }
            // laser off after the char
            rv.push("M5".to_string());
            if horiz {
                o_x += max_x + SPBWL;
            } else {
                o_y += max_y + SPBWL;
            }
        }
    }
    rv
}
