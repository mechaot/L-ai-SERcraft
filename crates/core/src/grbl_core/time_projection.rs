//! Port of `TimeProjection` (in `Core/GrblCore.cs`): job counters, timing and pause accounting.
//!
//! Differences to C#: `JobStart(file, queue, global)` takes the two values it reads from its arguments
//! (`file.EstimatedTime`, `queue.Count`); the clock is injectable for tests.

use super::types::DetectedIssue;
use lg_base::sound::{play_sound, EventId};
use lg_dotnet::TimeSpan;
use lg_gcode::GPoint;

/// Time source in ms (C# `HiResTimer.TotalMilliseconds`).
pub type Clock = fn() -> i64;

fn default_clock() -> i64 {
    lg_base::tools::hires_timer::total_milliseconds()
}

/// Job timing and counters.
#[derive(Debug, Clone)]
pub struct TimeProjection {
    clock: Clock,
    e_target: TimeSpan,
    e_progress: TimeSpan,
    start: i64,
    end: i64,
    global_start: i64,
    global_end: i64,
    pause_begin: i64,
    cumulated_pause: i64,
    in_pause: bool,
    completed: bool,
    started: bool,
    target_count: i32,
    executed_count: i32,
    sent_count: i32,
    error_count: i32,
    continue_correction: i32,
    last_issue: DetectedIssue,
    last_known_wco: GPoint,
    /// Silence the warning sound of `job_error` (tests).
    pub mute_sounds: bool,
}

impl Default for TimeProjection {
    /// Same as `new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl TimeProjection {
    /// New projection with the real clock (C# ctor: `Reset(true)`).
    pub fn new() -> Self {
        Self::with_clock(default_clock)
    }

    /// New projection with a custom clock.
    ///
    /// `clock`: function returning milliseconds.
    pub fn with_clock(clock: Clock) -> Self {
        let mut t = Self {
            clock,
            e_target: TimeSpan::ZERO,
            e_progress: TimeSpan::ZERO,
            start: 0,
            end: 0,
            global_start: 0,
            global_end: 0,
            pause_begin: 0,
            cumulated_pause: 0,
            in_pause: false,
            completed: false,
            started: false,
            target_count: 0,
            executed_count: 0,
            sent_count: 0,
            error_count: 0,
            continue_correction: 0,
            last_issue: DetectedIssue::Unknown,
            last_known_wco: GPoint::ZERO,
            mute_sounds: false,
        };
        t.reset(true);
        t
    }

    fn now(&self) -> i64 {
        (self.clock)()
    }

    /// `Reset(global)`.
    ///
    /// `global`: also reset the multi-pass start/end.
    pub fn reset(&mut self, global: bool) {
        self.e_target = TimeSpan::ZERO;
        self.e_progress = TimeSpan::ZERO;
        self.start = 0;
        self.end = 0;
        if global {
            self.global_start = 0;
            self.global_end = 0;
        }
        self.pause_begin = 0;
        self.cumulated_pause = 0;
        self.in_pause = false;
        self.completed = false;
        self.started = false;
        self.executed_count = 0;
        self.sent_count = 0;
        self.error_count = 0;
        self.target_count = 0;
        self.continue_correction = 0;
        self.last_issue = DetectedIssue::Unknown;
        self.last_known_wco = GPoint::ZERO;
    }

    /// `LastKnownWCO` getter.
    pub fn last_known_wco(&self) -> GPoint {
        self.last_known_wco
    }

    /// `LastKnownWCO` setter: only stored while a job runs.
    ///
    /// `v`: new WCO.
    pub fn set_last_known_wco(&mut self, v: GPoint) {
        if self.in_program() {
            self.last_known_wco = v;
        }
    }

    /// `EstimatedTarget`.
    pub fn estimated_target(&self) -> TimeSpan {
        self.e_target
    }

    /// `InProgram`.
    pub fn in_program(&self) -> bool {
        self.started && !self.completed
    }

    /// `Target`.
    pub fn target(&self) -> i32 {
        self.target_count
    }

    /// `Sent` (corrected for resumed jobs).
    pub fn sent(&self) -> i32 {
        self.sent_count - self.continue_correction
    }

