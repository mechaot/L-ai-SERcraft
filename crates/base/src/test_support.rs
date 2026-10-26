// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! TEST SUPPORT (not for production code): one scratch root per test process, removed at process exit.
//!
//! Layout: `<temp>/lg-tests/<tag>-<pid>-<unix_secs>/`, created once per process.
//! - [`isolate_for_tests`] points data dir (`<root>/data`), settings file (`<root>/settings.json`) and
//!   app temp dir (`<root>/tmp`) into it.
//! - [`test_dir`] hands out fresh sub dirs of the root for a test's own scratch files.
//!
//! Cleanup: creating the root registers a C `atexit` handler. It flushes the settings saver and the
//! loggers, then deletes the root (and `lg-tests` when empty). libtest leaves through `exit`, so the
//! handler runs after the last test. Aborted or killed processes leave their root behind: the next
//! process that creates a root sweeps stale entries (older than one hour) in `<temp>/lg-tests/` and, once
//! more, leftovers of the pre-`lg-tests` naming (`LEGACY_PREFIXES`) directly in `<temp>`.
//!
//! Debugging: set `LG_KEEP_TEST_DIRS=1` to keep the root at exit (its path is printed to stderr).
//!
//! Rule: test code never builds paths from `std::env::temp_dir()`. `tools/check_test_isolation.py` runs every
//! test with its own `TMPDIR` and flags anything left in it.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Dir under the system temp dir that holds all per-process roots.
const PARENT: &str = "lg-tests";
/// Entries older than this are leftovers of dead processes.
const STALE_AFTER: Duration = Duration::from_secs(3600);
/// Name starts of scratch dirs/files the tests made directly in `<temp>` before `lg-tests` existed.
/// Legacy sweep only; drop the list once nobody has old leftovers around.
const LEGACY_PREFIXES: &[&str] = &[
    "lg_test_",
    "lg_services_",
    "lg_llc_",
    "lg_usage_",
    "lg_emu_",
    "lg_presets_",
    "lg_raster_dock_",
    "lg-params-",
    "lg-e2e-",
    "lg_migrate_",
    "lg_ser_",
    "lg_base_",
    "lg_logger_",
    "lg_settings_",
    "lg_core_it_",
    "lg_proto_",
    "lg_cbform_",
    "lg_layers_",
    "lg-gcode-",
    "lg-image-dock-",
    "lg-u4",
    "lg_iso_",
    "lg-forms-settings-",
    "lasergrbl_gallery",
];

/// Env var that keeps the scratch root at exit (debugging).
const KEEP_ENV: &str = "LG_KEEP_TEST_DIRS";

/// Per-process scratch root, set once.
static ROOT: OnceLock<PathBuf> = OnceLock::new();
/// Result of [`isolate_for_tests`], set once.
static ISOLATED: OnceLock<PathBuf> = OnceLock::new();
/// Counter that makes [`test_dir`] names unique.
static SEQ: AtomicU64 = AtomicU64::new(0);

// SAFETY: `atexit` is plain libc/CRT (glibc, macOS libc, Windows UCRT). We only pass a valid
// `extern "C" fn()` that never unwinds (see `at_exit`).
unsafe extern "C" {
    fn atexit(cb: extern "C" fn()) -> core::ffi::c_int;
}

/// TEST HELPER: keep a test process away from the real config dir and the real temp dir.
///
/// Built into the normal lib (not `cfg(test)`) so integration tests and other crates can call it.
/// Points the data dir at `<root>/data`, the settings store at `<root>/settings.json` and the app temp dir
/// ([`crate::paths::temp_path`]) at `<root>/tmp`. Session log, laser life counter, emulator config and the
/// rest resolve their paths through the data dir later, so they follow. Runs once per process (later calls
/// only return the root). Call it FIRST in every test that can reach settings, logger, laser life, emulator
/// config, usage stats, presets, custom buttons or the app temp dir, before anything caches the real path.
/// The root is deleted at process exit. `tools/check_test_isolation.py` enforces all this.
///
/// `tag`: short name of the caller (crate or test binary); unusual chars become `_`. Only the first call that
/// creates the root (this one or [`test_dir`]) names it.
/// Returns the scratch root dir.
pub fn isolate_for_tests(tag: &str) -> PathBuf {
    ISOLATED
        .get_or_init(|| {
            let root = root(tag).to_path_buf();
            crate::paths::set_data_path_override(root.join("data"));
            crate::paths::set_temp_path_override(root.join("tmp"));
            crate::settings::init_at(root.join("settings.json"));
            root
        })
        .clone()
}

