//! Named layer presets: all per-layer values of the "Parameters" dock (except `enabled`).
//!
//! Not part of the C# app (intentional deviation): replaces the old material database
//! (`PSHelper/MaterialDB.cs`). Stored as JSON in `<config dir>/LaserGRBL/LayerPresets.json`.
//! A missing file yields one example preset; nothing is written until the user saves or deletes.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// File name inside the data dir.
pub const FILE_NAME: &str = "LayerPresets.json";

/// Name of the built-in example preset.
pub const EXAMPLE_NAME: &str = "Plywood 4mm engrave";

/// Longest accepted preset name (chars).
const MAX_NAME: usize = 80;

/// Power base when the firmware max PWM ($30) is unknown.
const DEFAULT_BASE: i32 = 1000;

/// Serde default of [`LayerPreset::filter`].
fn default_filter() -> String {
    "All".to_owned()
}

/// Serde default of [`LayerPreset::fill_distance`] (mm).
fn default_fill_distance() -> f64 {
    0.1
}

/// Serde default for flags that are on unless stored otherwise. Returns true.
fn default_true() -> bool {
    true
}

/// One named set of layer values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerPreset {
    /// Unique name (case-insensitive).
    pub name: String,
    /// Repeat count, 1..=100.
    pub passes: i32,
    /// Feed in mm/min, >= 1.
    pub speed: i32,
    /// Power in % of max S ($30), 0 < x <= 100.
    pub power_pct: f64,
    /// "M3" or "M4".
    pub laser_on: String,
    /// Color filter name: "All" | "Red" | "Green" | "Blue" | "Black".
    #[serde(default = "default_filter")]
    pub filter: String,
    /// Hatch fill of closed shapes on.
    #[serde(default)]
    pub fill: bool,
    /// Fill line distance in mm, 0.01..=10.
    #[serde(default = "default_fill_distance")]
    pub fill_distance: f64,
    /// Fill angle in degrees, 0..=360.
    #[serde(default)]
    pub fill_angle: f64,
    /// Second hatch rotated +90 degrees.
    #[serde(default)]
    pub fill_cross: bool,
    /// Finish each connected fill area before the next (order only).
    #[serde(default = "default_true")]
    pub fill_per_shape: bool,
}

/// Power base for a max PWM value.
///
/// `max_pwm`: firmware $30. Returns `max_pwm`, or 1000 when not positive.
fn base_of(max_pwm: i32) -> i32 {
    if max_pwm > 0 { max_pwm } else { DEFAULT_BASE }
}

impl LayerPreset {
    /// The built-in example preset. Returns "Plywood 4mm engrave" (1 pass, 1000 mm/min, 45 %, M4, All).
    pub fn example() -> Self {
        Self { name: EXAMPLE_NAME.to_owned(), passes: 1, speed: 1000, power_pct: 45.0, laser_on: "M4".to_owned(), filter: default_filter(), fill: false, fill_distance: default_fill_distance(), fill_angle: 0.0, fill_cross: false, fill_per_shape: true }
    }

    /// S value of the preset.
    ///
    /// `max_pwm`: firmware $30 (<= 0 uses 1000). Returns `round(pct * base / 100)` clamped to `1..=base`.
    pub fn power_s(&self, max_pwm: i32) -> i32 {
        let base = base_of(max_pwm);
        ((self.power_pct * base as f64 / 100.0).round() as i32).clamp(1, base)
    }

    /// Percent of an S value.
    ///
    /// `s`: S value. `max_pwm`: firmware $30 (<= 0 uses 1000). Returns `s * 100 / base` rounded to 0.1.
    pub fn pct_from_s(s: i32, max_pwm: i32) -> f64 {
        let base = base_of(max_pwm);
        (s as f64 * 100.0 / base as f64 * 10.0).round() / 10.0
    }
}

/// On-disk layout.
#[derive(Serialize, Deserialize)]
struct FileData {
    version: u32,
    presets: Vec<LayerPreset>,
}

/// Case-insensitive name order, ties by plain compare.
///
/// `a`, `b`: names. Returns the ordering.
fn name_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
}

/// Sort by name and drop duplicates (case-insensitive, later entry wins).
///
/// `v`: presets. Returns the cleaned list.
fn normalize(v: Vec<LayerPreset>) -> Vec<LayerPreset> {
    let mut out: Vec<LayerPreset> = Vec::new();
    for p in v {
        // later entry replaces the earlier one
        match out.iter().position(|o| o.name.trim().to_lowercase() == p.name.trim().to_lowercase()) {
            Some(i) => out[i] = p,
            None => out.push(p),
        }
    }
    out.sort_by(|a, b| name_cmp(&a.name, &b.name));
    out
}

/// Preset database.
#[derive(Debug, Clone, Default)]
pub struct PresetDb {
    presets: Vec<LayerPreset>,
    file: Option<PathBuf>,
}

impl PresetDb {
    /// Default file. Returns `<data dir>/LayerPresets.json`.
    pub fn default_file() -> PathBuf {
        lg_base::paths::data_path().join(FILE_NAME)
    }

