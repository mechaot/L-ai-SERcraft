// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Protocol golden test: replays every `tests/golden/protocol/*.scn` scenario on the Rust core and
//! compares the trace byte for byte with `*.txt`, which the ORIGINAL C# `GrblCore` (Mono) produced
//! (`tools/golden/protocol/run.sh`).
//!
//! The interpreter mirrors `tools/golden/protocol/Protocol.cs` verb by verb. Stepping mode: the TX/RX loop
//! bodies are called by hand (`GrblCore::step_tx/step_rx`), so the order of everything is deterministic.
//! Time dependent code (hang detection, buffer stuck) runs on a mocked clock here; the oracle really sleeps.
//! Events are printed before the writes of the same step (the Rust events are not ordered against writes).

use crossbeam_channel::Receiver;
use lasergrbl_core::com::{ComConfig, ComWrapper, WrapperType};
use lasergrbl_core::*;
use lg_dotnet::{fmt_decimal, parse_decimal, TimeSpan};
use lg_gcode::{configuration, CommandStatus, Firmware, GPoint, GrblCommand, GrblConfParam, GrblConfST, GrblRow, GrblVersionInfo};
use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static FAKE_NOW: AtomicI64 = AtomicI64::new(1_000_000);

fn fake_clock() -> i64 {
    FAKE_NOW.load(Ordering::SeqCst)
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------------------------- escapes

/// Same escape as `Esc.E` in the C# harness.
fn esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => o.push_str(&format!("\\x{:02X}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// Same as `Esc.U`.
fn unesc(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] != '\\' || i + 1 >= c.len() {
            o.push(c[i]);
            i += 1;
            continue;
        }
        i += 1;
        match c[i] {
            'n' => o.push('\n'),
            'r' => o.push('\r'),
            't' => o.push('\t'),
            '\\' => o.push('\\'),
            'x' => {
                let h: String = c[i + 1..i + 3].iter().collect();
                o.push(char::from_u32(u32::from_str_radix(&h, 16).unwrap()).unwrap());
                i += 2;
            }
            'u' => {
                let h: String = c[i + 1..i + 5].iter().collect();
                o.push(char::from_u32(u32::from_str_radix(&h, 16).unwrap()).unwrap());
                i += 4;
            }
            other => {
                o.push('\\');
                o.push(other);
            }
        }
        i += 1;
    }
    o
}

/// C# `bool.ToString()`.
fn b(x: bool) -> &'static str {
    if x { "True" } else { "False" }
}

// ---------------------------------------------------------------------------------------------- board

struct Rule {
    m: String,
    exact: bool,
    once: bool,
    used: bool,
    replies: Vec<String>,
}

#[derive(Default)]
struct Board {
    rules: Vec<Rule>,
    auto: bool,
    board_q: VecDeque<String>,
    tx_buf: Vec<String>,
    tx_total: usize,
    tx_all: Vec<String>,
    show_queries: bool,
    threads: bool,
}

enum Item {
    Line(String),
    Io,
    Fatal,
}

/// The fake link. Writes are recorded into the board; replies come from its rules.
struct ScriptCom {
    opened: AtomicBool,
    q: Mutex<VecDeque<Item>>,
    board: Arc<Mutex<Board>>,
}

impl ScriptCom {
    fn push(&self, i: Item) {
        lock(&self.q).push_back(i);
    }

    fn pending(&self) -> usize {
        lock(&self.q).len()
    }

    fn reply(&self, line: &str) {
        let mut bd = lock(&self.board);
        let mut rep: Option<Vec<String>> = None;
        for r in bd.rules.iter_mut() {
            if r.once && r.used {
                continue;
            }
            let hit = if r.exact { line == r.m } else { line.contains(&r.m) };
            if !hit {
                continue;
            }
            r.used = true;
            rep = Some(r.replies.clone());
            break;
        }
        if rep.is_none() && bd.auto {
            rep = Some(vec!["ok".to_string()]);
        }
        let Some(rep) = rep else { return };
        let threads = bd.threads;
        for s in rep {
            if threads {
                self.push(Item::Line(s));
            } else {
                bd.board_q.push_back(s);
            }
        }
    }
}

