// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `ComWrapper/UsbSerial.cs`, `UsbSerial2.cs` and `RJCPSerial.cs`.
//!
//! The three C# classes differ only in the .NET serial class they wrap (MySerial / SerialPort /
//! RJCP SerialPortStream) and in the text of the log lines. With the `serialport` crate one
//! implementation serves all three; the wrapper type only selects the label in the log.
//! NOTE(upstream): `RJCPSerial` logs "(UsbSerial)" in its open lines; kept.
//!
//! Differences to C#:
//! * Linux/pty drivers raise DTR/RTS when the device is opened, so the desired DTR/RTS state is applied
//!   right after `open` (C# sets it before `Open`).
//! * Open errors are always returned (C# swallows IOExceptions on ports whose name does not end in two
//!   digits and leaves a closed wrapper behind).
//! * The issue #31 retry (drop the last digit of a two-digit `COMxx` name) is only done on Windows: on
//!   Linux `/dev/ttyUSB10` -> `/dev/ttyUSB1` would open a different device.
//! * A reader thread splits the stream into lines; a read error (unplug) marks the link closed and the
//!   next `read_line_blocking` returns `Err(NotConnected)` so the core runs `CloseCom(false)`.

use super::line_queue::{LineQueue, LineSplitter};
use super::{reset_diagnostic, ComConfig, ComWrapper, WrapperType};
use lg_base::logger;
use lg_base::logger::com as comlog;
use lg_gcode::Firmware;
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

/// Read timeout of the reader thread (how fast it notices a close request).
const READER_TIMEOUT: Duration = Duration::from_millis(50);
/// `WriteTimeout = 1000` of the C# code.
const WRITE_TIMEOUT: Duration = Duration::from_millis(1000);
/// How long `read_line_blocking` waits before reporting a timeout.
const POLL_WAIT: Duration = Duration::from_millis(100);

/// One open port.
struct Link {
    name: String,
    open: AtomicBool,
    stop: AtomicBool,
    writer: Mutex<Box<dyn SerialPort>>,
    queue: LineQueue,
    thread: Mutex<Option<JoinHandle<()>>>,
}

/// Serial transport (UsbSerial / UsbSerial2 / RJCPSerial).
pub struct SerialCom {
    wtype: WrapperType,
    cfg: Mutex<(String, i32)>,
    link: RwLock<Option<Arc<Link>>>,
}

/// Serial ports present on the system.
///
/// Returns device names (`/dev/ttyUSB0`, `COM3`, ...). Without libudev the Linux listing comes from sysfs.
pub fn list_serial_ports() -> Vec<String> {
    let mut v: Vec<String> = serialport::available_ports().map(|p| p.into_iter().map(|i| i.port_name).collect()).unwrap_or_default();
    v.sort();
    v
}

/// Name for the issue #31 retry: last char removed when the name ends with two digits.
///
/// `name`: configured port name. Returns the alternative name, None when it does not apply.
pub(crate) fn issue31_alternative(name: &str) -> Option<String> {
    let chars: Vec<char> = name.chars().collect();
    let n = chars.len();
    if n >= 2 && lg_dotnet::is_digit(chars[n - 1]) && lg_dotnet::is_digit(chars[n - 2]) {
        Some(chars[..n - 1].iter().collect())
    } else {
        None
    }
}

impl SerialCom {
    /// New closed wrapper.
    ///
    /// `wtype`: `UsbSerial`, `UsbSerial2` or `RJCPSerial` (only changes log text).
    pub fn new(wtype: WrapperType) -> Self {
        Self { wtype, cfg: Mutex::new((String::new(), 115200)), link: RwLock::new(None) }
    }

    /// Label printed in the open log lines.
    fn label(&self) -> &'static str {
        match self.wtype {
            WrapperType::UsbSerial2 => "UsbSerial2",
            _ => "UsbSerial",
        }
    }

    /// Current link, if any.
    fn link(&self) -> Option<Arc<Link>> {
        self.link.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Open one port by name and start its reader.
    ///
    /// `name`: device. `baud`: speed. `dtr`/`rts`: desired line state. Returns the link.
    fn open_port(name: &str, baud: i32, dtr: bool, rts: bool) -> io::Result<Arc<Link>> {
        let mut port = serialport::new(name, baud.max(1) as u32)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(WRITE_TIMEOUT)
            .open()?;
        // line state; some virtual ports refuse, that is fine
        let _ = port.write_data_terminal_ready(dtr);
        let _ = port.write_request_to_send(rts);
        // C#: DiscardOutBuffer + DiscardInBuffer
        let _ = port.clear(ClearBuffer::All);
        let mut reader = port.try_clone()?;
        reader.set_timeout(READER_TIMEOUT)?;
        let link = Arc::new(Link {
            name: name.to_string(),
            open: AtomicBool::new(true),
            stop: AtomicBool::new(false),
            writer: Mutex::new(port),
            queue: LineQueue::new(),
            thread: Mutex::new(None),
        });
        let l2 = link.clone();
        let th = std::thread::Builder::new().name("SerialReader".into()).spawn(move || reader_loop(l2, reader))?;
        *link.thread.lock().unwrap_or_else(|e| e.into_inner()) = Some(th);
        Ok(link)
    }
}

