// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Types from `Core/GrblCore.cs`: `Firmware`, `GrblVersionInfo`, `GrblConfST`, obsolete `GrblConf`,
//! `GPoint`, the static `GrblCore.Configuration` property and the `GCODE_STD_*` constants.

use lg_dotnet::{dec_add, dec_div, fmt_decimal, parse_decimal, parse_int, Decimal};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Sub};
use std::sync::LazyLock;

/// Default custom header.
pub const GCODE_STD_HEADER: &str = "G90 (use absolute coordinates)";
/// Default between-passes code.
pub const GCODE_STD_PASSES: &str = ";(Uncomment if you want to sink Z axis)\r\n;G91 (use relative coordinates)\r\n;G0 Z-1 (sinks the Z axis, 1mm)\r\n;G90 (use absolute coordinates)";
/// Default footer.
pub const GCODE_STD_FOOTER: &str = "G0 X0 Y0 Z0 (move back to origin)";

/// Firmware flavour (C# `Firmware`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Firmware {
    #[default]
    Grbl,
    Smoothie,
    Marlin,
    VigoWork,
}

/// Grbl version + vendor info (C# `GrblCore.GrblVersionInfo`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrblVersionInfo {
    major: i32,
    minor: i32,
    build: char,
    ortur: bool,
    longer: bool,
    grbl_hal: bool,
    vendor_info: Option<String>,
    vendor_version: Option<String>,
}

impl GrblVersionInfo {
    /// Full ctor.
    ///
    /// `major`,`minor`,`build`: version (`build` '\0' = none). `vendor_info`: vendor text from
    /// `[VER:..]`. `vendor_version`: vendor fw version text. `is_hal`: grblHAL.
    pub fn with_vendor(major: i32, minor: i32, build: char, vendor_info: Option<&str>, vendor_version: Option<&str>, is_hal: bool) -> Self {
        let vi = vendor_info;
        Self {
            major,
            minor,
            build,
            ortur: vi.is_some_and(|v| v.contains("Ortur") || v.contains("Aufero")),
            longer: vi.is_some_and(|v| v.contains("Longer") || v.contains("NanoDuo")),
            grbl_hal: is_hal,
            vendor_info: vendor_info.map(str::to_string),
            vendor_version: vendor_version.map(str::to_string),
        }
    }

    /// `GrblVersionInfo(major, minor, build)`.
    ///
    /// `major`,`minor`,`build`: version parts.
    pub fn with_build(major: i32, minor: i32, build: char) -> Self {
        Self { major, minor, build, ortur: false, longer: false, grbl_hal: false, vendor_info: None, vendor_version: None }
    }

    /// `GrblVersionInfo(major, minor)`.
    ///
    /// `major`,`minor`: version parts.
    pub fn new(major: i32, minor: i32) -> Self {
        Self::with_build(major, minor, '\0')
    }

    /// C# `CompareTo`: major, minor, build only.
    ///
    /// `v`: other version. Returns -1, 0 or 1.
    pub fn compare_to(&self, v: &GrblVersionInfo) -> i32 {
        match self.major.cmp(&v.major) {
            Ordering::Greater => return 1,
            Ordering::Less => return -1,
            Ordering::Equal => {}
        }
        match self.minor.cmp(&v.minor) {
            Ordering::Greater => return 1,
            Ordering::Less => return -1,
            Ordering::Equal => {}
        }
        match self.build.cmp(&v.build) {
            Ordering::Greater => 1,
            Ordering::Less => -1,
            Ordering::Equal => 0,
        }
    }

    /// Major.
    pub fn major(&self) -> i32 {
        self.major
    }
    /// Minor.
    pub fn minor(&self) -> i32 {
        self.minor
    }
    /// Build letter ('\0' none).
    pub fn build(&self) -> char {
        self.build
    }
    /// Ortur / Aufero machine.
    pub fn is_ortur(&self) -> bool {
        self.ortur
    }
    /// Longer / NanoDuo machine.
    pub fn is_longer(&self) -> bool {
        self.longer
    }
    /// grblHAL.
    pub fn is_hal(&self) -> bool {
        self.grbl_hal
    }

