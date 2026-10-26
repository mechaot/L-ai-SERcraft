// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `GrblCore.DataPath`, `GrblCore.TempPath`, `GrblCore.ExePath` (Core/GrblCore.cs) + resource dir lookup.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static DATA: Mutex<Option<PathBuf>> = Mutex::new(None);
static TEMP: Mutex<Option<PathBuf>> = Mutex::new(None);
/// Scratch dir of [`isolate_for_tests`], set once per process.
static ISOLATED: OnceLock<PathBuf> = OnceLock::new();

/// Config folder names of older app names (migration sources), priority order.
const OLD_DIR_NAMES: [&str; 2] = ["L-ai-SERctrl", "LaserGRBL"];

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

/// Migrate the first existing old config dir under `base` into `new`.
///
/// `base`: config root holding the old dirs (see `OLD_DIR_NAMES`). `new`: target dir.
/// Returns the source dir used, or `None` when nothing was copied (`new` exists or no old dir).
fn migrate_first(base: &Path, new: &Path) -> std::io::Result<Option<PathBuf>> {
    // new exists: nothing to do
    if new.exists() {
        return Ok(None);
    }
    // first old dir that exists wins, rest ignored
    for name in OLD_DIR_NAMES {
        let old = base.join(name);
        if old.is_dir() {
            migrate_dir(&old, new)?;
            return Ok(Some(old));
        }
    }
    Ok(None)
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
/// Returns `<config_dir>/L-ai-SERcraft` or the override set by [`set_data_path_override`].
/// First call copies an old `<config_dir>/L-ai-SERctrl` folder over (else `<config_dir>/LaserGRBL`), once.
/// File names inside (`LaserGRBL.Settings.json`, `LayerPresets.json`, ...) are kept on purpose:
/// same names as the original app, so migration is a plain copy.
pub fn data_path() -> PathBuf {
    let mut g = DATA.lock().unwrap_or_else(|e| e.into_inner());
    // cache like C# static field
    if g.is_none() {
        let base = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
        let p = base.join(crate::APP_NAME);
        // one-time copy of the newest old folder; old stays untouched
        match migrate_first(&base, &p) {
            Ok(Some(old)) => log::info!("migrated config folder {} -> {}", old.display(), p.display()),
            Ok(None) => {}
            Err(e) => log::warn!("config folder migration failed: {e}"),
        }
        let _ = std::fs::create_dir_all(&p);
        *g = Some(p);
    }
    g.clone().unwrap_or_default()
}

/// Temp dir of the app, created on first call (C# `GrblCore.TempPath`).
///
/// Returns `<temp_dir>/L-ai-SERcraft`.
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
/// Order: `$LASERGRBL_RESOURCES`, `<exe>/resources`, `<exe>/../share/laisercraft/resources`,
/// `<exe>/../share/laiserctrl/resources` and `<exe>/../share/lasergrbl/resources` (old names),
/// `<exe>/../../resources`, compile-time workspace `resources`. Returns the first existing dir,
/// else the compile-time one.
pub fn resources_dir() -> PathBuf {
    let exe = exe_path();
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(v) = std::env::var_os("LASERGRBL_RESOURCES") {
        cands.push(PathBuf::from(v));
    }
    cands.push(exe.join("resources"));
    cands.push(exe.join("..").join("share").join("laisercraft").join("resources"));
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

/// TEST HELPER: keep a test process away from the real config dir. Not for production code.
///
/// Built into the normal lib (not `cfg(test)`) so integration tests and other crates can call it.
/// Points the data dir at `<temp>/lg_test_<tag>_<pid>/data` and the settings store at
/// `<temp>/lg_test_<tag>_<pid>/settings.json`. The session log, laser life counter, emulator config and
/// the rest resolve their paths through [`data_path`] later, so they follow. Runs once per process
/// (later calls only return the dir). Call it FIRST in every test that can reach settings, logger,
/// laser life, emulator config, usage stats, presets or custom buttons, before anything caches the real path.
/// `tools/check_test_isolation.py` enforces this.
///
/// `tag`: short name of the caller (crate or test binary); unusual chars become `_`. Only the first call's tag counts.
/// Returns the scratch root dir (data dir is `<root>/data`).
pub fn isolate_for_tests(tag: &str) -> PathBuf {
    ISOLATED
        .get_or_init(|| {
            // tag lands in a dir name: keep it plain
            let tag: String = tag.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
            let root = std::env::temp_dir().join(format!("lg_test_{tag}_{}", std::process::id()));
            // pid may be reused: start empty
            let _ = std::fs::remove_dir_all(&root);
            set_data_path_override(root.join("data"));
            crate::settings::init_at(root.join("settings.json"));
            root
        })
        .clone()
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
        // isolate helper first: same test fn, because both change the global data dir
        let root = isolate_for_tests("paths test");
        assert!(root.starts_with(std::env::temp_dir()));
        // tag is not asserted: other tests of this binary may have called the helper first
        assert!(root.to_string_lossy().contains("lg_test_"));
        assert!(root.to_string_lossy().ends_with(&format!("_{}", std::process::id())));
        assert!(root.join("data").is_dir());
        // second call keeps the first scratch dir
        assert_eq!(isolate_for_tests("other"), root);
        let d = std::env::temp_dir().join("lg_base_paths_test");
        set_data_path_override(d.clone());
        assert_eq!(data_path(), d);
        assert!(d.is_dir());
        assert!(temp_path().ends_with("L-ai-SERcraft"));
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

    #[test]
    fn migrate_first_priority() {
        let r = std::env::temp_dir().join(format!("lg_migrate_c_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        for (d, v) in [("LaserGRBL", "G"), ("L-ai-SERctrl", "C")] {
            std::fs::create_dir_all(r.join(d)).unwrap();
            std::fs::write(r.join(d).join("a.json"), v).unwrap();
        }
        let new = r.join("L-ai-SERcraft");
        // ctrl beats LaserGRBL
        assert_eq!(migrate_first(&r, &new).unwrap(), Some(r.join("L-ai-SERctrl")));
        assert_eq!(std::fs::read_to_string(new.join("a.json")).unwrap(), "C");
        // new exists now: nothing more
        assert_eq!(migrate_first(&r, &new).unwrap(), None);
        // only LaserGRBL left: fallback
        std::fs::remove_dir_all(r.join("L-ai-SERctrl")).unwrap();
        let new2 = r.join("new2");
        assert_eq!(migrate_first(&r, &new2).unwrap(), Some(r.join("LaserGRBL")));
        let _ = std::fs::remove_dir_all(&r);
    }
}
