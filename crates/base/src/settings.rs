// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Settings.cs`: global key/value store with debounced save.
//!
//! Differences to C#: values are `serde_json::Value` in `LaserGRBL.Settings.json` (enums by variant
//! name); the Windows `LaserGRBL.Settings.bin` (BinaryFormatter) is imported once when no JSON exists.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Condvar, Mutex, MutexGuard, Once};
use std::time::{Duration, Instant};

/// Current app version written to the settings file (C# `Program.CurrentVersion`).
pub const CURRENT_VERSION: (i32, i32, i32) = (7, 14, 1);
const VERSION_KEY: &str = "Current LaserGRBL Version";
const SAVE_DELAY: Duration = Duration::from_secs(2);

/// Rendering backend (C# `Settings.GraphicMode`). Stored by variant name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GraphicMode {
    #[default]
    Auto = 0,
    Gdi = 1,
    Dib = 2,
    Fbo = 3,
}

impl GraphicMode {
    /// C# `(GraphicMode)int`; unknown value gives `Auto`.
    ///
    /// `v`: integer value. Returns the mode.
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Gdi,
            2 => Self::Dib,
            3 => Self::Fbo,
            _ => Self::Auto,
        }
    }
}

struct State {
    dic: BTreeMap<String, Value>,
    path: PathBuf,
    last_cause: Option<String>,
    deadline: Option<Instant>,
    is_new_file: bool,
    prev_version: (i32, i32, i32),
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static CV: Condvar = Condvar::new();
static WORKER: Once = Once::new();
static GRAPHIC: Mutex<(GraphicMode, GraphicMode)> = Mutex::new((GraphicMode::Auto, GraphicMode::Auto));

/// Default settings file: `data_path()/LaserGRBL.Settings.json`.
fn default_file() -> PathBuf {
    crate::paths::data_path().join("LaserGRBL.Settings.json")
}

/// Parse "a.b.c[.d]" into a triple.
///
/// `s`: version text. Returns None when not parseable.
fn parse_version(s: &str) -> Option<(i32, i32, i32)> {
    let mut it = s.split('.').map(|p| p.trim().parse::<i32>());
    let a = it.next()?.ok()?;
    let b = it.next()?.ok()?;
    let c = it.next().transpose().ok()?.unwrap_or(0);
    Some((a, b, c))
}

/// Build state from file (C# static ctor).
///
/// `path`: JSON file to use. Returns the ready state.
fn load_state(path: PathBuf) -> State {
    let mut dic: BTreeMap<String, Value> = BTreeMap::new();
    let mut is_new_file = !path.exists();
    if !is_new_file {
        // existing file: parse JSON, ignore errors like C# catch{}
        if let Ok(txt) = std::fs::read_to_string(&path)
            && let Ok(m) = serde_json::from_str::<BTreeMap<String, Value>>(&txt)
        {
            dic = m;
        }
    } else {
        // no JSON: try Windows BinaryFormatter file sitting next to it
        let bin = path.with_file_name("LaserGRBL.Settings.bin");
        if bin.exists()
            && let Ok(m) = crate::nrbf::import_settings_bin(&bin)
        {
            dic = m;
            is_new_file = false;
        }
    }
    let prev_version = dic
        .get(VERSION_KEY)
        .and_then(|v| v.as_str())
        .and_then(parse_version)
        .unwrap_or((0, 0, 0));
    let mut st = State { dic, path, last_cause: None, deadline: None, is_new_file, prev_version };
    // C# ctor: SetObject(current version)
    let (a, b, c) = CURRENT_VERSION;
    set_in(&mut st, VERSION_KEY, Value::String(format!("{a}.{b}.{c}")), false);
    st
}

/// Lock the store, init lazily, start the save worker.
///
/// Returns the guard; inner Option is always Some.
fn lock() -> MutexGuard<'static, Option<State>> {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        *g = Some(load_state(default_file()));
    }
    WORKER.call_once(|| {
        let _ = std::thread::Builder::new().name("SettingsSaver".into()).spawn(worker);
    });
    g
}

/// Background debounce thread: saves when deadline passes.
fn worker() {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        let dl = g.as_ref().and_then(|s| s.deadline);
        match dl {
            None => g = CV.wait(g).unwrap_or_else(|e| e.into_inner()),
            Some(d) => {
                let now = Instant::now();
                if now >= d {
                    if let Some(s) = g.as_mut() {
                        save_now(s);
                    }
                } else {
                    g = CV.wait_timeout(g, d - now).unwrap_or_else(|e| e.into_inner()).0;
                }
            }
        }
    }
}

/// Write file if there is a pending cause (C# `InternalSaveFile`).
///
/// `s`: state. Clears timer and cause.
fn save_now(s: &mut State) {
    s.deadline = None;
    if s.last_cause.is_some() {
        let txt = serde_json::to_string_pretty(&s.dic).unwrap_or_default();
        let tmp = s.path.with_extension("json.tmp");
        // write tmp then rename = atomic; errors ignored like C#
        if write_private(&tmp, txt.as_bytes()).is_ok() {
            let _ = std::fs::rename(&tmp, &s.path);
        }
    }
    s.last_cause = None;
}

/// Write file with 0600 perms on unix.
///
/// `p`: target. `data`: bytes. Returns io result.
fn write_private(p: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(p)?.write_all(data)
}

