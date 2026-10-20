//! Port of `LaserLifeHandler` (+ `LaserLifeCounter`, `ListLLC`) from `Core/GrblCore.cs`.
//!
//! Differences to C#:
//! * Persisted as JSON `LaserLifeCounter.json` in the data dir (C#: BinaryFormatter `.bin`) with the same
//!   `.old` backup rotation.
//! * `RealDoSend` never uploads (PLAN section 6); `build_json` / `escape_json` are kept for completeness.
//! * `OnConnect(parent)` opened `LaserSelector`; here [`on_connect`] returns the alive counters when the UI
//!   must ask, and the UI then calls [`select`].
//! * The handler is a process-wide static like the C# static class.

use crate::grbl_core::MacStatus;
use crate::util::new_guid;
use chrono::{Local, NaiveDateTime, NaiveTime};
use lg_base::paths::data_path;
use lg_base::tools::hires_timer;
use lg_dotnet::{fmt_double_custom, TimeSpan};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, MutexGuard};

/// C# `DEF_NAME`.
pub const DEF_NAME: &str = "Default";
/// C# `DEF_BRAND`.
pub const DEF_BRAND: &str = "Unknown";
/// C# `DEF_MODEL`.
pub const DEF_MODEL: &str = "Unknown";

const DATE_FMT: &str = "%Y-%m-%dT%H:%M:%S";
const JSON_DATE_FMT: &str = "%Y-%m-%d %H:%M:%S";

/// `DateTime.Today` (local midnight).
fn today() -> NaiveDateTime {
    Local::now().date_naive().and_time(NaiveTime::MIN)
}

/// Usage record of one laser module (C# `LaserLifeCounter`).
#[derive(Debug, Clone, PartialEq)]
pub struct LaserLifeCounter {
    guid: String,
    pub name: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub optical_power: Option<f64>,
    pub purchase_date: Option<NaiveDateTime>,
    pub monitoring_date: Option<NaiveDateTime>,
    pub death_date: Option<NaiveDateTime>,
    pub last_usage: Option<NaiveDateTime>,
    time_in_run: TimeSpan,
    time_usage_normalized_power: TimeSpan,
    time_usage_non_zero: TimeSpan,
    time_classes: [TimeSpan; 10],
}

impl LaserLifeCounter {
    /// Private C# ctor: new guid, monitoring starts today.
    fn blank() -> Self {
        Self {
            guid: new_guid(),
            name: None,
            brand: None,
            model: None,
            optical_power: None,
            purchase_date: None,
            monitoring_date: Some(today()),
            death_date: None,
            last_usage: None,
            time_in_run: TimeSpan::ZERO,
            time_usage_normalized_power: TimeSpan::ZERO,
            time_usage_non_zero: TimeSpan::ZERO,
            time_classes: [TimeSpan::ZERO; 10],
        }
    }

    /// `CreateNew()`: empty counter for the "add laser" dialog.
    pub fn create_new() -> Self {
        Self::blank()
    }

    /// `CreateDefault()`: counter named "Default".
    pub fn create_default() -> Self {
        let mut c = Self::blank();
        c.name = Some(DEF_NAME.to_string());
        c
    }

    /// `Guid`.
    pub fn guid(&self) -> &str {
        &self.guid
    }
    /// `TimeInRun`.
    pub fn time_in_run(&self) -> TimeSpan {
        self.time_in_run
    }
    /// `TimeUsageNormalizedPower`.
    pub fn time_usage_normalized_power(&self) -> TimeSpan {
        self.time_usage_normalized_power
    }
    /// `StressTime` (class 9, 91-100 % power).
    pub fn stress_time(&self) -> TimeSpan {
        self.time_classes[9]
    }
    /// `Classes`: ten power classes (1-10 %, 11-20 %, ...).
    pub fn classes(&self) -> &[TimeSpan; 10] {
        &self.time_classes
    }
    /// `AveragePowerFactor`.
    pub fn average_power_factor(&self) -> f64 {
        if self.time_usage_non_zero.total_seconds() == 0.0 { 0.0 } else { self.time_usage_normalized_power.total_seconds() / self.time_usage_non_zero.total_seconds() }
    }

