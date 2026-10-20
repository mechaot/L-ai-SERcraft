//! Port of `Sound/SoundEvent.cs`: play event sounds without blocking the caller.
//!
//! C# uses `SoundPlayer.PlaySync` on a pool thread. Here a thread runs the first available external
//! player: `pw-play`, `paplay`, `aplay` (Linux), `afplay` (macOS), PowerShell `Media.SoundPlayer` (Windows).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

/// Event ids (C# `SoundEvent.EventId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventId {
    Success = 0,
    Warning = 1,
    Fatal = 2,
    Connect = 3,
    Disconnect = 4,
}

impl EventId {
    /// C# `id.ToString()`, used in settings keys.
    pub fn name(self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Fatal => "Fatal",
            Self::Connect => "Connect",
            Self::Disconnect => "Disconnect",
        }
    }
}

static BUSY: AtomicBool = AtomicBool::new(false);

/// Resolve the wav path of an event from settings.
///
/// `id`: event. Returns None when disabled (`Sound.<name>.Enabled` = false). The default file is
/// `resources/sound/<name lowercase>.wav`; a configured relative path (`Sound.<name>`) is taken
/// relative to the exe dir like C#.
pub fn sound_file(id: EventId) -> Option<PathBuf> {
    let name = id.name();
    if !crate::settings::get(&format!("Sound.{name}.Enabled"), true) {
        return None;
    }
    let default = crate::paths::resources_dir().join("sound").join(format!("{}.wav", name.to_lowercase()));
    let cfg: Option<String> = if crate::settings::exists(&format!("Sound.{name}")) {
        Some(crate::settings::get(&format!("Sound.{name}"), String::new()))
    } else {
        None
    };
    Some(match cfg {
        Some(c) if !c.is_empty() => crate::paths::exe_path().join(c),
        _ => default,
    })
}

/// Find an executable on PATH.
///
/// `name`: program name. Returns the full path.
#[cfg(not(target_os = "windows"))]
fn which(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).map(|d| d.join(name)).find(|p| p.is_file())
}

/// Build the player command for a file.
///
/// `file`: wav path. Returns None when no player exists.
fn player_command(file: &Path) -> Option<Command> {
    #[cfg(target_os = "windows")]
    {
        let mut c = Command::new("powershell");
        let script = format!("(New-Object Media.SoundPlayer '{}').PlaySync()", file.display().to_string().replace('\'', "''"));
        c.args(["-NoProfile", "-Command", &script]);
        Some(c)
    }
    #[cfg(target_os = "macos")]
    {
        which("afplay").map(|p| {
            let mut c = Command::new(p);
            c.arg(file);
            c
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        ["pw-play", "paplay", "aplay"].iter().find_map(|n| which(n)).map(|p| {
            let quiet = p.ends_with("aplay");
            let mut c = Command::new(p);
            if quiet {
                c.arg("-q");
            }
            c.arg(file);
            c
        })
    }
}

/// `SoundEvent.PlaySound`: play event sound, ignored when disabled, missing, or one is playing.
///
/// `id`: event to play.
pub fn play_sound(id: EventId) {
    if let Some(f) = sound_file(id) {
        play_file(&f);
    }
}

/// Play a wav file in the background (drops the request while another sound plays).
///
/// `file`: wav path; missing file or missing player is silently ignored.
pub fn play_file(file: &Path) {
    if !file.exists() || BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let file = file.to_path_buf();
    let spawned = std::thread::Builder::new().name("PlaySound".into()).spawn(move || {
        if let Some(mut c) = player_command(&file) {
            c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
            if let Ok(mut ch) = c.spawn() {
                let _ = ch.wait();
            }
        }
        BUSY.store(false, Ordering::SeqCst);
    });
    if spawned.is_err() {
        BUSY.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_files_exist() {
        crate::settings::init_at(std::env::temp_dir().join("lg_base_sound_test").join("s.json"));
        for id in [EventId::Success, EventId::Warning, EventId::Fatal, EventId::Connect, EventId::Disconnect] {
            let f = sound_file(id).unwrap();
            assert!(f.exists(), "{f:?}");
        }
        crate::settings::set("Sound.Fatal.Enabled", false);
        assert!(sound_file(EventId::Fatal).is_none());
        // must not panic or block
        play_file(Path::new("/nonexistent.wav"));
    }
}
