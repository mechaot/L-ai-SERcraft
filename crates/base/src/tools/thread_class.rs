//! Port of `Tools/ThreadClass.cs` (`ThreadClass` + `ThreadObject`).
//!
//! The loop body can reach its own [`ThreadHandle`] to change the sleep time or check the exit flag,
//! like C# code that touches `TX.SleepTime` / `TX.MustExitTH()`. Thread priority / apartment: not ported.
//! `Abort` after join timeout is impossible in Rust: the thread is detached and a debug line is logged.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Boxed loop callback.
pub type Callback = Box<dyn FnMut() + Send>;

#[derive(Default)]
struct Flags {
    must_exit: bool,
    user_event: bool,
}

struct Shared {
    flags: Mutex<Flags>,
    cv: Condvar,
    sleep_time: AtomicI32,
}

/// Cloneable view on a running [`ThreadObject`] (sleep time, exit flag, wake event).
#[derive(Clone)]
pub struct ThreadHandle {
    sh: Arc<Shared>,
}

impl ThreadHandle {
    /// Current sleep time in ms (C# `SleepTime`).
    pub fn sleep_time(&self) -> i32 {
        self.sh.sleep_time.load(Ordering::SeqCst)
    }

    /// Set sleep time; negative clamps to 0 (C# `Math.Max(value,0)`).
    ///
    /// `ms`: milliseconds between loop runs.
    pub fn set_sleep_time(&self, ms: i32) {
        self.sh.sleep_time.store(ms.max(0), Ordering::SeqCst);
    }

    /// Wake the sleeping loop early (C# user `EventWaitHandle.Set`).
    pub fn wake(&self) {
        self.sh.flags.lock().unwrap_or_else(|e| e.into_inner()).user_event = true;
        self.sh.cv.notify_all();
    }

    /// True when stop was requested (C# `MustExitTH()`).
    pub fn must_exit(&self) -> bool {
        self.sh.flags.lock().unwrap_or_else(|e| e.into_inner()).must_exit
    }

    /// Wait up to `ms` for the exit request (C# `MustExitTH(int)`).
    ///
    /// `ms`: max wait. Returns true if exit was requested.
    pub fn must_exit_wait(&self, ms: i32) -> bool {
        let deadline = Instant::now() + Duration::from_millis(ms.max(0) as u64);
        let mut g = self.sh.flags.lock().unwrap_or_else(|e| e.into_inner());
        while !g.must_exit {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            g = self.sh.cv.wait_timeout(g, deadline - now).unwrap_or_else(|e| e.into_inner()).0;
        }
        g.must_exit
    }