/// Reader thread body: bytes -> lines until stop or error.
///
/// `link`: shared link state. `port`: reader handle (own timeout).
fn reader_loop(link: Arc<Link>, mut port: Box<dyn SerialPort>) {
    let mut splitter = LineSplitter::new();
    let mut tmp = [0u8; 512];
    while !link.stop.load(Ordering::SeqCst) {
        match port.read(&mut tmp) {
            Ok(0) => std::thread::sleep(Duration::from_millis(1)),
            Ok(n) => splitter.feed(&tmp[..n], |l| {
                comlog::log_str("rx", &l);
                link.queue.push(l)
            }),
            Err(e) if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {}
            Err(_) => {
                // device gone: closed from below
                if !link.stop.load(Ordering::SeqCst) {
                    link.open.store(false, Ordering::SeqCst);
                    link.queue.set_error(io::ErrorKind::NotConnected);
                }
                break;
            }
        }
    }
}

impl ComWrapper for SerialCom {
    /// See trait.
    fn wrapper_type(&self) -> WrapperType {
        self.wtype
    }

    /// Takes `(port_name, baud_rate)`.
    fn configure(&self, conf: &ComConfig) {
        if let ComConfig::Serial { port_name, baud_rate } = conf {
            *self.cfg.lock().unwrap_or_else(|e| e.into_inner()) = (port_name.clone(), *baud_rate);
        }
    }

    /// Open the configured port (see module docs for the differences to C#).
    fn open(&self) -> io::Result<()> {
        if self.is_open() {
            return Ok(());
        }
        self.close(true);
        let (name, baud) = self.cfg.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let hard: bool = lg_base::settings::get("HardReset Grbl On Connect", false);
        let mut dtr = hard;
        let rts = hard;
        if lg_base::settings::get("Firmware Type", Firmware::Grbl) == Firmware::Marlin {
            dtr = true;
        }
        let soft: bool = lg_base::settings::get("Reset Grbl On Connect", false);
        let diag = reset_diagnostic(dtr, rts, soft);
        let label = self.label();
        let msg = format!("Open {} @ {} baud {} ({})", name.to_uppercase(), baud, diag, label);
        comlog::log_str("com", &msg);
        logger::log_message("OpenCom", &msg);

        let link = match Self::open_port(&name, baud, dtr, rts) {
            Ok(l) => l,
            Err(ioex) => {
                // issue #31 fix, windows only
                let alt = if cfg!(windows) { issue31_alternative(&name) } else { None };
                let Some(alt) = alt else { return Err(ioex) };
                comlog::log_str("com", &format!("Open {} @ {} baud {} ({})", alt.to_uppercase(), baud, diag, label));
                logger::log_message("OpenCom", &format!("Retry opening {} as {} ({}) (issue #31)", name.to_uppercase(), alt.to_uppercase(), label));
                // report the original error, not the new one
                Self::open_port(&alt, baud, dtr, rts).map_err(|_| ioex)?
            }
        };
        *self.link.write().unwrap_or_else(|e| e.into_inner()) = Some(link);
        Ok(())
    }

    /// Close and drop the port; safe when already closed.
    fn close(&self, auto: bool) {
        let link = self.link.write().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(l) = link {
            if l.open.load(Ordering::SeqCst) {
                let who = if auto { "CORE" } else { "USER" };
                comlog::log_str("com", &format!("Close {} [{}]", l.name.to_uppercase(), who));
                logger::log_message("CloseCom", &format!("Close {} [{}]", l.name.to_uppercase(), who));
                if let Ok(w) = l.writer.lock() {
                    let _ = w.clear(ClearBuffer::All);
                }
            }
            l.open.store(false, Ordering::SeqCst);
            l.stop.store(true, Ordering::SeqCst);
            let th = l.thread.lock().unwrap_or_else(|e| e.into_inner()).take();
            if let Some(th) = th {
                // reader never calls close, so joining is safe
                let _ = th.join();
            }
        }
    }

