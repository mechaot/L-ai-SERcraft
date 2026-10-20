//! Port of `GrblEmulator/*` (the `EmulatorUI` form is UI work, only its hook is here).

pub mod grbl_v11;
pub mod websocket_emulator;

pub use grbl_v11::{add_emulator_message_handler, clear_emulator_message_handlers, Grblv11Emulator};
pub use websocket_emulator::WebSocketEmulator;

use std::sync::{Arc, Mutex};

/// What the UI layer must provide for `EmulatorUI.ShowUI` / `HideUI`.
pub trait EmulatorUiHost: Send + Sync {
    /// Show the emulator window (C# `EmulatorUI.ShowUI(initmessage)`); `initmessage` is its first line.
    fn show_ui(&self, initmessage: &str);
    /// Hide it (C# `EmulatorUI.HideUI`).
    fn hide_ui(&self);
}

static HOST: Mutex<Option<Arc<dyn EmulatorUiHost>>> = Mutex::new(None);

/// Register (or clear) the UI host.
///
/// `host`: implementation or None.
pub fn set_ui_host(host: Option<Arc<dyn EmulatorUiHost>>) {
    *HOST.lock().unwrap_or_else(|e| e.into_inner()) = host;
}

/// `EmulatorUI.ShowUI`: forwards to the host; without host only the message goes to the log handlers.
///
/// `initmessage`: first line of the emulator log.
pub fn show_ui(initmessage: &str) {
    let h = HOST.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(h) = h {
        h.show_ui(initmessage);
    }
}

/// `EmulatorUI.HideUI`.
pub fn hide_ui() {
    let h = HOST.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(h) = h {
        h.hide_ui();
    }
}
