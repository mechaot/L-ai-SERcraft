//! Port of the `CSVD` static class of `GrblCommand.cs` on top of `lg_base::csv`.

use crate::config::GrblVersionInfo;
use lg_base::csv::{alarms, errors, load_appropriate_csv, settings, CsvVersionParams};

/// `CSVD.Settings.GetItem(key, item)`.
///
/// `key`: setting number text. `item`: column (0 brief, 1 unit, 2 description).
pub fn settings_item(key: &str, item: usize) -> Option<String> {
    settings().get_item(key, item).map(str::to_string)
}

/// `CSVD.Alarms.GetItem(key, item)`.
///
/// `key`: alarm code text. `item`: column (0 brief, 1 description).
pub fn alarms_item(key: &str, item: usize) -> Option<String> {
    alarms().get_item(key, item).map(str::to_string)
}

/// `CSVD.Errors.GetItem(key, item)`.
///
/// `key`: error code text. `item`: column (0 brief, 1 description).
pub fn errors_item(key: &str, item: usize) -> Option<String> {
    errors().get_item(key, item).map(str::to_string)
}

/// `CSVD.LoadAppropriateCSV(value)`: pick dictionaries for the firmware.
///
/// `value`: detected grbl version.
pub fn load_appropriate(value: &GrblVersionInfo) {
    load_appropriate_csv(&CsvVersionParams {
        is_ortur: value.is_ortur(),
        is_hal: value.is_hal(),
        ortur_fw_version_number: value.ortur_fw_version_number(),
        is_longer: value.is_longer(),
        machine_name: value.machine_name().unwrap_or_default().to_string(),
        major: value.major(),
        minor: value.minor(),
    });
}