    /// C# `IsLuckyWiFi`. NOTE(upstream): `a || (b && c) || d` precedence kept: `NanoDuo` matches
    /// regardless of `IsLonger`.
    pub fn is_lucky_wifi(&self) -> bool {
        let vi = self.vendor_info.as_deref();
        (self.ortur && vi == Some("Ortur Laser Master 3")) || (self.longer && vi == Some("Longer Nano")) || vi == Some("NanoDuo")
    }

    /// C# `MachineName` (vendor info text).
    pub fn machine_name(&self) -> Option<&str> {
        self.vendor_info.as_deref()
    }

    /// C# `VendorName`. NOTE(upstream): `"Vigotec"` is compared against the lower-cased text, so
    /// it never matches.
    pub fn vendor_name(&self) -> &'static str {
        if self.vendor_info.as_deref().is_some_and(|v| crate::text::dn_lower(v).contains("ortur")) {
            "Ortur"
        } else if self.vendor_info.as_deref().is_some_and(|v| crate::text::dn_lower(v).contains("Vigotec")) {
            "Vigotec"
        } else if self.build == '#' {
            "Emulator"
        } else {
            "Unknown"
        }
    }

    /// C# `OrturFWVersionNumber`: `int.Parse(vendorVersion)` or -1.
    pub fn ortur_fw_version_number(&self) -> i32 {
        self.vendor_version.as_deref().and_then(parse_int).unwrap_or(-1)
    }
}

impl PartialEq for GrblVersionInfo {
    /// C# `Equals`: major, minor, build, ortur, hal (NOT longer / vendor text).
    ///
    /// `o`: other. Returns equality.
    fn eq(&self, o: &Self) -> bool {
        self.major == o.major && self.minor == o.minor && self.build == o.build && self.ortur == o.ortur && self.grbl_hal == o.grbl_hal
    }
}

impl PartialOrd for GrblVersionInfo {
    /// C# `CompareTo` based operators (ignores ortur/hal, unlike `==`).
    ///
    /// `o`: other. Returns ordering.
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.compare_to(o).cmp(&0))
    }
}

impl fmt::Display for GrblVersionInfo {
    /// `major.minor` plus build letter if any.
    ///
    /// `f`: formatter. Returns fmt result.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.build == '\0' {
            write!(f, "{}.{}", self.major, self.minor)
        } else {
            write!(f, "{}.{}{}", self.major, self.minor, self.build)
        }
    }
}

const MAX_CONFIG_SPEED: i64 = 2_000_000;
const MAX_CONFIG_PWM: i64 = 2_000_000;
const MAX_CONFIG_RESOLUTION: i64 = 10_000;
const MAX_CONFIG_SIZE: i64 = 2_000_000;
const MAX_CONFIG_ACCEL: i64 = 2_000_000;

/// One `$n=value` entry (C# `GrblConfST.GrblConfParam`).
#[derive(Debug, Clone, PartialEq)]
pub struct GrblConfParam {
    pub number: i32,
    pub value: String,
}

impl GrblConfParam {
    /// Ctor.
    ///
    /// `number`: setting number. `value`: raw value text.
    pub fn new(number: i32, value: &str) -> Self {
        Self { number, value: value.to_string() }
    }

    /// `$n`.
    pub fn dollar_number(&self) -> String {
        format!("${}", self.number)
    }

    /// Brief name from the CSV table.
    pub fn parameter(&self) -> Option<String> {
        crate::csvd::settings_item(&self.number.to_string(), 0)
    }

    /// Unit from the CSV table.
    pub fn unit(&self) -> Option<String> {
        crate::csvd::settings_item(&self.number.to_string(), 1)
    }

