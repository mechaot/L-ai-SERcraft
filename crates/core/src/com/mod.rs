//! Port of `ComWrapper/IComWrapper.cs` (interface + `WrapperType`) and the wrapper factory used by
//! `GrblCore.Configure`.
//!
//! The C# interface has blocking reads. Here every wrapper owns a reader thread (or a pump) that fills a
//! [`LineQueue`]; `read_line_blocking` waits on it with a short timeout and returns `Ok(None)` on timeout
//! so the RX loop can re-check `is_open` and its exit flag (PLAN 2.1).

pub mod emulator_link;
pub mod fake;
pub mod ip_helper;
pub mod line_queue;
pub mod serial;
pub mod telnet;
pub mod websocket;

use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Arc;

pub use emulator_link::EmulatorCom;
pub use fake::FakeCom;
pub use ip_helper::parse_endpoint;
pub use line_queue::LineQueue;
pub use serial::{list_serial_ports, SerialCom};
pub use telnet::TelnetCom;
pub use websocket::LaserWebEsp8266Com;

/// Kind of link (C# `WrapperType`, same order). Stored by variant name under "ComWrapper Protocol".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WrapperType {
    #[default]
    UsbSerial,
    UsbSerial2,
    Telnet,
    LaserWebESP8266,
    Emulator,
    RJCPSerial,
}

impl WrapperType {
    /// C# `ToString()` of the enum.
    pub fn name(self) -> &'static str {
        match self {
            Self::UsbSerial => "UsbSerial",
            Self::UsbSerial2 => "UsbSerial2",
            Self::Telnet => "Telnet",
            Self::LaserWebESP8266 => "LaserWebESP8266",
            Self::Emulator => "Emulator",
            Self::RJCPSerial => "RJCPSerial",
        }
    }
}

/// Parameters of `IComWrapper.Configure(params object[])`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ComConfig {
    /// Serial wrappers: `(string)param[0]`, `(int)param[1]`.
    Serial { port_name: String, baud_rate: i32 },
    /// Telnet / websocket wrappers: `(string)param[0]`.
    Address(String),
    /// Emulator: no parameters.
    #[default]
    None,
}

/// Transport abstraction (C# `IComWrapper`). All methods take `&self`: reader and writer must not block
/// each other.
pub trait ComWrapper: Send + Sync {
    /// Which wrapper this is (C# `GetType()` check in `GrblCore.Configure`).
    fn wrapper_type(&self) -> WrapperType;
    /// Store the connection parameters. `conf`: parameters; a mismatching variant is ignored.
    fn configure(&self, conf: &ComConfig);
    /// Open the link. Returns Err with the message the UI shows in the connect error box.
    fn open(&self) -> io::Result<()>;
    /// Close the link. `auto`: true when closed by the core, false when closed by the user (log text only).
    fn close(&self, auto: bool);
    /// True while the link is up.
    fn is_open(&self) -> bool;
    /// Write one raw byte (real-time command). `b`: byte.
    fn write_byte(&self, b: u8) -> io::Result<()>;
    /// Write raw bytes. `data`: bytes.
    fn write_bytes(&self, data: &[u8]) -> io::Result<()>;
    /// Write text. `text`: string (ASCII, usually ends with `\n`).
    fn write_str(&self, text: &str) -> io::Result<()>;
    /// Wait a short time for one received line (without the `\n`). Returns `Ok(None)` on timeout or when the
    /// link is closed, `Err` when the link failed (core closes the connection).
    fn read_line_blocking(&self) -> io::Result<Option<String>>;
    /// True when a line (or unread bytes) is waiting.
    fn has_data(&self) -> bool;
}

/// Factory for `GrblCore.Configure`: new wrapper of the requested type.
///
/// `t`: wrapper type. Returns the new, closed and unconfigured wrapper.
pub fn create_wrapper(t: WrapperType) -> Arc<dyn ComWrapper> {
    match t {
        WrapperType::UsbSerial | WrapperType::UsbSerial2 | WrapperType::RJCPSerial => Arc::new(SerialCom::new(t)),
        WrapperType::Telnet => Arc::new(TelnetCom::new()),
        WrapperType::LaserWebESP8266 => Arc::new(LaserWebEsp8266Com::new()),
        WrapperType::Emulator => Arc::new(EmulatorCom::new()),
    }
}

/// Reset diagnostic text shown in the open log line (C# `GetResetDiagnosticString`).
///
/// `dtr`/`rts`: lines that are asserted. `soft`: ctrl-x soft reset on connect is enabled.
pub(crate) fn reset_diagnostic(dtr: bool, rts: bool, soft: bool) -> String {
    let mut rv = String::new();
    if dtr {
        rv.push_str("DTR, ");
    }
    if rts {
        rv.push_str("RTS, ");
    }
    if soft {
        rv.push_str("Ctrl-X, ");
    }
    rv.trim_matches(|c| c == ',' || c == ' ').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_text() {
        assert_eq!(reset_diagnostic(true, true, true), "DTR, RTS, Ctrl-X");
        assert_eq!(reset_diagnostic(false, false, true), "Ctrl-X");
        assert_eq!(reset_diagnostic(false, false, false), "");
    }

    #[test]
    fn wrapper_names_and_factory() {
        for t in [
            WrapperType::UsbSerial,
            WrapperType::UsbSerial2,
            WrapperType::Telnet,
            WrapperType::LaserWebESP8266,
            WrapperType::Emulator,
            WrapperType::RJCPSerial,
        ] {
            let w = create_wrapper(t);
            assert_eq!(w.wrapper_type(), t);
            assert!(!w.is_open());
            assert!(!t.name().is_empty());
        }
        assert_eq!(serde_json::to_string(&WrapperType::LaserWebESP8266).unwrap(), "\"LaserWebESP8266\"");
    }
}
