// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of the logic of `Firmware/FlashGrbl.cs` (the form itself is UI).
//!
//! Ported: firmware list from `resources/firmware/*.hex` with the C# name parsing, baud list,
//! port name purge, OK-button rule, the `avrdude` command line and a runner that streams the
//! tool output through a callback (C# let avrdude write to the console).
//! The system `avrdude` is used (setting `"Avrdude Path"` overrides); `-C<resources>/firmware/avrdude.conf`
//! is added when that file exists (the bundled Windows exe found its conf next to itself).

use chrono::NaiveDate;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;

/// C# `SELECT` item text of the firmware combo.
pub const SELECT_CUSTOM: &str = "--- select custom firmware ---";
/// C# `baudRates`.
pub const BAUD_RATES: [i32; 9] = [4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
/// Initial baud (`CbBaudRate.SelectedItem = 115200`).
pub const DEFAULT_BAUD: i32 = 115200;
/// Setting key with an explicit avrdude executable path.
pub const SETTING_AVRDUDE: &str = "Avrdude Path";

/// Target presets of `CbTarget` (index 0 = 115200 baud, 1 = 57600 baud).
///
/// `index`: selected target index. Returns the baud to select, None for other indexes.
pub fn baud_for_target(index: usize) -> Option<i32> {
    match index {
        0 => Some(115200),
        1 => Some(57600),
        _ => None,
    }
}

/// C# `UpdateBtnOK`: the flash button needs a port text, a firmware and a target.
///
/// `port_text`: text of the port combo. `firmware_selected`: a firmware item is selected.
/// `target_selected`: a target item is selected. Returns true when flashing may start.
pub fn can_flash(port_text: &str, firmware_selected: bool, target_selected: bool) -> bool {
    !port_text.trim().is_empty() && firmware_selected && target_selected
}

/// C# `InitPortCB` name purge (issue 31): drop a trailing non-digit char.
///
/// `portname`: name as listed by the OS. Returns the cleaned name.
pub fn purge_port_name(portname: &str) -> String {
    match portname.chars().last() {
        Some(c) if !c.is_ascii_digit() => {
            let mut s = portname.to_string();
            s.pop();
            s
        }
        _ => portname.to_string(),
    }
}

/// A `.hex` firmware (C# nested `Firmware`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Firmware {
    /// Full path of the file.
    pub path: PathBuf,
}

impl Firmware {
    /// C# ctor.
    ///
    /// `path`: hex file.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// C# `ToString`: `"{grblversion} {source} [{date}]"` for names like
    /// `01-v1.1h-original-20190825`, else the bare file name. The date is the invariant culture
    /// short date `MM/dd/yyyy` (C# used the user's culture).
    pub fn display_name(&self) -> String {
        let fname = self.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let arr: Vec<&str> = fname.split('-').collect();
        if arr.len() >= 4
            && let Ok(d) = NaiveDate::parse_from_str(arr[3], "%Y%m%d")
            && arr[3].len() == 8
        {
            return format!("{} {} [{}]", arr[1], arr[2], d.format("%m/%d/%Y"));
        }
        fname
    }
}

/// Folder of the bundled firmwares.
///
/// Returns `<resources>/firmware`.
pub fn firmware_folder() -> PathBuf {
    lg_base::paths::resources_dir().join("firmware")
}

/// C# `InitCBFirmware`: all `*.hex` files of a folder.
///
/// `folder`: directory. Returns firmwares sorted by file name (C# used directory order).
pub fn list_firmwares(folder: &Path) -> Vec<Firmware> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_file() && p.to_string_lossy().to_lowercase().ends_with(".hex")).collect())
        .unwrap_or_default();
    v.sort();
    v.into_iter().map(|p| Firmware::new(std::fs::canonicalize(&p).unwrap_or(p))).collect()
}

/// C# `AddOrSelect`: index of a firmware with the same path (case-insensitive), appending when new.
///
/// `list`: current items. `path`: custom hex path. Returns the index.
pub fn add_or_select(list: &mut Vec<Firmware>, path: &Path) -> usize {
    let lower = path.to_string_lossy().to_lowercase();
    if let Some(i) = list.iter().position(|f| f.path.to_string_lossy().to_lowercase() == lower) {
        return i;
    }
    list.push(Firmware::new(path.to_path_buf()));
    list.len() - 1
}

/// The avrdude argument list of `BtnOK_Click`:
/// `-patmega328p -b{baud} -P{com} -carduino -Uflash:w:"{firmware}":i`.
///
/// `com`: port text. `baud`: selected baud. `firmware`: hex path. `conf`: optional `avrdude.conf`
/// (adds `-C<conf>` first). Returns the arguments (the path needs no quotes, nothing re-parses it).
pub fn avrdude_args(com: &str, baud: i32, firmware: &Path, conf: Option<&Path>) -> Vec<String> {
    let mut a = Vec::new();
    if let Some(c) = conf {
        a.push(format!("-C{}", c.display()));
    }
    a.push("-patmega328p".into());
    a.push(format!("-b{baud}"));
    a.push(format!("-P{com}"));
    a.push("-carduino".into());
    a.push(format!("-Uflash:w:{}:i", firmware.display()));
    a
}