    /// Description from the CSV table.
    pub fn description(&self) -> Option<String> {
        crate::csvd::settings_item(&self.number.to_string(), 2)
    }
}

static CONF_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[$](\d+)\s*=(.*)").unwrap());
static FOXALIEN_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[$](\d+)\s*=\s*(\d+\.?\d*)\s*\(.*\)").unwrap());
static EXTRACT_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+\.?\d*)").unwrap());

/// Grbl configuration as text values (C# `GrblConfST`). Entry order = insertion order.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrblConfST {
    data: Vec<(i32, String)>,
    version: Option<GrblVersionInfo>,
}

impl GrblConfST {
    /// Empty config without version.
    pub fn new() -> Self {
        Self::default()
    }

    /// Empty config with version.
    ///
    /// `version`: detected grbl version.
    pub fn with_version(version: Option<GrblVersionInfo>) -> Self {
        Self { data: Vec::new(), version }
    }

    /// Config with initial table (C# throws on duplicate keys; later duplicates are dropped here).
    ///
    /// `version`: version. `table`: key/value pairs.
    pub fn with_table(version: Option<GrblVersionInfo>, table: &[(i32, String)]) -> Self {
        let mut s = Self::with_version(version);
        for (k, v) in table {
            if !s.contains_key(*k) {
                s.data.push((*k, v.clone()));
            }
        }
        s
    }

    /// Convert old numeric config.
    ///
    /// `old`: old config or None.
    pub fn from_old(old: Option<&GrblConf>) -> Self {
        let mut s = Self::with_version(old.and_then(|o| o.version.clone()));
        if let Some(o) = old {
            for (k, v) in &o.data {
                s.data.push((*k, fmt_decimal(*v)));
            }
        }
        s
    }

    /// Version.
    pub fn grbl_version(&self) -> Option<&GrblVersionInfo> {
        self.version.as_ref()
    }

    /// `mVersion >= new GrblVersionInfo(major, minor)`.
    fn version_at_least(&self, major: i32, minor: i32) -> bool {
        self.version.as_ref().is_some_and(|v| *v >= GrblVersionInfo::new(major, minor))
    }
    fn version11(&self) -> bool {
        self.version_at_least(1, 1)
    }
    fn version9(&self) -> bool {
        self.version_at_least(0, 9)
    }