/// TEST HELPER: fresh, empty, unique scratch dir for one test. Removed at process exit with the root.
///
/// Creates the process root when needed, so it works without [`isolate_for_tests`]. Use it instead of
/// `std::env::temp_dir()`.
///
/// `name`: short label for the dir name (unusual chars become `_`).
/// Returns the new dir (`<root>/<name>-<n>`).
pub fn test_dir(name: &str) -> PathBuf {
    let root = root(&exe_tag());
    let dir = root.join(format!("{}-{}", plain(name), SEQ.fetch_add(1, Ordering::Relaxed)));
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("cannot create test dir {}: {e}", dir.display()));
    dir
}

/// The process root; first call creates it, sweeps stale entries and registers the exit cleanup.
///
/// `tag`: name part of the root dir. Returns the root path. Panics when the root cannot be created.
fn root(tag: &str) -> &'static Path {
    ROOT.get_or_init(|| {
        let temp = std::env::temp_dir();
        let parent = temp.join(PARENT);
        // /tmp is shared: never follow a planted symlink
        if fs::symlink_metadata(&parent).is_ok_and(|m| m.file_type().is_symlink()) {
            panic!("{} is a symlink, refusing to use it as test scratch dir", parent.display());
        }
        let now = now_secs();
        sweep(&temp, &parent, now);
        let root = parent.join(format!("{}-{}-{now}", plain(tag), std::process::id()));
        // a parallel process may remove the empty parent between the two mkdirs: try again
        let mut made = fs::create_dir_all(&root);
        for _ in 0..5 {
            if made.is_ok() {
                break;
            }
            made = fs::create_dir_all(&root);
        }
        made.unwrap_or_else(|e| panic!("cannot create test root {}: {e}", root.display()));
        // SAFETY: see the extern block; `at_exit` is a valid `extern "C" fn()`. A failed registration only
        // means no cleanup at exit (the stale sweep gets it later), so the result is ignored.
        let _ = unsafe { atexit(at_exit) };
        root
    })
}

/// Process-exit handler: stop writers, delete the root. Never panics, never unwinds into C.
extern "C" fn at_exit() {
    let _ = std::panic::catch_unwind(|| {
        // nothing may write into the root after this
        crate::settings::flush_and_freeze();
        crate::logger::shutdown();
        let _ = crate::logger::com::stop_log();
        let Some(root) = ROOT.get() else { return };
        // debug aid: keep the files of a failed run for a look (the stale sweep removes them after an hour)
        if std::env::var_os(KEEP_ENV).is_some_and(|v| !v.is_empty()) {
            eprintln!("test scratch kept ({KEEP_ENV} is set): {}", root.display());
            return;
        }
        remove_root(root);
    });
}

