//! Shared application state: the engine, its event channel, custom buttons and hotkeys.
//!
//! Replaces the C# static/global wiring (`MainForm.Core`, `CustomButtons`, `HotKeysManager` owned by `GrblCore`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use lasergrbl_core::{CoreEvent, GrblCore};
use lg_gcode::Firmware;
use lg_services::custom_button::CustomButtons;
use lg_services::hotkeys::{HotKeyContext, HotKeysManager};

/// C# `UpdateTimer.Interval` is 200 ms, the port repaints every 100 ms.
pub const REPAINT_INTERVAL: Duration = Duration::from_millis(100);

/// Everything the forms share.
pub struct AppState {
    /// The engine (`MainForm.Core`).
    pub core: Arc<GrblCore>,
    /// Receiver of the core events, drained once per frame.
    events: Receiver<CoreEvent>,
    /// Custom button list (`CustomButtons` static class).
    pub custom_buttons: CustomButtons,
    /// Hotkey table (`GrblCore.HotKeys`).
    pub hotkeys: HotKeysManager,
    /// Set by the SincroStart listener thread: another instance asked this one to run the job.
    pub sincro_run: Arc<AtomicBool>,
    /// Process start, for the usage time record.
    pub started: Instant,
    /// Events seen but not yet consumed (kept for tests and for forms that peek).
    backlog: Mutex<Vec<CoreEvent>>,
}

impl AppState {
    /// Build the engine for the configured firmware and start the multi-instance listener.
    ///
    /// `firmware`: value of setting "Firmware Type". Returns the state.
    pub fn new(firmware: Firmware) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let core = GrblCore::new(firmware, tx);
        let mut custom_buttons = CustomButtons::new();
        custom_buttons.load_file();
        let hotkeys = HotKeysManager::load_from_settings();
        // SincroStart: listener thread only raises a flag, UI thread runs the countdown
        let sincro_run = Arc::new(AtomicBool::new(false));
        let flag = sincro_run.clone();
        let handler: lasergrbl_core::sincro_start::RunHandler = Box::new(move |_core| flag.store(true, Ordering::SeqCst));
        if let Err(e) = lasergrbl_core::sincro_start::start_listen(core.clone(), Some(handler)) {
            lg_base::logger::log_message("SincroStart", &format!("listen failed: {e}"));
        }
        AppState { core, events: rx, custom_buttons, hotkeys, sincro_run, started: Instant::now(), backlog: Mutex::new(Vec::new()) }
    }

    /// Take every pending core event.
    ///
    /// Returns the events in arrival order.
    pub fn drain_events(&self) -> Vec<CoreEvent> {
        let mut v: Vec<CoreEvent> = std::mem::take(&mut *self.backlog.lock().unwrap_or_else(|e| e.into_inner()));
        v.extend(self.events.try_iter());
        v
    }

    /// Facts the hotkey dispatcher needs.
    ///
    /// `custom_button_count`: number of buttons the preview form currently shows. Returns the context.
    pub fn hotkey_context(&self, custom_button_count: usize) -> HotKeyContext {
        HotKeyContext { custom_button_count, jog_enabled: self.core.jog_enabled() }
    }

    /// Take the "another instance asked to run" flag.
    ///
    /// Returns true once per request.
    pub fn take_sincro_run(&self) -> bool {
        self.sincro_run.swap(false, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sincro_flag_is_one_shot() {
        let flag = Arc::new(AtomicBool::new(true));
        assert!(flag.swap(false, Ordering::SeqCst));
        assert!(!flag.swap(false, Ordering::SeqCst));
    }
}