    /// Load the default file. Returns the DB (example preset when the file is missing).
    pub fn load() -> Self {
        Self::load_from(&Self::default_file())
    }

    /// Load a file.
    ///
    /// `path`: JSON file. Returns the DB; missing file gives the example (nothing written),
    /// a broken file is moved to `<name>.bak` and also gives the example.
    pub fn load_from(path: &Path) -> Self {
        let presets = if !path.exists() {
            vec![LayerPreset::example()]
        } else {
            let parsed = std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str::<FileData>(&t).map_err(|e| e.to_string()));
            match parsed {
                Ok(d) => normalize(d.presets),
                Err(e) => {
                    // keep the broken file for the user
                    log::warn!("layer presets {} unusable ({e}), using example", path.display());
                    let mut bak = path.as_os_str().to_owned();
                    bak.push(".bak");
                    if let Err(e) = std::fs::rename(path, PathBuf::from(bak)) {
                        log::warn!("layer presets backup failed: {e}");
                    }
                    vec![LayerPreset::example()]
                }
            }
        };
        Self { presets, file: Some(path.to_path_buf()) }
    }

    /// DB without a file (tests).
    ///
    /// `presets`: start content. Returns the DB; `save` does nothing.
    pub fn in_memory(presets: Vec<LayerPreset>) -> Self {
        Self { presets: normalize(presets), file: None }
    }

    /// Backing file. Returns None for an in-memory DB.
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// All presets. Returns the list sorted by name (case-insensitive).
    pub fn presets(&self) -> &[LayerPreset] {
        &self.presets
    }

    /// Look up a preset.
    ///
    /// `name`: name, matched trimmed and case-insensitive. Returns the preset or None.
    pub fn find(&self, name: &str) -> Option<&LayerPreset> {
        let n = name.trim().to_lowercase();
        self.presets.iter().find(|p| p.name.trim().to_lowercase() == n)
    }

    /// Add a preset or silently replace the one with the same name.
    ///
    /// `p`: preset; name is trimmed, values are clamped. Returns Err for an empty or too long name
    /// or a failed save.
    pub fn upsert(&mut self, mut p: LayerPreset) -> Result<(), String> {
        p.name = p.name.trim().to_owned();
        if p.name.is_empty() {
            return Err("Preset name is empty.".to_owned());
        }
        if p.name.chars().count() > MAX_NAME {
            return Err(format!("Preset name is longer than {MAX_NAME} characters."));
        }
        // clamp to valid ranges
        p.passes = p.passes.clamp(1, 100);
        p.speed = p.speed.max(1);
        p.power_pct = p.power_pct.clamp(0.1, 100.0);
        // NaN falls back to the default
        p.fill_distance = if p.fill_distance.is_finite() { p.fill_distance.clamp(0.01, 10.0) } else { default_fill_distance() };
        p.fill_angle = if p.fill_angle.is_finite() { p.fill_angle.clamp(0.0, 360.0) } else { 0.0 };
        let key = p.name.to_lowercase();
        match self.presets.iter().position(|o| o.name.trim().to_lowercase() == key) {
            Some(i) => self.presets[i] = p,
            None => self.presets.push(p),
        }
        self.presets.sort_by(|a, b| name_cmp(&a.name, &b.name));
        self.save()
    }

    /// Remove a preset.
    ///
    /// `name`: name, trimmed and case-insensitive. Returns Ok(true) when one was removed (and saved),
    /// Ok(false) when none matched, Err on a failed save.
    pub fn delete(&mut self, name: &str) -> Result<bool, String> {
        let n = name.trim().to_lowercase();
        let before = self.presets.len();
        self.presets.retain(|p| p.name.trim().to_lowercase() != n);
        if self.presets.len() == before {
            return Ok(false);
        }
        self.save().map(|_| true)
    }

    /// Write the file (temp file, then rename). Returns Ok(()) also for an in-memory DB.
    pub fn save(&self) -> Result<(), String> {
        let Some(file) = &self.file else { return Ok(()) };
        // parent dir may not exist yet
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&FileData { version: 1, presets: self.presets.clone() }).map_err(|e| e.to_string())?;
        let mut tmp = file.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, file).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh temp dir. `tag`: test tag. Returns the dir.
    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lg_presets_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn preset(name: &str, speed: i32) -> LayerPreset {
        LayerPreset { name: name.into(), passes: 2, speed, power_pct: 30.0, laser_on: "M3".into(), filter: "Red".into(), fill: false, fill_distance: 0.1, fill_angle: 0.0, fill_cross: false, fill_per_shape: true }
    }

    #[test]
    fn missing_file_gives_example_only() {
        let f = tmp("missing").join("LayerPresets.json");
        let db = PresetDb::load_from(&f);
        assert_eq!(db.presets(), [LayerPreset::example()]);
        assert!(!f.exists());
        let e = LayerPreset::example();
        assert_eq!((e.name.as_str(), e.passes, e.speed, e.power_pct, e.laser_on.as_str(), e.filter.as_str()), (EXAMPLE_NAME, 1, 1000, 45.0, "M4", "All"));
    }

    #[test]
    fn save_reload_roundtrip() {
        let f = tmp("roundtrip").join("sub").join(FILE_NAME);
        let mut db = PresetDb::load_from(&f);
        db.upsert(preset("beta", 500)).unwrap();
        db.upsert(preset("Alpha", 600)).unwrap();
        let db2 = PresetDb::load_from(&f);
        let names: Vec<&str> = db2.presets().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "beta", EXAMPLE_NAME]);
        assert_eq!(db2.find(" ALPHA ").unwrap().speed, 600);
        assert!(!f.with_extension("json.tmp").exists());
    }

    #[test]
    fn upsert_same_name_other_case_replaces() {
        let mut db = PresetDb::in_memory(vec![]);
        db.upsert(preset("Test", 500)).unwrap();
        db.upsert(preset("  test ", 600)).unwrap();
        assert_eq!(db.presets().len(), 1);
        assert_eq!((db.presets()[0].name.as_str(), db.presets()[0].speed), ("test", 600));
    }

    #[test]
    fn upsert_validates_and_clamps() {
        let mut db = PresetDb::in_memory(vec![]);
        assert!(db.upsert(preset("   ", 1)).is_err());
        assert!(db.upsert(preset(&"x".repeat(81), 1)).is_err());
        let mut p = preset("c", -5);
        p.passes = 500;
        p.power_pct = 0.0;
        db.upsert(p).unwrap();
        let c = db.find("c").unwrap();
        assert_eq!((c.passes, c.speed, c.power_pct), (100, 1, 0.1));
        // fill values clamp
        let mut p = preset("d", 1);
        (p.fill_distance, p.fill_angle) = (0.0, 999.0);
        db.upsert(p).unwrap();
        let d = db.find("d").unwrap();
        assert_eq!((d.fill_distance, d.fill_angle), (0.01, 360.0));
    }

    #[test]
    fn delete_persists_and_empty_stays_empty() {
        let f = tmp("delete").join(FILE_NAME);
        let mut db = PresetDb::load_from(&f);
        db.upsert(preset("a", 1)).unwrap();
        assert!(db.delete("A").unwrap());
        assert!(!db.delete("nope").unwrap());
        assert_eq!(PresetDb::load_from(&f).presets().len(), 1);
        assert!(db.delete(EXAMPLE_NAME).unwrap());
        assert!(PresetDb::load_from(&f).presets().is_empty());
    }

    #[test]
    fn corrupt_file_gives_example_and_backup() {
        let d = tmp("corrupt");
        let f = d.join(FILE_NAME);
        std::fs::write(&f, "{ not json").unwrap();
        let db = PresetDb::load_from(&f);
        assert_eq!(db.presets(), [LayerPreset::example()]);
        assert!(d.join("LayerPresets.json.bak").exists());
        assert!(!f.exists());
    }

    #[test]
    fn missing_filter_defaults_to_all() {
        let d = tmp("nofilter");
        let f = d.join(FILE_NAME);
        std::fs::write(&f, r#"{"version":1,"presets":[{"name":"x","passes":1,"speed":100,"power_pct":10.0,"laser_on":"M3"}]}"#).unwrap();
        assert_eq!(PresetDb::load_from(&f).presets()[0].filter, "All");
    }

    #[test]
    fn missing_fill_fields_default() {
        let d = tmp("nofill");
        let f = d.join(FILE_NAME);
        std::fs::write(&f, r#"{"version":1,"presets":[{"name":"x","passes":1,"speed":100,"power_pct":10.0,"laser_on":"M3"}]}"#).unwrap();
        let p = PresetDb::load_from(&f).presets()[0].clone();
        assert_eq!((p.fill, p.fill_distance, p.fill_angle, p.fill_cross), (false, 0.1, 0.0, false));
        // old JSON without the field: per shape on
        assert!(p.fill_per_shape);
    }

    #[test]
    fn load_dedups_case_insensitive_last_wins() {
        let d = tmp("dedup");
        let f = d.join(FILE_NAME);
        let data = FileData { version: 1, presets: vec![preset("Foo", 1), preset("foo", 2)] };
        std::fs::write(&f, serde_json::to_string(&data).unwrap()).unwrap();
        let db = PresetDb::load_from(&f);
        assert_eq!((db.presets().len(), db.presets()[0].speed), (1, 2));
    }

    #[test]
    fn power_conversions() {
        let p = LayerPreset::example();
        assert_eq!(p.power_s(1000), 450);
        assert_eq!(p.power_s(255), 115);
        assert_eq!(p.power_s(0), 450);
        assert_eq!(LayerPreset::pct_from_s(450, 1000), 45.0);
        assert_eq!(LayerPreset::pct_from_s(115, 255), 45.1);
        assert_eq!(LayerPreset::pct_from_s(450, 0), 45.0);
    }
}