    /// `AddRunTime`.
    ///
    /// `elapsed`: time spent in Run state.
    pub fn add_run_time(&mut self, elapsed: TimeSpan) {
        self.time_in_run = self.time_in_run + elapsed;
        self.last_usage = Some(today());
    }

    /// `AddTrueLaserTimePower`.
    ///
    /// `elapsed`: time slice. `powerperc`: laser power 0..1 during the slice.
    pub fn add_true_laser_time_power(&mut self, elapsed: TimeSpan, powerperc: f64) {
        // do not track any usage under 3% (framing etc)
        if powerperc > 0.03 {
            let normalized = TimeSpan::from_milliseconds(elapsed.total_milliseconds() * powerperc);
            self.time_usage_normalized_power = self.time_usage_normalized_power + normalized;
            self.time_usage_non_zero = self.time_usage_non_zero + elapsed;
        }
        if powerperc > 0.01 {
            // 1-10 = 0, 11-20 = 1; 91-100 = 9
            let clx = ((powerperc * 100.0 - 1.0) / 10.0).floor() as i32;
            let clx = clx.clamp(0, 9) as usize;
            self.time_classes[clx] = self.time_classes[clx] + elapsed;
        }
        self.last_usage = Some(today());
    }

    /// `Update(selected)`: copy the user editable fields.
    ///
    /// `selected`: edited copy.
    pub fn update_from(&mut self, selected: &LaserLifeCounter) {
        self.name = selected.name.clone();
        self.purchase_date = selected.purchase_date;
        self.death_date = selected.death_date;
        self.model = selected.model.clone();
        self.brand = selected.brand.clone();
        self.optical_power = selected.optical_power;
    }

    /// `HasWorked`: more than one hour in run.
    pub fn has_worked(&self) -> bool {
        self.time_in_run.total_hours() > 1.0
    }
}

/// C# `ListLLC`: counters + send bookkeeping.
#[derive(Debug, Clone, PartialEq)]
pub struct ListLlc {
    pub counters: Vec<LaserLifeCounter>,
    /// 2000-01-01 by default.
    pub last_attempt: NaiveDateTime,
    /// 2000-01-01 by default.
    pub last_sent: NaiveDateTime,
}

fn y2000() -> NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap_or_default().and_time(NaiveTime::MIN)
}

impl Default for ListLlc {
    /// Empty list, dates at 2000-01-01.
    fn default() -> Self {
        Self { counters: Vec::new(), last_attempt: y2000(), last_sent: y2000() }
    }
}

#[derive(Serialize, Deserialize)]
struct CounterDto {
    guid: String,
    name: Option<String>,
    brand: Option<String>,
    model: Option<String>,
    optical_power: Option<f64>,
    purchase_date: Option<String>,
    monitoring_date: Option<String>,
    death_date: Option<String>,
    last_usage: Option<String>,
    time_in_run_ticks: i64,
    time_usage_normalized_power_ticks: i64,
    time_usage_non_zero_ticks: i64,
    #[serde(default)]
    time_classes_ticks: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
struct ListDto {
    last_attempt: String,
    last_sent: String,
    counters: Vec<CounterDto>,
}

fn dt_out(d: &Option<NaiveDateTime>) -> Option<String> {
    d.map(|x| x.format(DATE_FMT).to_string())
}

fn dt_in(s: &Option<String>) -> Option<NaiveDateTime> {
    s.as_deref().and_then(|t| NaiveDateTime::parse_from_str(t, DATE_FMT).ok())
}

impl ListLlc {
    /// Serialize to the JSON file text.
    pub fn to_json(&self) -> String {
        let dto = ListDto {
            last_attempt: self.last_attempt.format(DATE_FMT).to_string(),
            last_sent: self.last_sent.format(DATE_FMT).to_string(),
            counters: self
                .counters
                .iter()
                .map(|c| CounterDto {
                    guid: c.guid.clone(),
                    name: c.name.clone(),
                    brand: c.brand.clone(),
                    model: c.model.clone(),
                    optical_power: c.optical_power,
                    purchase_date: dt_out(&c.purchase_date),
                    monitoring_date: dt_out(&c.monitoring_date),
                    death_date: dt_out(&c.death_date),
                    last_usage: dt_out(&c.last_usage),
                    time_in_run_ticks: c.time_in_run.ticks(),
                    time_usage_normalized_power_ticks: c.time_usage_normalized_power.ticks(),
                    time_usage_non_zero_ticks: c.time_usage_non_zero.ticks(),
                    time_classes_ticks: c.time_classes.iter().map(|t| t.ticks()).collect(),
                })
                .collect(),
        };
        serde_json::to_string_pretty(&dto).unwrap_or_default()
    }

