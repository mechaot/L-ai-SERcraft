// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Fake link for tests: records everything the core writes and lets the test inject received lines.
//!
//! Not part of the C# code. `FakeCom::with_responder` can answer written lines synchronously, which is
//! enough to play a deterministic Grbl without threads.

use super::line_queue::LineQueue;
use super::{ComConfig, ComWrapper, WrapperType};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One recorded write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Written {
    /// `write_byte` (real-time command).
    Byte(u8),
    /// `write_bytes`.
    Bytes(Vec<u8>),
    /// `write_str`.
    Text(String),
}

/// Reply generator: gets each written text line (with its `\n`), returns lines to inject.
pub type Responder = Box<dyn Fn(&str) -> Vec<String> + Send + Sync>;

/// Recording link.
#[derive(Default)]
pub struct FakeCom {
    opened: AtomicBool,
    queue: LineQueue,
    log: Mutex<Vec<Written>>,
    responder: Mutex<Option<Responder>>,
    fail_writes: AtomicBool,
}

impl FakeCom {
    /// New closed fake (shared handle, tests keep one clone).
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// New fake with a responder.
    ///
    /// `r`: called for every written text line.
    pub fn with_responder(r: Responder) -> Arc<Self> {
        let f = Self::default();
        *f.responder.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
        Arc::new(f)
    }

    /// Replace the responder.
    pub fn set_responder(&self, r: Option<Responder>) {
        *self.responder.lock().unwrap_or_else(|e| e.into_inner()) = r;
    }

    /// Inject a received line (no terminator).
    ///
    /// `line`: text as the board would send it.
    pub fn inject(&self, line: &str) {
        self.queue.push(line.to_string());
    }

    /// Make a link failure visible to the reader (`Err` from `read_line_blocking`).
    pub fn inject_error(&self) {
        self.opened.store(false, Ordering::SeqCst);
        self.queue.set_error(io::ErrorKind::NotConnected);
    }

    /// Make writes fail (`true`) or work again.
    pub fn set_fail_writes(&self, b: bool) {
        self.fail_writes.store(b, Ordering::SeqCst);
    }

    /// All writes so far.
    pub fn written(&self) -> Vec<Written> {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Written text lines (without the `\n`), in order; real-time bytes are skipped.
    pub fn written_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for w in self.written() {
            match w {
                Written::Text(t) => out.extend(t.split('\n').filter(|s| !s.is_empty()).map(str::to_string)),
                Written::Bytes(b) => {
                    let t = String::from_utf8_lossy(&b).into_owned();
                    out.extend(t.split('\n').filter(|s| !s.is_empty()).map(str::to_string));
                }
                Written::Byte(_) => {}
            }
        }
        out
    }

    /// Written real-time bytes, in order.
    pub fn written_bytes(&self) -> Vec<u8> {
        self.written().into_iter().filter_map(|w| if let Written::Byte(b) = w { Some(b) } else { None }).collect()
    }

    /// Forget the recorded writes.
    pub fn clear_written(&self) {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    fn record(&self, w: Written) -> io::Result<()> {
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "fake write failure"));
        }
        self.log.lock().unwrap_or_else(|e| e.into_inner()).push(w);
        Ok(())
    }

    fn respond(&self, text: &str) {
        let replies = self.responder.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|r| r(text));
        for l in replies.unwrap_or_default() {
            self.queue.push(l);
        }
    }
}

impl ComWrapper for FakeCom {
    /// Pretends to be the emulator wrapper.
    fn wrapper_type(&self) -> WrapperType {
        WrapperType::Emulator
    }

    /// Ignored.
    fn configure(&self, _conf: &ComConfig) {}

    /// Mark open.
    fn open(&self) -> io::Result<()> {
        self.opened.store(true, Ordering::SeqCst);
        self.queue.clear();
        Ok(())
    }

    /// Mark closed.
    fn close(&self, _auto: bool) {
        self.opened.store(false, Ordering::SeqCst);
    }

    /// Open flag.
    fn is_open(&self) -> bool {
        self.opened.load(Ordering::SeqCst)
    }

    /// Record a byte.
    fn write_byte(&self, b: u8) -> io::Result<()> {
        self.record(Written::Byte(b))
    }

    /// Record bytes.
    fn write_bytes(&self, data: &[u8]) -> io::Result<()> {
        self.record(Written::Bytes(data.to_vec()))?;
        self.respond(&String::from_utf8_lossy(data));
        Ok(())
    }

    /// Record text and run the responder.
    fn write_str(&self, text: &str) -> io::Result<()> {
        self.record(Written::Text(text.to_string()))?;
        self.respond(text);
        Ok(())
    }

    /// Wait up to 20 ms.
    fn read_line_blocking(&self) -> io::Result<Option<String>> {
        if !self.is_open() {
            // drain pending error once
            let r = self.queue.pop_timeout(Duration::from_millis(1));
            return match r {
                Err(e) => Err(e),
                _ => Ok(None),
            };
        }
        self.queue.pop_timeout(Duration::from_millis(20))
    }

    /// A line waits.
    fn has_data(&self) -> bool {
        self.is_open() && self.queue.has_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_responds() {
        let f = FakeCom::with_responder(Box::new(|l| if l.starts_with("G") { vec!["ok".into()] } else { vec![] }));
        f.open().unwrap();
        f.write_str("G0 X1\n").unwrap();
        f.write_byte(0x85).unwrap();
        f.write_str("$$\n").unwrap();
        assert_eq!(f.written_lines(), vec!["G0 X1", "$$"]);
        assert_eq!(f.written_bytes(), vec![0x85]);
        assert_eq!(f.read_line_blocking().unwrap().as_deref(), Some("ok"));
        assert_eq!(f.read_line_blocking().unwrap(), None);
        f.inject_error();
        assert!(f.read_line_blocking().is_err());
    }
}
