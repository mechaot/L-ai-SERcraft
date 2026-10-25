// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Custom code evaluation of `GrblCore` (`ExecuteCustomCode` helpers, `EvaluateExpression`, `EvaluateCB`,
//! `IsImmediate`, `GetImmediate`, `FormatNumber`).
//!
//! The variables are captured in an [`ExprVars`] snapshot so no core lock is needed while evaluating
//! (used by `save_program` which already holds the file lock).

use lg_dotnet::{dec_sub, decimal_to_f64, fmt_decimal_custom, parse_double, try_f64_to_decimal, Decimal};
use lg_gcode::{GPoint, GrblConfST, GrblFile};
use regex::Regex;
use std::sync::LazyLock;

/// C# `bracketsRegEx` = `\[(?:[^]]+)\]`.
static BRACKETS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[(?:[^\]]+)\]").expect("static regex"));

/// Drawing bounds of the loaded file (0 when no valid range).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FileBounds {
    pub left: Decimal,
    pub right: Decimal,
    pub top: Decimal,
    pub bottom: Decimal,
}

impl FileBounds {
    /// Bounds from a file (`LoadedFile.Range.DrawingRange`).
    ///
    /// `f`: the loaded file.
    pub fn from_file(f: &GrblFile) -> Self {
        let r = &f.range().drawing_range;
        if r.valid_range() { Self { left: r.x.min, right: r.x.max, top: r.y.max, bottom: r.y.min } } else { Self::default() }
    }
}

/// Everything `EvaluateCB` reads from the core.
#[derive(Debug, Clone)]
pub struct ExprVars {
    pub bounds: FileBounds,
    pub jog_step: Decimal,
    pub jog_speed: i32,
    pub working_offset: GPoint,
    pub machine_position: GPoint,
    pub configuration: GrblConfST,
}

/// `FormatNumber`: `{0:0.000}` invariant.
///
/// `v`: value. Returns text like `12.500`.
pub fn format_number(v: Decimal) -> String {
    fmt_decimal_custom(v, "0.000")
}

/// `EvaluateCB` for the text inside the brackets.
///
/// `vars`: core snapshot. `text`: expression without brackets. Returns the formatted number, None where
/// C# throws (caller keeps the original match).
fn evaluate_inner(vars: &ExprVars, text: &str) -> Option<String> {
    let b = &vars.bounds;
    let width = dec_sub(b.right, b.left)?;
    let height = dec_sub(b.top, b.bottom)?;
    let mut exp = lg_expr::Expression::new(text).ok()?;
    let d = decimal_to_f64;
    exp.add_set_variable("left", d(b.left));
    exp.add_set_variable("right", d(b.right));
    exp.add_set_variable("top", d(b.top));
    exp.add_set_variable("bottom", d(b.bottom));
    exp.add_set_variable("width", d(width));
    exp.add_set_variable("height", d(height));
    exp.add_set_variable("jogstep", d(vars.jog_step));
    exp.add_set_variable("jogspeed", vars.jog_speed as f64);
    exp.add_set_variable("WCO.X", vars.working_offset.x as f64);
    exp.add_set_variable("WCO.Y", vars.working_offset.y as f64);
    exp.add_set_variable("WCO.Z", vars.working_offset.z as f64);
    exp.add_set_variable("MPos.X", vars.machine_position.x as f64);
    exp.add_set_variable("MPos.Y", vars.machine_position.y as f64);
    exp.add_set_variable("MPos.Z", vars.machine_position.z as f64);
    // NOTE(upstream): WPos.* is filled with the machine position, kept
    exp.add_set_variable("WPos.X", vars.machine_position.x as f64);
    exp.add_set_variable("WPos.Y", vars.machine_position.y as f64);
    exp.add_set_variable("WPos.Z", vars.machine_position.z as f64);
    for (k, v) in vars.configuration.iter() {
        // only what converts to double
        if let Some(tod) = parse_double(v) {
            exp.add_set_variable(&format!("${k}"), tod);
        }
    }
    // fix for "Bad Number Format" when $30 cannot be read from the machine configuration
    if !exp.exist_token("$30") {
        exp.add_set_variable("$30", 1000.0);
    }
    let dval = exp.evaluate_d().ok()?;
    let dec = try_f64_to_decimal(dval)?;
    Some(format_number(dec))
}