    /// Parse the JSON file text. Returns None when it is not valid.
    ///
    /// `text`: file content.
    pub fn from_json(text: &str) -> Option<ListLlc> {
        let dto: ListDto = serde_json::from_str(text).ok()?;
        let mut list = ListLlc {
            counters: Vec::new(),
            last_attempt: NaiveDateTime::parse_from_str(&dto.last_attempt, DATE_FMT).unwrap_or_else(|_| y2000()),
            last_sent: NaiveDateTime::parse_from_str(&dto.last_sent, DATE_FMT).unwrap_or_else(|_| y2000()),
        };
        for d in dto.counters {
            let mut classes = [TimeSpan::ZERO; 10];
            // C# OnDeserialization: missing array -> zeros
            for (i, t) in d.time_classes_ticks.iter().take(10).enumerate() {
                classes[i] = TimeSpan::from_ticks(*t);
            }
            list.counters.push(LaserLifeCounter {
                guid: d.guid,
                name: d.name,
                brand: d.brand,
                model: d.model,
                optical_power: d.optical_power,
                purchase_date: dt_in(&d.purchase_date),
                monitoring_date: dt_in(&d.monitoring_date),
                death_date: dt_in(&d.death_date),
                last_usage: dt_in(&d.last_usage),
                time_in_run: TimeSpan::from_ticks(d.time_in_run_ticks),
                time_usage_normalized_power: TimeSpan::from_ticks(d.time_usage_normalized_power_ticks),
                time_usage_non_zero: TimeSpan::from_ticks(d.time_usage_non_zero_ticks),
                time_classes: classes,
            });
        }
        Some(list)
    }
}

/// Answer of [`on_connect`].
#[derive(Debug, Clone, PartialEq)]
pub enum OnConnectResult {
    /// A counter was chosen automatically (or none exists); `Some(guid)` of the active one.
    Selected(Option<String>),
    /// The UI must show `LaserSelector` with these alive counters and call [`select`] with the choice.
    NeedSelection(Vec<LaserLifeCounter>),
}

struct Handler {
    list: ListLlc,
    current: Option<String>,
    last_status_nano: Option<i64>,
    last_power_nano: Option<i64>,
    last_save: i64,
    file: PathBuf,
}

fn load_list(file: &PathBuf) -> ListLlc {
    let read = |p: &PathBuf| std::fs::read_to_string(p).ok().and_then(|t| ListLlc::from_json(&t));
    let old = PathBuf::from(format!("{}.old", file.display()));
    let mut list = read(file).or_else(|| read(&old)).unwrap_or_default();
    if list.counters.is_empty() {
        list.counters.push(LaserLifeCounter::create_default());
    }
    list
}

static HANDLER: LazyLock<Mutex<Handler>> = LazyLock::new(|| {
    let file = data_path().join("LaserLifeCounter.json");
    Mutex::new(Handler { list: load_list(&file), current: None, last_status_nano: None, last_power_nano: None, last_save: 0, file })
});

fn h() -> MutexGuard<'static, Handler> {
    HANDLER.lock().unwrap_or_else(|e| e.into_inner())
}