    /// C# `MustRun`: sleep `sleep_time` ms; false when exit requested, true on timeout or user event.
    fn must_run(&self) -> bool {
        let deadline = Instant::now() + Duration::from_millis(self.sleep_time() as u64);
        let mut g = self.sh.flags.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if g.must_exit {
                return false;
            }
            if g.user_event {
                g.user_event = false;
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            g = self.sh.cv.wait_timeout(g, deadline - now).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}

/// Looping worker thread.
pub struct ThreadObject {
    handle: ThreadHandle,
    name: String,
    delegate: Arc<Mutex<Option<Callback>>>,
    first_run: Arc<Mutex<Option<Callback>>>,
    th: Option<JoinHandle<()>>,
    stop_wait_timeout: i32,
}

impl ThreadObject {
    /// New stopped thread object without callbacks.
    ///
    /// `name`: thread name. `sleep_time`: ms between runs.
    pub fn new(name: &str, sleep_time: i32) -> Self {
        let sh = Arc::new(Shared { flags: Mutex::new(Flags::default()), cv: Condvar::new(), sleep_time: AtomicI32::new(sleep_time.max(0)) });
        Self {
            handle: ThreadHandle { sh },
            name: name.to_string(),
            delegate: Arc::new(Mutex::new(None)),
            first_run: Arc::new(Mutex::new(None)),
            th: None,
            stop_wait_timeout: 5000,
        }
    }

    /// New thread object with callbacks (C# ctor).
    ///
    /// `name`: thread name. `sleep_time`: ms between runs. `delegate`: loop body.
    /// `first_run`: optional callback run once before the loop.
    pub fn with_callbacks(name: &str, sleep_time: i32, delegate: Callback, first_run: Option<Callback>) -> Self {
        let t = Self::new(name, sleep_time);
        t.set_delegate(delegate);
        *t.first_run.lock().unwrap_or_else(|e| e.into_inner()) = first_run;
        t
    }

    /// Replace the loop body.
    ///
    /// `f`: new body.
    pub fn set_delegate(&self, f: Callback) {
        *self.delegate.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
    }

    /// Replace the first-run callback.
    ///
    /// `f`: new callback.
    pub fn set_first_run(&self, f: Callback) {
        *self.first_run.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
    }

    /// Handle usable inside callbacks (clone before moving closures in).
    ///
    /// Returns a cloneable [`ThreadHandle`].
    pub fn handle(&self) -> ThreadHandle {
        self.handle.clone()
    }

    /// Thread name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// True after `start`, until `stop`.
    pub fn running(&self) -> bool {
        self.th.is_some()
    }

    /// Current sleep time in ms.
    pub fn sleep_time(&self) -> i32 {
        self.handle.sleep_time()
    }

    /// Set sleep time in ms (clamped to >= 0).
    ///
    /// `ms`: new value.
    pub fn set_sleep_time(&self, ms: i32) {
        self.handle.set_sleep_time(ms);
    }

    /// Join timeout used by `stop` (default 5000 ms).
    pub fn stop_wait_timeout(&self) -> i32 {
        self.stop_wait_timeout
    }

    /// Set join timeout; negative clamps to 0 (0 = do not wait).
    ///
    /// `ms`: timeout.
    pub fn set_stop_wait_timeout(&mut self, ms: i32) {
        self.stop_wait_timeout = ms.max(0);
    }

    /// Wake the sleeping loop.
    pub fn wake(&self) {
        self.handle.wake();
    }

    /// True when stop was requested.
    pub fn must_exit(&self) -> bool {
        self.handle.must_exit()
    }

    /// Start the thread; no-op when already running.
    pub fn start(&mut self) {
        if self.th.is_some() {
            return;
        }
        // fresh flags for a new run
        *self.handle.sh.flags.lock().unwrap_or_else(|e| e.into_inner()) = Flags::default();
        let (h, d, f) = (self.handle.clone(), self.delegate.clone(), self.first_run.clone());
        self.th = std::thread::Builder::new().name(self.name.clone()).spawn(move || run_loop(h, d, f)).ok();
    }

    /// Request exit and join (max `stop_wait_timeout` ms). Safe to call from inside the thread.
    pub fn stop(&mut self) {
        let Some(th) = self.th.take() else { return };
        self.handle.sh.flags.lock().unwrap_or_else(|e| e.into_inner()).must_exit = true;
        self.handle.sh.cv.notify_all();
        // re-entrant stop: only flag, no join
        if th.thread().id() == std::thread::current().id() {
            log::debug!("re-entrant thread stop '{}'", self.name);
            return;
        }
        // join with timeout: poll is_finished
        let deadline = Instant::now() + Duration::from_millis(self.stop_wait_timeout as u64);
        while !th.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        if th.is_finished() {
            let _ = th.join();
        } else {
            // C# would Abort(); we detach
            log::debug!("thread '{}' did not stop in time, detached", self.name);
        }
    }
}

impl Drop for ThreadObject {
    /// C# `Dispose` = `Stop`.
    fn drop(&mut self) {
        self.stop();
    }
}

/// Thread body: first run, loop while must_run, call delegate.
///
/// `h`: handle. `d`: loop body slot. `f`: first-run slot.
fn run_loop(h: ThreadHandle, d: Arc<Mutex<Option<Callback>>>, f: Arc<Mutex<Option<Callback>>>) {
    if let Some(cb) = f.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        cb();
    }
    while h.must_run() {
        if let Some(cb) = d.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            cb();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn loop_runs_and_stops() {
        let n = Arc::new(AtomicUsize::new(0));
        let first = Arc::new(AtomicUsize::new(0));
        let (n2, f2) = (n.clone(), first.clone());
        let mut t = ThreadObject::with_callbacks(
            "t",
            1,
            Box::new(move || {
                n2.fetch_add(1, Ordering::SeqCst);
            }),
            Some(Box::new(move || {
                f2.fetch_add(1, Ordering::SeqCst);
            })),
        );
        assert!(!t.running());
        t.start();
        std::thread::sleep(Duration::from_millis(50));
        t.stop();
        assert!(!t.running());
        assert_eq!(first.load(Ordering::SeqCst), 1);
        let c = n.load(Ordering::SeqCst);
        assert!(c > 3, "{c}");
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(n.load(Ordering::SeqCst), c);
    }

    #[test]
    fn sleep_time_from_inside_and_wake() {
        let mut t = ThreadObject::new("t2", 10_000);
        let h = t.handle();
        let cnt = Arc::new(AtomicUsize::new(0));
        let c2 = cnt.clone();
        t.set_delegate(Box::new(move || {
            c2.fetch_add(1, Ordering::SeqCst);
            h.set_sleep_time(-5); // clamps to 0
        }));
        t.start();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(cnt.load(Ordering::SeqCst), 0); // still sleeping 10 s
        t.wake();
        std::thread::sleep(Duration::from_millis(50));
        assert!(cnt.load(Ordering::SeqCst) > 1);
        assert_eq!(t.sleep_time(), 0);
        t.stop();
    }

    #[test]
    fn restart_works() {
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        let mut t = ThreadObject::with_callbacks("t3", 1, Box::new(move || { n2.fetch_add(1, Ordering::SeqCst); }), None);
        t.start();
        std::thread::sleep(Duration::from_millis(10));
        t.stop();
        let a = n.load(Ordering::SeqCst);
        t.start();
        std::thread::sleep(Duration::from_millis(20));
        t.stop();
        assert!(n.load(Ordering::SeqCst) > a);
    }
}
