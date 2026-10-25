// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `ComWrapper/LaserWebESP8266.cs` on top of `tungstenite` (client).
//!
//! One IO thread owns the websocket: it drains the outgoing channel, then reads with a short socket
//! timeout. Text frames are split into lines exactly like `cln_OnMessage` (split on `\n`, drop empty
//! entries, keep an unfinished last piece as remainder).

use super::line_queue::LineQueue;
use super::{reset_diagnostic, ComConfig, ComWrapper, WrapperType};
use crossbeam_channel::{Receiver, Sender};
use lg_base::logger;
use lg_base::logger::com as comlog;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;
use tungstenite::{Message, WebSocket};

const IO_TIMEOUT: Duration = Duration::from_millis(2);
const POLL_WAIT: Duration = Duration::from_millis(100);

/// Outgoing frame.
enum Out {
    Text(String),
    Binary(Vec<u8>),
}

struct Link {
    open: AtomicBool,
    stop: AtomicBool,
    out: Sender<Out>,
    queue: Arc<LineQueue>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

/// Websocket transport for ESP8266 boards running LaserWeb firmware.
pub struct LaserWebEsp8266Com {
    address: Mutex<String>,
    link: RwLock<Option<Arc<Link>>>,
}

impl Default for LaserWebEsp8266Com {
    /// Same as `new()`.
    fn default() -> Self {
        Self::new()
    }
}

/// Port of `cln_OnMessage` line assembly.
///
/// `data`: message text. `remainder`: unfinished line kept between messages. `push`: sink for lines.
pub(crate) fn split_message(data: &str, remainder: &mut Option<String>, mut push: impl FnMut(String)) {
    let last_complete = data.ends_with('\n') || data.ends_with('\r');
    let items: Vec<&str> = data.split('\n').filter(|s| !s.is_empty()).collect();
    for (i, item) in items.iter().enumerate() {
        let mut item = item.to_string();
        if i == items.len() - 1 && !last_complete {
            *remainder = Some(item);
            break;
        }
        if let Some(r) = remainder.take() {
            item = r + &item;
        }
        push(item);
    }
}

impl LaserWebEsp8266Com {
    /// New closed wrapper.
    pub fn new() -> Self {
        Self { address: Mutex::new(String::new()), link: RwLock::new(None) }
    }

    fn link(&self) -> Option<Arc<Link>> {
        self.link.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn send(&self, o: Out) -> io::Result<()> {
        // C#: silently ignored when not open
        if let Some(l) = self.link()
            && l.open.load(Ordering::SeqCst)
        {
            l.out.send(o).map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "websocket closed"))?;
        }
        Ok(())
    }
}

/// IO thread body.
///
/// `link`: shared state. `ws`: connected socket with short read timeout. `rx`: outgoing frames.
fn io_loop<S: io::Read + io::Write>(link: Arc<Link>, mut ws: WebSocket<S>, rx: Receiver<Out>) {
    let mut remainder: Option<String> = None;
    let fail = |link: &Link| {
        link.open.store(false, Ordering::SeqCst);
        link.queue.set_error(io::ErrorKind::NotConnected);
    };
    while !link.stop.load(Ordering::SeqCst) {
        // send everything queued
        while let Ok(o) = rx.try_recv() {
            let m = match o {
                Out::Text(t) => Message::text(t),
                Out::Binary(b) => Message::binary(b),
            };
            if ws.send(m).is_err() {
                fail(&link);
                return;
            }
        }
        match ws.read() {
            Ok(Message::Text(t)) => feed(&link, &mut remainder, t.as_str()),
            // C#: Encoding.Default.GetString(RawData)
            Ok(Message::Binary(b)) => feed(&link, &mut remainder, &String::from_utf8_lossy(&b)),
            Ok(Message::Close(_)) => {
                fail(&link);
                return;
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted) => {}
            Err(_) => {
                if !link.stop.load(Ordering::SeqCst) {
                    fail(&link);
                }
                return;
            }
        }
    }
    let _ = ws.close(None);
    let _ = ws.flush();
}

/// Deliver a received message to the line queue.
fn feed(link: &Link, remainder: &mut Option<String>, text: &str) {
    split_message(text, remainder, |l| {
        comlog::log_str("rx", &l);
        link.queue.push(l)
    });
}

impl ComWrapper for LaserWebEsp8266Com {
    /// See trait.
    fn wrapper_type(&self) -> WrapperType {
        WrapperType::LaserWebESP8266
    }