/// Delete the root, then its parent when that is empty now.
///
/// `root`: per-process root. Retries a few times: a late writer thread can add a file while we delete.
/// Errors are ignored.
fn remove_root(root: &Path) {
    for _ in 0..5 {
        match fs::remove_dir_all(root) {
            Ok(()) => break,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    // fails (ignored) while roots of other processes live in it
    if let Some(parent) = root.parent() {
        let _ = fs::remove_dir(parent);
    }
}

/// Remove stale scratch entries: old roots in `<temp>/lg-tests/` and legacy leftovers in `<temp>`.
///
/// `temp`: system temp dir. `parent`: `<temp>/lg-tests`. `now`: unix seconds.
/// Only entries that match our names and are older than `STALE_AFTER` go; symlinks are never followed
/// or removed. Errors are ignored.
fn sweep(temp: &Path, parent: &Path, now: u64) {
    let limit = STALE_AFTER.as_secs();
    // new layout: age comes from the dir name (`<tag>-<pid>-<unix_secs>`), mtime when the name is odd
    if let Ok(rd) = fs::read_dir(parent) {
        for e in rd.flatten() {
            let name = e.file_name();
            let born = name.to_str().and_then(born_secs);
            let age = born.map(|b| now.saturating_sub(b)).or_else(|| mtime_age(&e, now));
            if age.is_some_and(|a| a > limit) {
                remove_entry(&e);
            }
        }
    }
    // legacy layout: names from the table, age from mtime
    if let Ok(rd) = fs::read_dir(temp) {
        for e in rd.flatten() {
            let name = e.file_name();
            let Some(name) = name.to_str() else { continue };
            if LEGACY_PREFIXES.iter().any(|p| name.starts_with(p)) && mtime_age(&e, now).is_some_and(|a| a > limit) {
                remove_entry(&e);
            }
        }
    }
}

/// Creation time encoded in a root dir name.
///
/// `name`: dir name `<tag>-<pid>-<unix_secs>`. Returns the unix seconds, `None` for any other shape.
fn born_secs(name: &str) -> Option<u64> {
    let mut it = name.rsplitn(3, '-');
    let secs = it.next()?.parse::<u64>().ok()?;
    it.next()?.parse::<u32>().ok()?;
    // tag part must exist
    it.next().filter(|t| !t.is_empty())?;
    Some(secs)
}

/// Age of a dir entry by its mtime.
///
/// `e`: entry. `now`: unix seconds. Returns seconds since the last change, `None` when unknown.
fn mtime_age(e: &fs::DirEntry, now: u64) -> Option<u64> {
    let m = e.metadata().ok()?.modified().ok()?;
    let secs = m.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(now.saturating_sub(secs))
}

/// Delete one dir entry (dir tree or file). Symlinks are skipped. Errors are ignored.
///
/// `e`: entry; its type comes from the entry itself, which does not follow links.
fn remove_entry(e: &fs::DirEntry) {
    let Ok(ft) = e.file_type() else { return };
    if ft.is_symlink() {
        return;
    }
    if ft.is_dir() {
        let _ = fs::remove_dir_all(e.path());
    } else {
        let _ = fs::remove_file(e.path());
    }
}

/// Plain dir-name part: ASCII letters, digits, `-`, `.`; the rest becomes `_`.
///
/// `s`: raw text. Returns the cleaned text (`x` when empty).
fn plain(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' }).collect();
    if t.is_empty() { "x".into() } else { t }
}

/// Name of the running test binary without cargo's `-<16 hex>` suffix.
///
/// Returns e.g. `lg_base` or `test` when unknown.
fn exe_tag() -> String {
    let stem = std::env::current_exe().ok().and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned())).unwrap_or_default();
    match strip_hash(&stem) {
        "" => "test".into(),
        t => t.to_string(),
    }
}

/// Cut cargo's `-<16 hex>` suffix off a test binary name.
///
/// `stem`: file name without extension. Returns the stem without the hash (unchanged when none).
fn strip_hash(stem: &str) -> &str {
    match stem.rsplit_once('-') {
        Some((head, hash)) if hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit()) => head,
        _ => stem,
    }
}

