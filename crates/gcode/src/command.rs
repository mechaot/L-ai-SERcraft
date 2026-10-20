//! Port of `GrblCommand.cs`: `Element`, `GrblCommand`, `GrblMessage`, `IGrblRow`.
//!
//! Colors of the C# `ColorScheme` become the semantic [`RowColor`]; UI maps them.
//! `LinkedDisplayList` (3D display-list invalidation) becomes [`DisplayListLink`]: a shared
//! generation counter bumped on every invalidation; the renderer polls it.

use crate::config::GrblConfST;
use crate::csvd;
use crate::text::{dn_lower, dn_trim, dn_upper};
use lg_dotnet::{fmt_decimal, is_letter, is_number, parse_decimal, Decimal, TimeSpan};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Semantic log-row colors (C# `ColorScheme.Log*` entries).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowColor {
    LogLeftCommand,
    LogRightGood,
    LogRightBad,
    LogRightOthers,
    LogLeftStartup,
    LogLeftAlarm,
    LogLeftConfig,
    LogLeftFeedback,
    LogLeftPosition,
    LogLeftOthers,
    /// `Color.Black`.
    Black,
}

/// C# `IGrblRow`: one row of the command log.
pub trait GrblRow {
    /// Text shown in the log (C# `GetDecodedMessage`). Null in C# becomes "".
    fn get_decoded_message(&self) -> String;
    /// Result text. `decode`: translate error codes; `erroronly`: hide non-error results.
    fn get_result(&self, decode: bool, erroronly: bool) -> Option<String>;
    /// Tooltip text. `decode`: translate error codes.
    fn get_tool_tip(&self, decode: bool) -> Option<String>;
    /// Left column color.
    fn left_color(&self) -> RowColor;
    /// Right column color.
    fn right_color(&self) -> RowColor;
    /// Icon index.
    fn image_index(&self) -> i32;
}

/// Shared invalidation counter (replaces C# `Object3DDisplayList.Invalidate()`).
///
/// Clones share the counter; renderer compares `generation()` with its last seen value.
#[derive(Debug, Clone, Default)]
pub struct DisplayListLink(Arc<AtomicU64>);

impl DisplayListLink {
    /// New link with generation 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the display list dirty.
    pub fn invalidate(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    /// Current dirty generation.
    pub fn generation(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// G-code word: letter + decimal number (C# `GrblCommand.Element`).
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub(crate) command: char,
    pub(crate) number: Decimal,
}

impl Element {
    /// C# ctor.
    ///
    /// `command`: letter. `number`: value (scale kept for printing).
    pub fn new(command: char, number: Decimal) -> Self {
        Self { command, number }
    }

    /// C# implicit conversion from string like `"X0"`: first char letter, rest decimal.
    ///
    /// `value`: text. Returns the element, or error where C# throws.
    pub fn parse(value: &str) -> crate::GResult<Element> {
        let mut it = value.chars();
        let c = it.next().ok_or_else(|| crate::GcodeError::Argument("empty".into()))?;
        let n = parse_decimal(it.as_str()).ok_or(crate::GcodeError::Overflow)?;
        Ok(Element::new(c, n))
    }

    /// Same as [`Element::parse`] for compile-time literals; panics on bad literal.
    ///
    /// `value`: literal like `"G91.1"`.
    pub fn lit(value: &str) -> Element {
        Element::parse(value).expect("bad element literal")
    }

    /// Letter.
    pub fn command(&self) -> char {
        self.command
    }

    /// Value.
    pub fn number(&self) -> Decimal {
        self.number
    }
}

impl fmt::Display for Element {
    /// `Command + Number.ToString(Invariant)`, decimal scale kept.
    ///
    /// `f`: formatter. Returns fmt result.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.command, fmt_decimal(self.number))
    }
}

/// Response state of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Queued,
    WaitingResponse,
    ResponseGood,
    ResponseBad,
    InvalidResponse,
}

/// Shared holder of the coded result. Clones of a command share one holder (C# `MemberwiseClone`).
#[derive(Debug, Default)]
struct ResultWrapper {
    coded_result: Mutex<Option<String>>,
}