/// Use another storage file and reload (tests, `--datadir`).
///
/// `file`: JSON path; loaded now (default list when missing).
pub fn reload_from(file: PathBuf) {
    let mut g = h();
    g.list = load_list(&file);
    g.file = file;
    g.current = None;
    g.last_status_nano = None;
    g.last_power_nano = None;
    g.last_save = 0;
}

/// `ComputeLaserTime(status)`: add run time to the current counter.
///
/// `status`: machine status that was active until now (Run counts as full power).
pub fn compute_laser_time(status: MacStatus) {
    {
        let mut g = h();
        let now = hires_timer::total_nano();
        if g.last_status_nano.is_some() {
            // NOTE(upstream): delta is measured from the last *power* sample
            let delta = now - g.last_power_nano.unwrap_or(now);
            if delta > 0 && delta < 600_000_000_000 {
                let perc = if status == MacStatus::Run { 1.0 } else { 0.0 };
                let normal = delta as f64 * perc;
                let cur = g.current.clone();
                if let Some(c) = cur.and_then(|guid| g.list.counters.iter_mut().find(|c| c.guid == guid)) {
                    c.add_run_time(TimeSpan::from_milliseconds(normal / 1000.0 / 1000.0));
                }
            }
        }
        g.last_status_nano = Some(now);
    }
    send_and_save();
}

/// `ComputeLaserTrueTime(power)`: add power-weighted time to the current counter.
///
/// `power`: current spindle value (`S`).
pub fn compute_laser_true_time(power: f32) {
    {
        let mut g = h();
        let now = hires_timer::total_nano();
        if let Some(last) = g.last_power_nano {
            let delta = now - last;
            if delta > 0 && delta < 600_000_000_000 {
                let maxpwm = lg_dotnet::decimal_to_f64(lg_gcode::configuration::configuration().max_pwm());
                if (10.0..=100_000.0).contains(&maxpwm) {
                    let powerperc = (power as f64 / maxpwm).clamp(0.0, 1.0);
                    let cur = g.current.clone();
                    if let Some(c) = cur.and_then(|guid| g.list.counters.iter_mut().find(|c| c.guid == guid)) {
                        c.add_true_laser_time_power(TimeSpan::from_milliseconds(delta as f64 / 1_000_000.0), powerperc);
                    }
                }
            }
        }
        g.last_power_nano = Some(now);
    }
    send_and_save();
}

/// `OnConnect`: pick the counter of the connected laser.
///
/// Returns [`OnConnectResult::Selected`] when decided (one counter, or exactly one alive), otherwise
/// [`OnConnectResult::NeedSelection`]. NOTE(upstream): with zero counters C# falls into the selector too.
pub fn on_connect() -> OnConnectResult {
    let mut g = h();
    let alive: Vec<LaserLifeCounter> = g.list.counters.iter().filter(|l| l.death_date.is_none()).cloned().collect();
    if g.list.counters.len() == 1 {
        let guid = g.list.counters[0].guid.clone();
        g.current = Some(guid.clone());
        lg_base::settings::set("Last laser used", Some(guid.clone()));
        OnConnectResult::Selected(Some(guid))
    } else if alive.len() == 1 {
        let guid = alive[0].guid.clone();
        g.current = Some(guid.clone());
        lg_base::settings::set("Last laser used", Some(guid.clone()));
        OnConnectResult::Selected(Some(guid))
    } else {
        g.current = None;
        OnConnectResult::NeedSelection(alive)
    }
}

/// Continuation of [`on_connect`] after the UI dialog.
///
/// `guid`: chosen counter; unknown/None leaves no active counter (C# `First()` throws, caught -> null).
pub fn select(guid: Option<&str>) {
    let mut g = h();
    g.current = guid.filter(|id| g.list.counters.iter().any(|c| c.guid == *id)).map(str::to_string);
    let cur = g.current.clone();
    lg_base::settings::set("Last laser used", cur);
}

/// `OnDisconnect`.
pub fn on_disconnect() {
    h().current = None;
}

/// Guid of the active counter.
pub fn current_guid() -> Option<String> {
    h().current.clone()
}