    /// Expected number of settings.
    pub fn expected_count(&self) -> i32 {
        if self.version11() { 34 } else if self.version9() { 31 } else { 23 }
    }
    /// `$22` / `$17`.
    pub fn homing_enabled(&self) -> bool {
        self.read_decimal(if self.version9() { 22 } else { 17 }, 1.into(), 0.into(), 1.into()) != Decimal::ZERO
    }
    /// Max rate X.
    pub fn max_rate_x(&self) -> Decimal {
        self.read_decimal(if self.version9() { 110 } else { 4 }, 4000.into(), 1.into(), MAX_CONFIG_SPEED.into())
    }
    /// Max rate Y.
    pub fn max_rate_y(&self) -> Decimal {
        self.read_decimal(if self.version9() { 111 } else { 5 }, 4000.into(), 1.into(), MAX_CONFIG_SPEED.into())
    }
    /// Laser mode (`$32`); true without version.
    pub fn laser_mode(&self) -> bool {
        if self.version.is_none() {
            true
        } else {
            self.read_decimal(if self.version11() { 32 } else { -1 }, 0.into(), 0.into(), 1.into()) != Decimal::ZERO
        }
    }
    /// Min PWM (`$31`).
    pub fn min_pwm(&self) -> Decimal {
        self.read_decimal(if self.version11() { 31 } else { -1 }, 0.into(), 0.into(), MAX_CONFIG_PWM.into())
    }
    /// Max PWM (`$30`).
    pub fn max_pwm(&self) -> Decimal {
        self.read_decimal(if self.version11() { 30 } else { -1 }, 1000.into(), 1.into(), MAX_CONFIG_PWM.into())
    }
    /// Resolution X.
    pub fn resolution_x(&self) -> Decimal {
        self.read_decimal(if self.version9() { 100 } else { 0 }, 250.into(), 1.into(), MAX_CONFIG_RESOLUTION.into())
    }
    /// Resolution Y.
    pub fn resolution_y(&self) -> Decimal {
        self.read_decimal(if self.version9() { 101 } else { 1 }, 250.into(), 1.into(), MAX_CONFIG_RESOLUTION.into())
    }
    /// Table width.
    pub fn table_width(&self) -> Decimal {
        self.read_decimal(if self.version9() { 130 } else { -1 }, 300.into(), 1.into(), MAX_CONFIG_SIZE.into())
    }
    /// Table height.
    pub fn table_height(&self) -> Decimal {
        self.read_decimal(if self.version9() { 131 } else { -1 }, 200.into(), 1.into(), MAX_CONFIG_SIZE.into())
    }
    /// Soft limit (`$20`).
    pub fn soft_limit(&self) -> bool {
        self.read_decimal(20, 0.into(), 0.into(), 1.into()) != Decimal::ZERO
    }
    /// `(AccelerationX + AccelerationY) / 2`.
    pub fn acceleration_xy(&self) -> Decimal {
        let sum = dec_add(self.acceleration_x(), self.acceleration_y()).expect("decimal overflow");
        dec_div(sum, 2.into()).expect("decimal overflow")
    }
    fn acceleration_x(&self) -> Decimal {
        self.read_decimal(if self.version9() { 120 } else { -1 }, 2000.into(), 1.into(), MAX_CONFIG_ACCEL.into())
    }
    fn acceleration_y(&self) -> Decimal {
        self.read_decimal(if self.version9() { 121 } else { -1 }, 2000.into(), 1.into(), MAX_CONFIG_ACCEL.into())
    }
    /// WiFi SSID (`$74`).
    pub fn wifi_ssid(&self) -> Option<String> {
        self.read_string(74, None)
    }
    /// WiFi password (`$75`).
    pub fn wifi_pwd(&self) -> Option<String> {
        self.read_string(75, None)
    }
    /// Telnet port (`$305`), default "23".
    pub fn telnet_port(&self) -> Option<String> {
        self.read_string(305, Some("23"))
    }

    /// Read a decimal setting, clamp to `[min, max]`.
    ///
    /// `key`: setting number. `defval`: value when missing/no version/unparsable. `min`,`max`: clamp.
    fn read_decimal(&self, key: i32, defval: Decimal, min: Decimal, max: Decimal) -> Decimal {
        let mut rv = defval;
        if self.version.is_none() {
            rv = defval;
        } else if let Some(raw) = self.get(key) {
            rv = match parse_decimal(raw) {
                Some(d) => d,
                // regex fallback (decimal.Parse without culture = invariant here)
                None => EXTRACT_NUMBER
                    .captures(raw)
                    .and_then(|c| parse_decimal(&c[1]))
                    .unwrap_or(defval),
            };
        }
        max_dec(min, min_dec(max, rv))
    }

    /// Read raw text.
    ///
    /// `number`: key. `defval`: fallback.
    fn read_string(&self, number: i32, defval: Option<&str>) -> Option<String> {
        if self.version.is_none() {
            return defval.map(str::to_string);
        }
        match self.get(number) {
            Some(v) => Some(v.to_string()),
            None => defval.map(str::to_string),
        }
    }

    /// Raw value of key.
    ///
    /// `key`: setting number.
    pub fn get(&self, key: i32) -> Option<&str> {
        self.data.iter().find(|(k, _)| *k == key).map(|(_, v)| v.as_str())
    }

    fn contains_key(&self, key: i32) -> bool {
        self.get(key).is_some()
    }

