//! Static `CSVD` from `GrblCommand.cs`: Settings / Alarms / Errors dictionaries and version selection.

use super::dictionary::CsvDictionary;
use std::sync::{Arc, LazyLock, RwLock};

macro_rules! res {
    ($f:literal) => {
        include_str!(concat!("../../../../resources/csv/", $f))
    };
}

/// Embedded resource by C# manifest name suffix (`setting_codes.v1.1.csv`).
///
/// `name`: file name. Returns the text or None.
fn resource(name: &str) -> Option<&'static str> {
    Some(match name {
        "setting_codes.v1.1.csv" => res!("setting_codes.v1.1.csv"),
        "setting_codes.v0.9.csv" => res!("setting_codes.v0.9.csv"),
        "setting_codes.v0.8.csv" => res!("setting_codes.v0.8.csv"),
        "setting_codes.ortur.GrblHal.csv" => res!("setting_codes.ortur.GrblHal.csv"),
        "setting_codes.ortur.v1.7.x.csv" => res!("setting_codes.ortur.v1.7.x.csv"),
        "setting_codes.ortur.v1.5.x.csv" => res!("setting_codes.ortur.v1.5.x.csv"),
        "setting_codes.ortur.v1.4.x.csv" => res!("setting_codes.ortur.v1.4.x.csv"),
        "setting_codes.longer.nanoduo.csv" => res!("setting_codes.longer.nanoduo.csv"),
        "alarm_codes.csv" => res!("alarm_codes.csv"),
        "alarm_codes.ortur.GrblHal.csv" => res!("alarm_codes.ortur.GrblHal.csv"),
        "alarm_codes.longer.nanoduo.csv" => res!("alarm_codes.longer.nanoduo.csv"),
        "error_codes.csv" => res!("error_codes.csv"),
        "error_codes.ortur.GrblHal.csv" => res!("error_codes.ortur.GrblHal.csv"),
        "error_codes.longer.nanoduo.csv" => res!("error_codes.longer.nanoduo.csv"),
        _ => return None,
    })
}

/// Load resource by name.
///
/// `name`: resource name. `len`: data columns. Returns Err if missing or malformed.
fn load(name: &str, len: usize) -> Result<CsvDictionary, String> {
    CsvDictionary::from_text(resource(name).ok_or("missing resource")?, len)
}

/// Load, else fallback resource (C# catch block).
///
/// `name`: wanted. `fallback`: default resource. `len`: columns.
fn load_or(name: &str, fallback: &str, len: usize) -> Arc<CsvDictionary> {
    Arc::new(load(name, len).or_else(|_| load(fallback, len)).unwrap_or_default())
}

static SETTINGS: LazyLock<RwLock<Arc<CsvDictionary>>> = LazyLock::new(|| RwLock::new(load_or("setting_codes.v1.1.csv", "setting_codes.v1.1.csv", 3)));
static ALARMS: LazyLock<RwLock<Arc<CsvDictionary>>> = LazyLock::new(|| RwLock::new(load_or("alarm_codes.csv", "alarm_codes.csv", 2)));
static ERRORS: LazyLock<RwLock<Arc<CsvDictionary>>> = LazyLock::new(|| RwLock::new(load_or("error_codes.csv", "error_codes.csv", 2)));

/// The `GrblVersionInfo` fields `LoadAppropriateCSV` looks at.
#[derive(Debug, Clone, Default)]
pub struct CsvVersionParams {
    pub is_ortur: bool,
    pub is_hal: bool,
    pub ortur_fw_version_number: i32,
    pub is_longer: bool,
    pub machine_name: String,
    pub major: i32,
    pub minor: i32,
}

