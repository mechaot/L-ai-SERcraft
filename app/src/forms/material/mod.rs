#![allow(dead_code, clippy::collapsible_if, clippy::collapsible_match, clippy::too_many_arguments, clippy::type_complexity)]
//! Material power/speed helper (WP-U3): `PSHelper/PSHelperForm.cs` and `PSHelper/PSEditorForm.cs`.
//!
//! The C# `GrblCore.MaterialDB` static becomes a shared handle ([`SharedMaterialDb`]); the core
//! can hand its own instance to the forms, [`default_db`] is a process wide fallback.

use lg_services::material_db::MaterialDb;
use std::sync::{Arc, Mutex, OnceLock};

pub mod ps_editor;
pub mod ps_helper;

pub use ps_editor::MaterialEditor;
pub use ps_helper::PsHelperForm;

/// Shared material database (`GrblCore.MaterialDB`).
pub type SharedMaterialDb = Arc<Mutex<MaterialDb>>;

/// Process wide database loaded on first use (`MaterialDB.Load()`).
///
/// Returns a clone of the shared handle.
pub fn default_db() -> SharedMaterialDb {
    static DB: OnceLock<SharedMaterialDb> = OnceLock::new();
    DB.get_or_init(|| Arc::new(Mutex::new(MaterialDb::load()))).clone()
}
