// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `System.TimeSpan` (.NET Fx 4.x semantics).

use std::fmt;
use std::ops::{Add, Neg, Sub};

const TICKS_PER_MS: i64 = 10_000;
const TICKS_PER_SECOND: i64 = 10_000_000;
const TICKS_PER_MINUTE: i64 = 600_000_000;
const TICKS_PER_HOUR: i64 = 36_000_000_000;
const TICKS_PER_DAY: i64 = 864_000_000_000;
const MS_PER_DAY: f64 = 86_400_000.0;
const MS_PER_HOUR: f64 = 3_600_000.0;
const MS_PER_MINUTE: f64 = 60_000.0;
const MS_PER_SECOND: f64 = 1_000.0;

/// Time interval in 100 ns ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeSpan(i64);

impl TimeSpan {
    /// `TimeSpan.Zero`.
    pub const ZERO: TimeSpan = TimeSpan(0);

    /// `new TimeSpan(ticks)`.
    ///
    /// `ticks`: 100 ns units.
    pub const fn from_ticks(ticks: i64) -> Self {
        TimeSpan(ticks)
    }

    /// Raw ticks.
    pub const fn ticks(self) -> i64 {
        self.0
    }

    /// `TimeSpan.Interval`: value*scale rounded to whole ms (half away from zero).
    ///
    /// `value`: amount. `scale`: ms per unit. Panics on NaN/overflow (C# throws).
    fn interval(value: f64, scale: f64) -> Self {
        assert!(!value.is_nan(), "TimeSpan: NaN");
        let tmp = value * scale;
        let millis = tmp + if value >= 0.0 { 0.5 } else { -0.5 };
        assert!(
            millis <= (i64::MAX / TICKS_PER_MS) as f64 && millis >= (i64::MIN / TICKS_PER_MS) as f64,
            "TimeSpan: overflow"
        );
        // (long)millis truncates toward zero
        TimeSpan((millis as i64) * TICKS_PER_MS)
    }

    /// `TimeSpan.FromDays`.
    ///
    /// `v`: days.
    pub fn from_days(v: f64) -> Self {
        Self::interval(v, MS_PER_DAY)
    }

    /// `TimeSpan.FromHours`.
    ///
    /// `v`: hours.
    pub fn from_hours(v: f64) -> Self {
        Self::interval(v, MS_PER_HOUR)
    }

    /// `TimeSpan.FromMinutes`.
    ///
    /// `v`: minutes.
    pub fn from_minutes(v: f64) -> Self {
        Self::interval(v, MS_PER_MINUTE)
    }

    /// `TimeSpan.FromSeconds`.
    ///
    /// `v`: seconds.
    pub fn from_seconds(v: f64) -> Self {
        Self::interval(v, MS_PER_SECOND)
    }

    /// `TimeSpan.FromMilliseconds`.
    ///
    /// `v`: milliseconds.
    pub fn from_milliseconds(v: f64) -> Self {
        Self::interval(v, 1.0)
    }

    /// `Days` component (truncated toward zero).
    pub fn days(self) -> i32 {
        (self.0 / TICKS_PER_DAY) as i32
    }

    /// `Hours` component (-23..=23).
    pub fn hours(self) -> i32 {
        ((self.0 / TICKS_PER_HOUR) % 24) as i32
    }

    /// `Minutes` component (-59..=59).
    pub fn minutes(self) -> i32 {
        ((self.0 / TICKS_PER_MINUTE) % 60) as i32
    }

    /// `Seconds` component (-59..=59).
    pub fn seconds(self) -> i32 {
        ((self.0 / TICKS_PER_SECOND) % 60) as i32
    }

    /// `Milliseconds` component (-999..=999).
    pub fn milliseconds(self) -> i32 {
        ((self.0 / TICKS_PER_MS) % 1000) as i32
    }

    /// `TotalDays` (multiplies by reciprocal like Fx).
    pub fn total_days(self) -> f64 {
        self.0 as f64 * (1.0 / TICKS_PER_DAY as f64)
    }

    /// `TotalHours`.
    pub fn total_hours(self) -> f64 {
        self.0 as f64 * (1.0 / TICKS_PER_HOUR as f64)
    }

    /// `TotalMinutes`.
    pub fn total_minutes(self) -> f64 {
        self.0 as f64 * (1.0 / TICKS_PER_MINUTE as f64)
    }

    /// `TotalSeconds`.
    pub fn total_seconds(self) -> f64 {
        self.0 as f64 * (1.0 / TICKS_PER_SECOND as f64)
    }

    /// `TotalMilliseconds`, clamped to the Int64/10000 range like Fx.
    pub fn total_milliseconds(self) -> f64 {
        let t = self.0 as f64 * (1.0 / TICKS_PER_MS as f64);
        let max = (i64::MAX / TICKS_PER_MS) as f64;
        let min = (i64::MIN / TICKS_PER_MS) as f64;
        if t > max {
            max
        } else if t < min {
            min
        } else {
            t
        }
    }
}

impl Add for TimeSpan {
    type Output = TimeSpan;

    /// `TimeSpan + TimeSpan`; panics on overflow (C# throws).
    fn add(self, rhs: TimeSpan) -> TimeSpan {
        TimeSpan(self.0.checked_add(rhs.0).expect("TimeSpan overflow"))
    }
}

impl Sub for TimeSpan {
    type Output = TimeSpan;

    /// `TimeSpan - TimeSpan`; panics on overflow (C# throws).
    fn sub(self, rhs: TimeSpan) -> TimeSpan {
        TimeSpan(self.0.checked_sub(rhs.0).expect("TimeSpan overflow"))
    }
}

impl Neg for TimeSpan {
    type Output = TimeSpan;

    /// Unary minus; panics on `MinValue` (C# throws).
    fn neg(self) -> TimeSpan {
        TimeSpan(self.0.checked_neg().expect("TimeSpan overflow"))
    }
}

impl fmt::Display for TimeSpan {
    /// "c" format: `[-][d.]hh:mm:ss[.fffffff]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let neg = self.0 < 0;
        let t = self.0.unsigned_abs();
        let days = t / TICKS_PER_DAY as u64;
        let rem = t % TICKS_PER_DAY as u64;
        let h = rem / TICKS_PER_HOUR as u64;
        let m = rem / TICKS_PER_MINUTE as u64 % 60;
        let s = rem / TICKS_PER_SECOND as u64 % 60;
        let frac = rem % TICKS_PER_SECOND as u64;
        let mut out = String::new();
        if neg {
            out.push('-');
        }
        if days != 0 {
            out.push_str(&format!("{days}."));
        }
        out.push_str(&format!("{h:02}:{m:02}:{s:02}"));
        if frac != 0 {
            out.push_str(&format!(".{frac:07}"));
        }
        f.write_str(&out)
    }
}