/// `GetCurrentTime()`: run time of the active counter.
pub fn get_current_time() -> TimeSpan {
    let g = h();
    g.current.as_ref().and_then(|id| g.list.counters.iter().find(|c| c.guid == *id)).map(|c| c.time_in_run).unwrap_or(TimeSpan::ZERO)
}

/// `GetListClone()`.
pub fn get_list_clone() -> ListLlc {
    h().list.clone()
}

/// `DoSend` + `DoSave`. The send part is intentionally empty (no remote telemetry).
fn send_and_save() {
    do_save();
}

/// `DoSave`: every 20 s snapshot and write on a worker thread.
fn do_save() {
    let (snapshot, file) = {
        let mut g = h();
        let now = hires_timer::total_milliseconds();
        if g.last_save == 0 || now - g.last_save > 20_000 {
            g.last_save = now;
            (Some(g.list.clone()), g.file.clone())
        } else {
            (None, g.file.clone())
        }
    };
    if let Some(list) = snapshot {
        let _ = std::thread::Builder::new().name("LLCSave".into()).spawn(move || real_do_save(&list, &file));
    }
}

/// `SaveNow`: synchronous save (used on exit and after every edit).
pub fn save_now() {
    let (list, file) = {
        let g = h();
        (g.list.clone(), g.file.clone())
    };
    real_do_save(&list, &file);
}

/// `RealDoSave`: rotate to `.old`, write new file. Errors are ignored like C#.
///
/// `list`: data to write. `file`: target path.
fn real_do_save(list: &ListLlc, file: &PathBuf) {
    static SAVE_LOCK: Mutex<()> = Mutex::new(());
    let _g = SAVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let old = PathBuf::from(format!("{}.old", file.display()));
    if file.exists() {
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::rename(file, &old);
    }
    let _ = std::fs::remove_file(file);
    let _ = std::fs::write(file, list.to_json());
}

/// `Add`.
///
/// `llc`: new counter.
pub fn add(llc: LaserLifeCounter) {
    h().list.counters.push(llc);
    save_now();
}

/// `Edit`: copy the editable fields into the counter with the same guid.
///
/// `selected`: edited counter.
pub fn edit(selected: &LaserLifeCounter) {
    for llc in h().list.counters.iter_mut() {
        if llc.guid == selected.guid {
            llc.update_from(selected);
        }
    }
    save_now();
}

/// `Delete`: refuses the active counter.
///
/// `selected`: counter to remove. Returns the error text or None.
pub fn delete(selected: &LaserLifeCounter) -> Option<String> {
    {
        let mut g = h();
        if g.current.as_deref() == Some(selected.guid.as_str()) {
            return Some("You cannot delete currently connected Laser module!".to_string());
        }
        g.list.counters.retain(|l| l.guid != selected.guid);
    }
    save_now();
    None
}

/// `Death`: mark as dead today.
///
/// `selected`: counter.
pub fn death(selected: &LaserLifeCounter) {
    for llc in h().list.counters.iter_mut() {
        if llc.guid == selected.guid && llc.death_date.is_none() {
            llc.death_date = Some(today());
        }
    }
    save_now();
}

/// `UnDeath`: clear the death date.
///
/// `selected`: counter.
pub fn undeath(selected: &LaserLifeCounter) {
    for llc in h().list.counters.iter_mut() {
        if llc.guid == selected.guid && llc.death_date.is_some() {
            llc.death_date = None;
        }
    }
    save_now();
}