    /// `Executed` (corrected for resumed jobs).
    pub fn executed(&self) -> i32 {
        self.executed_count - self.continue_correction
    }

    /// `ProjectedTarget`.
    pub fn projected_target(&self) -> TimeSpan {
        if self.started {
            let real = self.true_job_time().total_seconds();
            let target = self.e_target.total_seconds();
            let done = self.e_progress.total_seconds();
            if done != 0.0 { TimeSpan::from_seconds(real * target / done) + self.total_job_pauses() } else { self.estimated_target() }
        } else {
            TimeSpan::ZERO
        }
    }

    fn true_job_time(&self) -> TimeSpan {
        self.total_job_time() - self.total_job_pauses()
    }

    /// `TotalJobTime`.
    pub fn total_job_time(&self) -> TimeSpan {
        if self.completed {
            TimeSpan::from_milliseconds((self.end - self.start) as f64)
        } else if self.started {
            TimeSpan::from_milliseconds((self.now() - self.start) as f64)
        } else {
            TimeSpan::ZERO
        }
    }

    /// `TotalGlobalJobTime`.
    pub fn total_global_job_time(&self) -> TimeSpan {
        if self.completed {
            TimeSpan::from_milliseconds((self.global_end - self.global_start) as f64)
        } else if self.started {
            TimeSpan::from_milliseconds((self.now() - self.global_start) as f64)
        } else {
            TimeSpan::ZERO
        }
    }

    fn total_job_pauses(&self) -> TimeSpan {
        if self.in_pause {
            TimeSpan::from_milliseconds((self.cumulated_pause + (self.now() - self.pause_begin)) as f64)
        } else {
            TimeSpan::from_milliseconds(self.cumulated_pause as f64)
        }
    }

    /// `JobStart(file, queue, global)`.
    ///
    /// `estimated`: `file.EstimatedTime`. `queued`: `mQueuePtr.Count` (header + file lines). `global`: first
    /// pass of a multi-pass job.
    pub fn job_start(&mut self, estimated: TimeSpan, queued: usize, global: bool) {
        if !self.started {
            self.e_target = estimated;
            self.target_count = queued as i32;
            self.e_progress = TimeSpan::ZERO;
            self.start = self.now();
            if global {
                self.global_start = self.start;
            }
            self.pause_begin = 0;
            self.in_pause = false;
            self.completed = false;
            self.started = true;
            self.executed_count = 0;
            self.sent_count = 0;
            self.error_count = 0;
            self.continue_correction = 0;
            self.last_issue = DetectedIssue::Unknown;
            self.last_known_wco = GPoint::ZERO;
        }
    }

    /// `JobContinue(file, position, added)`.
    ///
    /// `estimated`: `file.EstimatedTime`. `file_count`: `file.Count`. `position`: resume line. `added`: commands
    /// queued before the resumed lines (`mQueuePtr.Count`).
    pub fn job_continue(&mut self, estimated: TimeSpan, file_count: usize, position: i32, added: usize) {
        if !self.started {
            if self.e_target == TimeSpan::ZERO {
                self.e_target = estimated;
            }
            if self.target_count == 0 {
                self.target_count = file_count as i32;
            }
            if self.start == 0 {
                self.start = self.now();
                self.global_start = self.start;
            }
            self.pause_begin = 0;
            self.in_pause = false;
            self.completed = false;
            self.started = true;
            self.executed_count = position;
            self.sent_count = position;
            self.last_issue = DetectedIssue::Unknown;
            self.continue_correction = added as i32;
        }
    }

    /// `JobSent`.
    pub fn job_sent(&mut self) {
        if self.started && !self.completed {
            self.sent_count += 1;
        }
    }

    /// `JobError`: warning sound + counter.
    pub fn job_error(&mut self) {
        if self.started && !self.completed {
            if !self.mute_sounds {
                play_sound(EventId::Warning);
            }
            self.error_count += 1;
        }
    }

    /// `JobExecuted(EstimatedProgress)`.
    ///
    /// `progress`: `TimeOffset` of the executed command.
    pub fn job_executed(&mut self, progress: TimeSpan) {
        if self.started && !self.completed {
            self.executed_count += 1;
            self.e_progress = progress;
        }
    }

