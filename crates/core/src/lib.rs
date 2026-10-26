// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `lasergrbl-core`: GRBL streaming engine, communication links, emulator and support services.
//!
//! Ports `Core/GrblCore.cs` (+ Marlin/Smoothie/Vigo), `ComWrapper/*`, `GrblEmulator/Grblv11Emulator.cs`,
//! `GrblEmulator/WebsocketEmulator.cs`, `SincroStart.cs` and the local part of `UsageStats.cs`.

pub mod com;
pub mod emulator;
pub mod grbl_core;
pub mod laser_life;
pub mod sincro_start;
pub mod usage_stats;
pub mod util;

pub use grbl_core::*;

/// Shared test setup: keep unit tests out of the real config dir.
#[cfg(test)]
pub(crate) mod test_support {
    /// Point data path and settings at a scratch dir (once per test binary, see `lg_base::paths::isolate_for_tests`).
    /// Call first in every test that can reach settings, session log, laser life, usage stats or emulator config.
    pub fn init_env() {
        lg_base::paths::isolate_for_tests("core");
    }
}
