// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `GrblCore.DataPath`, `GrblCore.TempPath`, `GrblCore.ExePath` (Core/GrblCore.cs) + resource dir lookup.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

static DATA: Mutex<Option<PathBuf>> = Mutex::new(None);
static TEMP: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Config folder name of the old LaserGRBL-named app (migration source).
const OLD_DIR_NAME: &str = "LaserGRBL";

/// Copy dir `old` into `new` once (recursive, files and subdirs). Never touches `old`.
///
/// `old`: source dir. `new`: target dir; when it already exists nothing is done.
/// Returns `Ok(true)` when copied, `Ok(false)` when skipped (`new` exists or `old` is no dir).
pub fn migrate_dir(old: &Path, new: &Path) -> std::io::Result<bool> {
    if new.exists() || !old.is_dir() {
        return Ok(false);
    }
    copy_tree(old, new)?;
    Ok(true)
}

/// Recursive dir copy.
///
/// `src`: source dir. `dst`: target dir, created. Returns io error of the first failure.
fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_tree(&e.path(), &to)?;
        } else {
            std::fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

/// Config dir of the app, created on first call (C# `GrblCore.DataPath`).
///
/// Returns `<config_dir>/L-ai-SERctrl` or the override set by [`set_data_path_override`].
/// First call copies an old `<config_dir>/LaserGRBL` folder over, once.
/// File names inside (`LaserGRBL.Settings.json`, `LayerPresets.json`, ...) are kept on purpose:
/// same names as the original app, so migration is a plain copy.
pub fn data_path() -> PathBuf {
    let mut g = DATA.lock().unwrap_or_else(|e| e.into_inner());
    // cache like C# static field
    if g.is_none() {
        let base = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
        let p = base.join(crate::APP_NAME);
        // one-time copy of the old folder; old stays untouched
        match migrate_dir(&base.join(OLD_DIR_NAME), &p) {
            Ok(true) => log::info!("migrated config folder {} -> {}", base.join(OLD_DIR_NAME).display(), p.display()),
            Ok(false) => {}
            Err(e) => log::warn!("config folder migration failed: {e}"),
        }
        let _ = std::fs::create_dir_all(&p);
        *g = Some(p);
    }
    g.clone().unwrap_or_default()
}

/// Temp dir of the app, created on first call (C# `GrblCore.TempPath`).
///
/// Returns `<temp_dir>/L-ai-SERctrl`.
pub fn temp_path() -> PathBuf {
    let mut g = TEMP.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        let p = std::env::temp_dir().join(crate::APP_NAME);
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
/// Order: `$LASERGRBL_RESOURCES`, `<exe>/resources`, `<exe>/../share/laiserctrl/resources`,
/// `<exe>/../share/lasergrbl/resources` (old name),
/// `<exe>/../../resources`, compile-time workspace `resources`. Returns the first existing dir,
/// else the compile-time one.
pub fn resources_dir() -> PathBuf {
    let exe = exe_path();
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(v) = std::env::var_os("LASERGRBL_RESOURCES") {
        cands.push(PathBuf::from(v));
    }
    cands.push(exe.join("resources"));
    cands.push(exe.join("..").join("share").join("laiserctrl").join("resources"));
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
        assert!(temp_path().ends_with("L-ai-SERctrl"));
        assert!(exe_path().is_dir());
    }

    #[test]
    fn migrate_copies_once() {
        let r = std::env::temp_dir().join(format!("lg_migrate_a_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        let (old, new) = (r.join("old"), r.join("new"));
        std::fs::create_dir_all(old.join("sub")).unwrap();
        std::fs::write(old.join("a.json"), "A").unwrap();
        std::fs::write(old.join("sub").join("b.txt"), "B").unwrap();
        assert!(migrate_dir(&old, &new).unwrap());
        assert_eq!(std::fs::read_to_string(new.join("a.json")).unwrap(), "A");
        assert_eq!(std::fs::read_to_string(new.join("sub").join("b.txt")).unwrap(), "B");
        // old stays
        assert!(old.join("a.json").is_file());
        // second run: new exists, nothing happens
        std::fs::write(new.join("a.json"), "CHANGED").unwrap();
        assert!(!migrate_dir(&old, &new).unwrap());
        assert_eq!(std::fs::read_to_string(new.join("a.json")).unwrap(), "CHANGED");
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn migrate_skips_when_new_exists_or_old_missing() {
        let r = std::env::temp_dir().join(format!("lg_migrate_b_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        let (old, new) = (r.join("old"), r.join("new"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("a.json"), "A").unwrap();
        assert!(!migrate_dir(&old, &new).unwrap());
        assert!(!new.join("a.json").exists());
        assert!(!migrate_dir(&r.join("missing"), &r.join("new2")).unwrap());
        let _ = std::fs::remove_dir_all(&r);
    }
}
