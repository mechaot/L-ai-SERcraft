//! Shared line buffer between a reader thread and `read_line_blocking` (replaces the blocking C# reads).

use std::collections::VecDeque;
use std::io;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

#[derive(Default)]
struct Inner {
    lines: VecDeque<String>,
    error: Option<io::ErrorKind>,
}

/// Thread-safe queue of received lines plus a sticky link error.
#[derive(Default)]
pub struct LineQueue {
    inner: Mutex<Inner>,
    cv: Condvar,
}

impl LineQueue {
    /// New empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Append one line and wake the reader.
    ///
    /// `line`: text without the line terminator.
    pub fn push(&self, line: String) {
        self.lock().lines.push_back(line);
        self.cv.notify_all();
    }

    /// Record a link failure; delivered once all queued lines were read.
    ///
    /// `kind`: error kind returned by `pop_timeout`.
    pub fn set_error(&self, kind: io::ErrorKind) {
        self.lock().error = Some(kind);
        self.cv.notify_all();
    }

    /// Drop all lines and the error (called on open/close).
    pub fn clear(&self) {
        let mut g = self.lock();
        g.lines.clear();
        g.error = None;
    }

    /// True when at least one line waits.
    pub fn has_data(&self) -> bool {
        !self.lock().lines.is_empty()
    }

    /// Pop the oldest line, waiting up to `timeout`.
    ///
    /// Returns `Ok(Some(line))`, `Ok(None)` on timeout, `Err` when a link error is pending and no line is left.
    pub fn pop_timeout(&self, timeout: Duration) -> io::Result<Option<String>> {
        let mut g = self.lock();
        if g.lines.is_empty() && g.error.is_none() {
            g = self.cv.wait_timeout(g, timeout).unwrap_or_else(|e| e.into_inner()).0;
        }
        if let Some(l) = g.lines.pop_front() {
            return Ok(Some(l));
        }
        match g.error {
            Some(k) => Err(io::Error::new(k, "link failed")),
            None => Ok(None),
        }
    }
}

/// Turns a byte stream into lines. Bytes are decoded as ASCII like the C# readers (`SerialPort.ReadLine`,
/// `StreamReader(Encoding.ASCII)`): bytes above 0x7F become `?`.
///
/// `new()` splits on `\n` only (`SerialPort.NewLine = "\n"`, the `\r` stays, the core trims it like C#);
/// `any_eol()` splits like `StreamReader.ReadLine` on `\r`, `\n` and `\r\n`.
#[derive(Default)]
pub struct LineSplitter {
    buf: Vec<u8>,
    any_eol: bool,
    skip_lf: bool,
}

impl LineSplitter {
    /// New splitter, `\n` only.
    pub fn new() -> Self {
        Self::default()
    }

    /// New splitter with `StreamReader.ReadLine` line ends.
    pub fn any_eol() -> Self {
        Self { any_eol: true, ..Self::default() }
    }

    fn take(&mut self) -> String {
        let s: String = self.buf.iter().map(|&b| if b < 0x80 { b as char } else { '?' }).collect();
        self.buf.clear();
        s
    }

    /// Feed bytes; complete lines go to `out`.
    ///
    /// `data`: new bytes. `out`: callback per finished line (terminator removed).
    pub fn feed(&mut self, data: &[u8], mut out: impl FnMut(String)) {
        for &b in data {
            if self.skip_lf {
                self.skip_lf = false;
                if b == b'\n' {
                    continue;
                }
            }
            if b == b'\n' {
                out(self.take());
            } else if b == b'\r' && self.any_eol {
                self.skip_lf = true;
                out(self.take());
            } else {
                self.buf.push(b);
            }
        }
    }

    /// Bytes of the unfinished line.
    pub fn pending(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_order_timeout_error() {
        let q = LineQueue::new();
        assert_eq!(q.pop_timeout(Duration::from_millis(5)).unwrap(), None);
        q.push("a".into());
        q.push("b".into());
        assert!(q.has_data());
        assert_eq!(q.pop_timeout(Duration::from_millis(5)).unwrap().as_deref(), Some("a"));
        q.set_error(io::ErrorKind::NotConnected);
        // queued lines come first
        assert_eq!(q.pop_timeout(Duration::from_millis(5)).unwrap().as_deref(), Some("b"));
        assert!(q.pop_timeout(Duration::from_millis(5)).is_err());
        q.clear();
        assert_eq!(q.pop_timeout(Duration::from_millis(1)).unwrap(), None);
    }

    #[test]
    fn splitter() {
        let mut s = LineSplitter::new();
        let mut got = Vec::new();
        s.feed(b"ok\r\nerr", |l| got.push(l));
        s.feed(b"or:1\n\n", |l| got.push(l));
        assert_eq!(got, vec!["ok\r", "error:1", ""]);
        assert_eq!(s.pending(), 0);
        // ASCII decode
        let mut s = LineSplitter::new();
        let mut got = Vec::new();
        s.feed(&[b'a', 0xC3, 0xA9, b'\n'], |l| got.push(l));
        assert_eq!(got, vec!["a??"]);
        // StreamReader style
        let mut s = LineSplitter::any_eol();
        let mut got = Vec::new();
        s.feed(b"ok\r\nb\rc", |l| got.push(l));
        s.feed(b"\n\nd\r", |l| got.push(l));
        s.feed(b"\ne\n", |l| got.push(l));
        assert_eq!(got, vec!["ok", "b", "c", "", "d", "e"]);
    }
}