/// Unix seconds now. Returns 0 when the clock is before 1970.
fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_layout_and_isolation() {
        let root = isolate_for_tests("support test");
        // `<temp>/lg-tests/<tag>-<pid>-<secs>`
        assert_eq!(root.parent().unwrap(), std::env::temp_dir().join(PARENT));
        let name = root.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.contains(&format!("-{}-", std::process::id())), "{name}");
        assert!(born_secs(&name).is_some(), "{name}");
        // data/temp dir overrides are shared globals, `paths` tests check them; the dirs exist here
        assert!(root.join("data").is_dir() && root.join("tmp").is_dir());
        // second call keeps the first root
        assert_eq!(isolate_for_tests("other"), root);
    }

    #[test]
    fn test_dir_is_fresh_and_unique() {
        let a = test_dir("my test/x");
        let b = test_dir("my test/x");
        assert_ne!(a, b);
        assert!(a.is_dir() && b.is_dir());
        assert_eq!(std::fs::read_dir(&a).unwrap().count(), 0);
        assert_eq!(a.parent(), b.parent());
        assert!(a.file_name().unwrap().to_string_lossy().starts_with("my_test_x-"));
        assert!(a.starts_with(std::env::temp_dir().join(PARENT)));
    }

    #[test]
    fn name_helpers() {
        assert_eq!(plain("a b/c-d.e"), "a_b_c-d.e");
        assert_eq!(plain(""), "x");
        assert_eq!(strip_hash("lg_base-0123456789abcdef"), "lg_base");
        assert_eq!(strip_hash("lg_base-xyz"), "lg_base-xyz");
        assert_eq!(strip_hash("laisercraft"), "laisercraft");
        assert!(!exe_tag().is_empty());
        assert_eq!(born_secs("lg_base-123-1700000000"), Some(1_700_000_000));
        assert_eq!(born_secs("my-tag-9-5"), Some(5));
        assert_eq!(born_secs("odd"), None);
        assert_eq!(born_secs("a-b-5"), None);
        assert_eq!(born_secs("-1-5"), None);
        assert_eq!(born_secs("5"), None);
    }

    #[test]
    fn sweep_removes_only_stale_known_entries() {
        let sandbox = test_dir("sweep");
        let temp = sandbox.join("temp");
        let parent = temp.join(PARENT);
        fs::create_dir_all(&parent).unwrap();
        let now = now_secs();
        // new layout: age from the name
        let old = parent.join(format!("x-1-{}", now - 7200));
        let young = parent.join(format!("x-2-{}", now - 60));
        // odd names fall back to mtime (fresh now, so they only go once the clock moved)
        let odd = parent.join("odd");
        for d in [&old, &young, &odd] {
            fs::create_dir_all(d.join("sub")).unwrap();
            fs::write(d.join("sub").join("f"), "x").unwrap();
        }
        // legacy layout (dir + file) and an unrelated entry
        let legacy_dir = temp.join("lg_test_app_1");
        let legacy_file = temp.join("lg-image-dock-x-1.png");
        let other = temp.join("something_else");
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::create_dir_all(&other).unwrap();
        fs::write(&legacy_file, "x").unwrap();

        sweep(&temp, &parent, now);
        // only the root with an old name is gone; everything else is younger than an hour
        assert!(!old.exists());
        assert!(young.exists() && odd.exists() && legacy_dir.exists() && legacy_file.exists() && other.exists());

        // two hours later: mtime based entries are stale too, unknown names stay
        sweep(&temp, &parent, now + 7200);
        assert!(!young.exists() && !odd.exists() && !legacy_dir.exists() && !legacy_file.exists());
        assert!(other.exists());
    }

    #[cfg(unix)]
    #[test]
    fn sweep_skips_symlinks() {
        let sandbox = test_dir("sweep-link");
        let temp = sandbox.join("temp");
        let target = sandbox.join("target");
        fs::create_dir_all(&temp).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("keep"), "x").unwrap();
        let link = temp.join("lg_test_link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        sweep(&temp, &temp.join(PARENT), now_secs() + 10_000);
        assert!(link.symlink_metadata().is_ok());
        assert!(target.join("keep").is_file());
    }

    #[test]
    fn remove_root_deletes_tree_and_empty_parent() {
        let sandbox = test_dir("rm-root");
        let parent = sandbox.join(PARENT);
        let root = parent.join("a-1-2");
        fs::create_dir_all(root.join("deep").join("er")).unwrap();
        fs::write(root.join("deep").join("f"), "x").unwrap();
        remove_root(&root);
        assert!(!root.exists() && !parent.exists());
        // parent with a foreign root stays
        let (r1, r2) = (parent.join("a-1-2"), parent.join("b-3-4"));
        fs::create_dir_all(&r1).unwrap();
        fs::create_dir_all(&r2).unwrap();
        remove_root(&r1);
        assert!(!r1.exists() && r2.exists());
        // missing root is fine
        remove_root(&r1);
    }
}
