// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `UsageStats.cs`, local part only (PLAN section 6: no upload to the upstream server).
//!
//! Kept: [`UsageCounters`] (per-core counters), the persisted [`UsageStats`] record (installation id,
//! usage count/time, last versions, accumulated counters) in `UsageStats.json` in the data dir, and `GetID()`.
//! Not ported: `TrueSend`/`UpdateAndSend` (network), `MessageManager`/`MessageData` (toolbar news from the
//! server), `FixFile360` (migration of 2020 BinaryFormatter files), `MyWebClient`.

use crate::util::new_guid_n;
use chrono::{DateTime, Utc};
use lg_base::paths::data_path;
use lg_gcode::GrblVersionInfo;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// Counters of imported/converted files (C# `UsageStats.UsageCounters`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCounters {
    pub gcode_file: i32,
    pub raster_file: i32,
    pub vectorization: i32,
    pub centerline: i32,
    pub dithering: i32,
    pub line2line: i32,
    pub svg_file: i32,
    pub passthrough: i32,
}

impl UsageCounters {
    /// `Update(c)`: add another counter set.
    ///
    /// `c`: counters to add.
    pub fn update(&mut self, c: &UsageCounters) {
        self.gcode_file += c.gcode_file;
        self.raster_file += c.raster_file;
        self.vectorization += c.vectorization;
        self.centerline += c.centerline;
        self.dithering += c.dithering;
        self.line2line += c.line2line;
        self.svg_file += c.svg_file;
        self.passthrough += c.passthrough;
    }
}

/// Persisted usage record (C# `UsageStats` instance fields that are not network related).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// `InstallationID.ToString("N")`.
    pub installation_id: String,
    /// `InstalledDate` (UTC, RFC 3339).
    pub installed_date: String,
    /// `Version` of the last session ("a.b.c").
    pub version: String,
    /// `GrblVersion` of the last session.
    pub grbl_version: Option<GrblVersionInfo>,
    /// `UsageCount`: sessions started.
    pub usage_count: i32,
    /// `UsageTime` in 100 ns ticks.
    pub usage_time_ticks: i64,
    /// `Wrapper` name.
    pub wrapper: String,
    /// `FirmwareString`.
    pub firmware: String,
    /// `VendorString`.
    pub vendor: String,
    /// `Counters` (accumulated over sessions).
    pub counters: UsageCounters,
}

impl Default for UsageStats {
    /// Fresh record: new installation id, now as install date.
    fn default() -> Self {
        Self {
            installation_id: new_guid_n(),
            installed_date: Utc::now().to_rfc3339(),
            version: "0.0.0".to_string(),
            grbl_version: None,
            usage_count: 0,
            usage_time_ticks: 0,
            wrapper: String::new(),
            firmware: String::new(),
            vendor: String::new(),
            counters: UsageCounters::default(),
        }
    }
}

static DATA: LazyLock<Mutex<Option<UsageStats>>> = LazyLock::new(|| Mutex::new(None));

/// Path of the record (C# `UsageStats.bin`).
pub fn data_file() -> PathBuf {
    data_path().join("UsageStats.json")
}

/// `LoadFile()` at start: read the record (or create it) and count this session.
pub fn load_file() {
    load_file_from(&data_file());
}

/// `LoadFile()` with an explicit file.
///
/// `file`: JSON path; missing/broken -> new record.
pub fn load_file_from(file: &PathBuf) {
    let mut data = std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str::<UsageStats>(&t).ok()).unwrap_or_default();
    data.usage_count += 1;
    *DATA.lock().unwrap_or_else(|e| e.into_inner()) = Some(data);
}

/// `SaveFile(core)` at exit without the upload: fold in this session and write the record.
///
/// `session`: counters of the closing core. `grbl_version`: version the core saw (None keeps the old one).
/// `vendor`: `GrblVersion.VendorName`. `uptime`: time since program start (adds to the usage time).
/// `file`: target path.
pub fn save_file(session: &UsageCounters, grbl_version: Option<&GrblVersionInfo>, vendor: Option<&str>, uptime: lg_dotnet::TimeSpan, file: &PathBuf) {
    let mut g = DATA.lock().unwrap_or_else(|e| e.into_inner());
    let data = g.get_or_insert_with(UsageStats::default);
    let (a, b, c) = lg_base::settings::CURRENT_VERSION;
    data.version = format!("{a}.{b}.{c}");
    if let Some(v) = grbl_version {
        data.grbl_version = Some(v.clone());
    }
    // prevent wrong values
    if data.usage_time_ticks < 0 {
        data.usage_time_ticks = 0;
    }
    if uptime > lg_dotnet::TimeSpan::ZERO {
        data.usage_time_ticks += uptime.ticks();
    }
    data.wrapper = lg_base::settings::get("ComWrapper Protocol", crate::com::WrapperType::UsbSerial).name().to_string();
    let fw: lg_gcode::Firmware = lg_base::settings::get("Firmware Type", lg_gcode::Firmware::Grbl);
    data.firmware = format!("{fw:?}");
    data.vendor = vendor.unwrap_or("Unknown").to_string();
    data.counters.update(session);
    if let Ok(t) = serde_json::to_string_pretty(&*data) {
        let _ = std::fs::write(file, t);
    }
}

/// `GetID()`: installation id ("N" format), None before `load_file`.
pub fn get_id() -> Option<String> {
    DATA.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|d| d.installation_id.clone())
}

/// Copy of the loaded record (UI "about"/diagnostics).
pub fn snapshot() -> Option<UsageStats> {
    DATA.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Parse the stored install date.
pub fn installed_date() -> Option<DateTime<Utc>> {
    snapshot().and_then(|d| DateTime::parse_from_rfc3339(&d.installed_date).ok()).map(|d| d.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_update() {
        let mut a = UsageCounters { gcode_file: 1, svg_file: 2, ..Default::default() };
        a.update(&UsageCounters { gcode_file: 2, raster_file: 5, passthrough: 1, ..Default::default() });
        assert_eq!((a.gcode_file, a.raster_file, a.svg_file, a.passthrough), (3, 5, 2, 1));
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = std::env::temp_dir().join(format!("lg_usage_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("UsageStats.json");
        let _ = std::fs::remove_file(&file);
        load_file_from(&file);
        let id = get_id().unwrap();
        assert_eq!(id.len(), 32);
        assert_eq!(snapshot().unwrap().usage_count, 1);
        let counters = UsageCounters { gcode_file: 4, ..Default::default() };
        save_file(&counters, Some(&GrblVersionInfo::new(1, 1)), Some("Ortur"), lg_dotnet::TimeSpan::from_seconds(60.0), &file);
        // next session
        load_file_from(&file);
        let s = snapshot().unwrap();
        assert_eq!(s.installation_id, id);
        assert_eq!(s.usage_count, 2);
        assert_eq!(s.counters.gcode_file, 4);
        assert_eq!(s.vendor, "Ortur");
        assert_eq!(s.usage_time_ticks, 600_000_000);
        assert!(installed_date().is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