    /// Takes the `ws://host:port/` address.
    fn configure(&self, conf: &ComConfig) {
        if let ComConfig::Address(a) = conf {
            *self.address.lock().unwrap_or_else(|e| e.into_inner()) = a.clone();
        }
    }

    /// Connect the websocket.
    fn open(&self) -> io::Result<()> {
        if self.link().is_some() {
            self.close(true);
        }
        let addr = self.address.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if addr.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Missing Address"));
        }
        logger::log_message("OpenCom", &format!("Open {addr}"));
        let soft: bool = lg_base::settings::get("Reset Grbl On Connect", false);
        comlog::log_str("com", &format!("Open {} {}", addr, reset_diagnostic(false, false, soft)));

        let (ws, _resp) = tungstenite::connect(addr.as_str()).map_err(|e| io::Error::other(e.to_string()))?;
        // short socket timeout so the IO thread can interleave writes and reads
        match ws.get_ref() {
            tungstenite::stream::MaybeTlsStream::Plain(s) => {
                let _ = s.set_nodelay(true);
                s.set_read_timeout(Some(IO_TIMEOUT))?;
            }
            _ => return Err(io::Error::other("unsupported websocket transport")),
        }
        let (otx, orx) = crossbeam_channel::unbounded();
        let link = Arc::new(Link {
            open: AtomicBool::new(true),
            stop: AtomicBool::new(false),
            out: otx,
            queue: Arc::new(LineQueue::new()),
            thread: Mutex::new(None),
        });
        let l2 = link.clone();
        let th = std::thread::Builder::new().name("WebSocketIO".into()).spawn(move || io_loop(l2, ws, orx))?;
        *link.thread.lock().unwrap_or_else(|e| e.into_inner()) = Some(th);
        *self.link.write().unwrap_or_else(|e| e.into_inner()) = Some(link);
        Ok(())
    }

    /// Close the websocket.
    fn close(&self, auto: bool) {
        let link = self.link.write().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(l) = link {
            let addr = self.address.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let who = if auto { "CORE" } else { "USER" };
            comlog::log_str("com", &format!("Close {addr} [{who}]"));
            logger::log_message("CloseCom", &format!("Close {addr} [{who}]"));
            l.open.store(false, Ordering::SeqCst);
            l.stop.store(true, Ordering::SeqCst);
            let th = l.thread.lock().unwrap_or_else(|e| e.into_inner()).take();
            if let Some(th) = th {
                let _ = th.join();
            }
        }
    }

    /// Connected.
    fn is_open(&self) -> bool {
        self.link().is_some_and(|l| l.open.load(Ordering::SeqCst))
    }

    /// One byte as a one-char text frame (C# `new string((char)b, 1)`).
    fn write_byte(&self, b: u8) -> io::Result<()> {
        comlog::log("tx", &[b]);
        self.send(Out::Text(char::from(b).to_string()))
    }

    /// Binary frame.
    fn write_bytes(&self, data: &[u8]) -> io::Result<()> {
        comlog::log("tx", data);
        self.send(Out::Binary(data.to_vec()))
    }

    /// Text frame.
    fn write_str(&self, text: &str) -> io::Result<()> {
        comlog::log_str("tx", text);
        self.send(Out::Text(text.to_string()))
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

    #[test]
    fn message_splitting_matches_csharp() {
        let mut rem = None;
        let mut got = Vec::new();
        split_message("ok\nok\n", &mut rem, |l| got.push(l));
        assert_eq!(got, vec!["ok", "ok"]);
        assert!(rem.is_none());
        split_message("<Idle|MPos:0", &mut rem, |l| got.push(l));
        assert_eq!(rem.as_deref(), Some("<Idle|MPos:0"));
        split_message(",0,0>\r\nok\r\n", &mut rem, |l| got.push(l));
        // first piece is not a start of line in C# either: remainder joins the first item
        assert_eq!(got[2], "<Idle|MPos:0,0,0>\r");
        assert_eq!(got[3], "ok\r");
        // only a newline: nothing
        split_message("\n", &mut rem, |l| got.push(l));
        assert_eq!(got.len(), 4);
    }

    #[test]
    fn open_without_address_fails() {
        let c = LaserWebEsp8266Com::new();
        assert!(c.open().is_err());
        assert!(!c.is_open());
        assert!(c.write_str("x").is_ok());
    }
}
