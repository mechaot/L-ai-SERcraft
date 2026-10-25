// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `GrblEmulator/WebsocketEmulator.cs`: a websocket server that serves one [`Grblv11Emulator`] per
//! connection (tungstenite server instead of websocket-sharp).
//!
//! C# listens on `ws://127.0.0.1:81/`. Port 81 needs privileges on Linux, so the port is a parameter here
//! ([`DEFAULT_PORT`] = 81 keeps the C# value).

use super::grbl_v11::Grblv11Emulator;
use crossbeam_channel::Receiver;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tungstenite::{Message, WebSocket};

/// Port used by the C# emulator.
pub const DEFAULT_PORT: u16 = 81;

struct Server {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    port: u16,
}

static SERVER: Mutex<Option<Server>> = Mutex::new(None);

/// Static facade (C# `static class` members `Start` / `Stop`).
pub struct WebSocketEmulator;

impl WebSocketEmulator {
    /// `Start`: listen on `127.0.0.1:port` when not running; shows the emulator UI line.
    ///
    /// `port`: TCP port (0 = pick a free one). Returns the bound port.
    pub fn start(port: u16) -> io::Result<u16> {
        let mut g = SERVER.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(s) = g.as_ref() {
            return Ok(s.port);
        }
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        listener.set_nonblocking(true)?;
        let bound = listener.local_addr()?.port();
        super::show_ui(&format!("ESP266 Grbl emulator (listening as ws://127.0.0.1:{bound}/)"));
        let stop = Arc::new(AtomicBool::new(false));
        let st = stop.clone();
        let thread = std::thread::Builder::new().name("WsEmuAccept".into()).spawn(move || accept_loop(listener, st))?;
        *g = Some(Server { stop, thread: Some(thread), port: bound });
        Ok(bound)
    }

    /// `Stop`: close the server and hide the UI.
    pub fn stop() {
        let s = SERVER.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(mut s) = s {
            s.stop.store(true, Ordering::SeqCst);
            if let Some(t) = s.thread.take() {
                let _ = t.join();
            }
            super::hide_ui();
        }
    }

    /// True while the server runs.
    pub fn running() -> bool {
        SERVER.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }
}

/// Accept connections until `stop`.
fn accept_loop(listener: TcpListener, stop: Arc<AtomicBool>) {
    let mut conns: Vec<JoinHandle<()>> = Vec::new();
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((s, _)) => {
                let st = stop.clone();
                if let Ok(t) = std::thread::Builder::new().name("WsEmuConn".into()).spawn(move || {
                    let _ = serve(s, st);
                }) {
                    conns.push(t);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    for c in conns {
        let _ = c.join();
    }
}

/// One session (`GrblWebSocketEmulator` behavior).
///
/// `stream`: accepted socket. `stop`: server stop flag.
fn serve(stream: TcpStream, stop: Arc<AtomicBool>) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    let _ = stream.set_nodelay(true);
    let mut ws = tungstenite::accept(stream).map_err(|e| io::Error::other(e.to_string()))?;
    ws.get_ref().set_read_timeout(Some(Duration::from_millis(2)))?;
    let (otx, orx) = crossbeam_channel::unbounded::<String>();
    let emu = Grblv11Emulator::new(Box::new(move |m| {
        let _ = otx.send(m.to_string());
    }));
    // OnOpen
    emu.open_com();
    let r = session_loop(&mut ws, &emu, &orx, &stop);
    // OnClose
    emu.close_com();
    r
}

/// Pump messages both ways until close.
fn session_loop(ws: &mut WebSocket<TcpStream>, emu: &Grblv11Emulator, orx: &Receiver<String>, stop: &AtomicBool) -> io::Result<()> {
    while !stop.load(Ordering::SeqCst) {
        while let Ok(m) = orx.try_recv() {
            ws.send(Message::text(m)).map_err(|e| io::Error::other(e.to_string()))?;
        }
        match ws.read() {
            Ok(Message::Text(t)) => emu.manage_message(t.as_bytes()),
            Ok(Message::Binary(b)) => emu.manage_message(&b),
            Ok(Message::Close(_)) => return Ok(()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted) => {}
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => return Ok(()),
            Err(e) => return Err(io::Error::other(e.to_string())),
        }
    }
    let _ = ws.close(None);
    let _ = ws.flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::com::{ComConfig, ComWrapper, LaserWebEsp8266Com};

    #[test]
    fn websocket_client_talks_to_emulator() {
        let port = WebSocketEmulator::start(0).unwrap();
        assert!(WebSocketEmulator::running());
        let c = LaserWebEsp8266Com::new();
        c.configure(&ComConfig::Address(format!("ws://127.0.0.1:{port}/")));
        c.open().unwrap();
        let mut got = Vec::new();
        let mut sent_status = false;
        for _ in 0..100 {
            if let Some(l) = c.read_line_blocking().unwrap() {
                got.push(l);
            }
            if !sent_status && !got.is_empty() {
                c.write_byte(63).unwrap();
                sent_status = true;
            }
            if got.iter().any(|l| l.starts_with("<Idle")) {
                break;
            }
        }
        assert_eq!(got[0], "Grbl 1.1# ['$' for help]");
        assert!(got.iter().any(|l| l.starts_with("<Idle|MPos:0.000,0.000,0.000>")), "{got:?}");
        c.close(false);
        WebSocketEmulator::stop();
        assert!(!WebSocketEmulator::running());
    }
}
