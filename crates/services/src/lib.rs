// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `lg-services`: non-UI services of LaserGRBL that do not need the `GrblCore` engine.
//!
//! * [`image_processor`] - `RasterConverter/ImageProcessor.cs` (preview + G-code generation threads)
//! * [`autotrace`] - `Autotrace/Autotrace.cs`
//! * [`project`] - `Tools/Project.cs` (.lps)
//! * [`raster_dock`] - backend of the raster-image dock (brightness to power map, no C# counterpart)
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
pub mod raster_dock;
pub mod wifi_discovery;

/// Shared test setup: isolated settings file and data dir (the settings store is process global).
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Mutex, MutexGuard};

    static SETTINGS_GUARD: Mutex<()> = Mutex::new(());

    /// Point settings and data path to a scratch dir (once per test binary, see `lg_base::paths::isolate_for_tests`).
    /// Call first in every test that can touch settings, logger or the data dir.
    pub fn init_env() {
        lg_base::paths::isolate_for_tests("services");
    }

    /// Write an executable shell script and make sure the kernel will run it.
    ///
    /// Another test thread may fork while we still hold the file open for writing; that child keeps the fd until
    /// its exec, and a spawn of the script fails meanwhile with ETXTBSY ("Text file busy"). So the script runs
    /// once here and that one error is retried: afterwards no writer is left.
    ///
    /// `path`: script file to create. `body`: script text, needs a shebang. Panics on I/O errors.
    #[cfg(unix)]
    pub fn write_script(path: &std::path::Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Stdio};
        const ETXTBSY: i32 = 26;
        std::fs::write(path, body).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        // warm-up run, up to 2 s of retries
        for _ in 0..200 {
            let run = Command::new(path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
            match run {
                Err(e) if e.raw_os_error() == Some(ETXTBSY) => std::thread::sleep(std::time::Duration::from_millis(10)),
                _ => break,
            }
        }
    }

    /// Serialise tests that change settings read by other tests.
    ///
    /// Returns the guard; keep it for the whole test.
    pub fn settings_lock() -> MutexGuard<'static, ()> {
        init_env();
        SETTINGS_GUARD.lock().unwrap_or_else(|e| e.into_inner())
    }
}