impl ComWrapper for ScriptCom {
    fn wrapper_type(&self) -> WrapperType {
        WrapperType::Emulator
    }
    fn configure(&self, _conf: &ComConfig) {}
    fn open(&self) -> io::Result<()> {
        self.opened.store(true, Ordering::SeqCst);
        lock(&self.q).clear();
        Ok(())
    }
    fn close(&self, _auto: bool) {
        self.opened.store(false, Ordering::SeqCst);
    }
    fn is_open(&self) -> bool {
        self.opened.load(Ordering::SeqCst)
    }
    fn write_byte(&self, byte: u8) -> io::Result<()> {
        let mut bd = lock(&self.board);
        if bd.show_queries || !(byte == 0x3F || byte == 0x88) {
            bd.tx_buf.push(format!("TXB {byte:02X}"));
            bd.tx_total += 1;
        }
        Ok(())
    }
    fn write_bytes(&self, data: &[u8]) -> io::Result<()> {
        let mut bd = lock(&self.board);
        let s: Vec<String> = data.iter().map(|x| format!(" {x:02X}")).collect();
        bd.tx_buf.push(format!("TXA{}", s.concat()));
        bd.tx_total += 1;
        Ok(())
    }
    fn write_str(&self, text: &str) -> io::Result<()> {
        {
            let mut bd = lock(&self.board);
            bd.tx_all.push(text.to_string());
            if bd.show_queries || text != "M114\n" {
                bd.tx_buf.push(format!("TX {}", esc(text)));
                bd.tx_total += 1;
            }
        }
        let line = text.trim_end_matches(['\r', '\n']);
        if !line.is_empty() {
            self.reply(line);
        }
        Ok(())
    }
    fn read_line_blocking(&self) -> io::Result<Option<String>> {
        let item = lock(&self.q).pop_front();
        match item {
            Some(Item::Line(l)) => Ok(Some(l)),
            Some(Item::Io) => Err(io::Error::other("io")),
            Some(Item::Fatal) => Err(io::Error::new(io::ErrorKind::NotConnected, "fatal")),
            None => {
                if self.is_open() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(None)
            }
        }
    }
    fn has_data(&self) -> bool {
        self.is_open() && !lock(&self.q).is_empty()
    }
}

// ---------------------------------------------------------------------------------------------- interpreter

struct Interp {
    out: String,
    core: Option<Arc<GrblCore>>,
    com: Option<Arc<ScriptCom>>,
    rx: Option<Receiver<CoreEvent>>,
    board: Arc<Mutex<Board>>,
    status_trace: VecDeque<MacStatus>,
    scratch: PathBuf,
    scn_no: usize,
    file_no: usize,
    async_t: Option<std::thread::JoinHandle<Result<(), String>>>,
    in_prog: bool,
    prog: String,
    prog_n: usize,
}

fn parse_ver(t: &str) -> Option<GrblVersionInfo> {
    if t == "none" {
        return None;
    }
    let (maj, m) = t.split_once('.').unwrap();
    let maj: i32 = maj.parse().unwrap();
    let mc: Vec<char> = m.chars().collect();
    if mc.len() > 1 {
        Some(GrblVersionInfo::with_build(maj, mc[0].to_digit(10).unwrap() as i32, mc[1]))
    } else {
        Some(GrblVersionInfo::new(maj, m.parse().unwrap()))
    }
}

fn parse_tm(n: &str) -> ThreadingMode {
    match n {
        "Slow" => ThreadingMode::Slow,
        "Quiet" => ThreadingMode::Quiet,
        "Fast" => ThreadingMode::Fast,
        "UltraFast" => ThreadingMode::UltraFast,
        _ => ThreadingMode::Insane,
    }
}

fn parse_dir(n: &str) -> JogDirection {
    use JogDirection::*;
    [Abort, Home, N, S, W, E, NW, NE, SW, SE, Zup, Zdown, Position].into_iter().find(|d| d.to_string() == n).unwrap_or_else(|| panic!("bad dir {n}"))
}

fn num(f: f32) -> String {
    format!("{f}")
}

fn pt(p: GPoint) -> String {
    format!("{},{},{}", num(p.x), num(p.y), num(p.z))
}

/// Replace the run time of a job summary message.
fn norm_time(s: &str) -> String {
    if s.starts_with('[') && s.ends_with(']') && s.contains(" lines, ") {
        let i = s.rfind(", ").unwrap();
        return format!("{}<time>]", &s[..i + 2]);
    }
    s.to_string()
}

impl Interp {
    fn new(scratch: &Path) -> Self {
        Interp {
            out: String::new(),
            core: None,
            com: None,
            rx: None,
            board: Arc::new(Mutex::new(Board::default())),
            status_trace: VecDeque::new(),
            scratch: scratch.to_path_buf(),
            scn_no: 0,
            file_no: 0,
            async_t: None,
            in_prog: false,
            prog: String::new(),
            prog_n: 0,
        }
    }

    fn core(&self) -> &Arc<GrblCore> {
        self.core.as_ref().expect("fw first")
    }

    fn com(&self) -> &Arc<ScriptCom> {
        self.com.as_ref().expect("fw first")
    }

    fn run(&mut self, text: &str) -> String {
        self.reset_all();
        for raw in text.split('\n') {
            let line = raw.trim_end_matches('\r');
            if self.in_prog {
                if line.trim() == "end" {
                    self.in_prog = false;
                    let p = std::mem::take(&mut self.prog);
                    self.load_program(&p);
                    self.out.push_str(&format!("> prog ({} lines)\n", self.prog_n));
                } else {
                    self.prog.push_str(line);
                    self.prog.push('\n');
                    self.prog_n += 1;
                }
                continue;
            }
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if line.trim() == "prog" {
                self.in_prog = true;
                self.prog.clear();
                self.prog_n = 0;
                continue;
            }
            self.out.push_str(&format!("> {line}\n"));
            if let Err(e) = self.exec(line) {
                self.out.push_str(&format!("  ERR {e}\n"));
            }
            if self.core.is_some() {
                self.flush();
            }
        }
        if let Some(c) = self.core.take() {
            let _ = c.close_com(true);
        }
        self.rx = None;
        self.com = None;
        std::mem::take(&mut self.out)
    }

