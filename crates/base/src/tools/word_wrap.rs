// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Tools/WordWrap.cs`. Works on UTF-16 units like C# so lengths match.

use lg_dotnet::is_white_space;

const NEWLINE: [u16; 2] = [13, 10];

/// C# `Char.IsWhiteSpace` on a UTF-16 unit.
///
/// `u`: code unit. Returns true for whitespace (surrogates are not).
fn ws(u: u16) -> bool {
    char::from_u32(u as u32).is_some_and(is_white_space)
}

/// Index of "\r\n" in `t` from `pos`.
///
/// `t`: text. `pos`: start. Returns index or None.
fn find_newline(t: &[u16], pos: usize) -> Option<usize> {
    (pos..t.len().saturating_sub(1)).find(|&i| t[i] == NEWLINE[0] && t[i + 1] == NEWLINE[1])
}

/// Wrap text at `width` UTF-16 units without breaking words; output lines end with "\r\n".
///
/// `the_string`: input. `width`: max line length; < 1 returns input unchanged. Returns wrapped text.
pub fn wrap_string(the_string: &str, width: i32) -> String {
    if the_string.is_empty() || width < 1 {
        return the_string.to_string();
    }
    let t: Vec<u16> = the_string.encode_utf16().collect();
    let width = width as usize;
    let mut out: Vec<u16> = Vec::new();
    let mut pos = 0usize;
    while pos < t.len() {
        // find end of line
        let (eol, next) = match find_newline(&t, pos) {
            None => (t.len(), t.len()),
            Some(e) => (e, e + 2),
        };
        if eol > pos {
            // copy line, break as needed
            loop {
                let mut len = eol - pos;
                if len > width {
                    len = break_line(&t, pos, width);
                }
                out.extend_from_slice(&t[pos..pos + len]);
                out.extend_from_slice(&NEWLINE);
                pos += len;
                // trim whitespace after break
                while pos < eol && ws(t[pos]) {
                    pos += 1;
                }
                if eol <= pos {
                    break;
                }
            }
        } else {
            out.extend_from_slice(&NEWLINE); // empty line
        }
        pos = next;
    }
    String::from_utf16_lossy(&out)
}

/// Break position that avoids splitting words.
///
/// `text`: UTF-16 text. `pos`: line start. `max`: max line length. Returns the line length to use.
pub fn break_line(text: &[u16], pos: usize, max: usize) -> usize {
    // find last whitespace in line
    let mut i = max as isize - 1;
    while i >= 0 && !ws(text[pos + i as usize]) {
        i -= 1;
    }
    if i < 0 {
        return max; // no whitespace: hard break
    }
    // skip the whitespace run
    while i >= 0 && ws(text[pos + i as usize]) {
        i -= 1;
    }
    (i + 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps() {
        assert_eq!(wrap_string("aaa bbb ccc", 7), "aaa\r\nbbb ccc\r\n");
        assert_eq!(wrap_string("abcdefghij", 4), "abcd\r\nefgh\r\nij\r\n");
        assert_eq!(wrap_string("a\r\n\r\nb", 5), "a\r\n\r\nb\r\n");
        assert_eq!(wrap_string("", 5), "");
        assert_eq!(wrap_string("x y", 0), "x y");
        assert_eq!(wrap_string("hello   world", 8), "hello\r\nworld\r\n");
    }
}