/// Arm debounce timer.
///
/// `s`: state. `cause`: key that changed.
fn trigger_save(s: &mut State, cause: &str) {
    s.last_cause = Some(cause.to_string());
    s.deadline = Some(Instant::now() + SAVE_DELAY);
    CV.notify_all();
}

/// Core of `SetObject`.
///
/// `s`: state. `key`: key. `v`: new value. `force`: save even if equal.
fn set_in(s: &mut State, key: &str, v: Value, force: bool) {
    // different? new keys always trigger
    let changed = s.dic.get(key).is_none_or(|old| *old != v);
    let existed = s.dic.contains_key(key);
    s.dic.insert(key.to_string(), v);
    if changed || force || !existed {
        trigger_save(s, key);
    }
}

/// `GetObject<T>`: typed read.
///
/// `key`: setting key. `default`: returned if missing, null or not convertible to `T`.
/// Returns the stored value as `T` or `default`.
pub fn get<T: DeserializeOwned>(key: &str, default: T) -> T {
    let g = lock();
    g.as_ref()
        .and_then(|s| s.dic.get(key))
        .filter(|v| !v.is_null())
        .and_then(|v| serde_json::from_value::<T>(v.clone()).ok())
        .unwrap_or(default)
}

/// `SetObject`: store value, debounced save (2 s) when it changed.
///
/// `key`: setting key. `value`: any serializable value.
pub fn set<T: Serialize>(key: &str, value: T) {
    set_impl(key, value, false);
}

/// `SetObject(.., forcesave: true)`: save even if value equals old one.
///
/// `key`: setting key. `value`: any serializable value.
pub fn set_force_save<T: Serialize>(key: &str, value: T) {
    set_impl(key, value, true);
}

/// Shared body of `set`.
///
/// `key`/`value`: as `set`. `force`: force save.
fn set_impl<T: Serialize>(key: &str, value: T, force: bool) {
    let Ok(v) = serde_json::to_value(value) else { return };
    let mut g = lock();
    if let Some(s) = g.as_mut() {
        set_in(s, key, v, force);
    }
}

/// `ExistObject`.
///
/// `key`: setting key. Returns true when present.
pub fn exists(key: &str) -> bool {
    lock().as_ref().is_some_and(|s| s.dic.contains_key(key))
}

/// `DeleteObject`: remove key, trigger save when it existed.
///
/// `key`: setting key.
pub fn delete(key: &str) {
    let mut g = lock();
    if let Some(s) = g.as_mut()
        && s.dic.remove(key).is_some()
    {
        trigger_save(s, key);
    }
}

/// `GetAndDeleteObject`: read then delete.
///
/// `key`: setting key. `default`: returned when missing or not convertible.
/// Returns the value (or default).
pub fn get_and_delete<T: DeserializeOwned>(key: &str, default: T) -> T {
    let rv = get(key, default);
    delete(key);
    rv
}

/// `Settings.Exiting`: flush pending changes now.
pub fn exiting() {
    let mut g = lock();
    if let Some(s) = g.as_mut() {
        save_now(s);
    }
}

/// True when no settings file existed at startup.
///
/// Returns the C# `IsNewFile` flag.
pub fn is_new_file() -> bool {
    lock().as_ref().is_some_and(|s| s.is_new_file)
}

/// Version stored by the previous run, (0,0,0) if none.
///
/// Returns (major, minor, build).
pub fn prev_version() -> (i32, i32, i32) {
    lock().as_ref().map_or((0, 0, 0), |s| s.prev_version)
}

/// Use a specific settings file and reset the store (tests, `--datadir`).
///
/// `path`: JSON file path (parent dir is created). Pending saves of the old store are flushed first.
pub fn init_at(path: PathBuf) {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    // flush old store so no data is lost
    if let Some(s) = g.as_mut() {
        save_now(s);
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    *g = Some(load_state(path));
    CV.notify_all();
}

/// Mode forced by the command line.
///
/// Returns the forced mode (default `Auto`).
pub fn forced_graphic_mode() -> GraphicMode {
    GRAPHIC.lock().unwrap_or_else(|e| e.into_inner()).0
}

/// Set the command-line forced mode.
///
/// `m`: mode.
pub fn set_forced_graphic_mode(m: GraphicMode) {
    GRAPHIC.lock().unwrap_or_else(|e| e.into_inner()).0 = m;
}

/// Mode stored in settings ("ConfiguredGraphicMode", int).
///
/// Returns the mode.
pub fn configured_graphic_mode() -> GraphicMode {
    GraphicMode::from_i32(get("ConfiguredGraphicMode", 0i32))
}

/// Store the configured mode as int.
///
/// `m`: mode.
pub fn set_configured_graphic_mode(m: GraphicMode) {
    set("ConfiguredGraphicMode", m as i32);
}

/// Forced mode if not `Auto`, else configured one.
///
/// Returns the requested mode.
pub fn requested_graphic_mode() -> GraphicMode {
    match forced_graphic_mode() {
        GraphicMode::Auto => configured_graphic_mode(),
        m => m,
    }
}

/// Mode actually in use.
///
/// Returns the current mode (default `Auto`).
pub fn current_graphic_mode() -> GraphicMode {
    GRAPHIC.lock().unwrap_or_else(|e| e.into_inner()).1
}

/// Set the mode actually in use.
///
/// `m`: mode.
pub fn set_current_graphic_mode(m: GraphicMode) {
    GRAPHIC.lock().unwrap_or_else(|e| e.into_inner()).1 = m;
}