/// One G-code line (C# `GrblCommand`, incl. subclass `JogCommand` as the `is_jog` flag).
#[derive(Debug, Clone)]
pub struct GrblCommand {
    line: String,
    result: Arc<ResultWrapper>,
    time_offset: TimeSpan,
    helper: Option<Vec<Element>>,
    repeat_count: i32,
    is_jog: bool,
    linked_display_list: Option<DisplayListLink>,
}

impl GrblCommand {
    /// C# `GrblCommand(line)`: trim + upper-case.
    ///
    /// `line`: raw text. Returns command.
    pub fn new(line: &str) -> Self {
        Self::with_options(line, 0, false)
    }

    /// C# `GrblCommand(line, repeat, preservecase)`.
    ///
    /// `line`: raw text. `repeat`: retry count. `preservecase`: skip upper-casing.
    pub fn with_options(line: &str, repeat: i32, preservecase: bool) -> Self {
        let mut l = dn_trim(line).to_string();
        if !preservecase {
            l = dn_upper(&l);
        }
        Self {
            line: l,
            result: Arc::new(ResultWrapper::default()),
            time_offset: TimeSpan::ZERO,
            helper: None,
            repeat_count: repeat,
            is_jog: false,
            linked_display_list: None,
        }
    }

    /// C# `JogCommand(line)` = `base(line.Substring(3))` (drops `$J=`).
    ///
    /// `line`: text starting with 3 chars to drop. Returns None when shorter than 3 (C# throws).
    pub fn new_jog(line: &str) -> Option<Self> {
        let mut units = 0usize;
        let mut idx = line.len();
        for (i, c) in line.char_indices() {
            if units >= 3 {
                idx = i;
                break;
            }
            units += c.len_utf16();
        }
        if units < 3 {
            return None;
        }
        let mut c = Self::new(&line[idx..]);
        c.is_jog = true;
        Some(c)
    }

    /// C# `GrblCommand(IEnumerable<Element>)`: words joined by space.
    ///
    /// `elements`: words. Returns command.
    pub fn from_elements<'a, I: IntoIterator<Item = &'a Element>>(elements: I) -> Self {
        let mut line = String::new();
        for e in elements {
            line.push_str(&e.to_string());
            line.push(' ');
        }
        let mut c = Self::new("");
        c.line = dn_trim(&dn_upper(&line)).to_string();
        c
    }

    /// C# `GrblCommand(Element first, GrblCommand toappend)`.
    ///
    /// `first`: leading word. `toappend`: command whose line follows. Returns command.
    pub fn from_first_and_append(first: &Element, toappend: &GrblCommand) -> Self {
        let mut c = Self::new("");
        c.line = dn_trim(&dn_upper(&format!("{} {}", first, toappend.line))).to_string();
        c
    }

    /// C# `ClearResult()`: fresh wrapper (no longer shared with clones), invalidates display list.
    pub fn clear_result(&mut self) {
        self.result = Arc::new(ResultWrapper::default());
        if let Some(l) = &self.linked_display_list {
            l.invalidate();
        }
    }

    /// True when `result` wrapper is the same object (C# reference equality of clones).
    ///
    /// `other`: other command. Returns true when they share the result holder.
    pub fn shares_result_with(&self, other: &GrblCommand) -> bool {
        Arc::ptr_eq(&self.result, &other.result)
    }

