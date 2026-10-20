//! Port of `Autotrace/Autotrace.cs`: run the external `autotrace` binary on a bitmap.
//!
//! The Windows build ships `Autotrace/autotrace.exe`; here the system binary is used (PLAN
//! section 6). Binary lookup: setting `"Autotrace Path"` (non-empty), else on Windows the bundled
//! `<exe>/Autotrace/autotrace.exe` when present, else `autotrace` from `PATH`.
//! `AutotraceTest.cs` is a developer test form (N/A).
//!
//! Deviation: the temp PNG goes to `<temp>/LaserGRBL/Autotrace/` (C# used `<data>\Autotrace\`).

use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

/// Setting key with an explicit path of the autotrace executable.
pub const SETTING_PATH: &str = "Autotrace Path";

/// Why a trace failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutotraceError {
    /// Executable not found / not startable.
    NotInstalled(String),
    /// Bitmap could not be written.
    Io(String),
    /// Process ended without producing output.
    Failed(String),
}

impl std::fmt::Display for AutotraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInstalled(p) => write!(
                f,
                "autotrace not found ('{p}'). Install the 'autotrace' package (e.g. 'apt install autotrace') or set the setting \"{SETTING_PATH}\"."
            ),
            Self::Io(s) => write!(f, "autotrace: {s}"),
            Self::Failed(s) => write!(f, "autotrace failed: {s}"),
        }
    }
}

impl std::error::Error for AutotraceError {}

/// C# `Autotrace.TempPath`: folder of the temporary PNGs (created on use).
///
/// Returns `<temp>/LaserGRBL/Autotrace`.
pub fn temp_path() -> PathBuf {
    lg_base::paths::temp_path().join("Autotrace")
}

/// C# `CleanupTmpFolder`: delete the temp folder, errors ignored.
pub fn cleanup_tmp_folder() {
    let _ = std::fs::remove_dir_all(temp_path());
}

/// Executable to start (see module doc).
///
/// Returns the program path or bare name.
pub fn executable() -> PathBuf {
    let configured: String = lg_base::settings::get(SETTING_PATH, String::new());
    if !configured.trim().is_empty() {
        return PathBuf::from(configured.trim());
    }
    if cfg!(windows) {
        let bundled = lg_base::paths::exe_path().join("Autotrace").join("autotrace.exe");
        if bundled.exists() {
            return bundled;
        }
        return PathBuf::from("autotrace.exe");
    }
    PathBuf::from("autotrace")
}

/// Argument list exactly as `BitmapToSvgString` builds it (without the quoting of the C# string).
///
/// `png`: input file. `uct`/`ct`: use corner threshold / value. `ult`/`lt`: use line threshold /
/// value (tenths, printed as `x.y`). Returns the arguments.
pub fn build_args(png: &Path, uct: bool, ct: i32, ult: bool, lt: i32) -> Vec<String> {
    let mut a: Vec<String> = ["-output-fo", "svg", "-background-color", "FFFFFF", "-centerline"].iter().map(|s| s.to_string()).collect();
    if uct {
        a.push("-corner-t".into());
        a.push(ct.to_string());
    }
    if ult {
        a.push("-line-t".into());
        // (lt / 10.0).ToString("0.0", Invariant)
        a.push(lg_dotnet::fmt_double_custom(lt as f64 / 10.0, "0.0"));
    }
    a.push(png.display().to_string());
    a
}

/// C# `ExecuteAutotrace(args)`: start the process, return its stdout.
///
/// `args`: argument list. Returns stdout text, Err when the program cannot start or exits
/// non-zero without output.
pub fn execute_autotrace(args: &[String]) -> Result<String, AutotraceError> {
    let exe = executable();
    let out = Command::new(&exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AutotraceError::NotInstalled(format!("{}: {e}", exe.display())))?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    // C# returns whatever the process printed; an empty failure is turned into a readable error
    if text.trim().is_empty() && !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(AutotraceError::Failed(format!("exit {:?}: {}", out.status.code(), err.trim())));
    }
    Ok(text)
}

/// `Path.GetRandomFileName()` replacement.
fn random_name() -> String {
    static N: AtomicU32 = AtomicU32::new(0);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{:x}{:x}{:x}", std::process::id(), t, N.fetch_add(1, Ordering::Relaxed))
}

/// C# `BitmapToSvgString`: save `bmp` as PNG, run autotrace, delete the PNG.
///
/// `bmp`: black/white bitmap. `uct`,`ct`,`ult`,`lt`: thresholds (see [`build_args`]).
/// Returns the SVG text or the error.
pub fn bitmap_to_svg_string(bmp: &RgbaImage, uct: bool, ct: i32, ult: bool, lt: i32) -> Result<String, AutotraceError> {
    let dir = temp_path();
    std::fs::create_dir_all(&dir).map_err(|e| AutotraceError::Io(e.to_string()))?;
    let png = dir.join(format!("{}.png", random_name()));
    let result = (|| {
        bmp.save_with_format(&png, image::ImageFormat::Png).map_err(|e| AutotraceError::Io(e.to_string()))?;
        execute_autotrace(&build_args(&png, uct, ct, ult, lt))
    })();
    // always remove the temp file
    let _ = std::fs::remove_file(&png);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_match_csharp_string() {
        let a = build_args(Path::new("/t/x.png"), true, 110, true, 10);
        assert_eq!(a.join(" "), "-output-fo svg -background-color FFFFFF -centerline -corner-t 110 -line-t 1.0 /t/x.png");
        let b = build_args(Path::new("x.png"), false, 0, true, 15);
        assert_eq!(b.join(" "), "-output-fo svg -background-color FFFFFF -centerline -line-t 1.5 x.png");
        let c = build_args(Path::new("x.png"), false, 0, false, 0);
        assert_eq!(c.len(), 6);
    }

    #[cfg(unix)]
    #[test]
    fn runs_configured_binary_and_cleans_up() {
        use std::os::unix::fs::PermissionsExt;
        // shares the global settings ("Autotrace Path") and the temp folder with the image processor tests
        let _g = crate::test_support::settings_lock();
        let d = std::env::temp_dir().join(format!("lg_services_at_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let script = d.join("fake-autotrace");
        // prints the args so the test can see them
        std::fs::write(&script, "#!/bin/sh\necho \"<svg>$*</svg>\"\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        lg_base::settings::set(SETTING_PATH, script.display().to_string());
        let img = RgbaImage::from_pixel(4, 4, image::Rgba([255, 255, 255, 255]));
        let svg = bitmap_to_svg_string(&img, true, 100, false, 0).unwrap();
        assert!(svg.starts_with("<svg>-output-fo svg"));
        assert!(svg.contains("-corner-t 100"));
        // no leftover png
        // a detached preview thread of another test may still own a png for a moment: wait for the folder to drain
        let count = || std::fs::read_dir(temp_path()).map(|r| r.filter_map(|e| e.ok()).filter(|e| e.path().extension().is_some_and(|x| x == "png")).count()).unwrap_or(0);
        let mut left = count();
        for _ in 0..40 {
            if left == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            left = count();
        }
        assert_eq!(left, 0);
        // missing binary gives a clear error
        lg_base::settings::set(SETTING_PATH, "/nonexistent/autotrace".to_string());
        let e = bitmap_to_svg_string(&img, false, 0, false, 0).unwrap_err();
        assert!(matches!(e, AutotraceError::NotInstalled(_)));
        assert!(e.to_string().contains("not found"));
        lg_base::settings::set(SETTING_PATH, String::new());
    }
}
