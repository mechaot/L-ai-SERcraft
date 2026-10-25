// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Tools/Cronometro.cs`: `TimingBase`, `ElapsedFromEvent`, `SimpleCrono`, `AdvancedCrono`,
//! `PeriodicEventTimer`.

use super::hires_timer;
use lg_dotnet::TimeSpan;
use std::sync::OnceLock;

static STARTUP: OnceLock<i64> = OnceLock::new();

/// Time since first call (C# `TimingBase.TimeFromApplicationStartup`).
///
/// Returns a [`TimeSpan`] (ticks = ns/100).
pub fn time_from_application_startup() -> TimeSpan {
    let start = *STARTUP.get_or_init(hires_timer::total_nano);
    TimeSpan::from_ticks((hires_timer::total_nano() - start) / 100)
}

/// Elapsed time since a start event.
#[derive(Debug, Clone, Default)]
pub struct ElapsedFromEvent {
    started: bool,
    starttime: TimeSpan,
}

impl ElapsedFromEvent {
    /// New stopped timer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start (or restart) counting.
    pub fn start(&mut self) {
        self.starttime = time_from_application_startup();
        self.started = true;
    }

    /// Elapsed time; zero when not started.
    pub fn elapsed_time(&self) -> TimeSpan {
        if !self.started {
            TimeSpan::ZERO
        } else {
            TimeSpan::from_ticks(time_from_application_startup().ticks() - self.starttime.ticks())
        }
    }

    /// Stop and clear.
    pub fn reset(&mut self) {
        self.starttime = TimeSpan::ZERO;
        self.started = false;
    }

    /// True while started.
    pub fn running(&self) -> bool {
        self.started
    }
}

/// Start/stop stopwatch that keeps last value after stop.
#[derive(Debug, Clone, Default)]
pub struct SimpleCrono {
    timer: ElapsedFromEvent,
    elapsed: TimeSpan,
}

impl SimpleCrono {
    /// New stopwatch.
    ///
    /// `start`: begin counting immediately.
    pub fn new(start: bool) -> Self {
        let mut c = Self::default();
        if start {
            c.start();
        }
        c
    }

    /// Start counting.
    pub fn start(&mut self) {
        self.timer.start();
    }

    /// Stop, keep elapsed.
    pub fn stop(&mut self) {
        self.elapsed = self.timer.elapsed_time();
        self.timer.reset();
    }

    /// Live value when running, else the value at stop.
    pub fn elapsed_time(&self) -> TimeSpan {
        if self.timer.running() { self.timer.elapsed_time() } else { self.elapsed }
    }

    /// Clear all.
    pub fn reset(&mut self) {
        self.elapsed = TimeSpan::ZERO;
        self.timer.reset();
    }
}

/// Stopwatch with lap list.
#[derive(Debug, Clone, Default)]
pub struct AdvancedCrono {
    base: ElapsedFromEvent,
    times: Vec<TimeSpan>,
}

impl AdvancedCrono {
    /// New stopped stopwatch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start counting.
    pub fn start(&mut self) {
        self.base.start();
    }

    /// Elapsed since start.
    pub fn elapsed_time(&self) -> TimeSpan {
        self.base.elapsed_time()
    }

    /// True while running.
    pub fn running(&self) -> bool {
        self.base.running()
    }

    /// Store current elapsed as lap (C# `SalvaIntermedio`).
    pub fn salva_intermedio(&mut self) {
        let e = self.base.elapsed_time();
        self.times.push(e);
    }

    /// Stop and clear laps.
    pub fn reset(&mut self) {
        self.base.reset();
        self.times.clear();
    }

    /// Lap list (C# `Intermedi`).
    pub fn intermedi(&self) -> &[TimeSpan] {
        &self.times
    }
}

/// Periodic expiry check.
#[derive(Debug, Clone)]
pub struct PeriodicEventTimer {
    period: TimeSpan,
    crono: ElapsedFromEvent,
    period_count: i64,
}

impl PeriodicEventTimer {
    /// New timer.
    ///
    /// `period`: period length. `start`: start now.
    pub fn new(period: TimeSpan, start: bool) -> Self {
        let mut t = Self { period, crono: ElapsedFromEvent::new(), period_count: 0 };
        if start {
            t.start();
        }
        t
    }

    /// Start counting.
    pub fn start(&mut self) {
        self.crono.start();
    }

    /// True while started.
    pub fn running(&self) -> bool {
        self.crono.running()
    }

    /// Current period.
    pub fn period(&self) -> TimeSpan {
        self.period
    }

    /// Change period; re-inits the stopwatch (C# `ReInit`) when different. The period count is
    /// NOT reset (C# `ReInit` leaves `m_periodcount` alone).
    ///
    /// `p`: new period.
    pub fn set_period(&mut self, p: TimeSpan) {
        if self.period != p {
            let (running, count) = (self.crono.running(), self.period_count);
            *self = Self::new(p, running);
            self.period_count = count;
        }
    }

    /// True once per elapsed period (side effect: updates counter).
    ///
    /// Returns whether a new period has passed since the last call.
    pub fn expired(&mut self) -> bool {
        let newperiod = self.crono.elapsed_time().ticks() / self.period.ticks();
        let rv = newperiod > self.period_count;
        self.period_count = newperiod;
        rv
    }

    /// Number of periods counted so far.
    pub fn num_period(&self) -> i64 {
        self.period_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn hires_monotonic() {
        let a = hires_timer::total_nano();
        std::thread::sleep(Duration::from_millis(5));
        let b = hires_timer::total_nano();
        assert!(b - a >= 4_000_000);
        assert!(hires_timer::total_milliseconds() >= 0);
    }

    #[test]
    fn crono_states() {
        let mut e = ElapsedFromEvent::new();
        assert_eq!(e.elapsed_time(), TimeSpan::ZERO);
        e.start();
        std::thread::sleep(Duration::from_millis(10));
        assert!(e.elapsed_time().total_milliseconds() >= 9.0);
        e.reset();
        assert!(!e.running());

        let mut s = SimpleCrono::new(true);
        std::thread::sleep(Duration::from_millis(10));
        s.stop();
        let v = s.elapsed_time();
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(s.elapsed_time(), v);
        s.reset();
        assert_eq!(s.elapsed_time(), TimeSpan::ZERO);

        let mut a = AdvancedCrono::new();
        a.start();
        a.salva_intermedio();
        a.salva_intermedio();
        assert_eq!(a.intermedi().len(), 2);
        a.reset();
        assert!(a.intermedi().is_empty());
    }

    #[test]
    fn periodic_timer() {
        let mut p = PeriodicEventTimer::new(TimeSpan::from_milliseconds(20.0), true);
        assert!(!p.expired());
        std::thread::sleep(Duration::from_millis(50));
        assert!(p.expired());
        assert!(p.num_period() >= 2);
        assert!(!p.expired());
        p.set_period(TimeSpan::from_milliseconds(30.0));
        assert!(p.running());
        // C# ReInit keeps m_periodcount
        assert!(p.num_period() >= 2);
    }
}