    fn reset_all(&mut self) {
        self.scn_no += 1;
        let dir = self.scratch.join(format!("scn{}", self.scn_no));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        lg_base::paths::set_data_path_override(dir.clone());
        lg_base::settings::init_at(dir.join("settings.json"));
        for s in ["Success", "Warning", "Fatal", "Connect", "Disconnect"] {
            lg_base::settings::set(&format!("Sound.{s}.Enabled"), false);
        }
        lg_base::settings::set("Threading Mode", ThreadingMode::Slow);
        lg_base::settings::set("GCode.CustomHeader", "G90".to_string());
        lg_base::settings::set("GCode.CustomFooter", "M5".to_string());
        lg_base::settings::set("GCode.CustomPasses", "G4 P0".to_string());
        lasergrbl_core::laser_life::reload_from(dir.join("llc.json"));
        lg_gcode::csvd::load_appropriate(&GrblVersionInfo::with_vendor(1, 1, 'f', None, None, false));
    }

    fn load_program(&mut self, text: &str) {
        self.file_no += 1;
        let path = self.scratch.join(format!("scn{}", self.scn_no)).join(format!("prog_{}.nc", self.file_no));
        std::fs::write(&path, text).unwrap();
        self.core().loaded_file().write().unwrap().load_file(path.to_str().unwrap(), false).expect("load");
    }

    /// Print events, then writes, of the step just done.
    fn flush(&mut self) {
        let core = self.core().clone();
        self.status_trace.extend(core.take_status_trace());
        if let Some(rx) = &self.rx {
            let evs: Vec<CoreEvent> = rx.try_iter().collect();
            for e in evs {
                match e {
                    CoreEvent::MachineStatusChanged => {
                        let s = self.status_trace.pop_front().expect("status trace");
                        self.out.push_str(&format!("  EV status {s}\n"));
                    }
                    CoreEvent::IssueDetected(i) => self.out.push_str(&format!("  EV issue {i}\n")),
                    CoreEvent::OverrideChanged => self.out.push_str("  EV override\n"),
                    CoreEvent::LoopCountChanged(d) => self.out.push_str(&format!("  EV loop {}\n", fmt_decimal(d))),
                    CoreEvent::ProgramEnded => self.out.push_str("  EV programended\n"),
                    _ => {}
                }
            }
        }
        let tx: Vec<String> = std::mem::take(&mut lock(&self.board).tx_buf);
        for s in tx {
            self.out.push_str(&format!("  {s}\n"));
        }
    }

    fn step_rx_with(&mut self, line: &str) {
        self.com().push(Item::Line(line.to_string()));
        self.core().step_rx();
    }

    fn pump_one(&mut self) -> bool {
        let s = lock(&self.board).board_q.pop_front();
        match s {
            Some(l) => {
                self.step_rx_with(&l);
                true
            }
            None => false,
        }
    }

