//! Ports `GrblEmulator/EmulatorUI.cs` + `EmulatorUI.Designer.cs`: log window of the built-in grbl emulator.
//!
//! [`EmulatorWindow`] implements `lasergrbl_core::emulator::EmulatorUiHost`; install it once with
//! [`EmulatorWindow::install`] and call [`EmulatorWindow::show`] every frame from the main window.

use eframe::egui::{self, RichText};
use lasergrbl_core::emulator::{self, EmulatorUiHost};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::common::trf;

/// Capacity of the log (`new RollingBuffer(30)`).
pub const LOG_LINES: usize = 30;

/// `RollingBuffer`: keeps the newest `capacity` lines.
#[derive(Debug, Clone)]
pub struct RollingBuffer {
    items: std::collections::VecDeque<String>,
    capacity: usize,
}

impl RollingBuffer {
    /// New buffer.
    ///
    /// `capacity`: maximum number of lines kept.
    pub fn new(capacity: usize) -> Self {
        RollingBuffer { items: std::collections::VecDeque::with_capacity(capacity), capacity }
    }

    /// `Add`: overwrite the oldest line when full.
    ///
    /// `item`: new line.
    pub fn add(&mut self, item: String) {
        if self.items.len() == self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    /// `Clear`.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// `Count`.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// `Capacity`.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// `ToArray`: oldest first.
    pub fn to_vec(&self) -> Vec<String> {
        self.items.iter().cloned().collect()
    }

    /// Text shown in the `RTB` (each line + CRLF in C#; LF here).
    pub fn text(&self) -> String {
        self.items.iter().map(|s| format!("{s}\n")).collect()
    }
}

/// Shared state: message buffer + visibility flag.
pub struct EmulatorWindow {
    buffer: Mutex<RollingBuffer>,
    visible: AtomicBool,
}

impl EmulatorWindow {
    /// Create the window state and register it as UI host and message handler.
    ///
    /// Returns the shared handle; keep it in the application and call [`show`](Self::show) per frame.
    pub fn install() -> Arc<EmulatorWindow> {
        let w = Arc::new(EmulatorWindow { buffer: Mutex::new(RollingBuffer::new(LOG_LINES)), visible: AtomicBool::new(false) });
        let w2 = w.clone();
        // Grblv11Emulator.EmulatorMessage += ...
        emulator::add_emulator_message_handler(Arc::new(move |m: Option<&str>| w2.message(m)));
        emulator::set_ui_host(Some(w.clone()));
        w
    }

    /// `Grblv11Emulator_EmulatorMessage`: None clears, text is appended.
    ///
    /// `m`: message from the emulator.
    pub fn message(&self, m: Option<&str>) {
        let mut b = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        match m {
            None => b.clear(),
            Some(s) => b.add(s.to_owned()),
        }
    }

    /// Window is shown.
    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::SeqCst)
    }

    /// Current log text.
    pub fn text(&self) -> String {
        self.buffer.lock().unwrap_or_else(|e| e.into_inner()).text()
    }

    /// Draw the window if visible (no close button, `CP_NOCLOSE_BUTTON`); call every frame.
    ///
    /// `ctx`: egui context.
    pub fn show(&self, ctx: &egui::Context) {
        if !self.is_visible() {
            return;
        }
        let text = self.text();
        let title = trf("EmulatorUI.$this.Text", "EmulatorUI");
        // StartPosition=CenterScreen, FormBorderStyle=FixedSingle (client 566x225)
        egui::Window::new(title).collapsible(false).resizable(false).fixed_size([566.0, 225.0]).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.content_rect().center()).show(ctx, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(text).monospace().size(11.0)).wrap());
            });
        });
        // RT timer: refresh while visible
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

impl EmulatorUiHost for EmulatorWindow {
    /// `EmulatorUI.ShowUI(initmessage)`: first call opens the window with `initmessage` as first line.
    ///
    /// `initmessage`: first log line.
    fn show_ui(&self, initmessage: &str) {
        if !self.visible.swap(true, Ordering::SeqCst) {
            // a new C# form instance starts with an empty buffer
            self.message(None);
            self.message(Some(initmessage));
        }
    }

    /// `EmulatorUI.HideUI`: hide the window.
    fn hide_ui(&self) {
        self.visible.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_buffer_overwrites_oldest() {
        let mut b = RollingBuffer::new(3);
        for i in 0..5 {
            b.add(format!("l{i}"));
        }
        assert_eq!((b.count(), b.capacity()), (3, 3));
        assert_eq!(b.to_vec(), ["l2", "l3", "l4"]);
        assert_eq!(b.text(), "l2\nl3\nl4\n");
        b.clear();
        assert_eq!(b.count(), 0);
    }

    #[test]
    fn host_show_hide_and_messages() {
        let w = EmulatorWindow { buffer: Mutex::new(RollingBuffer::new(LOG_LINES)), visible: AtomicBool::new(false) };
        w.show_ui("Emulator started");
        assert!(w.is_visible());
        // second ShowUI while open is ignored (static instance check)
        w.show_ui("again");
        w.message(Some("G0 X1"));
        assert_eq!(w.text(), "Emulator started\nG0 X1\n");
        w.message(None);
        assert_eq!(w.text(), "");
        w.hide_ui();
        assert!(!w.is_visible());
    }
}