    /// `JobPause`.
    pub fn job_pause(&mut self) {
        if self.started && !self.completed && !self.in_pause {
            self.in_pause = true;
            self.pause_begin = self.now();
        }
    }

    /// `JobResume`.
    pub fn job_resume(&mut self) {
        if self.started && !self.completed && self.in_pause {
            self.cumulated_pause += self.now() - self.pause_begin;
            self.in_pause = false;
        }
    }

    /// `JobEnd(global)`.
    ///
    /// `global`: also set the global end. Returns true when a running job was ended.
    pub fn job_end(&mut self, global: bool) -> bool {
        if self.started && !self.completed {
            // last command may have been a pause: close and accumulate it
            self.job_resume();
            self.end = self.now();
            if global {
                self.global_end = self.end;
            }
            self.completed = true;
            self.started = false;
            true
        } else {
            false
        }
    }

    /// `JobIssue`.
    ///
    /// `issue`: remembered as `LastIssue`.
    pub fn job_issue(&mut self, issue: DetectedIssue) {
        self.last_issue = issue;
    }

    /// `ErrorCount`.
    pub fn error_count(&self) -> i32 {
        self.error_count
    }

    /// `LastIssue`.
    pub fn last_issue(&self) -> DetectedIssue {
        self.last_issue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    static NOW: AtomicI64 = AtomicI64::new(1000);
    fn fake_now() -> i64 {
        NOW.load(Ordering::SeqCst)
    }

    #[test]
    fn counters_pause_and_times() {
        NOW.store(1000, Ordering::SeqCst);
        let mut tp = TimeProjection::with_clock(fake_now);
        tp.mute_sounds = true;
        assert!(!tp.in_program());
        tp.job_start(TimeSpan::from_seconds(100.0), 10, true);
        assert!(tp.in_program());
        assert_eq!(tp.target(), 10);
        tp.job_sent();
        tp.job_executed(TimeSpan::from_seconds(10.0));
        assert_eq!((tp.sent(), tp.executed()), (1, 1));
        NOW.store(6000, Ordering::SeqCst);
        assert_eq!(tp.total_job_time(), TimeSpan::from_seconds(5.0));
        // pause 2 s
        tp.job_pause();
        NOW.store(8000, Ordering::SeqCst);
        tp.job_resume();
        NOW.store(11000, Ordering::SeqCst);
        // real = 10 - 2 = 8 s for 10% -> 80 s + 2 s pause
        assert_eq!(tp.projected_target(), TimeSpan::from_seconds(82.0));
        tp.job_error();
        assert_eq!(tp.error_count(), 1);
        assert!(tp.job_end(true));
        assert!(!tp.job_end(true));
        assert_eq!(tp.total_job_time(), TimeSpan::from_seconds(10.0));
        assert_eq!(tp.total_global_job_time(), TimeSpan::from_seconds(10.0));
        assert!(!tp.in_program());
        // counters stay readable after the end
        assert_eq!(tp.executed(), 1);
    }

    #[test]
    fn continue_correction_and_wco() {
        NOW.store(5000, Ordering::SeqCst);
        let mut tp = TimeProjection::with_clock(fake_now);
        tp.set_last_known_wco(GPoint::new(1.0, 2.0, 3.0));
        assert_eq!(tp.last_known_wco(), GPoint::ZERO);
        tp.job_continue(TimeSpan::from_seconds(50.0), 100, 40, 3);
        assert_eq!(tp.target(), 100);
        // positions are corrected by what was queued ahead of the resumed lines
        assert_eq!((tp.sent(), tp.executed()), (37, 37));
        tp.set_last_known_wco(GPoint::new(1.0, 2.0, 3.0));
        assert_eq!(tp.last_known_wco(), GPoint::new(1.0, 2.0, 3.0));
        tp.job_issue(DetectedIssue::ManualAbort);
        assert_eq!(tp.last_issue(), DetectedIssue::ManualAbort);
        // restart does nothing while started
        tp.job_start(TimeSpan::ZERO, 1, true);
        assert_eq!(tp.target(), 100);
        tp.reset(true);
        assert_eq!(tp.target(), 0);
        assert_eq!(tp.last_issue(), DetectedIssue::Unknown);
    }
}