    /// All entries as params.
    pub fn to_list(&self) -> Vec<GrblConfParam> {
        self.data.iter().map(|(k, v)| GrblConfParam::new(*k, v)).collect()
    }

    /// Number of entries.
    pub fn count(&self) -> usize {
        self.data.len()
    }

    /// Entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (i32, &str)> {
        self.data.iter().map(|(k, v)| (*k, v.as_str()))
    }

    /// True when `p` is new or differs.
    ///
    /// `p`: param to compare.
    pub fn has_changes(&self, p: &GrblConfParam) -> bool {
        self.get(p.number) != Some(p.value.as_str())
    }

    /// `^$n =...` check.
    ///
    /// `p`: text to test.
    pub fn is_set_conf(p: &str) -> bool {
        CONF_REGEX.is_match(p)
    }

    /// Store a key (insert or replace in place).
    fn put(&mut self, key: i32, val: String) {
        match self.data.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1 = val,
            None => self.data.push((key, val)),
        }
    }

    /// C# `AddOrUpdate`: parse `$n=v`, errors swallowed.
    ///
    /// `line`: text line from grbl.
    pub fn add_or_update(&mut self, line: &str) {
        if !Self::is_set_conf(line) {
            return;
        }
        let line = Self::fix_foxalien_config(line);
        if let Some(c) = CONF_REGEX.captures(&line)
            && let Some(key) = parse_int(&c[1])
        {
            let val = crate::text::dn_trim(&c[2]).to_string();
            self.put(key, val);
        }
    }

    /// Foxalien: strip trailing `(description)`.
    ///
    /// `line`: config line. Returns cleaned line.
    fn fix_foxalien_config(line: &str) -> String {
        if let Some(c) = FOXALIEN_REGEX.captures(line)
            && let Some(key) = parse_int(&c[1])
        {
            let val = crate::text::dn_trim(&c[2]);
            return format!("${key}={val}");
        }
        line.to_string()
    }

    /// C# `SetValueIfKeyExist`.
    ///
    /// `p`: config line. Returns true when an existing key was updated.
    pub fn set_value_if_key_exist(&mut self, p: &str) -> bool {
        if Self::is_set_conf(p)
            && let Some(c) = CONF_REGEX.captures(p)
            && let Some(key) = parse_int(&c[1])
        {
            if !self.contains_key(key) {
                return false;
            }
            self.put(key, crate::text::dn_trim(&c[2]).to_string());
            return true;
        }
        false
    }

    /// C# `ValidateConfig`: Ortur safety warning for `$33`.
    ///
    /// `parid`: setting number. Returns warning text or None.
    pub fn validate_config(&self, parid: i32) -> Option<&'static str> {
        if parid == 33 && self.version.as_ref().is_some_and(|v| v.is_ortur()) {
            Some("This param control an Ortur safety feature. Please do not change this value!")
        } else {
            None
        }
    }
}

/// `Math.Min(decimal, decimal)`.
fn min_dec(a: Decimal, b: Decimal) -> Decimal {
    if b < a { b } else { a }
}

/// `Math.Max(decimal, decimal)`.
fn max_dec(a: Decimal, b: Decimal) -> Decimal {
    if b > a { b } else { a }
}

static OLD_CONF_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[$](\d+) *= *(\d+\.?\d*)").unwrap());

/// Obsolete numeric config (C# `GrblConf`), kept for settings migration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrblConf {
    #[serde(with = "dec_pairs")]
    data: Vec<(i32, Decimal)>,
    version: Option<GrblVersionInfo>,
}

impl GrblConf {
    /// Empty config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Empty config with version.
    ///
    /// `version`: grbl version.
    pub fn with_version(version: Option<GrblVersionInfo>) -> Self {
        Self { data: Vec::new(), version }
    }