/// `EscapeJson` for strings (`JavaScriptStringEncode(s, true)`).
///
/// `s`: text or None (= ""). Returns the quoted, escaped literal.
pub fn escape_json_str(s: Option<&str>) -> String {
    let mut out = String::from("\"");
    for c in s.unwrap_or("").chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\'' => out.push_str("\\'"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `EscapeJson` for doubles: quoted `0.000`.
///
/// `v`: value (None = 0).
pub fn escape_json_double(v: Option<f64>) -> String {
    format!("\"{}\"", fmt_double_custom(v.unwrap_or(0.0), "0.000"))
}

/// `BuildJson`: payload the C# code posted to the statistics server (never sent by this port).
///
/// `tosend`: counters. Returns the JSON-ish text with the exact C# spacing.
pub fn build_json(tosend: &[LaserLifeCounter]) -> String {
    let date = |d: &Option<NaiveDateTime>| escape_json_str(d.map(|x| x.format(JSON_DATE_FMT).to_string()).as_deref());
    let mut sb = String::from("[ ");
    for (i, llc) in tosend.iter().enumerate() {
        sb.push_str("{ ");
        sb.push_str(&format!("\"Guid\": {},", escape_json_str(Some(&llc.guid))));
        sb.push_str(&format!("\"Name\": {},", escape_json_str(llc.name.as_deref())));
        sb.push_str(&format!("\"Brand\": {},", escape_json_str(llc.brand.as_deref())));
        sb.push_str(&format!("\"Model\": {},", escape_json_str(llc.model.as_deref())));
        sb.push_str(&format!("\"OpticalPower\": {},", escape_json_double(llc.optical_power)));
        sb.push_str(&format!("\"PurchaseDate\": {},", date(&llc.purchase_date)));
        sb.push_str(&format!("\"MonitoringDate\": {},", date(&llc.monitoring_date)));
        sb.push_str(&format!("\"DeathDate\": {},", date(&llc.death_date)));
        sb.push_str(&format!("\"LastUsage\": {},", date(&llc.last_usage)));
        sb.push_str(&format!("\"TimeInRun\": {},", escape_json_double(Some(llc.time_in_run.total_hours()))));
        sb.push_str(&format!("\"TimeUsageNormalizedPower\": {},", escape_json_double(Some(llc.time_usage_normalized_power.total_hours()))));
        sb.push_str(&format!("\"AveragePowerFactor\": {},", escape_json_double(Some(llc.average_power_factor()))));
        sb.push_str("\"Classes\": ");
        sb.push_str("{ ");
        let n = llc.time_classes.len();
        for (j, c) in llc.time_classes.iter().enumerate() {
            sb.push_str(&fmt_double_custom(c.total_hours(), "0.000"));
            sb.push_str(if j == n - 1 { " } " } else { ", " });
        }
        sb.push_str(if i == tosend.len() - 1 { " } " } else { " }, " });
    }
    sb.push_str(" ]");
    sb
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lg_llc_{}_{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("LaserLifeCounter.json")
    }

    /// Handler state is process wide: tests that use it run in one test fn.
    #[test]
    fn handler_flow() {
        let file = tmp("flow");
        reload_from(file.clone());
        // default counter is created
        let l = get_list_clone();
        assert_eq!(l.counters.len(), 1);
        assert_eq!(l.counters[0].name.as_deref(), Some("Default"));
        // single counter selected on connect
        let guid = l.counters[0].guid().to_string();
        assert_eq!(on_connect(), OnConnectResult::Selected(Some(guid.clone())));
        assert_eq!(current_guid().as_deref(), Some(guid.as_str()));
        assert!(delete(&l.counters[0]).is_some());
        // second counter: two alive -> UI must choose
        let mut second = LaserLifeCounter::create_new();
        second.name = Some("Second".into());
        add(second.clone());
        on_disconnect();
        match on_connect() {
            OnConnectResult::NeedSelection(v) => assert_eq!(v.len(), 2),
            other => panic!("{other:?}"),
        }
        select(Some(second.guid()));
        assert_eq!(current_guid().as_deref(), Some(second.guid()));
        // one dies -> the other connects without asking
        death(&second);
        on_disconnect();
        assert_eq!(on_connect(), OnConnectResult::Selected(Some(guid.clone())));
        undeath(&second);
        // accounting
        {
            let mut g = h();
            let c = g.list.counters.iter_mut().find(|c| c.guid == guid).unwrap();
            c.add_run_time(TimeSpan::from_hours(2.0));
            c.add_true_laser_time_power(TimeSpan::from_seconds(10.0), 1.0);
            c.add_true_laser_time_power(TimeSpan::from_seconds(10.0), 0.5);
            c.add_true_laser_time_power(TimeSpan::from_seconds(10.0), 0.02);
            c.add_true_laser_time_power(TimeSpan::from_seconds(10.0), 0.005);
        }
        assert_eq!(get_current_time(), TimeSpan::from_hours(2.0));
        let l = get_list_clone();
        let c = l.counters.iter().find(|c| c.guid() == guid).unwrap();
        assert!(c.has_worked());
        assert_eq!(c.classes()[9], TimeSpan::from_seconds(10.0));
        assert_eq!(c.classes()[4], TimeSpan::from_seconds(10.0));
        assert_eq!(c.classes()[0], TimeSpan::from_seconds(10.0));
        assert_eq!(c.stress_time(), TimeSpan::from_seconds(10.0));
        // avg power: (10*1 + 10*0.5) / 20
        assert!((c.average_power_factor() - 0.75).abs() < 1e-9);
        // save + backup rotation + reload
        save_now();
        save_now();
        assert!(file.exists());
        assert!(PathBuf::from(format!("{}.old", file.display())).exists());
        reload_from(file.clone());
        assert_eq!(get_list_clone().counters.len(), 2);
        assert_eq!(get_list_clone().counters.iter().find(|c| c.guid() == guid).unwrap().time_in_run(), TimeSpan::from_hours(2.0));
        // broken main file falls back to .old
        std::fs::write(&file, "not json").unwrap();
        reload_from(file.clone());
        assert_eq!(get_list_clone().counters.len(), 2);
        // edit
        let mut e = get_list_clone().counters[0].clone();
        e.brand = Some("Acme".into());
        edit(&e);
        assert_eq!(get_list_clone().counters[0].brand.as_deref(), Some("Acme"));
        // run-time accounting via status
        select(Some(&guid));
        compute_laser_true_time(0.0);
        compute_laser_time(MacStatus::Run);
        compute_laser_time(MacStatus::Idle);
        on_disconnect();
        assert_eq!(get_current_time(), TimeSpan::ZERO);
    }

    #[test]
    fn json_builder_matches_csharp_layout() {
        let mut c = LaserLifeCounter::create_new();
        c.guid = "g1".into();
        c.name = Some("A\"b".into());
        c.optical_power = Some(5.0);
        c.monitoring_date = chrono::NaiveDate::from_ymd_opt(2024, 3, 5).map(|d| d.and_time(NaiveTime::MIN));
        let s = build_json(&[c]);
        assert!(s.starts_with("[ { \"Guid\": \"g1\",\"Name\": \"A\\\"b\",\"Brand\": \"\","), "{s}");
        assert!(s.contains("\"OpticalPower\": \"5.000\","));
        assert!(s.contains("\"MonitoringDate\": \"2024-03-05 00:00:00\","));
        assert!(s.contains("\"PurchaseDate\": \"\","));
        assert!(s.ends_with("0.000 }  }  ]"), "{s}");
        assert_eq!(escape_json_str(Some("a<b'")), "\"a\\u003cb\\'\"");
    }

    #[test]
    fn json_roundtrip() {
        let mut l = ListLlc::default();
        let mut c = LaserLifeCounter::create_default();
        c.add_true_laser_time_power(TimeSpan::from_seconds(5.0), 0.5);
        c.purchase_date = chrono::NaiveDate::from_ymd_opt(2020, 1, 2).map(|d| d.and_time(NaiveTime::MIN));
        l.counters.push(c);
        let back = ListLlc::from_json(&l.to_json()).unwrap();
        assert_eq!(back.counters[0].guid, l.counters[0].guid);
        assert_eq!(back.counters[0].purchase_date, l.counters[0].purchase_date);
        assert_eq!(back.counters[0].time_classes, l.counters[0].time_classes);
        assert_eq!(back.last_sent, y2000());
        assert!(ListLlc::from_json("{").is_none());
    }
}