    /// Raw coded result (C# `mCodedResult`). None = queued.
    pub fn coded_result(&self) -> Option<String> {
        self.result.coded_result.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Set raw coded result.
    ///
    /// `v`: new value (None = queued, "" = sent).
    pub fn set_coded_result(&self, v: Option<String>) {
        *self.result.coded_result.lock().unwrap_or_else(|e| e.into_inner()) = v;
    }

    /// Display-list link (3D invalidation); `None` clears it (C# `Dispose`).
    ///
    /// `link`: new link.
    pub fn set_linked_display_list(&mut self, link: Option<DisplayListLink>) {
        self.linked_display_list = link;
    }

    /// Current display-list link.
    pub fn linked_display_list(&self) -> Option<&DisplayListLink> {
        self.linked_display_list.as_ref()
    }

    /// C# `Dispose()`: drops the display-list link.
    pub fn dispose(&mut self) {
        self.linked_display_list = None;
    }

    /// True when `build_helper` already ran (and `delete_helper` not yet).
    pub fn just_built(&self) -> bool {
        self.helper.is_some()
    }

    /// C# `BuildHelper()`: parse words, rewrite the line (comments removed, spaces collapsed).
    ///
    /// Any parse failure (duplicate letter, empty number, bad number) aborts silently: words
    /// parsed so far stay, and the line is left unmodified (the rewrite is after the loop).
    pub fn build_helper(&mut self) {
        // errors swallowed on purpose, like the C# catch {}
        let _ = self.build_helper_report();
    }

    /// Same as [`build_helper`](Self::build_helper) but tells which exception C# swallowed.
    ///
    /// Returns None when parsing went fine (or the helper existed already).
    pub fn build_helper_report(&mut self) -> Option<BuildFailure> {
        if self.just_built() {
            return None;
        }
        self.build_helper_inner().err()
    }

    /// Body of `BuildHelper` in a fallible fn so `?` models the thrown exceptions.
    ///
    /// Returns Err where C# would have thrown.
    fn build_helper_inner(&mut self) -> Result<(), BuildFailure> {
        self.helper = Some(Vec::new());
        let mut cmd = '\0';
        let mut num = String::new();
        let mut comment = false;
        let mut oldspace = false;
        let mut sb = String::new();

        // grbl `$` commands are not parsed
        if self.is_grbl_command() {
            return Ok(());
        }
        let chars: Vec<char> = self.line.chars().collect();
        for &c in &chars {
            // semicolon: rest of line is comment
            if c == ';' {
                break;
            }
            if c == '(' {
                comment = true;
            }
            // collapse runs of spaces, skip comment text
            let space = c == ' ';
            if !comment {
                if space && !oldspace {
                    sb.push(' ');
                } else if !space {
                    sb.push(c);
                }
            }
            oldspace = space;

            if !comment {
                if is_letter(c) {
                    // close previous word
                    if cmd != '\0' {
                        self.add(Element::new(cmd, parse_num(&num)?))?;
                    }
                    cmd = c;
                    num.clear();
                } else if is_number(c) || c == '.' || c == '-' {
                    num.push(c);
                }
            }
            if c == ')' {
                comment = false;
            }
        }
        self.line = sb;
        // last word
        if cmd != '\0' {
            self.add(Element::new(cmd, parse_num(&num)?))?;
        }
        Ok(())
    }

    /// `Dictionary.Add`: fails on duplicate letter.
    ///
    /// `element`: word to store. Err(()) when the letter exists already.
    fn add(&mut self, element: Element) -> Result<(), BuildFailure> {
        let h = self.helper.as_mut().ok_or(BuildFailure::DuplicateLetter)?;
        if h.iter().any(|e| e.command == element.command) {
            return Err(BuildFailure::DuplicateLetter);
        }
        h.push(element);
        Ok(())
    }

    /// Parsed words in insertion order (empty before build).
    pub fn elements(&self) -> &[Element] {
        self.helper.as_deref().unwrap_or(&[])
    }

    /// Retry count.
    pub fn repeat_count(&self) -> i32 {
        self.repeat_count
    }

    /// C# `DeleteHelper()`.
    pub fn delete_helper(&mut self) {
        self.helper = None;
    }

    /// C# `SetOffset`.
    ///
    /// `time`: cumulative time at this command.
    pub fn set_offset(&mut self, time: TimeSpan) {
        self.time_offset = time;
    }

    /// C# `TimeOffset`.
    pub fn time_offset(&self) -> TimeSpan {
        self.time_offset
    }

    /// Whether this is a `JogCommand`.
    pub fn is_jog(&self) -> bool {
        self.is_jog
    }

    /// C# `Command` (current line text).
    pub fn command(&self) -> &str {
        &self.line
    }

    /// C# `SerialData`: line + `\n`, spaces stripped unless `$` command.
    pub fn serial_data(&self) -> String {
        let t = self.line.trim_matches(|c| c == '\r' || c == '\n' || c == ' ');
        if self.can_compress() {
            let mut s = t.replace(' ', "");
            s.push('\n');
            s
        } else {
            format!("{t}\n")
        }
    }

    /// Spaces may be stripped when not a `$` command.
    fn can_compress(&self) -> bool {
        !self.is_grbl_command()
    }

    /// C# `Status` from coded result prefix.
    pub fn status(&self) -> CommandStatus {
        match self.coded_result() {
            None => CommandStatus::Queued,
            Some(r) if r.is_empty() => CommandStatus::WaitingResponse,
            Some(r) if r.starts_with("OK") => CommandStatus::ResponseGood,
            Some(r) if r.starts_with("ERROR") => CommandStatus::ResponseBad,
            Some(_) => CommandStatus::InvalidResponse,
        }
    }

    /// C# `SetResult`: upper-case + trim, invalidates display list.
    ///
    /// `result`: raw text from grbl (`ok`, `error:3`). `_decode`: unused in C# too.
    pub fn set_result(&self, result: &str, _decode: bool) {
        self.set_coded_result(Some(dn_trim(&dn_upper(result)).to_string()));
        if let Some(l) = &self.linked_display_list {
            l.invalidate();
        }
    }

    /// C# internal `SetSending()`: result becomes "" (waiting).
    pub fn set_sending(&self) {
        self.set_coded_result(Some(String::new()));
    }

    /// `$...` command (not parsed, not compressed).
    pub fn is_grbl_command(&self) -> bool {
        self.line.starts_with('$')
    }

    /// Empty line.
    pub fn is_empty(&self) -> bool {
        self.line.is_empty()
    }

    /// `$n=v` that writes EEPROM.
    pub fn is_write_eeprom(&self) -> bool {
        self.is_grbl_command() && GrblConfST::is_set_conf(&self.line)
    }

    /// `G92` (set WCO).
    pub fn is_set_wco(&self) -> bool {
        self.g().is_some_and(|g| g.number == Decimal::from(92))
    }

    // ---- G codes ----

    /// G word.
    pub fn g(&self) -> Option<&Element> {
        self.get_element('G')
    }

    /// Linear or arc movement.
    pub fn is_movement(&self) -> bool {
        self.is_linear_movement() || self.is_arc_movement()
    }

    /// X/Y/Z present, no I/J/R, not G92.
    pub fn is_linear_movement(&self) -> bool {
        !self.is_set_wco()
            && (self.x().is_some() || self.y().is_some() || self.z().is_some())
            && (self.i().is_none() && self.j().is_none() && self.r().is_none())
    }

    /// I/J/R present, not G92.
    pub fn is_arc_movement(&self) -> bool {
        !self.is_set_wco() && (self.i().is_some() || self.j().is_some() || self.r().is_some())
    }

    /// Clockwise flag.
    ///
    /// `prev`: value to keep when the command is neither G2 nor G3.
    pub fn is_cw(&self, prev: bool) -> bool {
        match self.g() {
            Some(g) if g.number == Decimal::from(2) => true,
            Some(g) if g.number == Decimal::from(3) => false,
            _ => prev,
        }
    }

    /// G4.
    pub fn is_pause(&self) -> bool {
        self.g().is_some_and(|g| g.number == Decimal::from(4))
    }

    /// G90.
    pub fn is_absolute_coord(&self) -> bool {
        self.g().is_some_and(|g| g.number == Decimal::from(90))
    }

    /// G91.
    pub fn is_relative_coord(&self) -> bool {
        self.g().is_some_and(|g| g.number == Decimal::from(91))
    }

    // ---- M codes ----

    /// M word.
    pub fn m(&self) -> Option<&Element> {
        self.get_element('M')
    }

    /// M3 or M4.
    pub fn is_laser_on(&self) -> bool {
        self.is_m3() || self.is_m4()
    }

    /// M3.
    pub fn is_m3(&self) -> bool {
        self.m().is_some_and(|m| m.number == Decimal::from(3))
    }

    /// M4.
    pub fn is_m4(&self) -> bool {
        self.m().is_some_and(|m| m.number == Decimal::from(4))
    }

    /// M5.
    pub fn is_laser_off(&self) -> bool {
        self.is_m5()
    }

    /// M5.
    pub fn is_m5(&self) -> bool {
        self.m().is_some_and(|m| m.number == Decimal::from(5))
    }

    // ---- parameters ----

    /// T word.
    pub fn t(&self) -> Option<&Element> {
        self.get_element('T')
    }
    /// S word.
    pub fn s(&self) -> Option<&Element> {
        self.get_element('S')
    }
    /// P word.
    pub fn p(&self) -> Option<&Element> {
        self.get_element('P')
    }
    /// X word.
    pub fn x(&self) -> Option<&Element> {
        self.get_element('X')
    }
    /// Y word.
    pub fn y(&self) -> Option<&Element> {
        self.get_element('Y')
    }
    /// Z word.
    pub fn z(&self) -> Option<&Element> {
        self.get_element('Z')
    }
    /// I word.
    pub fn i(&self) -> Option<&Element> {
        self.get_element('I')
    }
    /// J word.
    pub fn j(&self) -> Option<&Element> {
        self.get_element('J')
    }
    /// F word.
    pub fn f(&self) -> Option<&Element> {
        self.get_element('F')
    }
    /// R word.
    pub fn r(&self) -> Option<&Element> {
        self.get_element('R')
    }

    /// Word lookup. NOTE(upstream): C# throws NullReferenceException when the helper was not
    /// built; here that case returns None.
    ///
    /// `key`: letter. Returns the word.
    fn get_element(&self, key: char) -> Option<&Element> {
        self.helper.as_ref()?.iter().find(|e| e.command == key)
    }

    /// Key part of a coded result: text after the first ':' (whole text when no ':').
    fn error_key(&self) -> Option<String> {
        let cr = self.coded_result()?;
        let idx = cr.find(':').map(|i| i + 1).unwrap_or(0);
        Some(cr[idx..].to_string())
    }
}

/// Exception swallowed by `BuildHelper`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildFailure {
    /// `Dictionary.Add` with a letter seen before (`ArgumentException`).
    DuplicateLetter,
    /// `Decimal.Parse` on empty / malformed text (`FormatException`).
    Format,
    /// `Decimal.Parse` on a well-formed number that is too big (`OverflowException`).
    Overflow,
}

