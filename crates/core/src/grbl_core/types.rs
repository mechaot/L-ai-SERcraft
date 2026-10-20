//! Enums, event and result types of `Core/GrblCore.cs` (nested types of `GrblCore`, delegates, events).

use lg_dotnet::Decimal;
use lg_gcode::{Firmware, GPoint, GrblCommand, GrblMessage, GrblRow, RowColor};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Enum with the C# type name and variant name, used to build the `Strings.<Type><Value>` resource key of
/// `GrblCore.TranslateEnum`.
pub trait NamedEnum {
    /// C# enum type name (`MacStatus`, `DetectedIssue`, ...).
    const TYPE_NAME: &'static str;
    /// C# `ToString()` of the value.
    fn variant_name(&self) -> &'static str;
}

/// `GrblCore.TranslateEnum(value)`: resource key `<TypeName><Value>` (without the `Strings.` prefix).
///
/// `v`: enum value. Returns e.g. `MacStatusIdle`.
pub fn translate_enum_key<T: NamedEnum>(v: &T) -> String {
    format!("{}{}", T::TYPE_NAME, v.variant_name())
}

/// `GrblCore.TranslateEnum(value)` with a caller supplied resource lookup.
///
/// `v`: enum value. `lookup`: gets the key `<TypeName><Value>` and returns the translation (None/empty =
/// missing). Returns the translation or the variant name.
pub fn translate_enum<T: NamedEnum>(v: &T, lookup: &dyn Fn(&str) -> Option<String>) -> String {
    match lookup(&translate_enum_key(v)) {
        Some(s) if !s.is_empty() => s,
        _ => v.variant_name().to_string(),
    }
}

macro_rules! named_enum {
    ($t:ident { $($v:ident),+ $(,)? }) => {
        impl NamedEnum for $t {
            const TYPE_NAME: &'static str = stringify!($t);
            fn variant_name(&self) -> &'static str {
                match self { $( Self::$v => stringify!($v), )+ }
            }
        }
        impl fmt::Display for $t {
            /// C# `ToString()`.
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.variant_name())
            }
        }
    };
}

/// Poll/sleep profile (C# `ThreadingMode`). The C# class is stored as object; here it is stored by name
/// under "Threading Mode".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThreadingMode {
    Slow,
    Quiet,
    #[default]
    Fast,
    UltraFast,
    Insane,
}

impl ThreadingMode {
    /// `StatusQuery` ms.
    pub fn status_query(self) -> i32 {
        match self {
            Self::Slow => 2000,
            Self::Quiet => 1000,
            Self::Fast => 500,
            Self::UltraFast => 250,
            Self::Insane => 200,
        }
    }
    /// `TxLong` ms.
    pub fn tx_long(self) -> i32 {
        match self {
            Self::Slow => 15,
            Self::Quiet => 10,
            Self::Fast => 5,
            Self::UltraFast | Self::Insane => 1,
        }
    }
    /// `TxShort` ms.
    pub fn tx_short(self) -> i32 {
        match self {
            Self::Slow => 4,
            Self::Quiet => 2,
            Self::Fast => 1,
            Self::UltraFast | Self::Insane => 0,
        }
    }
    /// `RxLong` ms.
    pub fn rx_long(self) -> i32 {
        match self {
            Self::Slow => 2,
            Self::Quiet | Self::Fast => 1,
            Self::UltraFast | Self::Insane => 0,
        }
    }
    /// `RxShort` ms.
    pub fn rx_short(self) -> i32 {
        match self {
            Self::Slow | Self::Quiet => 1,
            _ => 0,
        }
    }
    /// All modes in C# declaration order.
    pub fn all() -> [ThreadingMode; 5] {
        [Self::Slow, Self::Quiet, Self::Fast, Self::UltraFast, Self::Insane]
    }
}
named_enum!(ThreadingMode { Slow, Quiet, Fast, UltraFast, Insane });

/// Why a job stopped (C# `DetectedIssue`). Negative values are caused by the user and not reported to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DetectedIssue {
    #[default]
    Unknown = 0,
    ManualReset = -1,
    ManualDisconnect = -2,
    ManualAbort = -3,
    StopResponding = 1,
    UnexpectedReset = 3,
    UnexpectedDisconnect = 4,
    MachineAlarm = 5,
}
named_enum!(DetectedIssue { Unknown, ManualReset, ManualDisconnect, ManualAbort, StopResponding, UnexpectedReset, UnexpectedDisconnect, MachineAlarm });

impl DetectedIssue {
    /// Numeric C# value.
    pub fn value(self) -> i32 {
        self as i32
    }
}

/// Machine state (C# `MacStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MacStatus {
    #[default]
    Disconnected,
    Connecting,
    Idle,
    Run,
    Hold,
    Door,
    Home,
    Alarm,
    Check,
    Jog,
    Queue,
    Cooling,
    AutoHold,
    /// "Tool" added in GrblHal.
    Tool,
}
named_enum!(MacStatus { Disconnected, Connecting, Idle, Run, Hold, Door, Home, Alarm, Check, Jog, Queue, Cooling, AutoHold, Tool });

