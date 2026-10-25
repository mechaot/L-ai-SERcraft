// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `System.Drawing.ColorTranslator.FromHtml` (+ `ColorConverter` string rules) as used by `parseRgb` in GCodeFromSVG.cs.
//!
//! Behaviour mirrors Mono's System.Drawing (the golden oracle). Only RGB and "is Color.Empty" matter to callers.

use crate::SvgError;
use crate::color_table::{HTML_SYSTEM, KNOWN};

/// Parsed colour: `None` = `Color.Empty`, `Some([r, g, b])` otherwise.
pub type ParsedColor = Option<[u8; 3]>;

/// Build the `ArgumentException` the converter throws on bad colour text.
///
/// `s`: offending text. Returns the error.
fn invalid(s: &str) -> SvgError {
    SvgError::new("System.ArgumentException", format!("{s} is not a valid value for Color."))
}

/// Parse one integer token (decimal, `#hex`, `0xhex`, `&hhex`) like `Int32Converter`.
///
/// `t`: trimmed token. Returns the int bits or None on failure.
fn parse_int_token(t: &str) -> Option<i32> {
    let hex = if let Some(r) = t.strip_prefix('#') {
        Some(r)
    } else if let Some(r) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(r)
    } else {
        t.strip_prefix("&h").or_else(|| t.strip_prefix("&H"))
    };
    match hex {
        Some(h) => {
            if h.is_empty() || h.len() > 8 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            u32::from_str_radix(h, 16).ok().map(|v| v as i32)
        }
        None => lg_dotnet::parse_int(t),
    }
}

/// `ColorTranslator.FromHtml(htmlColor)`.
///
/// `html`: colour text (`#rgb`, `#rrggbb`, name, `r,g,b`, ...). Returns `Ok(None)` for `Color.Empty`,
/// `Ok(Some(rgb))` for a colour, `Err(ArgumentException)` for text the converter rejects.
pub fn from_html(html: &str) -> Result<ParsedColor, SvgError> {
    // empty color
    if html.is_empty() {
        return Ok(None);
    }
    let b = html.as_bytes();
    // #RRGGBB or #RGB
    if b[0] == b'#' && (b.len() == 7 || b.len() == 4) {
        let digits = &html[1..];
        if !digits.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(invalid(html));
        }
        let v = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
        if b.len() == 7 {
            return Ok(Some([v(&html[1..3]), v(&html[3..5]), v(&html[5..7])]));
        }
        let d = |i: usize| v(&format!("{0}{0}", &html[i..i + 1]));
        return Ok(Some([d(1), d(2), d(3)]));
    }
    let text = html.trim();
    if text.is_empty() {
        return Ok(None);
    }
    // named color (case-insensitive), html system colours, LightGrey alias
    let lower = text.to_lowercase();
    let key = if lower == "lightgrey" { "lightgray" } else { lower.as_str() };
    // html system names win over KnownColor (ButtonFace -> SystemColors.Control)
    if let Some(&(_, r, g, bl)) = HTML_SYSTEM.iter().find(|e| e.0 == key) {
        return Ok(Some([r, g, bl]));
    }
    if let Some(&(_, r, g, bl)) = KNOWN.iter().find(|e| e.0 == key) {
        return Ok(Some([r, g, bl]));
    }
    if !text.contains(',') {
        // single int: ARGB bits
        let v = parse_int_token(text).ok_or_else(|| invalid(html))?;
        let u = v as u32;
        return Ok(Some([(u >> 16) as u8, (u >> 8) as u8, u as u8]));
    }
    // "R,G,B" or "A,R,G,B"
    let mut parts = Vec::new();
    for p in text.split(',') {
        let v = parse_int_token(p.trim()).ok_or_else(|| invalid(html))?;
        if !(0..=255).contains(&v) {
            return Err(invalid(html));
        }
        parts.push(v as u8);
    }
    match parts.len() {
        3 => Ok(Some([parts[0], parts[1], parts[2]])),
        4 => Ok(Some([parts[1], parts[2], parts[3]])),
        _ => Err(invalid(html)),
    }
}