    fn exec(&mut self, line: &str) -> Result<(), String> {
        let (verb, rest) = match line.split_once(' ') {
            Some((v, r)) => (v, r),
            None => (line, ""),
        };
        let t: Vec<&str> = rest.split_whitespace().collect();
        match verb {
            "reset" => self.reset_all(),
            "set" => self.do_set(rest),
            "conf" => {
                let mut c = GrblConfST::with_version(parse_ver(t[0]));
                for l in &t[1..] {
                    c.add_or_update(l);
                }
                configuration::set_configuration(&c);
            }
            "lastver" => match parse_ver(t[0]) {
                Some(v) => lg_base::settings::set("Last GrblVersion known", Some(v)),
                None => lg_base::settings::delete("Last GrblVersion known"),
            },
            "fw" => self.do_fw(t[0]),
            "board" => self.do_board(rest),
            "connect" => self.do_connect(false)?,
            "fastconnect" => {
                self.do_connect(true)?;
                if !t.is_empty() {
                    self.step_rx_with(&unesc(rest));
                }
            }
            "connectthreads" => {
                lock(&self.board).threads = true;
                self.core().open_com()?;
            }
            "forcestatus" => self.core().force_status(parse_status(t[0])),
            "rx" => self.step_rx_with(&unesc(rest)),
            "rxio" => {
                let n: usize = t[0].parse().unwrap();
                let text = unesc(rest.split_once(' ').unwrap().1);
                for _ in 0..n {
                    self.com().push(Item::Io);
                }
                self.step_rx_with(&text);
            }
            "rxfatal" => {
                self.com().push(Item::Fatal);
                self.core().step_rx();
            }
            "tx" => {
                let n: usize = t.first().map_or(1, |x| x.parse().unwrap());
                for _ in 0..n {
                    self.core().step_tx();
                    self.flush();
                    self.out.push_str(&format!("  TXSLEEP {}\n", self.core().tx_sleep_time()));
                }
            }
            "rxstep" => {
                self.core().step_rx();
                self.out.push_str(&format!("  RXSLEEP {}\n", self.core().rx_sleep_time()));
            }
            "pump" => {
                let n: usize = t.first().map_or(usize::MAX, |x| x.parse().unwrap());
                for _ in 0..n {
                    if !self.pump_one() {
                        break;
                    }
                    self.flush();
                }
            }
            "rununtil" => {
                let sub = unesc(rest);
                self.run_while(|i| lock(&i.board).tx_all.iter().any(|x| x.contains(&sub)));
            }
            "rundone" => self.run_while(|i| !i.core().refresh_active()),
            "awaitasync" => {
                if let Some(h) = self.async_t.take() {
                    while !h.is_finished() {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    if let Err(e) = h.join().unwrap() {
                        self.out.push_str(&format!("  ASYNCERR {}\n", esc(&e)));
                    }
                }
            }
            "run" => self.do_run(t.first().map_or(5000, |x| x.parse().unwrap())),
            "sleep" | "rsleep" => {
                let ms: i64 = t[0].parse().unwrap();
                FAKE_NOW.fetch_add(ms, Ordering::SeqCst);
                if verb == "rsleep" {
                    std::thread::sleep(Duration::from_millis(ms as u64));
                }
            }
            "rxq" => self.com().push(Item::Line(unesc(rest))),
            "trace" => {
                if t[0] == "queries" {
                    lock(&self.board).show_queries = t[1] == "on";
                }
            }
            "dump" => self.dump(),
            "dumpconf" => self.dump_conf(),
            "async" => self.do_async(&t),
            "join" => self.do_join(),
            "waitfor" => self.do_wait_for(&t),
            _ => self.do_api(verb, rest, &t)?,
        }
        Ok(())
    }

    fn do_set(&mut self, rest: &str) {
        let t: Vec<&str> = rest.split_whitespace().collect();
        let ty = t[0];
        let (key, val) = if let Some(a) = rest.find('"') {
            let bq = rest[a + 1..].find('"').unwrap() + a + 1;
            (rest[a + 1..bq].to_string(), rest[bq + 1..].trim().to_string())
        } else {
            (t[1].to_string(), t.get(2).map_or(String::new(), |s| s.to_string()))
        };
        use lg_base::settings::set;
        match ty {
            "bool" => set(&key, val == "true"),
            "int" => set(&key, val.parse::<i32>().unwrap()),
            "str" => set(&key, unesc(&val)),
            "sm" => set(
                &key,
                match val.as_str() {
                    "Buffered" => StreamingMode::Buffered,
                    "Synchronous" => StreamingMode::Synchronous,
                    _ => StreamingMode::RepeatOnError,
                },
            ),
            "tm" => set(&key, parse_tm(&val)),
            "ts" => set(&key, TimeSpan::from_seconds(val.parse::<f64>().unwrap()).to_string()),
            _ => panic!("bad set type {ty}"),
        }
    }

    fn do_fw(&mut self, name: &str) {
        if let Some(c) = self.core.take() {
            let _ = c.close_com(true);
        }
        self.rx = None;
        let fw = match name {
            "vigo" => Firmware::VigoWork,
            "marlin" => Firmware::Marlin,
            "smoothie" => Firmware::Smoothie,
            _ => Firmware::Grbl,
        };
        *lock(&self.board) = Board::default();
        lock(&self.board).auto = true;
        let com = Arc::new(ScriptCom { opened: AtomicBool::new(false), q: Mutex::new(VecDeque::new()), board: self.board.clone() });
        let (tx, rx) = crossbeam_channel::unbounded();
        let core = GrblCore::with_com(fw, tx, com.clone(), fake_clock);
        core.enable_status_trace();
        self.status_trace.clear();
        self.core = Some(core);
        self.com = Some(com);
        self.rx = Some(rx);
    }

    fn do_board(&mut self, rest: &str) {
        let t: Vec<&str> = rest.split_whitespace().collect();
        let mut bd = lock(&self.board);
        match t[0] {
            "auto" => bd.auto = t[1] == "on",
            "clear" => {
                bd.rules.clear();
                bd.board_q.clear();
            }
            "rule" => {
                let mut body = &rest[rest.find("rule").unwrap() + 5..];
                let mut once = false;
                if let Some(r) = body.strip_prefix("once ") {
                    once = true;
                    body = r;
                }
                let arrow = body.find(" => ").unwrap();
                let mut m = unesc(&body[..arrow]);
                let rep = &body[arrow + 4..];
                let mut exact = false;
                if let Some(r) = m.strip_prefix('=') {
                    exact = true;
                    m = r.to_string();
                }
                let replies = if rep.trim() == "<none>" { vec![] } else { rep.split('|').map(|s| unesc(s.trim())).collect() };
                bd.rules.push(Rule { m, exact, once, used: false, replies });
            }
            _ => panic!("bad board cmd"),
        }
    }

    fn do_connect(&mut self, fast: bool) -> Result<(), String> {
        self.core().connect_manual()?;
        if fast {
            self.core().force_status(MacStatus::Idle);
        }
        Ok(())
    }

    fn do_run(&mut self, max: usize) {
        for _ in 0..max {
            let before = lock(&self.board).tx_total;
            self.core().step_tx();
            let mut progress = lock(&self.board).tx_total != before;
            self.flush();
            if self.pump_one() {
                progress = true;
                self.flush();
            }
            if !progress && lock(&self.board).board_q.is_empty() {
                break;
            }
        }
    }

    /// Step TX and pump replies until `until` holds (90 s cap), like `RunWhile` in the C# harness.
    fn run_while(&mut self, until: impl Fn(&Interp) -> bool) {
        let end = std::time::Instant::now() + Duration::from_secs(90);
        while !until(self) && std::time::Instant::now() < end {
            let before = lock(&self.board).tx_total;
            self.core().step_tx();
            let mut progress = lock(&self.board).tx_total != before;
            self.flush();
            if self.pump_one() {
                progress = true;
                self.flush();
            }
            if !progress {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if !until(self) {
            self.out.push_str("  RUNWHILE TIMEOUT\n");
        }
    }

    fn do_wait_for(&mut self, t: &[&str]) {
        let end = std::time::Instant::now() + Duration::from_secs(90);
        while std::time::Instant::now() < end {
            let c = self.core().clone();
            let ok = match t[0] {
                "status" => c.machine_status().to_string() == t[1],
                "txtext" => {
                    let sub = unesc(&t[1..].join(" "));
                    lock(&self.board).tx_all.iter().any(|x| x.contains(&sub))
                }
                "refreshdone" => !c.refresh_active(),
                "txlines" => lock(&self.board).tx_total >= t[1].parse::<usize>().unwrap(),
                "idleprog" => !c.in_program() && c.queue_counts().queue == 0 && c.queue_counts().pending == 0 && self.com().pending() == 0,
                "inprogram" => c.in_program(),
                "quiet" => self.com().pending() == 0 && c.queue_counts().pending == 0,
                _ => false,
            };
            if ok {
                std::thread::sleep(Duration::from_millis(50));
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.out.push_str("  WAITFOR TIMEOUT\n");
    }

    fn do_async(&mut self, t: &[&str]) {
        let core = self.core().clone();
        let t: Vec<String> = t.iter().map(|s| s.to_string()).collect();
        self.async_t = Some(std::thread::spawn(move || -> Result<(), String> {
            match t[0].as_str() {
                "refreshconfig" => {
                    let cause = match t.get(1).map(String::as_str).unwrap_or("OnDialog") {
                        "OnConnect" => RefreshCause::OnConnect,
                        "OnRead" => RefreshCause::OnRead,
                        "OnExport" => RefreshCause::OnExport,
                        "OnImport" => RefreshCause::OnImport,
                        "OnWriteBegin" => RefreshCause::OnWriteBegin,
                        "OnWriteEnd" => RefreshCause::OnWriteEnd,
                        _ => RefreshCause::OnDialog,
                    };
                    core.refresh_config(cause).map_err(|e| ex_text(&e))
                }
                "refreshinfo" => core.refresh_machine_info().map_err(|e| ex_text(&e)),
                "writeconfig" => {
                    let l: Vec<GrblConfParam> = t[1..]
                        .iter()
                        .map(|s| {
                            let (k, v) = s.split_once('=').unwrap();
                            GrblConfParam::new(k.parse().unwrap(), v)
                        })
                        .collect();
                    core.write_config(&l).map_err(|e| ex_text(&e))
                }
                "wifi" => {
                    core.write_wifi_config(&t[1], &t[2]);
                    Ok(())
                }
                _ => panic!("bad async op"),
            }
        }));
    }

    fn do_join(&mut self) {
        let mut guard = 0;
        while !self.async_t.as_ref().unwrap().is_finished() && guard < 20000 {
            guard += 1;
            self.core().step_tx();
            self.flush();
            if !self.pump_one() {
                std::thread::sleep(Duration::from_millis(1));
            } else {
                self.flush();
            }
        }
        let r = self.async_t.take().unwrap().join().unwrap();
        if let Err(e) = r {
            self.out.push_str(&format!("  ASYNCERR {}\n", esc(&e)));
        }
    }

    fn dump(&mut self) {
        let c = self.core().clone();
        let q = c.queue_counts();
        let mut d = String::from("  DUMP");
        d += &format!(" status={}", c.machine_status());
        d += &format!(" inprogram={}", b(c.in_program()));
        d += &format!(" target={} sent={} executed={} errors={}", c.program_target(), c.program_sent(), c.program_executed(), c.program_error_count());
        d += &format!(" lastissue={}", c.last_issue());
        d += &format!(" used={} bsize={} free={}", c.used_buffer(), c.buffer_size(), c.free_buffer());
        d += &format!(" grblblock={} grblbuffer={}", c.grbl_block(), c.grbl_buffer());
        d += &format!(" ovcur={},{},{}", c.override_g1(), c.override_g0(), c.override_s());
        d += &format!(" ovtar={},{},{}", c.t_override_g1(), c.t_override_g0(), c.t_override_s());
        d += &format!(" mpos={} wco={} lastwco={}", pt(c.machine_position()), pt(c.working_offset()), pt(c.last_known_wco()));
        d += &format!(" queue={} pending={} retry={}", q.queue, q.pending, b(q.retry));
        d += &format!(" loop={}", fmt_decimal(c.loop_count()));
        d += &format!(" curf={} curs={}", num(c.current_f()), num(c.current_s()));
        d += &format!(" ver={}", c.grbl_version().map_or("none".to_string(), |v| v.to_string()));
        d += &format!(" ip={}", c.detected_ip().unwrap_or_else(|| "none".into()));
        d += &format!(" failed={}", c.failed_connection_count());
        d += &format!(" holduser={} holdcool={}", b(q.hold_by_user), b(q.hold_by_cooling));
        d += &format!(" connected={} stuck={}", b(c.is_connected()), b(c.is_buffer_stuck()));
        d += &format!(" canjog={}", b(c.jog_enabled()));
        d += &format!(" comopen={}", b(self.com().is_open()));
        self.out.push_str(&d);
        self.out.push('\n');
        let n = c.executed() as i64;
        for (i, row) in c.sent_command(0, n).iter().enumerate() {
            match row {
                SentRow::Command(g) => {
                    let st = match g.status() {
                        CommandStatus::Queued => "Queued",
                        CommandStatus::WaitingResponse => "WaitingResponse",
                        CommandStatus::ResponseGood => "ResponseGood",
                        CommandStatus::ResponseBad => "ResponseBad",
                        CommandStatus::InvalidResponse => "InvalidResponse",
                    };
                    let res = g.get_result(false, false).unwrap_or_else(|| "<null>".into());
                    self.out.push_str(&format!("  SENT[{i}] CMD {} | {} | {}\n", esc(g.command()), st, esc(&res)));
                }
                SentRow::Message(m) => self.out.push_str(&format!("  SENT[{i}] MSG {}\n", esc(&norm_time(&m.get_decoded_message())))),
            }
        }
    }

    fn dump_conf(&mut self) {
        let c = GrblCore::configuration();
        let mut l: Vec<(i32, String)> = c.iter().map(|(k, v)| (k, v.to_string())).collect();
        l.sort_by_key(|x| x.0);
        let mut d = format!("  CONF count={}", c.count());
        for (k, v) in l {
            d += &format!(" ${k}={v}");
        }
        self.out.push_str(&d);
        self.out.push('\n');
    }

    fn do_api(&mut self, verb: &str, rest: &str, t: &[&str]) -> Result<(), String> {
        let c = self.core().clone();
        let pointf = |x: &str, y: &str| PointF::new(x.parse().unwrap(), y.parse().unwrap());
        let speed = |s: &str| s.parse::<f32>().unwrap();
        match verb {
            "runprogram" => match c.run_program() {
                RunDecision::Started => {}
                RunDecision::NotAllowed => self.out.push_str("  NOTALLOWED\n"),
                RunDecision::AskResume(i) => self.out.push_str(&format!(
                    "  ASKRESUME executed={} sent={} target={} issue={} homingenabled={} sughoming={} sugwco={} lastwco={}\n",
                    i.executed,
                    i.sent,
                    i.target,
                    i.last_issue,
                    b(i.homing_enabled),
                    b(i.suggest_homing),
                    b(i.suggest_setwco),
                    pt(i.last_known_wco)
                )),
            },
            "runfromstart" => c.run_program_from_start_user(t.first() == Some(&"true")),
            "continue" => c.continue_program(t[0].parse().unwrap(), t[1] == "true", t[2] == "true").map_err(|e| e.to_string())?,
            "abort" => c.abort_program(),
            "feedhold" => c.feed_hold(t.first() == Some(&"cooling")),
            "resume" => c.cycle_start_resume(false),
            "safetydoor" => c.safety_door(),
            "grblreset" => c.grbl_reset(),
            "closecom" => c.close_com(t[0] == "user")?,
            "jogset" => {
                c.set_jog_speed(t[0].parse().unwrap());
                c.set_jog_step(parse_decimal(t[1]).unwrap());
            }
            "jog" => {
                let d = parse_dir(t[0]);
                let fast = t[1] == "fast";
                let r = if t.len() > 2 {
                    let step = parse_decimal(t[2]).unwrap();
                    if fast { c.jog_to_direction_step_fast(d, true, step) } else { c.jog_to_direction(d, speed(t[1]), step) }
                } else if fast {
                    c.jog_to_direction_fast(d, true)
                } else {
                    c.jog_to_direction(d, speed(t[1]), c.jog_step())
                };
                r.map_err(|_| "ArgumentException".to_string())?;
            }
            "jogpos" => {
                if t[2] == "fast" {
                    c.jog_to_position_fast(pointf(t[0], t[1]), true)
                } else {
                    c.jog_to_position(pointf(t[0], t[1]), speed(t[2]))
                }
            }
            "cjog" => c.continuous_jog_to_direction(parse_dir(t[0]), speed(t[1])).map_err(|_| "ArgumentException".to_string())?,
            "cjogpos" => c.continuous_jog_to_position(pointf(t[0], t[1]), speed(t[2])),
            "jogabort" => c.jog_abort(),
            "cjogabort" => c.continuous_jog_abort(),
            "hkov" => {
                let a = match t[0] {
                    "PowerDefault" => OverrideAction::PowerDefault,
                    "PowerUp" => OverrideAction::PowerUp,
                    "PowerDown" => OverrideAction::PowerDown,
                    "LinearDefault" => OverrideAction::LinearDefault,
                    "LinearUp" => OverrideAction::LinearUp,
                    "LinearDown" => OverrideAction::LinearDown,
                    "RapidDefault" => OverrideAction::RapidDefault,
                    "RapidUp" => OverrideAction::RapidUp,
                    _ => OverrideAction::RapidDown,
                };
                c.hot_key_override(a);
            }
            "tov" => {
                let v: i32 = t[1].parse().unwrap();
                match t[0] {
                    "g1" => c.set_t_override_g1(v),
                    "g0" => c.set_t_override_g0(v),
                    _ => c.set_t_override_s(v),
                }
            }
            "manageov" => c.manage_overrides(),
            "custom" => c.execute_custom_code(&unesc(rest)),
            "eval" => {
                let r = c.evaluate_expression(&unesc(rest));
                self.out.push_str(&format!("  EVAL {}\n", esc(&r)));
            }
            "unstuck" => c.unlock_from_buffer_stuck(t[0] == "auto"),
            "loopcount" => c.set_loop_count(parse_decimal(t[0]).unwrap()),
            "enqueue" => c.enqueue_command(&GrblCommand::new(&unesc(rest))),
            "homing" => c.grbl_homing(),
            "unlock" => c.grbl_unlock(),
            "zero" => c.set_new_zero(),
            "sendhoming" => c.send_homing_command(),
            "sendunlock" => c.send_unlock_command(),
            "imm" => c.send_immediate(u8::from_str_radix(t[0], 16).unwrap(), false),
            "newprog" => {
                let p = unesc(rest);
                self.load_program(&p);
            }
            _ => panic!("unknown verb {verb}"),
        }
        Ok(())
    }
}

/// "<.NET exception type> <message>" like the C# harness prints.
fn ex_text(e: &CoreError) -> String {
    let ty = match e {
        CoreError::Timeout(_) => "TimeoutException",
        CoreError::WriteConfig { .. } => "WriteConfigException",
        CoreError::Other(_) => "Exception",
    };
    format!("{ty} {e}")
}

fn parse_status(n: &str) -> MacStatus {
    MacStatus::parse(n).unwrap_or_else(|| panic!("bad status {n}"))
}

// ---------------------------------------------------------------------------------------------- test

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/protocol")
}

/// Index of the first differing line and both lines.
fn first_diff(a: &str, b: &str) -> String {
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    for i in 0..la.len().max(lb.len()) {
        let (x, y) = (la.get(i).copied().unwrap_or("<eof>"), lb.get(i).copied().unwrap_or("<eof>"));
        if x != y {
            let from = i.saturating_sub(3);
            let ctx: Vec<&str> = la[from..i.min(la.len())].to_vec();
            return format!("line {}:\n  context: {:?}\n  golden : {x}\n  rust   : {y}", i + 1, ctx);
        }
    }
    "identical".into()
}

#[test]
fn protocol_scenarios_match_the_original() {
    // first: both tests share the global data dir + settings; init must finish before either uses them
    lg_base::paths::isolate_for_tests("golden_protocol");
    let dir = golden_dir();
    let only = std::env::var("LG_PROTO_ONLY").ok();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("tests/golden/protocol")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".scn").map(str::to_string)))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no scenarios found");
    let scratch = lg_base::paths::test_dir("proto");
    let mut ip = Interp::new(&scratch);
    let mut bad = Vec::new();
    for n in &names {
        if only.as_ref().is_some_and(|o| o != n) {
            continue;
        }
        let scn = std::fs::read_to_string(dir.join(format!("{n}.scn"))).unwrap();
        let golden = std::fs::read_to_string(dir.join(format!("{n}.txt"))).unwrap();
        let got = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ip.run(&scn)));
        match got {
            Ok(got) if got == golden => {}
            Ok(got) => {
                let _ = std::fs::write(scratch.join(format!("{n}.rust.txt")), &got);
                bad.push(format!("{n}: {}", first_diff(&golden, &got)));
            }
            Err(_) => bad.push(format!("{n}: panic")),
        }
    }
    if !bad.is_empty() {
        panic!("{} scenario(s) differ (full Rust traces in {}, kept when LG_KEEP_TEST_DIRS=1):\n{}", bad.len(), scratch.display(), bad.join("\n---\n"));
    }
}