    /// Config with table.
    ///
    /// `version`: version. `table`: key/value pairs.
    pub fn with_table(version: Option<GrblVersionInfo>, table: &[(i32, Decimal)]) -> Self {
        let mut s = Self::with_version(version);
        for (k, v) in table {
            if !s.contains_key(*k) {
                s.data.push((*k, *v));
            }
        }
        s
    }

    /// Version.
    pub fn grbl_version(&self) -> Option<&GrblVersionInfo> {
        self.version.as_ref()
    }
    fn version11(&self) -> bool {
        self.version.as_ref().is_some_and(|v| *v >= GrblVersionInfo::new(1, 1))
    }
    fn version9(&self) -> bool {
        self.version.as_ref().is_some_and(|v| *v >= GrblVersionInfo::new(0, 9))
    }
    /// Expected count.
    pub fn expected_count(&self) -> i32 {
        if self.version11() { 34 } else if self.version9() { 31 } else { 23 }
    }
    /// Homing enabled.
    pub fn homing_enabled(&self) -> bool {
        self.read(if self.version9() { 22 } else { 17 }, 1) != Decimal::ZERO
    }
    /// Max rate X.
    pub fn max_rate_x(&self) -> Decimal {
        self.read(if self.version9() { 110 } else { 4 }, 4000)
    }
    /// Max rate Y.
    pub fn max_rate_y(&self) -> Decimal {
        self.read(if self.version9() { 111 } else { 5 }, 4000)
    }
    /// Laser mode.
    pub fn laser_mode(&self) -> bool {
        if self.version.is_none() { true } else { self.read(if self.version11() { 32 } else { -1 }, 0) != Decimal::ZERO }
    }
    /// Min PWM.
    pub fn min_pwm(&self) -> Decimal {
        self.read(if self.version11() { 31 } else { -1 }, 0)
    }
    /// Max PWM.
    pub fn max_pwm(&self) -> Decimal {
        self.read(if self.version11() { 30 } else { -1 }, 1000)
    }
    /// Resolution X.
    pub fn resolution_x(&self) -> Decimal {
        self.read(if self.version9() { 100 } else { 0 }, 250)
    }
    /// Resolution Y.
    pub fn resolution_y(&self) -> Decimal {
        self.read(if self.version9() { 101 } else { 1 }, 250)
    }
    /// Table width.
    pub fn table_width(&self) -> Decimal {
        self.read(if self.version9() { 130 } else { -1 }, 300)
    }
    /// Table height.
    pub fn table_height(&self) -> Decimal {
        self.read(if self.version9() { 131 } else { -1 }, 200)
    }
    /// Soft limit.
    pub fn soft_limit(&self) -> bool {
        self.read(20, 0) != Decimal::ZERO
    }
    /// Mean acceleration.
    pub fn acceleration_xy(&self) -> Decimal {
        let x = self.read(if self.version9() { 120 } else { -1 }, 2000);
        let y = self.read(if self.version9() { 121 } else { -1 }, 2000);
        dec_div(dec_add(x, y).expect("decimal overflow"), 2.into()).expect("decimal overflow")
    }

    /// Read value or default.
    ///
    /// `number`: key. `defval`: fallback.
    fn read(&self, number: i32, defval: i64) -> Decimal {
        if self.version.is_none() {
            return defval.into();
        }
        self.data.iter().find(|(k, _)| *k == number).map(|(_, v)| *v).unwrap_or(defval.into())
    }

    fn contains_key(&self, key: i32) -> bool {
        self.data.iter().any(|(k, _)| *k == key)
    }

