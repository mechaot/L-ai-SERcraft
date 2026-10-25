// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `System.Char` classification (UTF-16 code unit semantics) and string helpers.

use crate::unicode_tables::{DIGIT_RANGES, LETTER_RANGES, NUMBER_RANGES, WHITE_RANGES};

/// Binary search over inclusive BMP ranges.
///
/// `t`: sorted ranges. `c`: char. Non-BMP chars never match (C# char is a UTF-16 unit).
fn in_ranges(t: &[(u16, u16)], c: char) -> bool {
    let Ok(u) = u16::try_from(c as u32) else {
        return false;
    };
    t.binary_search_by(|&(lo, hi)| {
        if u < lo {
            std::cmp::Ordering::Greater
        } else if u > hi {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    })
    .is_ok()
}

/// `Char.IsDigit`: Unicode category Nd (BMP only).
///
/// `c`: char to test.
pub fn is_digit(c: char) -> bool {
    in_ranges(DIGIT_RANGES, c)
}

/// `Char.IsLetter`: categories Lu Ll Lt Lm Lo (BMP only).
///
/// `c`: char to test.
pub fn is_letter(c: char) -> bool {
    in_ranges(LETTER_RANGES, c)
}

/// `Char.IsNumber`: categories Nd Nl No (BMP only).
///
/// `c`: char to test.
pub fn is_number(c: char) -> bool {
    in_ranges(NUMBER_RANGES, c)
}

/// `Char.IsWhiteSpace` (BMP only).
///
/// `c`: char to test.
pub fn is_white_space(c: char) -> bool {
    in_ranges(WHITE_RANGES, c)
}

/// `string.Split(char)`: keeps empty entries.
///
/// `s`: text. `sep`: separator char. Returns pieces (always at least one).
pub fn split_keep_empty(s: &str, sep: char) -> Vec<&str> {
    s.split(sep).collect()
}
