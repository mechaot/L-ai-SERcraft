// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `ComWrapper/Telnet.cs` (plain TCP line stream, no telnet option negotiation, like C#).
//!
//! Differences to C#: a reader thread feeds a [`LineQueue`]; a closed/failed socket makes
//! `read_line_blocking` return `Err(NotConnected)` (C# keeps polling a dead stream); `TCP_NODELAY` is set.

use super::ip_helper::parse_endpoint;
use super::line_queue::{LineQueue, LineSplitter};
use super::{reset_diagnostic, ComConfig, ComWrapper, WrapperType};
use lg_base::logger;
use lg_base::logger::com as comlog;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

const READER_TIMEOUT: Duration = Duration::from_millis(50);
const POLL_WAIT: Duration = Duration::from_millis(100);

struct Link {
    open: AtomicBool,
    stop: AtomicBool,
    stream: Mutex<TcpStream>,
    queue: LineQueue,
    thread: Mutex<Option<JoinHandle<()>>>,
}

/// Telnet transport.
pub struct TelnetCom {
    address: Mutex<String>,
    link: RwLock<Option<Arc<Link>>>,
}

impl Default for TelnetCom {
    /// Same as `new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl TelnetCom {
    /// New closed wrapper.
    pub fn new() -> Self {
        Self { address: Mutex::new(String::new()), link: RwLock::new(None) }
    }

    fn link(&self) -> Option<Arc<Link>> {
        self.link.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Write helper: socket must exist (C# NRE otherwise).
    fn write_raw(&self, data: &[u8]) -> io::Result<()> {
        let l = self.link().ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "not connected"))?;
        let mut s = l.stream.lock().unwrap_or_else(|e| e.into_inner());
        s.write_all(data)?;
        s.flush()
    }
}

/// Reader thread body: socket bytes -> lines.
///
/// `link`: shared state. `stream`: reader handle with read timeout.
fn reader_loop(link: Arc<Link>, mut stream: TcpStream) {
    let mut splitter = LineSplitter::any_eol();
    let mut tmp = [0u8; 1024];
    while !link.stop.load(Ordering::SeqCst) {
        match stream.read(&mut tmp) {
            Ok(0) => {
                // remote closed
                link.open.store(false, Ordering::SeqCst);
                link.queue.set_error(io::ErrorKind::NotConnected);
                break;
            }
            Ok(n) => splitter.feed(&tmp[..n], |l| {
                comlog::log_str("rx", &l);
                link.queue.push(l)
            }),
            Err(e) if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {}
            Err(_) => {
                link.open.store(false, Ordering::SeqCst);
                link.queue.set_error(io::ErrorKind::NotConnected);
                break;
            }
        }
    }
}

impl ComWrapper for TelnetCom {
    /// See trait.
    fn wrapper_type(&self) -> WrapperType {
        WrapperType::Telnet
    }

    /// Takes the address string.
    fn configure(&self, conf: &ComConfig) {
        if let ComConfig::Address(a) = conf {
            *self.address.lock().unwrap_or_else(|e| e.into_inner()) = a.clone();
        }
    }

    /// Connect to `host:port`.
    fn open(&self) -> io::Result<()> {
        if self.link().is_some() {
            self.close(true);
        }
        let addr = self.address.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if addr.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Missing HostName"));
        }
        logger::log_message("OpenCom", &format!("Open {addr}"));
        let soft: bool = lg_base::settings::get("Reset Grbl On Connect", false);
        comlog::log_str("com", &format!("Open {} {}", addr, reset_diagnostic(false, false, soft)));

        let ep = parse_endpoint(&addr)?;
        let stream = TcpStream::connect(ep)?;
        let _ = stream.set_nodelay(true);
        let reader = stream.try_clone()?;
        reader.set_read_timeout(Some(READER_TIMEOUT))?;
        let link = Arc::new(Link {
            open: AtomicBool::new(true),
            stop: AtomicBool::new(false),
            stream: Mutex::new(stream),
            queue: LineQueue::new(),
            thread: Mutex::new(None),
        });
        let l2 = link.clone();
        let th = std::thread::Builder::new().name("TelnetReader".into()).spawn(move || reader_loop(l2, reader))?;
        *link.thread.lock().unwrap_or_else(|e| e.into_inner()) = Some(th);
        *self.link.write().unwrap_or_else(|e| e.into_inner()) = Some(link);
        Ok(())
    }

    /// Close the socket.
    fn close(&self, auto: bool) {
        let link = self.link.write().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(l) = link {
            let addr = self.address.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let who = if auto { "CORE" } else { "USER" };
            comlog::log_str("com", &format!("Close {addr} [{who}]"));
            logger::log_message("CloseCom", &format!("Close {addr} [{who}]"));
            l.open.store(false, Ordering::SeqCst);
            l.stop.store(true, Ordering::SeqCst);
            if let Ok(s) = l.stream.lock() {
                let _ = s.shutdown(Shutdown::Both);
            }
            let th = l.thread.lock().unwrap_or_else(|e| e.into_inner()).take();
            if let Some(th) = th {
                let _ = th.join();
            }
        }
    }

    /// Socket up.
    fn is_open(&self) -> bool {
        self.link().is_some_and(|l| l.open.load(Ordering::SeqCst))
    }

    /// Write one byte.
    fn write_byte(&self, b: u8) -> io::Result<()> {
        comlog::log("tx", &[b]);
        self.write_raw(&[b])
    }

    /// Write bytes.
    fn write_bytes(&self, data: &[u8]) -> io::Result<()> {
        comlog::log("tx", data);
        self.write_raw(data)
    }

    /// Write text (ASCII).
    fn write_str(&self, text: &str) -> io::Result<()> {
        comlog::log_str("tx", text);
        // C# StreamWriter(Encoding.ASCII): non-ASCII becomes '?'
        self.write_raw(&crate::util::ascii_lossy(text))
    }

    /// Wait up to 100 ms for a line.
    fn read_line_blocking(&self) -> io::Result<Option<String>> {
        match self.link() {
            Some(l) => l.queue.pop_timeout(POLL_WAIT),
            None => {
                std::thread::sleep(Duration::from_millis(1));
                Ok(None)
            }
        }
    }

    /// A line waits.
    fn has_data(&self) -> bool {
        self.link().is_some_and(|l| l.open.load(Ordering::SeqCst) && l.queue.has_data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn roundtrip_and_remote_close() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let srv = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            s.write_all(b"Grbl 1.1f\r\nok\n").unwrap();
            let mut buf = [0u8; 8];
            let n = s.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"$$\n");
            // drop = remote close
        });
        let c = TelnetCom::new();
        c.configure(&ComConfig::Address(format!("127.0.0.1:{port}")));
        c.open().unwrap();
        assert!(c.is_open());
        let mut got = Vec::new();
        for _ in 0..30 {
            if let Some(x) = c.read_line_blocking().unwrap() {
                got.push(x);
            }
            if got.len() == 2 {
                break;
            }
        }
        assert_eq!(got, vec!["Grbl 1.1f", "ok"]);
        c.write_str("$$\n").unwrap();
        srv.join().unwrap();
        // remote closed: error shows up, link not open
        let mut failed = false;
        for _ in 0..30 {
            if c.read_line_blocking().is_err() {
                failed = true;
                break;
            }
        }
        assert!(failed);
        assert!(!c.is_open());
        c.close(true);
    }

    #[test]
    fn missing_host_is_error() {
        let c = TelnetCom::new();
        assert!(c.open().is_err());
        c.configure(&ComConfig::Address("127.0.0.1".into()));
        assert!(c.open().is_err());
    }
}