/// `Decimal.Parse(num, InvariantInfo)`; Err = exception.
///
/// `num`: accumulated number text.
fn parse_num(num: &str) -> Result<Decimal, BuildFailure> {
    let body = num.strip_prefix('-').unwrap_or(num);
    let mut parts = body.splitn(2, '.');
    let a = parts.next().unwrap_or("");
    let b = parts.next();
    parse_decimal(num).ok_or_else(|| {
        // well-formed ascii number that failed = too big
        let ascii = |t: &str| t.chars().all(|c| c.is_ascii_digit());
        let digits = a.len() + b.map_or(0, str::len);
        if digits > 0 && ascii(a) && b.is_none_or(ascii) { BuildFailure::Overflow } else { BuildFailure::Format }
    })
}

impl fmt::Display for GrblCommand {
    /// C# `ToString()` = current line.
    ///
    /// `f`: formatter. Returns fmt result.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.line)
    }
}

impl GrblRow for GrblCommand {
    /// Line, plus `(Retry n)` when repeated.
    fn get_decoded_message(&self) -> String {
        if self.repeat_count == 0 {
            self.line.clone()
        } else {
            format!("{} (Retry {})", self.line, self.repeat_count)
        }
    }

    /// Error brief from CSV when bad + decode, else coded result.
    ///
    /// `decode`: translate error code. `erroronly`: return None unless translated.
    fn get_result(&self, decode: bool, erroronly: bool) -> Option<String> {
        if self.status() == CommandStatus::ResponseBad && decode {
            if let Some(key) = self.error_key()
                && let Some(brief) = csvd::errors_item(&key, 0)
            {
                return Some(brief);
            }
            return self.coded_result();
        }
        if erroronly { None } else { self.coded_result() }
    }