    /// Port open and not failed.
    fn is_open(&self) -> bool {
        self.link().is_some_and(|l| l.open.load(Ordering::SeqCst))
    }

    /// Write one byte; no-op when closed (like C# `com != null`).
    fn write_byte(&self, b: u8) -> io::Result<()> {
        let Some(l) = self.link() else { return Ok(()) };
        comlog::log("tx", &[b]);
        let mut w = l.writer.lock().unwrap_or_else(|e| e.into_inner());
        w.write_all(&[b])
    }

    /// Write bytes; no-op when closed.
    fn write_bytes(&self, data: &[u8]) -> io::Result<()> {
        let Some(l) = self.link() else { return Ok(()) };
        comlog::log("tx", data);
        let mut w = l.writer.lock().unwrap_or_else(|e| e.into_inner());
        w.write_all(data)
    }

    /// Write text; no-op when closed.
    fn write_str(&self, text: &str) -> io::Result<()> {
        let Some(l) = self.link() else { return Ok(()) };
        comlog::log_str("tx", text);
        let mut w = l.writer.lock().unwrap_or_else(|e| e.into_inner());
        // .NET SerialPort default encoding is ASCII: non-ASCII becomes '?'
        w.write_all(&crate::util::ascii_lossy(text))
    }

    /// Wait up to 100 ms for a line. Closed port -> `Ok(None)` (C# throws; the core loop ends on `is_open`).
    fn read_line_blocking(&self) -> io::Result<Option<String>> {
        match self.link() {
            Some(l) => l.queue.pop_timeout(POLL_WAIT),
            None => {
                std::thread::sleep(Duration::from_millis(1));
                Ok(None)
            }
        }
    }

    /// A complete line waits in the queue.
    fn has_data(&self) -> bool {
        self.link().is_some_and(|l| l.queue.has_data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue31_name() {
        assert_eq!(issue31_alternative("COM23").as_deref(), Some("COM2"));
        assert_eq!(issue31_alternative("COM3"), None);
        assert_eq!(issue31_alternative("/dev/ttyUSB10").as_deref(), Some("/dev/ttyUSB1"));
        assert_eq!(issue31_alternative("x"), None);
    }

    #[test]
    fn open_missing_port_fails() {
        let c = SerialCom::new(WrapperType::UsbSerial);
        c.configure(&ComConfig::Serial { port_name: "/dev/lasergrbl-does-not-exist".into(), baud_rate: 115200 });
        assert!(c.open().is_err());
        assert!(!c.is_open());
        assert_eq!(c.read_line_blocking().unwrap(), None);
        // write on closed is a no-op
        assert!(c.write_str("x\n").is_ok());
        c.close(true);
    }

    #[cfg(unix)]
    #[test]
    fn pty_roundtrip() {
        use serialport::TTYPort;
        // some CI sandboxes (macOS runner) cannot open a pty: nothing to test then
        let Ok((mut master, slave)) = TTYPort::pair() else {
            eprintln!("pty_roundtrip skipped: no pty available");
            return;
        };
        let name = slave.name().expect("slave name");
        // keep the slave fd open so the pty stays alive
        let c = SerialCom::new(WrapperType::UsbSerial2);
        c.configure(&ComConfig::Serial { port_name: name, baud_rate: 115200 });
        // macOS/BSD: the serial library cannot configure a pty slave (ENOTTY, "Not a typewriter"); Linux must work
        if let Err(e) = c.open() {
            if cfg!(target_os = "linux") {
                panic!("open pty: {e:?}");
            }
            eprintln!("pty_roundtrip skipped: serial library cannot open a pty on this OS ({e:?})");
            return;
        }
        assert!(c.is_open());
        master.write_all(b"Grbl 1.1f ['$' for help]\r\nok\n").unwrap();
        let mut got = Vec::new();
        for _ in 0..30 {
            if let Some(l) = c.read_line_blocking().unwrap() {
                got.push(l);
            }
            if got.len() == 2 {
                break;
            }
        }
        assert_eq!(got, vec!["Grbl 1.1f ['$' for help]\r".to_string(), "ok".to_string()]);
        c.write_str("$$\n").unwrap();
        let mut buf = [0u8; 16];
        master.set_timeout(Duration::from_millis(500)).unwrap();
        let n = master.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"$$\n");
        c.close(false);
        assert!(!c.is_open());
        drop(slave);
    }
}