/// `EvaluateExpression(str)`: replace every `[...]` by its value, then trim.
///
/// `vars`: core snapshot. `s`: one line of custom code. Returns the line with expressions evaluated; a
/// failing expression stays as written.
pub fn evaluate_expression(vars: &ExprVars, s: &str) -> String {
    let r = BRACKETS.replace_all(s, |caps: &regex::Captures<'_>| {
        let m = &caps[0];
        let inner = &m[1..m.len() - 1];
        evaluate_inner(vars, inner).unwrap_or_else(|| m.to_string())
    });
    r.trim().to_string()
}

/// `IsImmediate(code)`: `ctrl-x`, `?`, `!`, `~` or `0xNN`.
///
/// `code`: evaluated line.
pub fn is_immediate(code: &str) -> bool {
    let lower = code.to_lowercase();
    if lower == "ctrl-x" || code == "?" || code == "!" || code == "~" {
        return true;
    }
    if lower.starts_with("0x") {
        return convert_to_byte_hex(code).is_some();
    }
    false
}

/// `Convert.ToByte(code, 16)`: accepts an optional `0x`/`0X` prefix, 1-2 hex digits.
///
/// `code`: text. Returns None where C# throws.
fn convert_to_byte_hex(code: &str) -> Option<u8> {
    let t = code.strip_prefix("0x").or_else(|| code.strip_prefix("0X")).unwrap_or(code);
    if t.is_empty() || t.len() > 2 {
        // NOTE: "0x" alone throws in .NET as well
        return None;
    }
    u8::from_str_radix(t, 16).ok()
}

/// `GetImmediate(code)`.
///
/// `code`: line accepted by [`is_immediate`]. Returns the byte to send.
pub fn get_immediate(code: &str) -> Option<u8> {
    let lower = code.to_lowercase();
    match (lower.as_str(), code) {
        ("ctrl-x", _) => Some(24),
        (_, "?") => Some(63),
        (_, "!") => Some(33),
        (_, "~") => Some(126),
        _ => convert_to_byte_hex(code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> ExprVars {
        let conf = GrblConfST::with_table(Some(lg_gcode::GrblVersionInfo::new(1, 1)), &[(30, "255".into()), (32, "1".into()), (5, "abc".into())]);
        ExprVars {
            bounds: FileBounds { left: Decimal::from(10), right: Decimal::from(30), top: Decimal::from(50), bottom: Decimal::from(20) },
            jog_step: Decimal::from(5),
            jog_speed: 1000,
            working_offset: GPoint::new(1.5, 2.0, 0.0),
            machine_position: GPoint::new(10.0, 20.0, 3.0),
            configuration: conf,
        }
    }

    #[test]
    fn brackets_are_evaluated() {
        let v = vars();
        assert_eq!(evaluate_expression(&v, "G0 X[left] Y[bottom]"), "G0 X10.000 Y20.000");
        assert_eq!(evaluate_expression(&v, "G0 X[width/2+left]"), "G0 X20.000");
        assert_eq!(evaluate_expression(&v, "  M3 S[$30*0.5] "), "M3 S127.500");
        assert_eq!(evaluate_expression(&v, "G1 X[MPos.X+WCO.X]F[jogspeed]"), "G1 X11.500F1000.000");
        assert_eq!(evaluate_expression(&v, "[jogstep]"), "5.000");
        // unknown variable: match stays untouched
        assert_eq!(evaluate_expression(&v, "G0 X[nope+1]"), "G0 X[nope+1]");
        // no brackets: only trim
        assert_eq!(evaluate_expression(&v, "  G90 "), "G90");
    }

    #[test]
    fn default_s_max_and_invalid_range() {
        let mut v = vars();
        v.configuration = GrblConfST::new();
        assert_eq!(evaluate_expression(&v, "S[$30]"), "S1000.000");
        v.bounds = FileBounds::default();
        assert_eq!(evaluate_expression(&v, "X[width]"), "X0.000");
    }

    #[test]
    fn immediates() {
        for (s, b) in [("ctrl-x", 24u8), ("CTRL-X", 24), ("?", 63), ("!", 33), ("~", 126), ("0x85", 0x85), ("0X18", 0x18)] {
            assert!(is_immediate(s), "{s}");
            assert_eq!(get_immediate(s), Some(b), "{s}");
        }
        assert!(!is_immediate("G0"));
        assert!(!is_immediate("0xZZ"));
        assert!(!is_immediate("0x123"));
        assert!(!is_immediate("0x"));
    }
}