    /// Error tooltip from CSV when bad + decode, else "".
    ///
    /// `decode`: translate error code.
    fn get_tool_tip(&self, decode: bool) -> Option<String> {
        if self.status() == CommandStatus::ResponseBad
            && decode
            && let Some(key) = self.error_key()
            && let Some(tt) = csvd::errors_item(&key, 1)
        {
            return Some(tt);
        }
        Some(String::new())
    }

    /// Always the command color.
    fn left_color(&self) -> RowColor {
        RowColor::LogLeftCommand
    }

    /// Good / bad / others by status.
    fn right_color(&self) -> RowColor {
        match self.status() {
            CommandStatus::ResponseGood => RowColor::LogRightGood,
            CommandStatus::ResponseBad => RowColor::LogRightBad,
            _ => RowColor::LogRightOthers,
        }
    }

    /// 0 queued/waiting, 1 good, 2 other.
    fn image_index(&self) -> i32 {
        match self.status() {
            CommandStatus::Queued | CommandStatus::WaitingResponse => 0,
            CommandStatus::ResponseGood => 1,
            _ => 2,
        }
    }
}

/// Kind of a log message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Startup,
    Config,
    Alarm,
    Feedback,
    Position,
    Others,
    Warning,
    Diagnostic,
}

/// Message from grbl or LaserGRBL (C# `GrblMessage`).
#[derive(Debug, Clone)]
pub struct GrblMessage {
    native_message: String,
    message: Option<String>,
    tool_tip: Option<String>,
    mtype: MessageType,
}

