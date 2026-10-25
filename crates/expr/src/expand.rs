// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2004-2006 Jeremy Roberts, Will Gray (Expression Evaluator)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Infix text normalisation (`Expand`, `ScientificNotationCorrection`) of `Tools/ExpressionEvaluator.cs`.
//! Regexes are hand-coded to keep .NET semantics and avoid dependencies.

use crate::{ExprError, num::parse_net};

/// Regex `<([^=]|$)` -> ` < $1` (same for `>`): each match eats the char after the operator.
///
/// * `s` - input text.
/// * `op` - `'<'` or `'>'`.
///
/// Returns the rewritten text.
fn space_cmp(s: &str, op: char) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == op && (i + 1 == chars.len() || chars[i + 1] != '=') {
            out.push(' ');
            out.push(op);
            out.push(' ');
            // the consumed follower is copied verbatim; it cannot start a new match
            if i + 1 < chars.len() {
                out.push(chars[i + 1]);
            }
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Collapse runs of `' '` (regex `[ ]+`) into `repl`.
///
/// * `s` - input text.
/// * `repl` - replacement for each run ("" or " ").
///
/// Returns the rewritten text.
fn collapse_spaces(s: &str, repl: &str) -> String {
    let mut out = String::new();
    let mut in_run = false;
    for c in s.chars() {
        if c == ' ' {
            if !in_run {
                out.push_str(repl);
            }
            in_run = true;
        } else {
            in_run = false;
            out.push(c);
        }
    }
    out
}

/// Match the tail `' ' (\d+|[0-9]+[eE][+-]\d+)(\s|$)` of the negative-number regex at `p`.
///
/// * `c` - text chars.
/// * `p` - index of the expected space.
///
/// Returns the match end (exclusive, trailing `\s` included) on success.
fn neg_tail(c: &[char], p: usize) -> Option<usize> {
    if c.get(p) != Some(&' ') {
        return None;
    }
    let s = p + 1;
    let mut q = s;
    while q < c.len() && c[q].is_ascii_digit() {
        q += 1;
    }
    if q == s {
        return None;
    }
    // alt 1: digits then whitespace or end
    let ws_or_end = |q: usize| -> Option<usize> {
        if q == c.len() {
            Some(q)
        } else if c[q].is_whitespace() {
            Some(q + 1)
        } else {
            None
        }
    };
    if let Some(e) = ws_or_end(q) {
        return Some(e);
    }
    // alt 2: digits e/E sign digits
    if q < c.len() && (c[q] == 'e' || c[q] == 'E') && q + 1 < c.len() && (c[q + 1] == '+' || c[q + 1] == '-') {
        let mut r = q + 2;
        let ds = r;
        while r < c.len() && c[r].is_ascii_digit() {
            r += 1;
        }
        if r > ds {
            return ws_or_end(r);
        }
    }
    None
}

/// Regex `([<>=/*+(-] -|sign -|^-) (\d+|[0-9]+[eE][+-]\d+)(\s|$)` -> `$1$2$3` (drops one space).
///
/// * `s` - expanded text.
///
/// Returns the text with "op - 2" glued to "op -2".
fn fix_negative(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        let mut hit: Option<(usize, usize)> = None; // (space index, end)
        // alt a: one op char + " -"
        if "<>=/*+(-".contains(c[i])
            && c.get(i + 1) == Some(&' ')
            && c.get(i + 2) == Some(&'-')
            && let Some(e) = neg_tail(&c, i + 3)
        {
            hit = Some((i + 3, e));
        }
        // alt b: "sign -"
        if hit.is_none()
            && c[i..].starts_with(&['s', 'i', 'g', 'n', ' ', '-'])
            && let Some(e) = neg_tail(&c, i + 6)
        {
            hit = Some((i + 6, e));
        }
        // alt c: ^-
        if hit.is_none()
            && i == 0
            && c[0] == '-'
            && let Some(e) = neg_tail(&c, 1)
        {
            hit = Some((1, e));
        }
        if let Some((sp, e)) = hit {
            out.extend(&c[i..sp]);
            out.extend(&c[sp + 1..e]);
            i = e;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

/// Port of `Expression.Expand`: put single spaces around operators and tokens.
///
/// * `function` - raw infix text.
///
/// Returns the expanded text, or the .NET exception the quirky scientific fix can raise.
pub fn expand(function: &str) -> Result<String, ExprError> {
    let mut f = collapse_spaces(function, "");
    for (from, to) in [
        ("(", " ( "),
        (")", " ) "),
        ("+", " + "),
        ("-", " - "),
        ("*", " * "),
        ("/", " / "),
        ("^", " ^ "),
        ("||", " || "),
        ("&&", " && "),
        ("==", " == "),
        (">=", " >= "),
        ("<=", " <= "),
    ] {
        f = f.replace(from, to);
    }
    f = space_cmp(&f, '<');
    f = space_cmp(&f, '>');
    for (from, to) in [("!=", " != "), ("sign", " sign "), ("abs", " abs "), ("neg", " neg ")] {
        f = f.replace(from, to);
    }
    f = f.trim().to_string();
    f = collapse_spaces(&f, " ");
    f = scientific_notation_correction(&f)?;
    // glue "op - 2" into "op -2"
    Ok(fix_negative(&f))
}

/// `string.IndexOfAny(['-','+'], start)` with .NET range check.
///
/// * `c` - text chars.
/// * `start` - first index to look at.
///
/// Returns the index or `None` for -1; errors when `start > len`.
fn index_of_ops(c: &[char], start: usize) -> Result<Option<usize>, ExprError> {
    if start > c.len() {
        return Err(ExprError::ArgumentOutOfRange);
    }
    Ok((start..c.len()).find(|&i| c[i] == '-' || c[i] == '+'))
}

/// Port of `Expression.ScientificNotationCorrection`: re-glue "1e - 5" style splits.
///
/// * `function` - expanded text.
///
/// Returns the corrected text.
pub fn scientific_notation_correction(function: &str) -> Result<String, ExprError> {
    let mut function = function.to_string();
    let mut c: Vec<char> = function.chars().collect();
    let mut n = index_of_ops(&c, 0)?;
    while let Some(ni) = n {
        if ni > c.len() {
            break;
        }
        // previous space (LastIndexOf(" ", n-2, n-2) searches [1, n-2])
        let prev_cut = if (ni as i64) - 2 <= 0 {
            0
        } else {
            (1..=ni - 2).rev().find(|&k| c[k] == ' ').map_or(0, |k| k + 1)
        };
        // next space after n+2
        let next_cut = if ni + 2 < c.len() {
            (ni + 2..c.len()).find(|&k| c[k] == ' ').unwrap_or(c.len())
        } else {
            c.len()
        };
        if prev_cut > next_cut {
            return Err(ExprError::ArgumentOutOfRange);
        }
        let check_space: String = c[prev_cut..next_cut].iter().collect();
        let check_me: String = check_space.replace(' ', "");
        let mut ni2 = ni;
        if parse_net(&check_me).is_some() {
            // NOTE(upstream): replaces every occurrence, may shift later indices
            function = function.replace(&check_space, &check_me);
            c = function.chars().collect();
            ni2 = prev_cut + check_me.chars().count() - 1;
        }
        n = index_of_ops(&c, ni2 + 1)?;
    }
    Ok(function)
}