impl MacStatus {
    /// `Enum.Parse(typeof(MacStatus), s)`: names are case sensitive, the text is trimmed, `a,b` ORs the values
    /// and plain numbers are accepted (all like .NET).
    ///
    /// `s`: text. Returns None where C# throws. NOTE: a numeric result outside the defined range is a valid
    /// (undefined) enum value in C#; it cannot be represented here and gives None.
    pub fn parse(s: &str) -> Option<MacStatus> {
        use MacStatus::*;
        const ALL: [MacStatus; 14] = [Disconnected, Connecting, Idle, Run, Hold, Door, Home, Alarm, Check, Jog, Queue, Cooling, AutoHold, Tool];
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let mut value: i64 = 0;
        for tok in s.split(',') {
            let tok = tok.trim();
            let first = tok.chars().next()?;
            if first.is_ascii_digit() || first == '-' || first == '+' {
                value |= tok.parse::<i64>().ok()?;
            } else {
                value |= ALL.iter().position(|m| m.variant_name() == tok)? as i64;
            }
        }
        usize::try_from(value).ok().and_then(|i| ALL.get(i).copied())
    }
}

/// Jog direction (C# `JogDirection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JogDirection {
    Abort,
    Home,
    N,
    S,
    W,
    E,
    NW,
    NE,
    SW,
    SE,
    Zup,
    Zdown,
    Position,
}
named_enum!(JogDirection { Abort, Home, N, S, W, E, NW, NE, SW, SE, Zup, Zdown, Position });

/// Streaming mode (C# `StreamingMode`), stored by name under "Streaming Mode".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StreamingMode {
    #[default]
    Buffered,
    Synchronous,
    RepeatOnError,
}
named_enum!(StreamingMode { Buffered, Synchronous, RepeatOnError });

/// Reason for a config refresh (C# `RefreshCause`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshCause {
    OnConnect,
    OnDialog,
    OnRead,
    OnExport,
    OnImport,
    OnWriteBegin,
    OnWriteEnd,
}
named_enum!(RefreshCause { OnConnect, OnDialog, OnRead, OnExport, OnImport, OnWriteBegin, OnWriteEnd });

impl NamedEnum for Firmware {
    const TYPE_NAME: &'static str = "Firmware";
    fn variant_name(&self) -> &'static str {
        match self {
            Firmware::Grbl => "Grbl",
            Firmware::Smoothie => "Smoothie",
            Firmware::Marlin => "Marlin",
            Firmware::VigoWork => "VigoWork",
        }
    }
}

impl NamedEnum for crate::com::WrapperType {
    const TYPE_NAME: &'static str = "WrapperType";
    fn variant_name(&self) -> &'static str {
        self.name()
    }
}

/// Hot-key override actions handled by `GrblCore.HotKeyOverride` (subset of `HotKeysManager.HotKey.Actions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideAction {
    PowerDefault,
    PowerUp,
    PowerDown,
    LinearDefault,
    LinearUp,
    LinearDown,
    RapidDefault,
    RapidUp,
    RapidDown,
}

/// Events the C# core raises (`event` members), sent over the channel given to `GrblCore::new`.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreEvent {
    /// `IssueDetected(issue)`; only issues > 0.
    IssueDetected(DetectedIssue),
    /// `MachineStatusChanged`.
    MachineStatusChanged,
    /// `OnFileLoading(elapsed, filename)`.
    FileLoading { elapsed_ms: i64, filename: String },
    /// `OnFileLoaded(elapsed, filename)`.
    FileLoaded { elapsed_ms: i64, filename: String },
    /// `OnOverrideChange`.
    OverrideChanged,
    /// `OnLoopCountChange(current)`.
    LoopCountChanged(Decimal),
    /// `JogStateChange(jog)`.
    JogStateChanged(bool),
    /// `OnAutoSizeDrawing`.
    AutoSizeDrawing,
    /// `OnZoomInDrawing`.
    ZoomInDrawing,
    /// `OnZoomOutDrawing`.
    ZoomOutDrawing,
    /// `OnProgramEnded`.
    ProgramEnded,
}

/// Row of the "sent" list (C# `IGrblRow`: either a sent command or a message from the board).
#[derive(Debug, Clone)]
pub enum SentRow {
    /// A command that was sent (shares its result with the queued clone, so status updates are visible).
    Command(GrblCommand),
    /// A message received from the board or generated by LaserGRBL.
    Message(GrblMessage),
}

impl GrblRow for SentRow {
    /// Delegates to the inner row.
    fn get_decoded_message(&self) -> String {
        match self {
            Self::Command(c) => c.get_decoded_message(),
            Self::Message(m) => m.get_decoded_message(),
        }
    }
    /// Delegates to the inner row.
    fn get_result(&self, decode: bool, erroronly: bool) -> Option<String> {
        match self {
            Self::Command(c) => c.get_result(decode, erroronly),
            Self::Message(m) => m.get_result(decode, erroronly),
        }
    }
    /// Delegates to the inner row.
    fn get_tool_tip(&self, decode: bool) -> Option<String> {
        match self {
            Self::Command(c) => c.get_tool_tip(decode),
            Self::Message(m) => m.get_tool_tip(decode),
        }
    }
    /// Delegates to the inner row.
    fn left_color(&self) -> RowColor {
        match self {
            Self::Command(c) => c.left_color(),
            Self::Message(m) => m.left_color(),
        }
    }
    /// Delegates to the inner row.
    fn right_color(&self) -> RowColor {
        match self {
            Self::Command(c) => c.right_color(),
            Self::Message(m) => m.right_color(),
        }
    }
    /// Delegates to the inner row.
    fn image_index(&self) -> i32 {
        match self {
            Self::Command(c) => c.image_index(),
            Self::Message(m) => m.image_index(),
        }
    }
}

