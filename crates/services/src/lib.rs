//! `lg-services`: non-UI services of LaserGRBL that do not need the `GrblCore` engine.
//!
//! * [`image_processor`] - `RasterConverter/ImageProcessor.cs` (preview + G-code generation threads)
//! * [`autotrace`] - `Autotrace/Autotrace.cs`
//! * [`project`] - `Tools/Project.cs` (.lps)
//! * [`hotkeys`] - `HotKeysManager.cs`
//! * [`custom_button`] - `CustomButton.cs`
//! * [`layer_presets`] - named layer presets (replaces `PSHelper/MaterialDB.cs`)
//! * [`flash_grbl`] - logic of `Firmware/FlashGrbl.cs`
//! * [`wifi_discovery`] - `WiFiDiscovery/IPAddressHelper.cs` + scan logic of `DiscoveryForm.cs`
//!
//! No update check: `AutoUpdate/*` and `Tools/Json.cs` (only used by it) are not ported.

// stylistic lints that fight with the line-by-line C# port
#![allow(clippy::too_many_arguments, clippy::needless_lifetimes, clippy::chunks_exact_to_as_chunks, clippy::manual_is_multiple_of, clippy::field_reassign_with_default, clippy::manual_checked_ops)]

pub mod autotrace;
pub mod custom_button;
pub mod flash_grbl;
pub mod hotkeys;
pub mod image_processor;
pub mod layer_presets;
pub mod project;
pub mod wifi_discovery;

/// Shared test setup: isolated settings file and data dir (the settings store is process global).
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Mutex, MutexGuard, Once};

    static INIT: Once = Once::new();
    static SETTINGS_GUARD: Mutex<()> = Mutex::new(());

    /// Point settings and data path to a scratch dir (once per test binary).
    pub fn init_env() {
        INIT.call_once(|| {
            let d = std::env::temp_dir().join(format!("lg_services_env_{}", std::process::id()));
            let _ = std::fs::create_dir_all(&d);
            lg_base::paths::set_data_path_override(d.join("data"));
            lg_base::settings::init_at(d.join("settings.json"));
        });
    }

    /// Serialise tests that change settings read by other tests.
    pub fn settings_lock() -> MutexGuard<'static, ()> {
        init_env();
        SETTINGS_GUARD.lock().unwrap_or_else(|e| e.into_inner())
    }
}