// ---------------------------------------------------------------------------------------------- emulator

/// "Time" of the emulated board for the `.emu` replay: short delays pass at once, a long one (homing, >= 1 s) blocks
/// until the script releases it. This makes the trace independent of wall clock and scheduler speed.
struct Gate {
    /// (released, threads parked in a long delay)
    st: Mutex<(bool, usize)>,
    cv: std::sync::Condvar,
}

impl Gate {
    /// New closed gate.
    fn new() -> Arc<Self> {
        Arc::new(Gate { st: Mutex::new((false, 0)), cv: std::sync::Condvar::new() })
    }

    /// Emulated delay. `d`: requested duration. Short ones return at once, long ones wait for [`Gate::release`].
    fn delay(&self, d: Duration) {
        if d < Duration::from_secs(1) {
            return;
        }
        let mut g = lock(&self.st);
        g.1 += 1;
        while !g.0 {
            g = self.cv.wait(g).unwrap_or_else(|e| e.into_inner());
        }
        g.1 -= 1;
    }

    /// True while a long delay is blocked (false from the moment of [`Gate::release`], before the thread wakes up).
    fn parked(&self) -> bool {
        let g = lock(&self.st);
        !g.0 && g.1 > 0
    }

    /// Let blocked and future long delays finish.
    fn release(&self) {
        lock(&self.st).0 = true;
        self.cv.notify_all();
    }
}