/// Data for the `ResumeJobForm` dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct ResumeInfo {
    /// Commands executed so far.
    pub executed: i32,
    /// Commands sent so far.
    pub sent: i32,
    /// Commands of the job.
    pub target: i32,
    /// Last detected issue.
    pub last_issue: DetectedIssue,
    /// `Configuration.HomingEnabled`.
    pub homing_enabled: bool,
    /// Initial state of the "homing" checkbox.
    pub suggest_homing: bool,
    /// Initial state of the "set WCO" checkbox (and whether the option is offered).
    pub suggest_setwco: bool,
    /// `mTP.LastKnownWCO`.
    pub last_known_wco: GPoint,
}

/// Result of `GrblCore::run_program`.
#[derive(Debug, Clone, PartialEq)]
pub enum RunDecision {
    /// Fresh job was queued.
    Started,
    /// An earlier job was interrupted: show the resume dialog, then call `continue_program` or
    /// `run_program_from_start_user`.
    AskResume(ResumeInfo),
    /// `CanSendFile` was false: nothing happened.
    NotAllowed,
}

/// Data for the `RunFromPositionForm` dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFromPositionInfo {
    /// `LoadedFile.Count`.
    pub line_count: usize,
    /// `Configuration.HomingEnabled`.
    pub homing_enabled: bool,
}

/// What `GrblCore::open_file` decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenFileOutcome {
    /// G-code load started on a worker thread (`FileLoading` / `FileLoaded` events follow).
    GCodeLoading,
    /// Raster image: UI shows `RasterToLaserForm` for this file.
    Raster(PathBuf),
    /// SVG: UI shows `SvgToGCodeForm` for this file.
    Svg(PathBuf),
    /// `.lps` project: UI/services load the project.
    Project(PathBuf),
    /// Unknown extension: UI shows `Strings.UnsupportedFiletype`.
    Unsupported,
    /// `CanLoadNewFile` was false or the loader was busy: nothing happened.
    Busy,
}

/// Errors returned by the core.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreError {
    /// "No response received from grbl!" and similar (C# `TimeoutException`).
    Timeout(String),
    /// `WriteConfigException`: lines that were not answered `ok`; `message` is the C# `Message`.
    WriteConfig { message: String, errors: Vec<String> },
    /// Any other failure (C# generic exception text).
    Other(String),
}

impl fmt::Display for CoreError {
    /// Message text like the C# exception `Message`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout(m) | Self::Other(m) => f.write_str(m),
            Self::WriteConfig { message, .. } => f.write_str(message),
        }
    }
}

impl std::error::Error for CoreError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_names_and_keys() {
        assert_eq!(MacStatus::AutoHold.to_string(), "AutoHold");
        assert_eq!(translate_enum_key(&DetectedIssue::StopResponding), "DetectedIssueStopResponding");
        assert_eq!(translate_enum_key(&Firmware::VigoWork), "FirmwareVigoWork");
        assert_eq!(DetectedIssue::ManualAbort.value(), -3);
        assert_eq!(MacStatus::parse("Run"), Some(MacStatus::Run));
        assert_eq!(MacStatus::parse("run"), None);
        assert_eq!(MacStatus::parse(" Run "), Some(MacStatus::Run));
        assert_eq!(MacStatus::parse("3"), Some(MacStatus::Run));
        assert_eq!(MacStatus::parse("Hold,Door"), Some(MacStatus::Door));
        assert_eq!(MacStatus::parse("99"), None);
        assert_eq!(MacStatus::parse(""), None);
        assert_eq!(translate_enum(&JogDirection::NE, &|k| if k == "JogDirectionNE" { Some("Nord-Est".into()) } else { None }), "Nord-Est");
        assert_eq!(translate_enum(&JogDirection::NE, &|_| None), "NE");
    }

    #[test]
    fn threading_modes() {
        assert_eq!(ThreadingMode::Fast.status_query(), 500);
        assert_eq!((ThreadingMode::Fast.tx_long(), ThreadingMode::Fast.tx_short()), (5, 1));
        assert_eq!((ThreadingMode::Fast.rx_long(), ThreadingMode::Fast.rx_short()), (1, 0));
        assert_eq!(ThreadingMode::Insane.status_query(), 200);
        assert_eq!(serde_json::to_string(&ThreadingMode::UltraFast).unwrap(), "\"UltraFast\"");
    }
}

/// `System.Drawing.PointF`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

impl PointF {
    /// Ctor.
    ///
    /// `x`,`y`: coordinates.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
