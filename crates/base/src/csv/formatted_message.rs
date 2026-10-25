// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `CSV/FormattedMessage.cs` (`FormattedMessage`, `StringList`).
//!
//! Strings are handled as `char`s (C# UTF-16 units; same result for BMP text). Null entries are `None`.

use lg_dotnet::{fmt_decimal, fmt_double, fmt_float, parse_decimal, parse_double, parse_float, Decimal, TimeSpan};

/// Date format of [`FormattedMessage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtFormat {
    /// Culture it-IT `dd/MM/yyyy HH:mm:ss`.
    Clear,
    /// Seconds since 1970-01-01.
    UnixDt,
}

/// Separated string list with escaping.
#[derive(Debug, Clone)]
pub struct StringList {
    esc: String,
    sep: String,
    phl: String,
    orig: Option<String>,
    list: Vec<Option<String>>,
}

impl Default for StringList {
    fn default() -> Self {
        Self { esc: "\\".into(), sep: ";".into(), phl: "#".into(), orig: None, list: Vec::new() }
    }
}

/// C# `substring` helper of StringList (negative len looks backwards, clamps).
///
/// `s`: chars. `start`/`len`: as C#. Returns the string.
fn substring(s: &[char], mut start: i64, mut len: i64) -> String {
    if len < 0 {
        len = -len;
        start -= len;
    }
    if start < 0 {
        start = 0;
    }
    len = len.min(s.len() as i64 - start);
    if len < 0 {
        len = 0;
    }
    s[start as usize..(start + len) as usize].iter().collect()
}

impl StringList {
    /// Empty list (default `;` separator).
    pub fn new() -> Self {
        Self::default()
    }

    /// Split a message.
    ///
    /// `msg`: text. `sep`: separator. Returns the list (C# `FromMessage`).
    pub fn from_message(msg: &str, sep: &str) -> Self {
        let mut s = Self { sep: sep.to_string(), ..Self::default() };
        s.list = s.split_message(msg);
        s.orig = Some(msg.to_string());
        s
    }

    /// Original message text, if built from one.
    pub fn original_message(&self) -> Option<&str> {
        self.orig.as_deref()
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// True when no elements.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Append an element (None = null).
    ///
    /// `s`: value.
    pub fn add(&mut self, s: Option<String>) {
        self.list.push(s);
    }

    /// Element access.
    ///
    /// `i`: index. Returns None for out of range, Some(None) for null element.
    pub fn get(&self, i: usize) -> Option<&Option<String>> {
        self.list.get(i)
    }

    /// Split with escape handling (C# `SplitMessage`).
    fn split_message(&self, msg: &str) -> Vec<Option<String>> {
        let m: Vec<char> = msg.chars().collect();
        let sep_len = self.sep.chars().count() as i64;
        let esc_len = self.esc.chars().count() as i64;
        let mut rv = Vec::new();
        let mut last = 0i64;
        let mut i = 0i64;
        while i < m.len() as i64 + 1 {
            let fend = i == m.len() as i64;
            let fsep = !fend && substring(&m, i, sep_len) == self.sep && substring(&m, i, -esc_len) != self.esc;
            if fend || fsep {
                let part = m[last as usize..i as usize].iter().collect::<String>();
                rv.push(self.sget(&part));
                i += sep_len;
                last = i;
            } else {
                i += 1;
            }
        }
        rv
    }

    /// Unescape one element.
    fn sget(&self, s: &str) -> Option<String> {
        if s == self.phl {
            return None; // null placeholder
        }
        let mut r = s.replace(&format!("{}{}", self.esc, self.phl), &self.phl);
        r = r.replace(&format!("{}{}", self.esc, self.sep), &self.sep);
        Some(r.replace(&format!("{}{}", self.esc, self.esc), &self.esc))
    }

    /// Escape one element.
    fn sput(&self, s: &Option<String>) -> String {
        match s {
            None => self.phl.clone(),
            Some(s) => {
                let mut r = s.replace(&self.esc, &format!("{0}{0}", self.esc));
                r = r.replace(&self.sep, &format!("{}{}", self.esc, self.sep));
                r.replace(&self.phl, &format!("{}{}", self.esc, self.phl))
            }
        }
    }

    /// Join to message (C# `ToMessage`). Err on empty list like the C# exception.
    pub fn to_message(&self) -> Result<String, String> {
        if self.list.is_empty() {
            return Err("empty messages not supported".into());
        }
        Ok(self.list.iter().map(|e| self.sput(e)).collect::<Vec<_>>().join(&self.sep))
    }
}

/// Typed append/read over a [`StringList`] (invariant en-US numbers).
#[derive(Debug, Clone, Default)]
pub struct FormattedMessage {
    list: StringList,
    i: usize,
}

const UNIX_EPOCH_DAYS_TICKS: i64 = 621_355_968_000_000_000;

impl FormattedMessage {
    /// Empty message.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap a list (None -> None, like C#).
    ///
    /// `l`: list.
    pub fn from_string_list(l: StringList) -> Self {
        Self { list: l, i: 0 }
    }