/// Wait until the emulator did everything it was asked to, or is blocked in a long delay. Event based: no sleep
/// time is assumed, the deadline only turns a hang into a failure.
///
/// `emu`: emulator. `gate`: its delay gate.
fn wait_settled(emu: &lasergrbl_core::emulator::Grblv11Emulator, gate: &Gate) {
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    while !(emu.is_idle() || gate.parked()) {
        assert!(std::time::Instant::now() < deadline, "emulator did not settle");
        std::thread::yield_now();
    }
}

/// Replay one `.emu` script on the Rust emulator (mirror of `EmuInterp` in the C# harness).
///
/// The C# original cut its output into per-command groups with fixed waits, so a slow command (`$H`, 2 s) showed up
/// in a later group. Here every command is followed by "wait until settled"; a long delay is held back by the
/// [`Gate`] until the end of the script, and what it prints then joins the last group, as in the golden trace.
///
/// `text`: script. `cfg_file`: scratch config file. Returns the trace.
fn run_emu(text: &str, cfg_file: &Path) -> String {
    use lasergrbl_core::emulator::{add_emulator_message_handler, clear_emulator_message_handlers, Grblv11Emulator};
    let outq: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let add = {
        let q = outq.clone();
        move |s: String| lock(&q).push(s)
    };
    clear_emulator_message_handlers();
    let a1 = add.clone();
    add_emulator_message_handler(Arc::new(move |m: Option<&str>| a1(format!("LOG {}", m.map_or("<null>".to_string(), esc)))));
    let a2 = add.clone();
    let _ = std::fs::remove_file(cfg_file);
    let gate = Gate::new();
    let g2 = gate.clone();
    let emu = Grblv11Emulator::with_sleeper(Box::new(move |m: &str| a2(format!("RX {}", esc(m)))), cfg_file.to_path_buf(), Box::new(move |d| g2.delay(d)));
    // (header, events): events of one group are compared sorted, threads race on the order inside a group
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    let collect = |groups: &mut Vec<(String, Vec<String>)>| {
        let ev = std::mem::take(&mut *lock(&outq));
        groups.last_mut().expect("group").1.extend(ev);
    };
    for raw in text.split('\n') {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        groups.push((format!("> {line}\n"), Vec::new()));
        let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
        match verb {
            "open" => emu.open_com(),
            "close" => emu.close_com(),
            "send" => emu.manage_message(unesc(rest).as_bytes()),
            "byte" => emu.manage_message(&[u8::from_str_radix(rest, 16).unwrap()]),
            // timing hints of the C# script: nothing to wait for, the replay waits for events
            "wait" | "settle" => {}
            _ => panic!("bad emu verb {verb}"),
        }
        wait_settled(&emu, &gate);
        collect(&mut groups);
    }
    // finish what a long delay held back; its output belongs to the last group
    gate.release();
    wait_settled(&emu, &gate);
    collect(&mut groups);
    emu.close_com();
    groups.push((String::new(), Vec::new()));
    wait_settled(&emu, &gate);
    collect(&mut groups);
    clear_emulator_message_handlers();
    let mut out = String::new();
    for (head, mut ev) in groups {
        // byte order like StringComparer.Ordinal
        ev.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        out.push_str(&head);
        for e in ev {
            out.push_str(&format!("  {e}\n"));
        }
    }
    out
}

#[test]
fn emulator_scenarios_match_the_original() {
    // first: both tests share the global data dir + settings; init must finish before either uses them
    lg_base::paths::isolate_for_tests("golden_protocol");
    let dir = golden_dir();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".emu").map(str::to_string)))
        .collect();
    names.sort();
    assert!(!names.is_empty());
    let scratch = lg_base::paths::test_dir("emu");
    let mut bad = Vec::new();
    for n in &names {
        let src = std::fs::read_to_string(dir.join(format!("{n}.emu"))).unwrap();
        let golden = std::fs::read_to_string(dir.join(format!("{n}.emu.txt"))).unwrap();
        let got = run_emu(&src, &scratch.join(format!("{n}.json")));
        if got != golden {
            let _ = std::fs::write(scratch.join(format!("{n}.rust.txt")), &got);
            bad.push(format!("{n}: {}", first_diff(&golden, &got)));
        }
    }
    if !bad.is_empty() {
        panic!("{} emulator scenario(s) differ (traces in {}, kept when LG_KEEP_TEST_DIRS=1):\n{}", bad.len(), scratch.display(), bad.join("\n---\n"));
    }
}
