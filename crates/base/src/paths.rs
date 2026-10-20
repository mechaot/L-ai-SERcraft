//! Port of `GrblCore.DataPath`, `GrblCore.TempPath`, `GrblCore.ExePath` (Core/GrblCore.cs) + resource dir lookup.

use std::path::PathBuf;
use std::sync::Mutex;

static DATA: Mutex<Option<PathBuf>> = Mutex::new(None);
static TEMP: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Config dir of the app, created on first call (C# `GrblCore.DataPath`).
///
/// Returns `<config_dir>/LaserGRBL` or the override set by [`set_data_path_override`].
pub fn data_path() -> PathBuf {
    let mut g = DATA.lock().unwrap_or_else(|e| e.into_inner());
    // cache like C# static field
    if g.is_none() {
        let p = dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("LaserGRBL");
        let _ = std::fs::create_dir_all(&p);
        *g = Some(p);
    }
    g.clone().unwrap_or_default()
}

/// Temp dir of the app, created on first call (C# `GrblCore.TempPath`).
///
/// Returns `<temp_dir>/LaserGRBL`.
pub fn temp_path() -> PathBuf {
    let mut g = TEMP.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        let p = std::env::temp_dir().join("LaserGRBL");
        let _ = std::fs::create_dir_all(&p);
        *g = Some(p);
    }
    g.clone().unwrap_or_default()
}

/// Directory of the running executable (C# `GrblCore.ExePath`).
///
/// Returns the exe dir, or `.` when unknown.
pub fn exe_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Find the `resources` dir.
///
/// Order: `$LASERGRBL_RESOURCES`, `<exe>/resources`, `<exe>/../share/lasergrbl/resources`,
/// `<exe>/../../resources`, compile-time workspace `resources`. Returns the first existing dir,
/// else the compile-time one.
pub fn resources_dir() -> PathBuf {
    let exe = exe_path();
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(v) = std::env::var_os("LASERGRBL_RESOURCES") {
        cands.push(PathBuf::from(v));
    }
    cands.push(exe.join("resources"));
    cands.push(exe.join("..").join("share").join("lasergrbl").join("resources"));
    cands.push(exe.join("..").join("..").join("resources"));
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("resources");
    cands.push(built.clone());
    cands.into_iter().find(|p| p.is_dir()).unwrap_or(built)
}

/// Override the data dir (tests, `--datadir`). Creates it.
///
/// `p`: new data dir.
pub fn set_data_path_override(p: PathBuf) {
    let _ = std::fs::create_dir_all(&p);
    *DATA.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resources_exist() {
        assert!(resources_dir().join("csv").is_dir());
    }

    #[test]
    fn override_works() {
        let d = std::env::temp_dir().join("lg_base_paths_test");
        set_data_path_override(d.clone());
        assert_eq!(data_path(), d);
        assert!(d.is_dir());
        assert!(temp_path().ends_with("LaserGRBL"));
        assert!(exe_path().is_dir());
    }
}