/// `CSVD.Settings`.
///
/// Returns the current settings dictionary (columns: brief, unit, description).
pub fn settings() -> Arc<CsvDictionary> {
    SETTINGS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// `CSVD.Alarms`.
///
/// Returns the current alarm dictionary (columns: brief, description).
pub fn alarms() -> Arc<CsvDictionary> {
    ALARMS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// `CSVD.Errors`.
///
/// Returns the current error dictionary (columns: brief, description).
pub fn errors() -> Arc<CsvDictionary> {
    ERRORS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Name of the settings resource for a version.
///
/// `v`: version info. Returns the resource name.
pub fn settings_resource_name(v: &CsvVersionParams) -> String {
    if v.is_ortur && v.is_hal {
        "setting_codes.ortur.GrblHal.csv".into()
    } else if v.is_ortur && v.ortur_fw_version_number >= 170 {
        "setting_codes.ortur.v1.7.x.csv".into()
    } else if v.is_ortur && v.ortur_fw_version_number >= 150 {
        "setting_codes.ortur.v1.5.x.csv".into()
    } else if v.is_ortur {
        "setting_codes.ortur.v1.4.x.csv".into()
    } else if v.is_longer && v.machine_name == "NanoDuo" {
        "setting_codes.longer.nanoduo.csv".into()
    } else {
        format!("setting_codes.v{}.{}.csv", v.major, v.minor)
    }
}

/// Name of the alarm / error resource.
///
/// `v`: version info. `base`: "alarm_codes" or "error_codes". Returns the resource name.
fn alarm_error_name(v: &CsvVersionParams, base: &str) -> String {
    if v.is_ortur && v.is_hal {
        format!("{base}.ortur.GrblHal.csv")
    } else if v.is_longer && v.machine_name == "NanoDuo" {
        format!("{base}.longer.nanoduo.csv")
    } else {
        format!("{base}.csv")
    }
}

/// `CSVD.LoadAppropriateCSV`: pick settings/alarm/error tables for the connected firmware.
///
/// `v`: version fields (see [`CsvVersionParams`]). Missing resources fall back to the defaults.
pub fn load_appropriate_csv(v: &CsvVersionParams) {
    *SETTINGS.write().unwrap_or_else(|e| e.into_inner()) = load_or(&settings_resource_name(v), "setting_codes.v1.1.csv", 3);
    *ALARMS.write().unwrap_or_else(|e| e.into_inner()) = load_or(&alarm_error_name(v, "alarm_codes"), "alarm_codes.csv", 2);
    *ERRORS.write().unwrap_or_else(|e| e.into_inner()) = load_or(&alarm_error_name(v, "error_codes"), "error_codes.csv", 2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_resources_parse() {
        for (n, l) in [
            ("setting_codes.v1.1.csv", 3),
            ("setting_codes.v0.9.csv", 3),
            ("setting_codes.v0.8.csv", 3),
            ("setting_codes.ortur.GrblHal.csv", 3),
            ("setting_codes.ortur.v1.7.x.csv", 3),
            ("setting_codes.ortur.v1.5.x.csv", 3),
            ("setting_codes.ortur.v1.4.x.csv", 3),
            ("setting_codes.longer.nanoduo.csv", 3),
            ("alarm_codes.csv", 2),
            ("alarm_codes.ortur.GrblHal.csv", 2),
            ("alarm_codes.longer.nanoduo.csv", 2),
            ("error_codes.csv", 2),
            ("error_codes.ortur.GrblHal.csv", 2),
            ("error_codes.longer.nanoduo.csv", 2),
        ] {
            let d = load(n, l).unwrap_or_else(|e| panic!("{n}: {e}"));
            assert!(d.len() > 5, "{n}");
        }
    }

    #[test]
    fn lookups_and_selection() {
        assert_eq!(errors().get_item("1", 0), Some("Expected command letter"));
        assert_eq!(alarms().get_item("1", 0), Some("Hard limit"));
        assert_eq!(settings().get_item("0", 1), Some("microseconds"));
        assert_eq!(settings().get_item("9999", 0), None);
        let v = CsvVersionParams { major: 0, minor: 9, ..Default::default() };
        assert_eq!(settings_resource_name(&v), "setting_codes.v0.9.csv");
        let o = CsvVersionParams { is_ortur: true, ortur_fw_version_number: 160, ..Default::default() };
        assert_eq!(settings_resource_name(&o), "setting_codes.ortur.v1.5.x.csv");
        // unknown version -> fallback v1.1
        load_appropriate_csv(&CsvVersionParams { major: 7, minor: 7, ..Default::default() });
        assert_eq!(settings().get_item("0", 1), Some("microseconds"));
    }
}