    /// Entries.
    pub fn iter(&self) -> impl Iterator<Item = (i32, Decimal)> + '_ {
        self.data.iter().copied()
    }

    /// Entry count.
    pub fn count(&self) -> usize {
        self.data.len()
    }

    /// Insert or replace.
    fn put(&mut self, key: i32, val: Decimal) {
        match self.data.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1 = val,
            None => self.data.push((key, val)),
        }
    }

    /// `^$n = v` check.
    ///
    /// `p`: text.
    pub fn is_set_conf(p: &str) -> bool {
        OLD_CONF_REGEX.is_match(p)
    }

    /// Parse a config line into the table; errors swallowed.
    ///
    /// `p`: line.
    pub fn add_or_update(&mut self, p: &str) {
        if let Some(c) = OLD_CONF_REGEX.captures(p)
            && let (Some(key), Some(val)) = (parse_int(&c[1]), parse_decimal(&c[2]))
        {
            self.put(key, val);
        }
    }

    /// Update an existing key.
    ///
    /// `p`: line. Returns true when updated.
    pub fn set_value_if_key_exist(&mut self, p: &str) -> bool {
        if let Some(c) = OLD_CONF_REGEX.captures(p)
            && let (Some(key), Some(val)) = (parse_int(&c[1]), parse_decimal(&c[2]))
        {
            if !self.contains_key(key) {
                return false;
            }
            self.put(key, val);
            return true;
        }
        false
    }
}

/// Machine position (C# struct `GPoint`, f32 components).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl GPoint {
    /// `GPoint.Zero`.
    pub const ZERO: GPoint = GPoint { x: 0.0, y: 0.0, z: 0.0 };

    /// Ctor.
    ///
    /// `x`,`y`,`z`: coordinates.
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl Add for GPoint {
    type Output = GPoint;
    /// Component sum.
    ///
    /// `b`: other point.
    fn add(self, b: GPoint) -> GPoint {
        GPoint::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}

impl Sub for GPoint {
    type Output = GPoint;
    /// Component difference.
    ///
    /// `b`: other point.
    fn sub(self, b: GPoint) -> GPoint {
        GPoint::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}

const CONF_KEY: &str = "Grbl Configuration";
const CONF_ST_KEY: &str = "Grbl Configuration ST";

/// Static `GrblCore.Configuration` getter / setter on the settings store.
pub mod configuration {
    use super::*;

    /// Getter: migrates old `Grbl Configuration` to `Grbl Configuration ST` first.
    ///
    /// Returns the stored config or an empty one.
    pub fn configuration() -> GrblConfST {
        if lg_base::settings::exists(CONF_KEY) {
            let old: Option<GrblConf> = lg_base::settings::get(CONF_KEY, None);
            lg_base::settings::set(CONF_ST_KEY, GrblConfST::from_old(old.as_ref()));
            lg_base::settings::delete(CONF_KEY);
        }
        lg_base::settings::get(CONF_ST_KEY, GrblConfST::new())
    }

    /// Setter: stores only non-empty configs.
    ///
    /// `value`: config to store.
    pub fn set_configuration(value: &GrblConfST) {
        if value.count() > 0 {
            lg_base::settings::set(CONF_ST_KEY, value);
        }
    }
}

/// serde helper: `(i32, Decimal)` pairs stored as `(i32, "text")` (keeps decimal scale).
mod dec_pairs {
    use super::{fmt_decimal, parse_decimal, Decimal};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Serialize pairs with decimals as strings.
    ///
    /// `v`: pairs. `s`: serializer.
    pub fn serialize<S: Serializer>(v: &[(i32, Decimal)], s: S) -> Result<S::Ok, S::Error> {
        let t: Vec<(i32, String)> = v.iter().map(|(k, d)| (*k, fmt_decimal(*d))).collect();
        t.serialize(s)
    }

    /// Deserialize pairs; bad decimal text is an error.
    ///
    /// `d`: deserializer.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<(i32, Decimal)>, D::Error> {
        let t: Vec<(i32, String)> = Vec::deserialize(d)?;
        t.into_iter()
            .map(|(k, s)| parse_decimal(&s).map(|v| (k, v)).ok_or_else(|| serde::de::Error::custom("bad decimal")))
            .collect()
    }
}
