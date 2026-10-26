// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `GrblEmulator/Grblv11Emulator.cs`: an in-process Grbl 1.1 look-alike.
//!
//! Differences to C#: the config is persisted as JSON (`GrblEmulator.v11.json` in the data dir) instead of a
//! BinaryFormatter file; the static `EmulatorMessage` event becomes a handler list.
//! Locking: C# touches `mPaused/mCheck/SPB/px..` from several threads without locks. Here flags are atomics
//! and the position state sits in one mutex that is never held across a sleep, so `?` is answered while a
//! long move "executes".

use lg_base::paths::data_path;
use lg_base::tools::hires_timer;
use lg_base::tools::thread_class::ThreadObject;
use lg_dotnet::{fmt_decimal_custom, Decimal, TimeSpan};
use lg_gcode::{GrblCommand, GrblConfST, GrblVersionInfo, StatePositionBuilder};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// C# `EMULATE_RANDOM_ERRORS` (const false).
const EMULATE_RANDOM_ERRORS: bool = false;
/// C# `EMULATE_MISSING_OK` (const false).
const EMULATE_MISSING_OK: bool = false;

/// Message sink (C# `SendMessage` delegate); gets text that already ends with `\n`.
pub type SendMessage = Box<dyn Fn(&str) + Send + Sync>;

/// Sleep used for the emulated board delays (command duration, homing, ...). Tests replace it to control "time".
pub type Sleeper = Box<dyn Fn(Duration) + Send + Sync>;

/// Handler of the static `EmulatorMessage` event; `None` = clear the log.
pub type MessageHandler = Arc<dyn Fn(Option<&str>) + Send + Sync>;

static HANDLERS: Mutex<Vec<MessageHandler>> = Mutex::new(Vec::new());

/// Subscribe to `Grblv11Emulator.EmulatorMessage`.
///
/// `h`: handler, called with `Some(line)` per log line and `None` on reset.
pub fn add_emulator_message_handler(h: MessageHandler) {
    HANDLERS.lock().unwrap_or_else(|e| e.into_inner()).push(h);
}

/// Remove all handlers (tests / UI teardown).
pub fn clear_emulator_message_handlers() {
    HANDLERS.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// `EmuLog`: errors in handlers are ignored like C#.
fn emu_log(p: Option<&str>) {
    let hs: Vec<MessageHandler> = HANDLERS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for h in hs {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h(p)));
    }
}

/// Position state (C# `SPB`, `px..pz`, `wx..wz`, `toSleep`).
struct Machine {
    spb: StatePositionBuilder,
    px: Decimal,
    py: Decimal,
    pz: Decimal,
    wx: Decimal,
    wy: Decimal,
    wz: Decimal,
    to_sleep: TimeSpan,
}

struct Inner {
    send: SendMessage,
    machine: Mutex<Machine>,
    conf: Mutex<GrblConfST>,
    paused: AtomicBool,
    check: AtomicBool,
    opened: AtomicBool,
    rx_buf: Mutex<VecDeque<String>>,
    tx_buf: Mutex<VecDeque<String>>,
    rng_state: Mutex<u64>,
    file: PathBuf,
    /// A queued line was taken by the RX thread and its handling (incl. the log line) is not finished.
    busy: AtomicBool,
    sleeper: Sleeper,
}

/// The emulator. Create with a sink that receives every line the "board" sends.
pub struct Grblv11Emulator {
    inner: Arc<Inner>,
    rx: Mutex<ThreadObject>,
    tx: Mutex<ThreadObject>,
}

/// Default config table of the C# ctor.
fn default_conf() -> GrblConfST {
    let t: [(i32, &str); 34] = [
        (0, "10"), (1, "25"), (2, "0"), (3, "0"), (4, "0"), (5, "0"), (6, "0"), (10, "1"), (11, "0.010"), (12, "0.002"),
        (13, "0"), (20, "0"), (21, "0"), (22, "0"), (23, "0"), (24, "25.000"), (25, "500.000"), (26, "250"), (27, "1.000"),
        (30, "1000.0"), (31, "0.0"), (32, "0"), (100, "250.000"), (101, "250.000"), (102, "250.000"), (110, "500.000"),
        (111, "500.000"), (112, "500.000"), (120, "10.000"), (121, "10.000"), (122, "10.000"), (130, "200.000"),
        (131, "200.000"), (132, "200.000"),
    ];
    let table: Vec<(i32, String)> = t.iter().map(|(k, v)| (*k, v.to_string())).collect();
    GrblConfST::with_table(Some(GrblVersionInfo::with_build(1, 1, '#')), &table)
}

