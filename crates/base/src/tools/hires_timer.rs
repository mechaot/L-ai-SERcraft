//! Port of `Tools/HiResTimer.cs` (class `HiResTimer`).
//!
//! C# corrects the QPC frequency against `GetTickCount64`. On Rust we use the monotonic `Instant`
//! (ns resolution, no drift), so the frequency-correction part is not needed and not ported.
//! The sleep/hibernation guard (delta > 10 s is dropped, negative delta ignored) is kept.

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const NANO_IN_MILLI: i64 = 1_000_000;
/// Gap above this between two calls = system sleep, not counted.
const NANO_SLEEP_THRESHOLD: i64 = 10 * 1000 * 1000 * 1000;

struct Inner {
    origin: Instant,
    last_ref: i64,
    total_nano: i64,
}

static INNER: OnceLock<Mutex<Inner>> = OnceLock::new();

/// Lazy static state (C# static ctor).
fn inner() -> &'static Mutex<Inner> {
    INNER.get_or_init(|| Mutex::new(Inner { origin: Instant::now(), last_ref: 0, total_nano: 0 }))
}

/// Monotonic nanoseconds since first use, with sleep gaps removed.
///
/// Returns the accumulated nanoseconds (C# `HiResTimer.TotalNano`).
pub fn total_nano() -> i64 {
    let mut g = inner().lock().unwrap_or_else(|e| e.into_inner());
    let now = g.origin.elapsed().as_nanos() as i64;
    let delta = now - g.last_ref;
    if delta < 0 {
        crate::logger::log_message("Issue detector", "Negative delta detected!");
    } else if delta > NANO_SLEEP_THRESHOLD {
        // NOTE(upstream): C# prints `delta/1000*1000*1000` (operator precedence bug), kept
        let shown = (delta / 1000).wrapping_mul(1000).wrapping_mul(1000);
        crate::logger::log_message("Issue detector", &format!("System sleep/hibernation detected ({shown}s)"));
    } else {
        g.total_nano += delta;
    }
    g.last_ref = now;
    g.total_nano
}

/// Same as [`total_nano`] in ms.
///
/// Returns whole milliseconds (C# `HiResTimer.TotalMilliseconds`).
pub fn total_milliseconds() -> i64 {
    total_nano() / NANO_IN_MILLI
}

/// Timer frequency in Hz (C# `CurrentFrequency`).
///
/// Returns 1e9 (ns ticks).
pub fn current_frequency() -> i64 {
    1_000_000_000
}

/// Timer frequency at startup (C# `OriginalFrequency`).
///
/// Returns 1e9.
pub fn original_frequency() -> i64 {
    1_000_000_000
}
