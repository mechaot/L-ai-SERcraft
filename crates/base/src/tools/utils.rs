// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of class `Utils` in `Tools/HiResTimer.cs` (TimeSpanToString, HumanReadableDateDiff, OpenLink).

use chrono::{Months, NaiveDateTime};
use lg_dotnet::TimeSpan;

/// Time unit granularity (C# `Utils.TimePrecision`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimePrecision {
    Years = 0,
    Month = 1,
    Day = 2,
    Hour = 3,
    Minute = 4,
    Second = 5,
    Millisecond = 6,
}

impl TimePrecision {
    /// Int to enum.
    ///
    /// `v`: 0..=6. Returns None otherwise.
    pub fn from_i32(v: i32) -> Option<Self> {
        use TimePrecision::*;
        Some(match v {
            0 => Years,
            1 => Month,
            2 => Day,
            3 => Hour,
            4 => Minute,
            5 => Second,
            6 => Millisecond,
            _ => return None,
        })
    }
}

/// `Utils.TimeSpanToString`: human readable span ("1 hour 5 min").
///
/// `span`: time span. `precision`: smallest unit shown. `max_fallback`: coarsest fallback unit when
/// leading units are zero. `separator`: between parts. `write_suffix`: "now"/"ago" texts.
/// Returns text; "+∞"/"-∞" for Max/Min. Spans that overflow DateTime (C# throws) give "+∞"/"-∞" too.
pub fn time_span_to_string(span: TimeSpan, precision: TimePrecision, max_fallback: TimePrecision, separator: &str, write_suffix: bool) -> String {
    if span.ticks() == i64::MAX {
        return "+∞".into();
    }
    if span.ticks() == i64::MIN {
        return "-∞".into();
    }
    let n = chrono::Local::now().naive_local();
    // ticks are 100 ns
    match add_ticks(n, span.ticks()) {
        Some(o) => human_readable_date_diff(n, o, precision, max_fallback, separator, write_suffix),
        None if span.ticks() > 0 => "+∞".into(),
        None => "-∞".into(),
    }
}

/// Add 100 ns ticks to a datetime.
///
/// `n`: base. `ticks`: signed ticks. Returns None on overflow.
fn add_ticks(n: NaiveDateTime, ticks: i64) -> Option<NaiveDateTime> {
    let secs = ticks.div_euclid(10_000_000);
    let rem = ticks.rem_euclid(10_000_000) * 100;
    n.checked_add_signed(chrono::Duration::try_seconds(secs)?)?.checked_add_signed(chrono::Duration::nanoseconds(rem))
}

/// `Utils.HumanReadableDateDiff`.
///
/// `main_date`,`other_date`: the two times. `precision`: smallest unit. `max_fallback`: coarsest unit
/// allowed when higher parts are zero. `separator`: part separator. `write_suffix`: "now"/"ago".
/// Returns text like "2 days, 3 hours".
pub fn human_readable_date_diff(
    main_date: NaiveDateTime,
    other_date: NaiveDateTime,
    mut precision: TimePrecision,
    max_fallback: TimePrecision,
    separator: &str,
    write_suffix: bool,
) -> String {
    use TimePrecision as P;
    let (mut bigger, smallest) = if main_date >= other_date { (main_date, other_date) } else { (other_date, main_date) };
    let (mut years, mut months, mut days) = (0, 0, 0);
    // peel years, months, days (AddMonths clamps day like .NET)
    while let Some(d) = bigger.checked_sub_months(Months::new(12)).filter(|d| *d >= smallest) {
        years += 1;
        bigger = d;
    }
    while let Some(d) = bigger.checked_sub_months(Months::new(1)).filter(|d| *d >= smallest) {
        months += 1;
        bigger = d;
    }
    while let Some(d) = bigger.checked_sub_signed(chrono::Duration::days(1)).filter(|d| *d >= smallest) {
        days += 1;
        bigger = d;
    }
    let dns = (bigger - smallest).num_nanoseconds().unwrap_or(0);
    let diff = TimeSpan::from_ticks(dns / 100);
    let (hours, minutes, seconds, ms) = (diff.hours(), diff.minutes(), diff.seconds(), diff.milliseconds());

    // precision fallback
    if precision == P::Years && max_fallback > P::Years && years == 0 {
        precision = P::Month;
    }
    if precision == P::Month && max_fallback > P::Month && years == 0 && months == 0 {
        precision = P::Day;
    }
    if precision == P::Day && max_fallback > P::Day && years == 0 && months == 0 && days == 0 {
        precision = P::Hour;
    }
    if precision == P::Hour && max_fallback > P::Hour && years == 0 && months == 0 && days == 0 && hours == 0 {
        precision = P::Minute;
    }
    if precision == P::Minute && max_fallback > P::Minute && years == 0 && months == 0 && days == 0 && hours == 0 && minutes == 0 {
        precision = P::Second;
    }
    if precision == P::Second && max_fallback > P::Second && years == 0 && months == 0 && days == 0 && hours == 0 && minutes == 0 && seconds == 0 {
        precision = P::Millisecond;
    }

    let mut parts: Vec<String> = Vec::new();
    let mut add = |n: i32, one: &str, many: &str| parts.push(format!("{} {}", n, if n == 1 { one } else { many }));
    if years > 0 {
        add(years, "year", "years");
    }
    if precision > P::Years && months > 0 {
        add(months, "month", "months");
    }
    if precision > P::Month && days > 0 {
        add(days, "day", "days");
    }
    if precision > P::Day && hours > 0 {
        add(hours, "hour", "hours");
    }
    if precision > P::Hour && minutes > 0 {
        add(minutes, "min", "min");
    }
    if precision > P::Minute && seconds > 0 {
        add(seconds, "sec", "sec");
    }
    if precision > P::Second && ms > 0 {
        add(ms, "ms", "ms");
    }

    if parts.is_empty() {
        if write_suffix {
            return "now".into();
        }
        // write zero + precision
        let unit = match precision {
            P::Years => "years",
            P::Month => "months",
            P::Day => "days",
            P::Hour => "hours",
            P::Minute => "min",
            P::Second => "sec",
            P::Millisecond => "ms",
        };
        return format!("0 {unit}");
    }
    let mut rv = parts.join(separator);
    if write_suffix && main_date > other_date {
        rv.push_str(" ago");
    }
    rv
}

/// `Utils.OpenLink`: open every `http(s)://` entry of a `|` separated list.
///
/// `url`: list of URLs. No usage id is ever inserted (C# filled `{0}` with `UsageStats.GetID()`).
pub fn open_link(url: &str) {
    for s in url.split('|') {
        let l = s.to_lowercase();
        if l.starts_with("http://") || l.starts_with("https://") {
            open_target(s);
        }
    }
}

/// Open a file or folder with the OS default handler.
///
/// `p`: path to open.
pub fn open_path(p: &std::path::Path) {
    open_target(&p.to_string_lossy());
}

/// Spawn the OS opener for `target` (URL or path), non blocking.
///
/// `target`: string handed to the opener.
pub fn open_target(target: &str) {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd").args(["/C", "start", "", target]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(target).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(target).spawn();
    let _ = r;
}
