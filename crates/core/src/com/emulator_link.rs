//! Port of `ComWrapper/Emulator.cs`: the in-process emulator as a transport.

use super::line_queue::LineQueue;
use super::{ComConfig, ComWrapper, WrapperType};
use crate::emulator::Grblv11Emulator;
use lg_base::logger::com as comlog;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const POLL_WAIT: Duration = Duration::from_millis(100);

/// Emulator transport.
pub struct EmulatorCom {
    emu: Grblv11Emulator,
    queue: Arc<LineQueue>,
    opened: AtomicBool,
}

impl Default for EmulatorCom {
    /// Same as `new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl EmulatorCom {
    /// New closed link with a fresh emulator (config from the data dir).
    pub fn new() -> Self {
        Self::with_emulator(|sink| Grblv11Emulator::new(sink))
    }

    /// Link around a custom-built emulator (tests use another config file).
    ///
    /// `build`: gets the board-output sink and returns the emulator.
    pub fn with_emulator(build: impl FnOnce(crate::emulator::grbl_v11::SendMessage) -> Grblv11Emulator) -> Self {
        let queue = Arc::new(LineQueue::new());
        let q = queue.clone();
        // C# SendData: split on '\n', drop empty entries
        let sink: crate::emulator::grbl_v11::SendMessage = Box::new(move |message: &str| {
            for line in message.split('\n').filter(|s| !s.is_empty()) {
                q.push(line.to_string());
            }
        });
        Self { emu: build(sink), queue, opened: AtomicBool::new(false) }
    }
}

impl ComWrapper for EmulatorCom {
    /// See trait.
    fn wrapper_type(&self) -> WrapperType {
        WrapperType::Emulator
    }

    /// Nothing to configure.
    fn configure(&self, _conf: &ComConfig) {}

    /// Show the emulator UI, start the emulator.
    fn open(&self) -> io::Result<()> {
        if !self.opened.load(Ordering::SeqCst) {
            crate::emulator::show_ui("Grbl Emulator v1.1#");
            comlog::log_str("com", "Open Emulator");
            self.emu.open_com();
            self.queue.clear();
            self.opened.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    /// Stop the emulator.
    fn close(&self, auto: bool) {
        if self.opened.swap(false, Ordering::SeqCst) {
            comlog::log_str("com", &format!("Close Emulator [{}]", if auto { "CORE" } else { "USER" }));
            self.emu.close_com();
            self.queue.clear();
        }
    }

    /// Opened.
    fn is_open(&self) -> bool {
        self.opened.load(Ordering::SeqCst)
    }

    /// One real-time byte.
    fn write_byte(&self, b: u8) -> io::Result<()> {
        comlog::log("tx", &[b]);
        self.emu.manage_message(&[b]);
        Ok(())
    }

    /// Raw bytes.
    fn write_bytes(&self, data: &[u8]) -> io::Result<()> {
        comlog::log("tx", data);
        self.emu.manage_message(data);
        Ok(())
    }

    /// Text (UTF-8).
    fn write_str(&self, text: &str) -> io::Result<()> {
        comlog::log_str("tx", text);
        self.emu.manage_message(text.as_bytes());
        Ok(())
    }

    /// Wait up to 100 ms for a line; closed link returns `Ok(None)` after a short sleep.
    fn read_line_blocking(&self) -> io::Result<Option<String>> {
        if !self.is_open() {
            std::thread::sleep(Duration::from_millis(1));
            return Ok(None);
        }
        let r = self.queue.pop_timeout(POLL_WAIT)?;
        if let Some(l) = &r {
            comlog::log_str("rx", l);
        }
        Ok(r)
    }

    /// A line waits.
    fn has_data(&self) -> bool {
        self.is_open() && self.queue.has_data()
    }
}
