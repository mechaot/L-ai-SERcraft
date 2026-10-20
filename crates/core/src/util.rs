//! Small helpers shared by the modules (GUID generation, TimeSpan setting codec, UTF-16 style substrings).

use lg_dotnet::TimeSpan;
use std::hash::{BuildHasher, Hasher};

/// 16 random bytes without extra crates (std `RandomState` is OS seeded) mixed with the clock.
fn random_bytes() -> [u8; 16] {
    let mut out = [0u8; 16];
    for chunk in out.chunks_mut(8) {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        h.write_u64(std::process::id() as u64);
        chunk.copy_from_slice(&h.finish().to_le_bytes());
    }
    out
}

/// `Guid.NewGuid().ToString()` (lower case, with dashes, version 4 bits set).
///
/// Returns a fresh random GUID text.
pub fn new_guid() -> String {
    let mut b = random_bytes();
    b[6] = (b[6] & 0x0F) | 0x40;
    b[8] = (b[8] & 0x3F) | 0x80;
    let h: Vec<String> = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", h[0..4].concat(), h[4..6].concat(), h[6..8].concat(), h[8..10].concat(), h[10..16].concat())
}

/// `Guid.NewGuid().ToString("N")`.
///
/// Returns 32 hex chars.
pub fn new_guid_n() -> String {
    new_guid().replace('-', "")
}

/// Parse the .NET "c" TimeSpan text `[-][d.]hh:mm:ss[.fffffff]` (how settings store TimeSpan).
///
/// `s`: text. Returns None when the text is not in that format.
pub fn parse_timespan_c(s: &str) -> Option<TimeSpan> {
    let s = s.trim();
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let (days, rest) = match body.split_once('.') {
        // "d.hh:mm:ss[.f]" only when the part before the first '.' has no ':'
        Some((d, r)) if !d.contains(':') => (d.parse::<i64>().ok()?, r),
        _ => (0, body),
    };
    let (hms, frac) = match rest.split_once('.') {
        Some((a, f)) => (a, Some(f)),
        None => (rest, None),
    };
    let parts: Vec<&str> = hms.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: i64 = parts[0].parse().ok()?;
    let m: i64 = parts[1].parse().ok()?;
    let sec: i64 = parts[2].parse().ok()?;
    let mut ticks = ((days * 24 + h) * 60 + m) * 60 + sec;
    ticks *= 10_000_000;
    if let Some(f) = frac {
        if f.is_empty() || f.len() > 7 || !f.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let mut padded = f.to_string();
        while padded.len() < 7 {
            padded.push('0');
        }
        ticks += padded.parse::<i64>().ok()?;
    }
    Some(TimeSpan::from_ticks(if neg { -ticks } else { ticks }))
}

/// Read a TimeSpan setting (stored as "c" text).
///
/// `key`: settings key. `default`: value when missing or unparsable.
pub fn get_timespan_setting(key: &str, default: TimeSpan) -> TimeSpan {
    let s: Option<String> = lg_base::settings::get(key, None);
    s.and_then(|t| parse_timespan_c(&t)).unwrap_or(default)
}

/// C# `string.Length`: UTF-16 code units (an emoji counts 2, `chars().count()` would say 1).
///
/// `s`: text. Returns the number of UTF-16 units.
pub fn utf16_len(s: &str) -> i32 {
    s.encode_utf16().count() as i32
}

/// `ASCIIEncoding.GetBytes(text)` as used by .NET `SerialPort.Write(string)` and the telnet `StreamWriter`:
/// every non-ASCII character (a surrogate pair counts once) becomes `?`.
///
/// `text`: text to send. Returns the wire bytes.
pub fn ascii_lossy(text: &str) -> Vec<u8> {
    text.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).collect()
}

/// `string.Substring(start, len)` on chars; None where C# throws `ArgumentOutOfRangeException`.
///
/// `s`: source. `start`: first char index. `len`: char count.
pub fn substring(s: &str, start: usize, len: usize) -> Option<String> {
    let v: Vec<char> = s.chars().collect();
    if start + len > v.len() {
        return None;
    }
    Some(v[start..start + len].iter().collect())
}

/// `string.Substring(start)`; None where C# throws.
///
/// `s`: source. `start`: first char index.
pub fn substring_from(s: &str, start: usize) -> Option<String> {
    let v: Vec<char> = s.chars().collect();
    if start > v.len() {
        return None;
    }
    Some(v[start..].iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_shape() {
        let g = new_guid();
        assert_eq!(g.len(), 36);
        assert_eq!(g.matches('-').count(), 4);
        assert_ne!(g, new_guid());
        assert_eq!(new_guid_n().len(), 32);
    }

    #[test]
    fn timespan_c() {
        assert_eq!(parse_timespan_c("00:10:00").unwrap(), TimeSpan::from_minutes(10.0));
        assert_eq!(parse_timespan_c("1.02:03:04").unwrap().ticks(), ((26 * 60 + 3) * 60 + 4) * 10_000_000);
        assert_eq!(parse_timespan_c("00:00:01.5").unwrap().ticks(), 15_000_000);
        assert_eq!(parse_timespan_c("-00:00:01").unwrap().ticks(), -10_000_000);
        assert!(parse_timespan_c("abc").is_none());
        // roundtrip with Display
        let t = TimeSpan::from_seconds(61.0);
        assert_eq!(parse_timespan_c(&t.to_string()).unwrap(), t);
    }

    #[test]
    fn utf16_and_ascii() {
        assert_eq!(utf16_len("G1X1\n"), 5);
        assert_eq!(utf16_len("\u{1F600}"), 2);
        assert_eq!(utf16_len("\u{e9}"), 1);
        assert_eq!(ascii_lossy("a\u{e9}\u{1F600}b"), b"a??b".to_vec());
    }

    #[test]
    fn substrings() {
        assert_eq!(substring("Grbl 1.1f", 5, 1).as_deref(), Some("1"));
        assert_eq!(substring("Grbl", 5, 1), None);
        assert_eq!(substring_from("abc", 3).as_deref(), Some(""));
        assert_eq!(substring_from("abc", 4), None);
    }
}