/// Executable to start: setting `"Avrdude Path"`, else bundled Windows exe, else `avrdude` on `PATH`.
pub fn avrdude_executable() -> PathBuf {
    let configured: String = lg_base::settings::get(SETTING_AVRDUDE, String::new());
    if !configured.trim().is_empty() {
        return PathBuf::from(configured.trim());
    }
    if cfg!(windows) {
        let bundled = lg_base::paths::exe_path().join("Firmware").join("avrdude.exe");
        if bundled.exists() {
            return bundled;
        }
        return PathBuf::from("avrdude.exe");
    }
    PathBuf::from("avrdude")
}

/// Run avrdude and stream its output.
///
/// `exe`: program. `args`: see [`avrdude_args`]. `on_output`: called with every text chunk of
/// stdout and stderr (avrdude ends progress lines with `\r`, chunks are split on `\r` and `\n`).
/// Returns the exit code (C# `retval`, 0 = ok); `Err(1)`-style failures of the C# `catch` are
/// `Err(message)` and the caller shows `retval = 1`.
pub fn run_avrdude(exe: &Path, args: &[String], on_output: &mut dyn FnMut(&str)) -> Result<i32, String> {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start '{}': {e} (install avrdude or set \"{SETTING_AVRDUDE}\")", exe.display()))?;
    let (tx, rx) = mpsc::channel::<String>();
    let mut readers = Vec::new();
    // one reader thread per pipe
    macro_rules! pump {
        ($pipe:expr) => {
            if let Some(mut p) = $pipe {
                let tx = tx.clone();
                readers.push(std::thread::spawn(move || {
                    let mut buf = [0u8; 512];
                    let mut acc = String::new();
                    while let Ok(n) = p.read(&mut buf) {
                        if n == 0 {
                            break;
                        }
                        acc.push_str(&String::from_utf8_lossy(&buf[..n]));
                        // flush complete segments
                        while let Some(i) = acc.find(['\r', '\n']) {
                            let seg: String = acc.drain(..=i).collect();
                            if tx.send(seg).is_err() {
                                return;
                            }
                        }
                    }
                    if !acc.is_empty() {
                        let _ = tx.send(acc);
                    }
                }));
            }
        };
    }
    pump!(child.stdout.take());
    pump!(child.stderr.take());
    drop(tx);
    for chunk in rx {
        on_output(&chunk);
    }
    for r in readers {
        let _ = r.join();
    }
    let st = child.wait().map_err(|e| e.to_string())?;
    Ok(st.code().unwrap_or(-1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names() {
        let f = |n: &str| Firmware::new(PathBuf::from(n)).display_name();
        assert_eq!(f("/x/01-v1.1h-original-20190825.hex"), "v1.1h original [08/25/2019]");
        assert_eq!(f("/x/02-v1.1h-custom, XY Homing-20190830.hex"), "v1.1h custom, XY Homing [08/30/2019]");
        assert_eq!(f("/x/mine.hex"), "mine");
        assert_eq!(f("/x/a-b-c-notadate.hex"), "a-b-c-notadate");
    }

    #[test]
    fn bundled_list() {
        let l = list_firmwares(&firmware_folder());
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].display_name(), "v1.1h original [08/25/2019]");
        assert_eq!(l[2].display_name(), "v1.1f original [08/01/2017]");
    }

    #[test]
    fn args_identical_to_csharp() {
        let a = avrdude_args("COM3", 115200, Path::new("C:\\fw\\a.hex"), None);
        assert_eq!(a.join(" "), "-patmega328p -b115200 -PCOM3 -carduino -Uflash:w:C:\\fw\\a.hex:i");
        let c = avrdude_args("/dev/ttyUSB0", 57600, Path::new("/f.hex"), Some(Path::new("/c.conf")));
        assert_eq!(c[0], "-C/c.conf");
        assert_eq!(c.len(), 6);
    }

    #[test]
    fn ui_rules() {
        assert!(can_flash("COM1", true, true));
        assert!(!can_flash(" ", true, true));
        assert!(!can_flash("COM1", false, true));
        assert_eq!(baud_for_target(1), Some(57600));
        assert_eq!(baud_for_target(2), None);
        assert_eq!(purge_port_name("COM3"), "COM3");
        assert_eq!(purge_port_name("COM3x"), "COM3");
        let mut l = vec![Firmware::new(PathBuf::from("/A.hex"))];
        assert_eq!(add_or_select(&mut l, Path::new("/a.HEX")), 0);
        assert_eq!(add_or_select(&mut l, Path::new("/b.hex")), 1);
    }

    #[cfg(unix)]
    #[test]
    fn streams_output_and_exit_code() {
        use std::os::unix::fs::PermissionsExt;
        let d = std::env::temp_dir().join(format!("lg_services_fg_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let s = d.join("fake-avrdude");
        std::fs::write(&s, "#!/bin/sh\necho out1\necho err1 >&2\nprintf 'prog\\rdone\\n'\nexit 3\n").unwrap();
        std::fs::set_permissions(&s, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut got = String::new();
        let rc = run_avrdude(&s, &[], &mut |c| got.push_str(c)).unwrap();
        assert_eq!(rc, 3);
        for t in ["out1", "err1", "prog", "done"] {
            assert!(got.contains(t), "{got}");
        }
        assert!(run_avrdude(Path::new("/nonexistent/avrdude"), &[], &mut |_| {}).is_err());
    }
}
