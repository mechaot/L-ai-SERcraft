// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Logger/Logger.cs` (session log) and `Logger/ComLogger.cs` (TX/RX log).

mod async_file;
pub mod com;

pub use async_file::AsyncLogFile;

use std::path::PathBuf;
use std::sync::Mutex;

static FILE: Mutex<Option<AsyncLogFile>> = Mutex::new(None);

/// Path of the session log: `data_path()/sessionlog.txt`.
///
/// Returns the path.
pub fn log_file_path() -> PathBuf {
    crate::paths::data_path().join("sessionlog.txt")
}

/// Run `f` on the session log file; opens it (rotating to 1000 lines) on first use.
///
/// `f`: action on the file.
fn with_file<R>(f: impl FnOnce(&AsyncLogFile) -> R) -> R {
    let mut g = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let file = g.get_or_insert_with(|| AsyncLogFile::new(log_file_path(), 1000));
    f(file)
}

/// OS description from `std::env::consts` and `/etc/os-release`.
///
/// Returns e.g. `linux|x86_64|Ubuntu 24.04 LTS`.
pub fn os_info() -> String {
    let mut name = String::new();
    if let Ok(t) = std::fs::read_to_string("/etc/os-release") {
        for l in t.lines() {
            if let Some(v) = l.strip_prefix("PRETTY_NAME=") {
                name = v.trim_matches('"').to_string();
            }
        }
    }
    format!("{}|{}|{}", std::env::consts::OS, std::env::consts::ARCH, name)
}

/// Format and enqueue a multi-line message (C# `LogMultiLine`).
///
/// `context`: padded to 12 chars. `text`: split on `\r\n` and `\n`, one log line each.
fn log_multi_line(context: &str, text: &str) {
    let dt = chrono::Local::now().format("%d/%m/%Y %H:%M:%S%.3f").to_string();
    let mut sb = String::new();
    for line in text.replace("\r\n", "\n").split('\n') {
        sb.push_str(&format!("{dt}\t{context:<12}\t{line}\r\n"));
    }
    with_file(|f| f.log(&sb));
}

/// `Logger.LogMessage`: message already formatted by the caller.
///
/// `context`: log context column. `msg`: text (may be multi-line).
pub fn log_message(context: &str, msg: &str) {
    log_multi_line(context, msg);
}

/// `Logger.LogException`.
///
/// `context`: log context column. `err`: error, logged via `Display`.
pub fn log_exception(context: &str, err: &dyn std::fmt::Display) {
    log_multi_line(context, &err.to_string());
}

/// `Logger.Start`: write the START banner (+ OS line).
pub fn start() {
    let bits = if cfg!(target_pointer_width = "64") { "64bit" } else { "32bit" };
    // own app version, not CURRENT_VERSION (that one is the LaserGRBL compat version)
    log_multi_line("Program", &format!("------- {} v{} [{bits}] START -------", crate::APP_NAME, env!("CARGO_PKG_VERSION")));
    log_multi_line("Program", &format!("OS {}", os_info()));
}

/// `Logger.Stop`: write STOP banner, flush and close. Later logs reopen the file.
pub fn stop() {
    log_multi_line("Program", "---------------- PROGRAM STOP -----------------");
    with_file(|f| f.log("\r\n"));
    shutdown();
}

/// Drain the queue and stop the writer thread, without a banner. No-op when the log never opened.
///
/// Process-exit helper (`test_support` cleanup). Later logs reopen the file.
pub fn shutdown() {
    let taken = FILE.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(f) = taken {
        f.stop();
    }
}

/// True when the session log file exists.
///
/// Returns existence (C# `Logger.ExistLog`).
pub fn exist_log() -> bool {
    with_file(|f| f.exist_log())
}

/// Open the session log with the system viewer (C# `ShowLog`).
pub fn show_log() {
    crate::tools::utils::open_path(&log_file_path());
}
