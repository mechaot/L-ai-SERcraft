//! Port of `UrlManager.cs`: endpoint configuration.
//!
//! In the public C# source every field is `null` (the real values live in a git `skip-worktree` file).
//! Same here: the port never contacts the upstream servers unless the app sets a value explicitly.

use std::sync::RwLock;

/// Endpoints; `None` = feature off.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UrlManager {
    pub update_main: Option<String>,
    pub update_mirror: Option<String>,
    pub statistics: Option<String>,
    pub telegram_handler: Option<String>,
    pub telegram_service_key: Option<String>,
    pub update_material_db: Option<String>,
    pub download_material_db: Option<String>,
    pub laser_statistics: Option<String>,
}

static URLS: RwLock<Option<UrlManager>> = RwLock::new(None);

/// Current endpoints (all None by default).
///
/// Returns a copy.
pub fn urls() -> UrlManager {
    URLS.read().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default()
}

/// Replace the endpoints.
///
/// `u`: new configuration.
pub fn set_urls(u: UrlManager) {
    *URLS.write().unwrap_or_else(|e| e.into_inner()) = Some(u);
}
