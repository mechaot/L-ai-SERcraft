//! .NET string helpers (`Trim()`, `ToUpper()`, `ToLower()`) with Fx semantics.

use lg_dotnet::is_white_space;

/// `string.Trim()`: strips `Char.IsWhiteSpace` chars both ends.
///
/// `s`: input. Returns trimmed slice.
pub fn dn_trim(s: &str) -> &str {
    s.trim_matches(is_white_space)
}

/// `string.ToUpper()` (invariant): per-char mapping, no length change (`ß` stays `ß`).
///
/// `s`: input. Returns upper-cased copy.
pub fn dn_upper(s: &str) -> String {
    s.chars().map(single_upper).collect()
}

/// `string.ToLower()` (invariant): per-char mapping, no length change.
///
/// `s`: input. Returns lower-cased copy.
pub fn dn_lower(s: &str) -> String {
    s.chars().map(single_lower).collect()
}

/// One-to-one upper mapping. Multi-char expansions keep the original char.
///
/// `c`: char. Returns mapped char.
fn single_upper(c: char) -> char {
    // Mono invariant ToUpper leaves dotless i and titlecase digraphs alone
    if matches!(c, '\u{131}' | '\u{1C5}' | '\u{1C8}' | '\u{1CB}' | '\u{1F2}') {
        return c;
    }
    let mut it = c.to_uppercase();
    match (it.next(), it.next()) {
        (Some(u), None) => u,
        _ => c,
    }
}

/// One-to-one lower mapping. Multi-char expansions keep the original char.
///
/// `c`: char. Returns mapped char.
fn single_lower(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(u), None) => u,
        _ => c,
    }
}

/// `StreamReader` text decoding with BOM detection: UTF-8 (default, BOM skipped), UTF-16 LE/BE and
/// UTF-32 LE/BE BOMs. Invalid sequences become U+FFFD like .NET.
///
/// `b`: file bytes. Returns the decoded text.
pub fn decode_stream_reader(b: &[u8]) -> String {
    let utf32 = |chunks: &[u8], le: bool| -> String {
        chunks
            .as_chunks::<4>()
            .0
            .iter()
            .map(|a| char::from_u32(if le { u32::from_le_bytes(*a) } else { u32::from_be_bytes(*a) }).unwrap_or('\u{FFFD}'))
            .collect()
    };
    let utf16 = |chunks: &[u8], le: bool| -> String {
        let v: Vec<u16> = chunks.as_chunks::<2>().0.iter().map(|a| if le { u16::from_le_bytes(*a) } else { u16::from_be_bytes(*a) }).collect();
        String::from_utf16_lossy(&v)
    };
    if b.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&b[3..]).into_owned()
    } else if b.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        utf32(&b[4..], true)
    } else if b.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
        utf32(&b[4..], false)
    } else if b.starts_with(&[0xFF, 0xFE]) {
        utf16(&b[2..], true)
    } else if b.starts_with(&[0xFE, 0xFF]) {
        utf16(&b[2..], false)
    } else {
        String::from_utf8_lossy(b).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_reader_boms() {
        assert_eq!(decode_stream_reader(b"G0 X1"), "G0 X1");
        assert_eq!(decode_stream_reader(b"\xEF\xBB\xBFG0"), "G0");
        let mut le = vec![0xFF, 0xFE];
        le.extend("G1 é".encode_utf16().flat_map(|u| u.to_le_bytes()));
        assert_eq!(decode_stream_reader(&le), "G1 é");
        let mut be = vec![0xFE, 0xFF];
        be.extend("G1 é".encode_utf16().flat_map(|u| u.to_be_bytes()));
        assert_eq!(decode_stream_reader(&be), "G1 é");
        let mut u32le = vec![0xFF, 0xFE, 0, 0];
        u32le.extend("G2".chars().flat_map(|c| (c as u32).to_le_bytes()));
        assert_eq!(decode_stream_reader(&u32le), "G2");
    }
}