/// Path of the persisted config (C# `GrblEmulator.v11.bin`).
pub fn config_file() -> PathBuf {
    data_path().join("GrblEmulator.v11.json")
}

impl Grblv11Emulator {
    /// New closed emulator.
    ///
    /// `send_func`: sink for board output (C# `SendMessage`); each call carries one line + `\n`.
    pub fn new(send_func: SendMessage) -> Self {
        Self::with_config_file(send_func, config_file())
    }

    /// Same as `new` with an explicit config file (tests).
    ///
    /// `send_func`: sink. `file`: where the config is loaded from and saved to on close.
    pub fn with_config_file(send_func: SendMessage, file: PathBuf) -> Self {
        Self::with_sleeper(send_func, file, Box::new(std::thread::sleep))
    }

    /// Same as `with_config_file` with a custom sleep function (deterministic tests).
    ///
    /// `send_func`: sink. `file`: config file. `sleeper`: called instead of `thread::sleep` for every emulated delay.
    pub fn with_sleeper(send_func: SendMessage, file: PathBuf, sleeper: Sleeper) -> Self {
        // C#: try load, fallback to default table
        let conf = std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str::<GrblConfST>(&t).ok())
            .unwrap_or_else(default_conf);
        let inner = Arc::new(Inner {
            send: send_func,
            machine: Mutex::new(Machine::new()),
            conf: Mutex::new(conf),
            paused: AtomicBool::new(false),
            check: AtomicBool::new(false),
            opened: AtomicBool::new(false),
            rx_buf: Mutex::new(VecDeque::new()),
            tx_buf: Mutex::new(VecDeque::new()),
            rng_state: Mutex::new(0x2545F4914F6CDD1D),
            file,
            busy: AtomicBool::new(false),
            sleeper,
        });
        let rx = ThreadObject::new("Emulator RX", 0);
        let tx = ThreadObject::new("Emulator TX", 0);
        let (h_rx, h_tx) = (rx.handle(), tx.handle());
        let i1 = inner.clone();
        rx.set_delegate(Box::new(move || {
            let queue = i1.manage_rx();
            h_rx.set_sleep_time(if queue > 0 { 0 } else { 1 });
        }));
        let i2 = inner.clone();
        tx.set_delegate(Box::new(move || {
            let more = i2.manage_tx();
            h_tx.set_sleep_time(if more { 0 } else { 1 });
        }));
        Self { inner, rx: Mutex::new(rx), tx: Mutex::new(tx) }
    }

    /// `CloseCom`: stop threads, clear buffers, save config.
    pub fn close_com(&self) {
        if self.inner.opened.swap(false, Ordering::SeqCst) {
            self.rx.lock().unwrap_or_else(|e| e.into_inner()).stop();
            self.tx.lock().unwrap_or_else(|e| e.into_inner()).stop();
            self.inner.rx_buf.lock().unwrap_or_else(|e| e.into_inner()).clear();
            self.inner.tx_buf.lock().unwrap_or_else(|e| e.into_inner()).clear();
            emu_log(Some("Connection lost!"));
            // C# Serializer.ObjToFile; errors would throw in C#, here ignored
            if let Ok(t) = serde_json::to_string_pretty(&*self.inner.conf.lock().unwrap_or_else(|e| e.into_inner())) {
                let _ = std::fs::write(&self.inner.file, t);
            }
        }
    }

    /// `OpenCom`: start threads and send the version line.
    pub fn open_com(&self) {
        if !self.inner.opened.swap(true, Ordering::SeqCst) {
            emu_log(Some("Client connected!"));
            self.inner.rx_buf.lock().unwrap_or_else(|e| e.into_inner()).clear();
            self.inner.tx_buf.lock().unwrap_or_else(|e| e.into_inner()).clear();
            self.inner.paused.store(false, Ordering::SeqCst);
            self.inner.check.store(false, Ordering::SeqCst);
            self.rx.lock().unwrap_or_else(|e| e.into_inner()).start();
            self.tx.lock().unwrap_or_else(|e| e.into_inner()).start();
            self.inner.send_version();
        }
    }

    /// True between `open_com` and `close_com`.
    pub fn is_opened(&self) -> bool {
        self.inner.opened.load(Ordering::SeqCst)
    }

    /// `ManageMessage`: one write from the host.
    ///
    /// `data`: a single byte (real-time command) or text lines.
    pub fn manage_message(&self, data: &[u8]) {
        if data.len() == 1 && data[0] != b'\n' {
            match data[0] {
                24 => self.inner.grbl_reset(),
                63 => self.inner.send_status(),
                33 => self.inner.pause(true),
                126 => self.inner.pause(false),
                _ => {}
            }
        } else {
            let message = String::from_utf8_lossy(data).into_owned();
            if message == "version\n" || message == "{fb:n}\n" {
                // ignored
            } else {
                self.inner.rx_buf.lock().unwrap_or_else(|e| e.into_inner()).push_back(message);
            }
        }
    }

    /// True when nothing is left to do: no queued line is waiting (a paused queue does not count) and none is being
    /// handled, and every answer was sent. Event based, no timing: lets a test wait for "done" instead of sleeping.
    pub fn is_idle(&self) -> bool {
        {
            let q = self.inner.rx_buf.lock().unwrap_or_else(|e| e.into_inner());
            // busy is set under this lock when a line is taken, so (empty, !busy) is a consistent state
            if (!q.is_empty() && !self.inner.paused.load(Ordering::SeqCst)) || self.inner.busy.load(Ordering::SeqCst) {
                return false;
            }
        }
        // answers are queued before busy is cleared; the TX thread holds this lock until a line is sent
        self.inner.tx_buf.lock().unwrap_or_else(|e| e.into_inner()).is_empty()
    }

    /// Current config copy (tests / UI).
    pub fn config(&self) -> GrblConfST {
        self.inner.conf.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Drop for Grblv11Emulator {
    /// Stop threads.
    fn drop(&mut self) {
        self.rx.lock().unwrap_or_else(|e| e.into_inner()).stop();
        self.tx.lock().unwrap_or_else(|e| e.into_inner()).stop();
    }
}

impl Machine {
    fn new() -> Self {
        Self {
            spb: StatePositionBuilder::new(),
            px: Decimal::ZERO,
            py: Decimal::ZERO,
            pz: Decimal::ZERO,
            wx: Decimal::ZERO,
            wy: Decimal::ZERO,
            wz: Decimal::ZERO,
            to_sleep: TimeSpan::ZERO,
        }
    }
}

impl Inner {
    /// `ImmediateTX`: send now (under the tx lock like C#).
    fn immediate_tx(&self, response: &str) {
        let _g = self.tx_buf.lock().unwrap_or_else(|e| e.into_inner());
        (self.send)(&format!("{response}\n"));
    }

    /// `EnqueueTX`.
    fn enqueue_tx(&self, response: &str) {
        self.tx_buf.lock().unwrap_or_else(|e| e.into_inner()).push_back(format!("{response}\n"));
    }

    fn send_version(&self) {
        self.immediate_tx("Grbl 1.1# ['$' for help]");
    }

    fn pause(&self, value: bool) {
        self.paused.store(value, Ordering::SeqCst);
        self.send_status();
    }

    /// `Status` property.
    fn status(&self) -> &'static str {
        if self.check.load(Ordering::SeqCst) {
            "Check"
        } else if self.paused.load(Ordering::SeqCst) {
            "Hold"
        } else if !self.rx_buf.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
            "Run"
        } else {
            "Idle"
        }
    }

    /// `SendStatus`: `<Idle|MPos:..>` (+ WCO when an offset exists).
    fn send_status(&self) {
        let status = self.status();
        let f = |d: Decimal| fmt_decimal_custom(d, "0.000");
        let line = {
            let m = self.machine.lock().unwrap_or_else(|e| e.into_inner());
            if m.spb.has_wco() {
                format!("<{}|MPos:{},{},{}|WCO:{},{},{}>\n", status, f(m.px), f(m.py), f(m.pz), f(m.wx), f(m.wy), f(m.wz))
            } else {
                format!("<{}|MPos:{},{},{}>\n", status, f(m.px), f(m.py), f(m.pz))
            }
        };
        // C# `ImmediateTX(.. + "\n")` adds a second newline; the receiver drops empty lines
        self.immediate_tx(&line);
    }

    fn grbl_reset(&self) {
        self.rx_buf.lock().unwrap_or_else(|e| e.into_inner()).clear();
        (self.sleeper)(Duration::from_millis(50));
        self.check.store(false, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
        {
            let mut m = self.machine.lock().unwrap_or_else(|e| e.into_inner());
            m.to_sleep = TimeSpan::ZERO;
            m.spb = StatePositionBuilder::new();
        }
        emu_log(None);
        emu_log(Some("Grbl Reset"));
        self.send_version();
    }

    /// `ManageRX` body. Returns the queue length left after dequeue (C# `queue`).
    fn manage_rx(&self) -> usize {
        let mut line = None;
        let mut queue = 0;
        {
            let mut q = self.rx_buf.lock().unwrap_or_else(|e| e.into_inner());
            if !q.is_empty() && !self.paused.load(Ordering::SeqCst) {
                line = q.pop_front();
                queue = q.len();
                self.busy.store(true, Ordering::SeqCst);
            }
        }
        if let Some(line) = line {
            // C# try/catch(Exception) {} around the whole dispatch
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if line == "$$\n" {
                    self.send_config();
                } else if GrblConfST::is_set_conf(&line) {
                    self.set_config(&line);
                } else if line == "$C\n" {
                    self.swap_check();
                } else if line == "$H\n" {
                    self.emulate_homing();
                } else if line == "$I\n" {
                    self.send_info();
                } else if line.starts_with("$J=") {
                    if let Some(c) = GrblCommand::new_jog(&line) {
                        let _ = self.emulate_command(c);
                    }
                } else {
                    let _ = self.emulate_command(GrblCommand::new(&line));
                }
                emu_log(Some(line.trim_matches('\n')));
            }));
            self.busy.store(false, Ordering::SeqCst);
        }
        queue
    }

    /// `ManageTX` body. Returns true when more output waits.
    fn manage_tx(&self) -> bool {
        let mut g = self.tx_buf.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(line) = g.pop_front() {
            (self.send)(&line);
        }
        !g.is_empty()
    }

    fn set_config(&self, p: &str) {
        (self.sleeper)(Duration::from_millis(10));
        let ok = self.conf.lock().unwrap_or_else(|e| e.into_inner()).set_value_if_key_exist(p);
        self.enqueue_tx(if ok { "ok" } else { "error" });
    }

    fn send_config(&self) {
        // reply to $$ first, then the immediate dump
        self.enqueue_tx("ok");
        let rows: Vec<(i32, String)> = self.conf.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|(k, v)| (k, v.to_string())).collect();
        for (k, v) in rows {
            self.immediate_tx(&format!("${k}={v}"));
        }
    }

    fn send_info(&self) {
        self.enqueue_tx("ok");
        // standard Grbl
        self.enqueue_tx("[VER:1.1#.20220607:]");
        self.enqueue_tx("[OPT:V,15,1024]");
    }

    fn emulate_homing(&self) {
        (self.sleeper)(Duration::from_millis(2000));
        {
            let mut m = self.machine.lock().unwrap_or_else(|e| e.into_inner());
            m.spb.homing();
            m.px = Decimal::ZERO;
            m.py = Decimal::ZERO;
            m.pz = Decimal::ZERO;
        }
        self.enqueue_tx("ok");
    }

    fn swap_check(&self) {
        let v = !self.check.load(Ordering::SeqCst);
        self.check.store(v, Ordering::SeqCst);
        self.enqueue_tx("ok");
        self.enqueue_tx(if v { "[Enabled]" } else { "[Disabled]" });
    }

    /// Tiny xorshift for the (disabled) random error switches; C# `Random`.
    fn rng_next(&self, bound: u64) -> u64 {
        let mut s = self.rng_state.lock().unwrap_or_else(|e| e.into_inner());
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s % bound
    }

    /// `EmulateCommand`: run a command through the position builder, sleep for its duration, answer `ok`.
    ///
    /// `cmd`: command. Returns Err(message) where C# rethrows (caught by the caller).
    fn emulate_command(&self, mut cmd: GrblCommand) -> Result<(), String> {
        if !self.check.load(Ordering::SeqCst) {
            let conf = self.conf.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let cmd_time = {
                let mut m = self.machine.lock().unwrap_or_else(|e| e.into_inner());
                m.spb.analyze_command(&mut cmd, true, Some(&conf)).map_err(|e| e.to_string())
            };
            let res = (|| -> Result<(), String> {
                let cmd_time = cmd_time?;
                let to_sleep = {
                    let mut m = self.machine.lock().unwrap_or_else(|e| e.into_inner());
                    m.to_sleep = m.to_sleep + TimeSpan::from_ticks(cmd_time.ticks());
                    m.to_sleep
                };
                // execute sleep (not under the lock: status stays answerable)
                if to_sleep.total_milliseconds() > 15.0 {
                    let start = hires_timer::total_nano();
                    (self.sleeper)(Duration::from_secs_f64(to_sleep.total_seconds().max(0.0)));
                    let stop = hires_timer::total_nano();
                    let mut m = self.machine.lock().unwrap_or_else(|e| e.into_inner());
                    m.to_sleep = m.to_sleep - TimeSpan::from_milliseconds((stop - start) as f64 / 1000.0 / 1000.0);
                }
                let mut m = self.machine.lock().unwrap_or_else(|e| e.into_inner());
                m.px = m.spb.x().number();
                m.py = m.spb.y().number();
                m.pz = m.spb.z().number();
                m.wx = m.spb.wco_x();
                m.wy = m.spb.wco_y();
                m.wz = m.spb.wco_z();
                Ok(())
            })();
            cmd.delete_helper();
            res?;
        }
        if EMULATE_RANDOM_ERRORS && self.rng_next(50) == 0 {
            self.enqueue_tx("error:2");
        } else if !EMULATE_MISSING_OK || self.rng_next(20) != 0 {
            self.enqueue_tx("ok");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn emu() -> (Grblv11Emulator, mpsc::Receiver<String>, PathBuf) {
        // no real config dir
        crate::test_support::init_env();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let file = std::env::temp_dir().join(format!("lg_emu_test_{}_{}.json", std::process::id(), rand_id()));
        let e = Grblv11Emulator::with_config_file(Box::new(move |s| {
            let _ = tx.lock().unwrap().send(s.to_string());
        }), file.clone());
        (e, rx, file)
    }

    fn rand_id() -> u128 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    }

    fn wait_for(rx: &mpsc::Receiver<String>, pred: impl Fn(&str) -> bool) -> Vec<String> {
        let mut got = Vec::new();
        let end = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < end {
            if let Ok(s) = rx.recv_timeout(Duration::from_millis(50)) {
                let hit = pred(&s);
                got.push(s);
                if hit {
                    return got;
                }
            }
        }
        panic!("timeout, got {got:?}");
    }

    #[test]
    fn welcome_status_and_ok() {
        let (e, rx, file) = emu();
        e.open_com();
        let g = wait_for(&rx, |s| s.starts_with("Grbl "));
        assert_eq!(g.last().unwrap(), "Grbl 1.1# ['$' for help]\n");
        e.manage_message(&[63]);
        let g = wait_for(&rx, |s| s.starts_with('<'));
        assert_eq!(g.last().unwrap(), "<Idle|MPos:0.000,0.000,0.000>\n\n");
        e.manage_message(b"G0 X5 Y2\n");
        wait_for(&rx, |s| s == "ok\n");
        e.manage_message(&[63]);
        let g = wait_for(&rx, |s| s.starts_with('<'));
        assert!(g.last().unwrap().starts_with("<Idle|MPos:5.000,2.000,0.000>"), "{g:?}");
        e.close_com();
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn config_dump_and_set() {
        let (e, rx, file) = emu();
        e.open_com();
        e.manage_message(b"$$\n");
        let g = wait_for(&rx, |s| s == "$132=200.000\n");
        // NOTE(upstream): "ok" goes through the TX queue, the dump is immediate, so "ok" is not first
        assert_eq!(g.iter().filter(|s| s.starts_with('$')).count(), 34);
        assert!(g.iter().any(|s| s == "ok\n") || rx.recv_timeout(Duration::from_secs(1)).unwrap() == "ok\n");
        e.manage_message(b"$100=300\n");
        wait_for(&rx, |s| s == "ok\n");
        e.manage_message(b"$999=1\n");
        wait_for(&rx, |s| s == "error\n");
        assert_eq!(e.config().get(100), Some("300"));
        e.manage_message(b"$I\n");
        let g = wait_for(&rx, |s| s.starts_with("[OPT:"));
        assert!(g.iter().any(|s| s == "[VER:1.1#.20220607:]\n"));
        e.close_com();
        // persisted
        let e2 = Grblv11Emulator::with_config_file(Box::new(|_| {}), file.clone());
        assert_eq!(e2.config().get(100), Some("300"));
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn hold_and_reset() {
        let (e, rx, file) = emu();
        e.open_com();
        wait_for(&rx, |s| s.starts_with("Grbl "));
        e.manage_message(&[33]);
        let g = wait_for(&rx, |s| s.starts_with('<'));
        assert!(g.last().unwrap().starts_with("<Hold|"));
        // paused: queued command is not executed
        e.manage_message(b"G0 X1\n");
        std::thread::sleep(Duration::from_millis(60));
        assert!(rx.try_recv().is_err());
        e.manage_message(&[126]);
        wait_for(&rx, |s| s == "ok\n");
        e.manage_message(&[24]);
        wait_for(&rx, |s| s.starts_with("Grbl "));
        e.close_com();
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn check_mode_toggles() {
        let (e, rx, file) = emu();
        e.open_com();
        e.manage_message(b"$C\n");
        wait_for(&rx, |s| s == "[Enabled]\n");
        e.manage_message(&[63]);
        let g = wait_for(&rx, |s| s.starts_with('<'));
        assert!(g.last().unwrap().starts_with("<Check|"));
        e.close_com();
        let _ = std::fs::remove_file(file);
    }
}