impl GrblMessage {
    /// C# `GrblMessage(message, MessageType)`.
    ///
    /// `message`: text (trimmed). `mtype`: kind.
    pub fn with_type(message: &str, mtype: MessageType) -> Self {
        let m = dn_trim(message).to_string();
        Self { native_message: m.clone(), message: Some(m), tool_tip: None, mtype }
    }

    /// C# `GrblMessage(message, decode)`: classify by prefix; decode via CSV when asked.
    ///
    /// `message`: raw line from grbl. `decode`: translate config/alarm codes.
    pub fn new(message: &str, decode: bool) -> Self {
        let m = dn_trim(message).to_string();
        let lower = dn_lower(&m);
        let mtype = if lower.starts_with('$') && m.contains('=') {
            MessageType::Config
        } else if lower.starts_with("grbl") {
            MessageType::Startup
        } else if lower.starts_with("alarm") {
            MessageType::Alarm
        } else if m.starts_with('<') && m.ends_with('>') {
            MessageType::Position
        } else if m.starts_with('[') && m.ends_with(']') {
            MessageType::Feedback
        } else {
            MessageType::Others
        };
        let mut r = Self { native_message: m.clone(), message: Some(m), tool_tip: None, mtype };

        if decode {
            // C# try/catch: any failure leaves the state as is
            match mtype {
                MessageType::Config => {
                    // key = text between '$' and '=' of the ORIGINAL (untrimmed) message
                    let eq = message.find('=');
                    if let Some(eq) = eq
                        && message.is_char_boundary(1)
                        && eq >= 1
                    {
                        let key = &message[1..eq];
                        let brief = csvd::settings_item(key, 0);
                        let unit = csvd::settings_item(key, 1);
                        let desc = csvd::settings_item(key, 2);
                        if let Some(b) = brief {
                            r.message = Some(format!("{message} ({b})"));
                        }
                        if let Some(d) = desc {
                            r.tool_tip = Some(format!("{} [{}]", d, unit.unwrap_or_default()));
                        }
                    }
                }
                MessageType::Alarm => {
                    let idx = message.find(':').map(|i| i + 1).unwrap_or(0);
                    let key = &message[idx..];
                    r.message = csvd::alarms_item(key, 0);
                    r.tool_tip = csvd::alarms_item(key, 1);
                }
                _ => {}
            }
        }
        r
    }

    /// Kind of message.
    pub fn message_type(&self) -> MessageType {
        self.mtype
    }

    /// Message as received (trimmed).
    pub fn get_native_message(&self) -> &str {
        &self.native_message
    }
}

impl GrblRow for GrblMessage {
    /// Decoded text ("" when C# would hold null).
    fn get_decoded_message(&self) -> String {
        self.message.clone().unwrap_or_default()
    }

    /// Messages have no result.
    fn get_result(&self, _decode: bool, _erroronly: bool) -> Option<String> {
        None
    }

    /// Tooltip built at construction.
    fn get_tool_tip(&self, _decode: bool) -> Option<String> {
        self.tool_tip.clone()
    }

    /// By type.
    fn left_color(&self) -> RowColor {
        match self.mtype {
            MessageType::Startup => RowColor::LogLeftStartup,
            MessageType::Alarm => RowColor::LogLeftAlarm,
            MessageType::Config => RowColor::LogLeftConfig,
            MessageType::Feedback => RowColor::LogLeftFeedback,
            MessageType::Position => RowColor::LogLeftPosition,
            _ => RowColor::LogLeftOthers,
        }
    }

    /// Always black.
    fn right_color(&self) -> RowColor {
        RowColor::Black
    }

    /// 4 warning, 5 diagnostic, else 3.
    fn image_index(&self) -> i32 {
        match self.mtype {
            MessageType::Warning => 4,
            MessageType::Diagnostic => 5,
            _ => 3,
        }
    }
}
