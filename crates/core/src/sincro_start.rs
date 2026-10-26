// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `SincroStart.cs`: "start all instances" signal between LaserGRBL processes.
//!
//! The Windows named `EventWaitHandle` is replaced by UDP datagrams on loopback: every listening instance
//! binds `127.0.0.1:0` and writes its port into `<temp>/SincroStart/<pid>.port`; `signal()` sends a datagram
//! to every registered live instance (own process included, like the global event). Same semantics:
//! `RunProgram` / `CycleStartResume` / `FeedHold` chosen by state, then 1 s without looking at the flag.

use crate::grbl_core::GrblCore;
use lg_base::paths::temp_path;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const MAGIC: &[u8] = b"LASERGRBL-SYNCRO";

/// What a signal asks the instance to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SincroAction {
    /// `CanSendFile`: start the program (UI: safety countdown, then `run_program`).
    RunProgram,
    /// `CanResumeHold`: cycle start / resume.
    CycleStartResume,
    /// `CanFeedHold`: feed hold.
    FeedHold,
}

/// Handler for [`SincroAction::RunProgram`] (UI shows the countdown and calls `GrblCore::run_program`).
pub type RunHandler = Box<dyn Fn(&Arc<GrblCore>) + Send + Sync>;

struct Listener {
    exit: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    reg: PathBuf,
}

static LISTENER: Mutex<Option<Listener>> = Mutex::new(None);

fn registry_dir() -> PathBuf {
    let d = temp_path().join("SincroStart");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Pure decision of the listener loop.
///
/// `can_send`/`can_resume`/`can_hold`: the core `Can*` flags. Returns the action or None.
pub fn decide(can_send: bool, can_resume: bool, can_hold: bool) -> Option<SincroAction> {
    if can_send {
        Some(SincroAction::RunProgram)
    } else if can_resume {
        Some(SincroAction::CycleStartResume)
    } else if can_hold {
        Some(SincroAction::FeedHold)
    } else {
        None
    }
}

/// `StartListen(core)`.
///
/// `core`: the engine. `on_run`: handler for run requests (None: `core.run_program()` directly, no countdown).
/// Returns Err when the socket or registry file cannot be created.
pub fn start_listen(core: Arc<GrblCore>, on_run: Option<RunHandler>) -> std::io::Result<()> {
    let mut g = LISTENER.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_some() {
        return Ok(());
    }
    let sock = UdpSocket::bind("127.0.0.1:0")?;
    sock.set_read_timeout(Some(Duration::from_millis(100)))?;
    let port = sock.local_addr()?.port();
    let reg = registry_dir().join(format!("{}.port", std::process::id()));
    std::fs::write(&reg, port.to_string())?;
    let exit = Arc::new(AtomicBool::new(false));
    let ex = exit.clone();
    let th = std::thread::Builder::new().name("SincroStart".into()).spawn(move || {
        let mut buf = [0u8; 64];
        while !ex.load(Ordering::SeqCst) {
            match sock.recv(&mut buf) {
                Ok(n) if &buf[..n] == MAGIC => {
                    match decide(core.can_send_file(), core.can_resume_hold(), core.can_feed_hold()) {
                        Some(SincroAction::RunProgram) => match &on_run {
                            Some(h) => h(&core),
                            None => {
                                let _ = core.run_program();
                            }
                        },
                        Some(SincroAction::CycleStartResume) => core.cycle_start_resume(false),
                        Some(SincroAction::FeedHold) => core.feed_hold(false),
                        None => {}
                    }
                    // do not test the flag for 1 sec, then drop what arrived meanwhile
                    let end = std::time::Instant::now() + Duration::from_millis(1000);
                    while std::time::Instant::now() < end && !ex.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    let _ = sock.set_nonblocking(true);
                    while sock.recv(&mut buf).is_ok() {}
                    let _ = sock.set_nonblocking(false);
                    let _ = sock.set_read_timeout(Some(Duration::from_millis(100)));
                }
                _ => {}
            }
        }
    })?;
    *g = Some(Listener { exit, thread: Some(th), reg });
    Ok(())
}

/// `Running()`.
pub fn running() -> bool {
    LISTENER.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

/// `StopListen()`.
pub fn stop_listen() {
    let l = LISTENER.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(mut l) = l {
        l.exit.store(true, Ordering::SeqCst);
        if let Some(t) = l.thread.take() {
            let _ = t.join();
        }
        let _ = std::fs::remove_file(&l.reg);
    }
}

/// Is the process of a registry entry alive? (Linux: `/proc/<pid>`; elsewhere assume yes.)
fn pid_alive(pid: u32) -> bool {
    if cfg!(target_os = "linux") { std::path::Path::new(&format!("/proc/{pid}")).exists() } else { true }
}

/// Ports of all registered live instances; stale registry files are deleted.
pub fn registered_ports() -> Vec<u16> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(registry_dir()) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        let pid = p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<u32>().ok());
        let port = std::fs::read_to_string(&p).ok().and_then(|t| t.trim().parse::<u16>().ok());
        match (pid, port) {
            (Some(pid), Some(port)) if pid_alive(pid) => out.push(port),
            _ => {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
    out
}

/// `Signal()`: tell every running instance to start/resume/hold (runs on a worker thread like `BeginInvoke`).
pub fn signal() {
    let _ = std::thread::Builder::new().name("SincroSignal".into()).spawn(signal_now);
}

/// Synchronous body of [`signal`]. Returns the number of datagrams sent.
pub fn signal_now() -> usize {
    let Ok(sock) = UdpSocket::bind("127.0.0.1:0") else { return 0 };
    registered_ports().into_iter().filter(|p| sock.send_to(MAGIC, ("127.0.0.1", *p)).is_ok()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::com::FakeCom;
    use lg_gcode::Firmware;

    #[test]
    fn decision_order() {
        assert_eq!(decide(true, true, true), Some(SincroAction::RunProgram));
        assert_eq!(decide(false, true, true), Some(SincroAction::CycleStartResume));
        assert_eq!(decide(false, false, true), Some(SincroAction::FeedHold));
        assert_eq!(decide(false, false, false), None);
    }

    #[test]
    fn signal_reaches_listener() {
        // no real config dir
        crate::test_support::init_env();
        let (tx, _rx) = crossbeam_channel::unbounded();
        let core = GrblCore::with_com(Firmware::Grbl, tx, FakeCom::new(), lg_base::tools::hires_timer::total_milliseconds);
        start_listen(core, None).unwrap();
        assert!(running());
        assert!(registered_ports().len() == 1 || !registered_ports().is_empty());
        assert!(signal_now() >= 1);
        stop_listen();
        assert!(!running());
    }
}