    /// Copy of the list.
    pub fn to_string_list(&self) -> StringList {
        self.list.clone()
    }

    /// Serialized message.
    pub fn to_message(&self) -> Result<String, String> {
        self.list.to_message()
    }

    /// Original text.
    pub fn original_message(&self) -> Option<&str> {
        self.list.original_message()
    }

    /// Append bool as "True"/"False".
    pub fn append_bool(&mut self, v: bool) {
        self.list.add(Some(if v { "True" } else { "False" }.into()));
    }
    /// Append int.
    pub fn append_i32(&mut self, v: i32) {
        self.list.add(Some(v.to_string()));
    }
    /// Append long.
    pub fn append_i64(&mut self, v: i64) {
        self.list.add(Some(v.to_string()));
    }
    /// Append decimal.
    pub fn append_decimal(&mut self, v: Decimal) {
        self.list.add(Some(fmt_decimal(v)));
    }
    /// Append float.
    pub fn append_f32(&mut self, v: f32) {
        self.list.add(Some(fmt_float(v)));
    }
    /// Append double.
    pub fn append_f64(&mut self, v: f64) {
        self.list.add(Some(fmt_double(v)));
    }
    /// Append string (None = null).
    pub fn append_str(&mut self, v: Option<&str>) {
        self.list.add(v.map(str::to_string));
    }
    /// Append TimeSpan in "c" format.
    pub fn append_timespan(&mut self, v: TimeSpan) {
        self.list.add(Some(v.to_string()));
    }
    /// Append date.
    ///
    /// `v`: naive date time. `f`: format.
    pub fn append_datetime(&mut self, v: chrono::NaiveDateTime, f: DtFormat) {
        self.list.add(Some(match f {
            DtFormat::Clear => v.format("%d/%m/%Y %H:%M:%S").to_string(),
            DtFormat::UnixDt => dotnet_to_unix(v).to_string(),
        }));
    }

    /// Reset sequential read position.
    pub fn begin_read(&mut self) {
        self.i = 0;
    }

    /// Raw element.
    fn raw(&self, idx: usize) -> Result<&Option<String>, String> {
        self.list.get(idx).ok_or_else(|| format!("index {idx} out of range"))
    }

    /// Non-null element.
    fn text(&self, idx: usize) -> Result<&str, String> {
        self.raw(idx)?.as_deref().ok_or_else(|| "null element".to_string())
    }

    /// Next index (sequential read).
    fn next_idx(&mut self) -> usize {
        self.i += 1;
        self.i - 1
    }

    /// `ReadString(index)`.
    ///
    /// `index`: element. Returns Ok(None) for a null element, Err when out of range.
    pub fn read_string(&self, index: usize) -> Result<Option<String>, String> {
        self.raw(index).cloned()
    }
    /// `ReadString()` sequential.
    pub fn read_string_next(&mut self) -> Result<Option<String>, String> {
        let i = self.next_idx();
        self.read_string(i)
    }
    /// `ReadBool(index)`: "true"/"false" case-insensitive, trimmed.
    pub fn read_bool(&self, index: usize) -> Result<bool, String> {
        match self.text(index)?.trim().to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err("bad bool".into()),
        }
    }
    /// `ReadInt(index)`.
    pub fn read_i32(&self, index: usize) -> Result<i32, String> {
        lg_dotnet::parse_int(self.text(index)?).ok_or_else(|| "bad int".into())
    }
    /// `ReadLong(index)`.
    pub fn read_i64(&self, index: usize) -> Result<i64, String> {
        self.text(index)?.trim().parse::<i64>().map_err(|e| e.to_string())
    }
    /// `ReadDecimal(index)`.
    pub fn read_decimal(&self, index: usize) -> Result<Decimal, String> {
        parse_decimal(self.text(index)?).ok_or_else(|| "bad decimal".into())
    }
    /// `ReadFloat(index)`.
    pub fn read_f32(&self, index: usize) -> Result<f32, String> {
        parse_float(self.text(index)?).ok_or_else(|| "bad float".into())
    }
    /// `ReadDouble(index)`.
    pub fn read_f64(&self, index: usize) -> Result<f64, String> {
        parse_double(self.text(index)?).ok_or_else(|| "bad double".into())
    }
    /// `ReadTimeSpan(index)`: `[-][d.]hh:mm:ss[.fffffff]`.
    pub fn read_timespan(&self, index: usize) -> Result<TimeSpan, String> {
        parse_timespan(self.text(index)?)
    }
    /// `ReadDateTime(index, format)`.
    ///
    /// `index`: element. `f`: format (default in C# is Unix). Returns naive date time.
    pub fn read_datetime(&self, index: usize, f: DtFormat) -> Result<chrono::NaiveDateTime, String> {
        let t = self.text(index)?;
        match f {
            DtFormat::Clear => chrono::NaiveDateTime::parse_from_str(t.trim(), "%d/%m/%Y %H:%M:%S").map_err(|e| e.to_string()),
            DtFormat::UnixDt => {
                let secs: i32 = lg_dotnet::parse_int(t).ok_or("bad unix time")?;
                unix_to_dotnet(secs).ok_or_else(|| "range".to_string())
            }
        }
    }
    /// Sequential `ReadInt()`.
    pub fn read_i32_next(&mut self) -> Result<i32, String> {
        let i = self.next_idx();
        self.read_i32(i)
    }
}

