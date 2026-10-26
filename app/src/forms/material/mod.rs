// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, clippy::collapsible_if, clippy::collapsible_match, clippy::too_many_arguments, clippy::type_complexity)]
//! Named layer presets (replaces the C# material helper `PSHelper/*`, intentional deviation).
//!
//! The preset DB is a shared handle ([`SharedPresetDb`]); [`default_presets`] is the process wide
//! instance loaded from `<config dir>/L-ai-SERcraft/LayerPresets.json` (created with an example preset
//! on first start). [`PresetEditor`] lists and deletes presets, [`PresetPickerForm`] is the picker of
//! the raster import "P/S helper" button.

use lg_services::layer_presets::PresetDb;
use std::sync::{Arc, Mutex, OnceLock};

pub mod preset_editor;
pub mod preset_picker;

pub use preset_editor::PresetEditor;
pub use preset_picker::PresetPickerForm;

/// Shared preset database.
pub type SharedPresetDb = Arc<Mutex<PresetDb>>;

/// Process wide database, loaded on first use (eager at app start).
///
/// Returns a clone of the shared handle.
pub fn default_presets() -> SharedPresetDb {
    static DB: OnceLock<SharedPresetDb> = OnceLock::new();
    DB.get_or_init(|| Arc::new(Mutex::new(PresetDb::load()))).clone()
}
