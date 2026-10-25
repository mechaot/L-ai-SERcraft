// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Logger/ComLogger.cs`: TX/RX recording to a file.

use super::AsyncLogFile;
use crate::tools::cronometro::time_from_application_startup;
use std::path::PathBuf;
use std::sync::Mutex;

struct Session {
    file: AsyncLogFile,
    logcnt: u32,
    logid: String,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// Local timestamp as C# `{0:dd/MM/yyyy HH:mm:ss.fff}`.
fn now_text() -> String {
    chrono::Local::now().format("%d/%m/%Y %H:%M:%S%.3f").to_string()
}

/// Random 8-hex session id (C# first GUID segment).
fn short_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u128);
    format!("{:08x}", h.finish() as u32)
}

/// Default log path used by [`set_enabled`]: `data_path()/comlog.txt`.
pub fn default_file() -> PathBuf {
    crate::paths::data_path().join("comlog.txt")
}

/// Write one entry on an open session.
///
/// `s`: session. `operation`/`line`: columns.
fn log_in(s: &mut Session, operation: &str, line: &str) {
    let line = line.replace('\r', "\\r").replace('\n', "\\n");
    let ms = time_from_application_startup().total_milliseconds();
    let text = format!("{:05}\t{}\t{}\t{}\r\n", s.logcnt, lg_dotnet::fmt_double_custom(ms, "00000000"), operation, line);
    s.file.log(&text);
    s.logcnt += 1;
}

/// Start recording (C# `StartLog`); stops a running session first.
///
/// `filename`: target (truncated). Returns the start message.
pub fn start_log(filename: PathBuf) -> String {
    stop_log();
    let logid = short_id();
    let message = format!("Recording session started @ {} - Session ID [{}]", now_text(), logid);
    let mut s = Session { file: AsyncLogFile::new(filename, 0), logcnt: 0, logid };
    log_in(&mut s, "log", &message);
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
    message
}

/// Stop recording (C# `StopLog`).
///
/// Returns the stop message, None when not recording.
pub fn stop_log() -> Option<String> {
    let taken = SESSION.lock().unwrap_or_else(|e| e.into_inner()).take();
    taken.map(|mut s| {
        let message = format!("Recording session stopped @ {} - Session ID [{}]", now_text(), s.logid);
        log_in(&mut s, "log", &message);
        s.file.stop();
        message
    })
}

/// True while recording (C# `Enabled`).
pub fn enabled() -> bool {
    SESSION.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

/// Switch recording on (to [`default_file`]) or off.
///
/// `b`: desired state.
pub fn set_enabled(b: bool) {
    if b && !enabled() {
        start_log(default_file());
    } else if !b {
        stop_log();
    }
}

/// Log a string line (C# `Log(op, line)`); CR/LF are escaped.
///
/// `dir`: operation column (tx, rx, com, log). `s`: text.
pub fn log_str(dir: &str, s: &str) {
    if let Some(sess) = SESSION.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        log_in(sess, dir, s);
    }
}

/// Log bytes as `[AA, BB]` (C# `Log(op, byte[])` / `Log(op, byte)` = `[AA]`).
///
/// `dir`: operation column. `data`: bytes.
pub fn log(dir: &str, data: &[u8]) {
    if enabled() {
        let parts: Vec<String> = data.iter().map(|b| format!("{b:02X}")).collect();
        log_str(dir, &format!("[{}]", parts.join(", ")));
    }
}