/// `UnixToDotNet`.
fn unix_to_dotnet(secs: i32) -> Option<chrono::NaiveDateTime> {
    chrono::DateTime::from_timestamp(secs as i64, 0).map(|d| d.naive_utc())
}

/// `DotNetToUnix` (ticks / 1e7, truncated to int).
fn dotnet_to_unix(t: chrono::NaiveDateTime) -> i32 {
    let ticks = t.and_utc().timestamp() * 10_000_000 + (t.and_utc().timestamp_subsec_nanos() / 100) as i64;
    let _ = UNIX_EPOCH_DAYS_TICKS;
    (ticks / 10_000_000) as i32
}

/// Parse `TimeSpan.Parse` common forms.
///
/// `s`: text. Returns span or Err.
fn parse_timespan(s: &str) -> Result<TimeSpan, String> {
    let s = s.trim();
    let (neg, s) = s.strip_prefix('-').map_or((false, s), |r| (true, r));
    let (main, frac) = match s.split_once('.') {
        // "d.hh:mm:ss" vs "hh:mm:ss.fff": dot after the last colon is fraction
        Some(_) if s.rfind('.') > s.rfind(':') => s.rsplit_once('.').map(|(a, b)| (a, Some(b))).unwrap(),
        _ => (s, None),
    };
    let (days, hms) = match main.split_once('.') {
        Some((d, r)) => (d.parse::<i64>().map_err(|e| e.to_string())?, r),
        None => (0, main),
    };
    let p: Vec<&str> = hms.split(':').collect();
    let num = |i: usize| p.get(i).map_or(Ok(0i64), |x| x.parse::<i64>().map_err(|e| e.to_string()));
    let (h, m, sec) = if p.len() == 3 { (num(0)?, num(1)?, num(2)?) } else { return Err("bad timespan".into()) };
    let mut ticks = ((days * 24 + h) * 60 + m) * 60 * 10_000_000 + sec * 10_000_000;
    if let Some(f) = frac {
        let mut f7 = f.to_string();
        f7.truncate(7);
        while f7.len() < 7 {
            f7.push('0');
        }
        ticks += f7.parse::<i64>().map_err(|e| e.to_string())?;
    }
    Ok(TimeSpan::from_ticks(if neg { -ticks } else { ticks }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_join() {
        // NOTE(upstream): a value ending in the escape char before a separator is mis-split; not tested
        let l = StringList::from_message("a,b\\,c,#,", ",");
        assert_eq!(l.len(), 4);
        assert_eq!(l.get(0).unwrap().as_deref(), Some("a"));
        assert_eq!(l.get(1).unwrap().as_deref(), Some("b,c"));
        assert_eq!(l.get(2).unwrap(), &None);
        assert_eq!(l.get(3).unwrap().as_deref(), Some(""));
        let mut f = FormattedMessage::new();
        f.append_i32(5);
        f.append_str(Some("x;y#z\\w"));
        f.append_str(None);
        f.append_f64(1.5);
        f.append_bool(true);
        let msg = f.to_message().unwrap();
        assert_eq!(msg, r"5;x\;y\#z\\w;#;1.5;True");
        let g = FormattedMessage::from_string_list(StringList::from_message(&msg, ";"));
        assert_eq!(g.read_i32(0).unwrap(), 5);
        assert_eq!(g.read_string(1).unwrap().unwrap(), "x;y#z\\w");
        assert_eq!(g.read_string(2).unwrap(), None);
        assert_eq!(g.read_f64(3).unwrap(), 1.5);
        assert!(g.read_bool(4).unwrap());
    }

    #[test]
    fn dates_and_spans() {
        let mut f = FormattedMessage::new();
        let d = chrono::NaiveDate::from_ymd_opt(2024, 3, 5).unwrap().and_hms_opt(1, 2, 3).unwrap();
        f.append_datetime(d, DtFormat::UnixDt);
        f.append_datetime(d, DtFormat::Clear);
        f.append_timespan(TimeSpan::from_ticks(36_000_000_000 + 5));
        assert_eq!(f.read_datetime(0, DtFormat::UnixDt).unwrap(), d);
        assert_eq!(f.read_datetime(1, DtFormat::Clear).unwrap(), d);
        assert_eq!(f.read_timespan(2).unwrap().ticks(), 36_000_000_005);
    }
}
