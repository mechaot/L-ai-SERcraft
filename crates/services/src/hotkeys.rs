// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `HotKeysManager.cs`: hotkey model, default bindings, key event dispatch (pure).
//!
//! The C# class calls into `GrblCore`, `PreviewForm` and `JogForm` directly. Here
//! [`HotKeysManager::manage_hotkeys`] only decides WHAT to do and returns [`HotKeyEffect`]s;
//! the UI executes them. Key combinations are stored as WinForms `Keys` ints (see [`keys`]) so
//! saved setups stay compatible. Persistence: setting `"Hotkey Setup"` as JSON
//! `[{"action":10,"combination":123}, ...]`.

use serde::{Deserialize, Serialize};

/// Setting key of the hotkey list (C# `Settings.GetObject("Hotkey Setup")`).
pub const SETTING_KEY: &str = "Hotkey Setup";

/// WinForms `System.Windows.Forms.Keys` values, name tables and egui mapping.
///
/// # egui mapping table (UI uses this)
///
/// egui `Key` names are the `Debug` text of the variant (`format!("{key:?}")`). egui has NO numpad
/// keys (numpad digits arrive as `Num0..Num9`); the physical numpad keys need winit `KeyCode`
/// names (`Numpad0`..`Numpad9`, `NumpadAdd`, ...), also accepted by [`from_name`].
///
/// | WinForms | egui `Key` (Debug) | winit `KeyCode` |
/// |---|---|---|
/// | `A`..`Z` (65..90) | `A`..`Z` | `KeyA`..`KeyZ` |
/// | `D0`..`D9` (48..57) | `Num0`..`Num9` | `Digit0`..`Digit9` |
/// | `NumPad0`..`NumPad9` (96..105) | - | `Numpad0`..`Numpad9` |
/// | `Multiply` 106 / `Add` 107 / `Subtract` 109 / `Decimal` 110 / `Divide` 111 | - | `NumpadMultiply` / `NumpadAdd` / `NumpadSubtract` / `NumpadDecimal` / `NumpadDivide` |
/// | `F1`..`F24` (112..135) | `F1`..`F24` | `F1`..`F24` |
/// | `Back` 8 / `Tab` 9 / `Return` 13 / `Escape` 27 / `Space` 32 | `Backspace` / `Tab` / `Enter` / `Escape` / `Space` | `Backspace` / `Tab` / `Enter`,`NumpadEnter` / `Escape` / `Space` |
/// | `PageUp` 33 / `PageDown` 34 / `End` 35 / `Home` 36 | `PageUp` / `PageDown` / `End` / `Home` | same |
/// | `Left` 37 / `Up` 38 / `Right` 39 / `Down` 40 | `ArrowLeft` / `ArrowUp` / `ArrowRight` / `ArrowDown` | same |
/// | `Insert` 45 / `Delete` 46 | `Insert` / `Delete` | same |
/// | `OemSemicolon` 186 / `Oemplus` 187 / `Oemcomma` 188 / `OemMinus` 189 / `OemPeriod` 190 | `Semicolon`(`Colon`) / `Equals`(`Plus`) / `Comma` / `Minus` / `Period` | `Semicolon` / `Equal` / `Comma` / `Minus` / `Period` |
/// | `Oem2` 191 / `Oemtilde` 192 / `OemOpenBrackets` 219 / `Oem5` 220 / `OemCloseBrackets` 221 / `OemQuotes` 222 | `Slash`(`Questionmark`) / `Backtick` / `OpenBracket` / `Backslash`(`Pipe`) / `CloseBracket` / `Quote` | `Slash` / `Backquote` / `BracketLeft` / `Backslash` / `BracketRight` / `Quote` |
///
/// Modifiers: `Shift` 0x10000, `Control` 0x20000, `Alt` 0x40000 (egui `Modifiers::shift/ctrl/alt`).
pub mod keys {
    pub const NONE: i32 = 0;
    pub const BACK: i32 = 8;
    pub const TAB: i32 = 9;
    pub const RETURN: i32 = 13;
    pub const ESCAPE: i32 = 27;
    pub const SPACE: i32 = 32;
    pub const PRIOR: i32 = 33;
    pub const NEXT: i32 = 34;
    pub const END: i32 = 35;
    pub const HOME: i32 = 36;
    pub const LEFT: i32 = 37;
    pub const UP: i32 = 38;
    pub const RIGHT: i32 = 39;
    pub const DOWN: i32 = 40;
    pub const INSERT: i32 = 45;
    pub const DELETE: i32 = 46;
    pub const D0: i32 = 48;
    pub const D1: i32 = 49;
    pub const D2: i32 = 50;
    pub const D3: i32 = 51;
    pub const D4: i32 = 52;
    pub const D5: i32 = 53;
    pub const D6: i32 = 54;
    pub const D7: i32 = 55;
    pub const D8: i32 = 56;
    pub const D9: i32 = 57;
    pub const A: i32 = 65;
    pub const H: i32 = 72;
    pub const O: i32 = 79;
    pub const R: i32 = 82;
    pub const S: i32 = 83;
    pub const U: i32 = 85;
    pub const X: i32 = 88;
    pub const Z: i32 = 90;
    pub const NUMPAD0: i32 = 96;
    pub const NUMPAD1: i32 = 97;
    pub const NUMPAD2: i32 = 98;
    pub const NUMPAD3: i32 = 99;
    pub const NUMPAD4: i32 = 100;
    pub const NUMPAD5: i32 = 101;
    pub const NUMPAD6: i32 = 102;
    pub const NUMPAD7: i32 = 103;
    pub const NUMPAD8: i32 = 104;
    pub const NUMPAD9: i32 = 105;
    pub const MULTIPLY: i32 = 106;
    pub const ADD: i32 = 107;
    pub const SEPARATOR: i32 = 108;
    pub const SUBTRACT: i32 = 109;
    pub const DECIMAL: i32 = 110;
    pub const DIVIDE: i32 = 111;
    pub const F1: i32 = 112;
    pub const F5: i32 = 116;
    pub const F6: i32 = 117;
    pub const F7: i32 = 118;
    pub const F12: i32 = 123;
    pub const F24: i32 = 135;
    /// `Keys.Shift` modifier flag.
    pub const SHIFT: i32 = 0x1_0000;
    /// `Keys.Control` modifier flag.
    pub const CONTROL: i32 = 0x2_0000;
    /// `Keys.Alt` modifier flag.
    pub const ALT: i32 = 0x4_0000;
    /// `Keys.Modifiers` mask.
    pub const MODIFIERS: i32 = 0xFFFF_0000u32 as i32;
    /// `Keys.KeyCode` mask.
    pub const KEY_CODE: i32 = 0xFFFF;

    /// Key code names as `Keys.ToString()` prints them (C# enum member names; first alias wins).
    const NAMES: &[(i32, &str)] = &[
        (1, "LButton"), (2, "RButton"), (3, "Cancel"), (4, "MButton"), (5, "XButton1"), (6, "XButton2"),
        (8, "Back"), (9, "Tab"), (10, "LineFeed"), (12, "Clear"), (13, "Return"),
        (16, "ShiftKey"), (17, "ControlKey"), (18, "Menu"), (19, "Pause"), (20, "CapsLock"),
        (21, "HanguelMode"), (23, "JunjaMode"), (24, "FinalMode"), (25, "KanjiMode"), (27, "Escape"),
        (28, "IMEConvert"), (29, "IMENonconvert"), (30, "IMEAccept"), (31, "IMEModeChange"),
        (32, "Space"), (33, "PageUp"), (34, "PageDown"), (35, "End"), (36, "Home"),
        (37, "Left"), (38, "Up"), (39, "Right"), (40, "Down"), (41, "Select"), (42, "Print"),
        (43, "Execute"), (44, "PrintScreen"), (45, "Insert"), (46, "Delete"), (47, "Help"),
        (48, "D0"), (49, "D1"), (50, "D2"), (51, "D3"), (52, "D4"), (53, "D5"), (54, "D6"), (55, "D7"), (56, "D8"), (57, "D9"),
        (65, "A"), (66, "B"), (67, "C"), (68, "D"), (69, "E"), (70, "F"), (71, "G"), (72, "H"), (73, "I"),
        (74, "J"), (75, "K"), (76, "L"), (77, "M"), (78, "N"), (79, "O"), (80, "P"), (81, "Q"), (82, "R"),
        (83, "S"), (84, "T"), (85, "U"), (86, "V"), (87, "W"), (88, "X"), (89, "Y"), (90, "Z"),
        (91, "LWin"), (92, "RWin"), (93, "Apps"), (95, "Sleep"),
        (96, "NumPad0"), (97, "NumPad1"), (98, "NumPad2"), (99, "NumPad3"), (100, "NumPad4"),
        (101, "NumPad5"), (102, "NumPad6"), (103, "NumPad7"), (104, "NumPad8"), (105, "NumPad9"),
        (106, "Multiply"), (107, "Add"), (108, "Separator"), (109, "Subtract"), (110, "Decimal"), (111, "Divide"),
        (112, "F1"), (113, "F2"), (114, "F3"), (115, "F4"), (116, "F5"), (117, "F6"), (118, "F7"), (119, "F8"),
        (120, "F9"), (121, "F10"), (122, "F11"), (123, "F12"), (124, "F13"), (125, "F14"), (126, "F15"),
        (127, "F16"), (128, "F17"), (129, "F18"), (130, "F19"), (131, "F20"), (132, "F21"), (133, "F22"),
        (134, "F23"), (135, "F24"), (144, "NumLock"), (145, "Scroll"),
        (160, "LShiftKey"), (161, "RShiftKey"), (162, "LControlKey"), (163, "RControlKey"), (164, "LMenu"), (165, "RMenu"),
        (166, "BrowserBack"), (167, "BrowserForward"), (168, "BrowserRefresh"), (169, "BrowserStop"),
        (170, "BrowserSearch"), (171, "BrowserFavorites"), (172, "BrowserHome"), (173, "VolumeMute"),
        (174, "VolumeDown"), (175, "VolumeUp"), (176, "MediaNextTrack"), (177, "MediaPreviousTrack"),
        (178, "MediaStop"), (179, "MediaPlayPause"), (180, "LaunchMail"), (181, "SelectMedia"),
        (182, "LaunchApplication1"), (183, "LaunchApplication2"),
        (186, "OemSemicolon"), (187, "Oemplus"), (188, "Oemcomma"), (189, "OemMinus"), (190, "OemPeriod"),
        (191, "Oem2"), (192, "Oemtilde"), (219, "OemOpenBrackets"), (220, "Oem5"), (221, "OemCloseBrackets"),
        (222, "OemQuotes"), (223, "Oem8"), (226, "OemBackslash"), (229, "ProcessKey"), (231, "Packet"),
        (246, "Attn"), (247, "Crsel"), (248, "Exsel"), (249, "EraseEof"), (250, "Play"), (251, "Zoom"),
        (252, "NoName"), (253, "Pa1"), (254, "OemClear"),
    ];

    /// Enum member name of a bare key code (no modifiers).
    ///
    /// `code`: `Keys` value without modifier bits. Returns the member name or None when undefined.
    pub fn code_name(code: i32) -> Option<&'static str> {
        if code == NONE {
            return Some("None");
        }
        NAMES.iter().find(|(c, _)| *c == code).map(|(_, n)| *n)
    }

    /// `KeysConverter.ConvertToString` (.NET Framework names).
    ///
    /// `keys`: `Keys` value (code plus modifier bits). Returns e.g. `Ctrl+Shift+O`, `F12`, `PgUp`.
    /// Modifier order is Ctrl, Shift, Alt (.NET Framework `KeysConverter.DisplayOrder`).
    /// NOTE: the Mono reference build prints raw enum names (`D1`, `Ctrl+Alt+Shift`); the original
    /// Windows build prints `1`, `Enter`, `PgUp`, `PgDn`, `Ins`, `Del`.
    pub fn to_string(keys: i32) -> String {
        let mut parts: Vec<String> = Vec::new();
        if keys & CONTROL != 0 {
            parts.push("Ctrl".into());
        }
        if keys & SHIFT != 0 {
            parts.push("Shift".into());
        }
        if keys & ALT != 0 {
            parts.push("Alt".into());
        }
        let code = keys & KEY_CODE;
        let name = match code {
            RETURN => "Enter".to_string(),
            PRIOR => "PgUp".to_string(),
            NEXT => "PgDn".to_string(),
            INSERT => "Ins".to_string(),
            DELETE => "Del".to_string(),
            D0..=D9 => ((b'0' + (code - D0) as u8) as char).to_string(),
            _ => match code_name(code) {
                Some("None") => String::new(),
                Some(n) => n.to_string(),
                None => code.to_string(),
            },
        };
        if !name.is_empty() {
            parts.push(name);
        }
        parts.join("+")
    }

    /// egui / winit name table: (name, WinForms code). Names are egui `Key` Debug text or winit
    /// `KeyCode` Debug text (both accepted by [`from_name`]).
    const NAME_MAP: &[(&str, i32)] = &[
        ("ArrowDown", DOWN), ("ArrowLeft", LEFT), ("ArrowRight", RIGHT), ("ArrowUp", UP),
        ("Escape", ESCAPE), ("Tab", TAB), ("Backspace", BACK), ("Enter", RETURN), ("NumpadEnter", RETURN),
        ("Space", SPACE), ("Insert", INSERT), ("Delete", DELETE), ("Home", HOME), ("End", END),
        ("PageUp", PRIOR), ("PageDown", NEXT),
        ("Colon", 186), ("Semicolon", 186), ("Equals", 187), ("Equal", 187), ("Plus", 187), ("Comma", 188),
        ("Minus", 189), ("Period", 190), ("Slash", 191), ("Questionmark", 191), ("Backtick", 192),
        ("Backquote", 192), ("OpenBracket", 219), ("BracketLeft", 219), ("OpenCurlyBracket", 219),
        ("Backslash", 220), ("Pipe", 220), ("CloseBracket", 221), ("BracketRight", 221),
        ("CloseCurlyBracket", 221), ("Quote", 222),
        ("NumpadMultiply", MULTIPLY), ("NumpadAdd", ADD), ("NumpadSubtract", SUBTRACT),
        ("NumpadDecimal", DECIMAL), ("NumpadDivide", DIVIDE),
        ("BrowserBack", 166),
    ];

    /// Key name (egui `Key` Debug text or winit `KeyCode` Debug text) to a WinForms key code.
    ///
    /// `name`: e.g. `"A"`, `"KeyA"`, `"Num5"`, `"Digit5"`, `"Numpad5"`, `"F12"`, `"ArrowUp"`.
    /// Returns the code without modifiers, or None for unmapped keys.
    pub fn from_name(name: &str) -> Option<i32> {
        if let Some(&(_, c)) = NAME_MAP.iter().find(|(n, _)| *n == name) {
            return Some(c);
        }
        // single letter (egui) or KeyX (winit)
        let letter = name.strip_prefix("Key").unwrap_or(name);
        if letter.len() == 1 {
            let b = letter.as_bytes()[0];
            if b.is_ascii_uppercase() {
                return Some(b as i32);
            }
        }
        // Num0..Num9 (egui) / Digit0..Digit9 (winit)
        for p in ["Num", "Digit"] {
            if let Some(d) = name.strip_prefix(p)
                && d.len() == 1
                && d.as_bytes()[0].is_ascii_digit()
            {
                return Some(D0 + (d.as_bytes()[0] - b'0') as i32);
            }
        }
        // winit Numpad0..Numpad9
        if let Some(d) = name.strip_prefix("Numpad")
            && d.len() == 1
            && d.as_bytes()[0].is_ascii_digit()
        {
            return Some(NUMPAD0 + (d.as_bytes()[0] - b'0') as i32);
        }
        // F1..F24
        if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<i32>().ok())
            && (1..=24).contains(&n)
        {
            return Some(F1 + n - 1);
        }
        None
    }

    /// Inverse of [`from_name`] for egui: preferred egui `Key` name of a code.
    ///
    /// `code`: WinForms code without modifiers. Returns the egui `Key` Debug name or None
    /// (numpad codes have no egui key).
    pub fn to_egui_name(code: i32) -> Option<String> {
        match code {
            65..=90 => Some(((code as u8) as char).to_string()),
            D0..=D9 => Some(format!("Num{}", code - D0)),
            112..=135 => Some(format!("F{}", code - F1 + 1)),
            BACK => Some("Backspace".into()),
            TAB => Some("Tab".into()),
            RETURN => Some("Enter".into()),
            ESCAPE => Some("Escape".into()),
            SPACE => Some("Space".into()),
            PRIOR => Some("PageUp".into()),
            NEXT => Some("PageDown".into()),
            END => Some("End".into()),
            HOME => Some("Home".into()),
            LEFT => Some("ArrowLeft".into()),
            UP => Some("ArrowUp".into()),
            RIGHT => Some("ArrowRight".into()),
            DOWN => Some("ArrowDown".into()),
            INSERT => Some("Insert".into()),
            DELETE => Some("Delete".into()),
            186 => Some("Semicolon".into()),
            187 => Some("Equals".into()),
            188 => Some("Comma".into()),
            189 => Some("Minus".into()),
            190 => Some("Period".into()),
            191 => Some("Slash".into()),
            192 => Some("Backtick".into()),
            219 => Some("OpenBracket".into()),
            220 => Some("Backslash".into()),
            221 => Some("CloseBracket".into()),
            222 => Some("Quote".into()),
            _ => None,
        }
    }

    /// Build a `Keys` value from a key name and modifier flags.
    ///
    /// `name`: see [`from_name`]. `ctrl`,`shift`,`alt`: modifier state. Returns the combined
    /// value or None for unmapped keys.
    pub fn combine(name: &str, ctrl: bool, shift: bool, alt: bool) -> Option<i32> {
        let code = from_name(name)?;
        Some(code | if ctrl { CONTROL } else { 0 } | if shift { SHIFT } else { 0 } | if alt { ALT } else { 0 })
    }
}

macro_rules! actions {
    ($($name:ident = $val:expr),* $(,)?) => {
        /// C# `HotKey.Actions` (same numeric values, same declaration order).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(i32)]
        pub enum Actions { $($name = $val),* }

        impl Actions {
            /// All actions in declaration order.
            pub const ALL: &'static [Actions] = &[$(Actions::$name),*];

            /// `Enum.ToString()`.
            ///
            /// Returns the variant name.
            pub fn name(self) -> &'static str {
                match self { $(Actions::$name => stringify!($name)),* }
            }

            /// `(Actions)int` when defined (`Enum.IsDefined`).
            ///
            /// `v`: numeric value. Returns the action or None.
            pub fn from_i32(v: i32) -> Option<Actions> {
                match v { $($val => Some(Actions::$name),)* _ => None }
            }
        }
    };
}

actions! {
    ConnectDisconnect = 10, Connect = 11, Disconnect = 12,
    OpenFile = 20, ReopenLastFile = 21, SaveFile = 22, ExecuteFile = 23, AbortFile = 24,
    HelpOnline = 30,
    AutoSizeDrawing = 40, ZoomInDrawing = 41, ZoomOutDrawing = 42,
    Reset = 100, Homing = 101, Unlock = 102, PauseJob = 103, ResumeJob = 104, SetNewZero = 105,
    JogHome = 1000, JogN = 1001, JogNE = 1002, JogE = 1003, JogSE = 1004, JogS = 1005, JogSW = 1006, JogW = 1007, JogNW = 1008, JogUp = 1009, JogDown = 1010,
    JogStepIncrease = 1020, JogStepDecrease = 1021,
    JogSpeedIncrease = 1030, JogSpeedDecrease = 1031,
    OverridePowerDefault = 1100, OverridePowerUp = 1101, OverridePowerDown = 1102,
    OverrideLinearDefault = 1110, OverrideLinearUp = 1111, OverrideLinearDown = 1112,
    OverrideRapidDefault = 1120, OverrideRapidUp = 1121, OverrideRapidDown = 1122,
    CustomButton1 = 2000, CustomButton2 = 2001, CustomButton3 = 2002, CustomButton4 = 2003, CustomButton5 = 2004,
    CustomButton6 = 2005, CustomButton7 = 2006, CustomButton8 = 2007, CustomButton9 = 2008, CustomButton10 = 2009,
}

/// Jog direction (C# `GrblCore.JogDirection` as used by the hotkeys; the UI maps to the core enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JogDir {
    Home,
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
    Zup,
    Zdown,
}

/// One hotkey binding (C# `HotKeysManager.HotKey`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotKey {
    action: Actions,
    combination: i32,
}

impl HotKey {
    /// C# ctor.
    ///
    /// `action`: action to run. `keys`: WinForms `Keys` value (0 = unassigned).
    pub fn new(action: Actions, keys: i32) -> Self {
        Self { action, combination: keys }
    }

    /// C# `Action`.
    pub fn action(&self) -> Actions {
        self.action
    }

    /// C# `Combination` (WinForms `Keys` value).
    pub fn combination(&self) -> i32 {
        self.combination
    }

    /// C# `ActionString`: the action name (an undefined action prints `-`, impossible here).
    pub fn action_string(&self) -> &'static str {
        self.action.name()
    }

    /// C# `SetShortcut`.
    ///
    /// `shortcut`: new `Keys` value.
    pub fn set_shortcut(&mut self, shortcut: i32) {
        self.combination = shortcut;
    }

    /// C# `HotKey.ToString()` is not overridden (prints the type name); this gives the key text
    /// the UI shows: `KeysConverter` text of the combination, empty when unassigned.
    pub fn combination_string(&self) -> String {
        if self.combination == keys::NONE { String::new() } else { keys::to_string(self.combination) }
    }

    /// C# `ValidShortcut`: a shortcut needs a letter, digit, numpad digit or F-key.
    ///
    /// `combination`: `Keys` value. Returns true when a "standard" key is contained.
    pub fn valid_shortcut(combination: i32) -> bool {
        if combination == keys::NONE {
            return false;
        }
        // C# loops over all enum values x with (x & comb) == x and keeps the standard ones
        let standard: Vec<i32> = (keys::D0..=keys::D9).chain(keys::A..=90).chain(keys::NUMPAD0..=keys::NUMPAD9).chain(keys::F1..=keys::F24).collect();
        standard.iter().any(|x| x & combination == *x)
    }
}

/// What the UI has to do after a key event (replaces the direct calls of `PerformAction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotKeyEffect {
    /// Plain core call; the action tells which (`HKConnectDisconnect`, `OpenFile`, `ReOpenFile`,
    /// `SaveProgram(parent,false,false,false,1,false)`, `RunProgram`, `AbortProgram`, `HelpOnLine`,
    /// `AutoSizeDrawing`, `ZoomInDrawing`, `ZoomOutDrawing`, `GrblReset`, `GrblHoming`, `GrblUnlock`,
    /// `FeedHold(false)`, `CycleStartResume(false)`, `SetNewZero`, `HKConnect`, `HKDisconnect`).
    Core(Actions),
    /// `mCore.HotKeyOverride(action)`.
    Override(Actions),
    /// `mCore.JogToDirection(dir, false)`.
    Jog(JogDir),
    /// `mJogForm.ChangeJogStepIndexBy(+1/-1)` (already filtered by `JogEnabled`); true = increase.
    ChangeJogStep(bool),
    /// `mJogForm.ChangeJogSpeedIndexBy(+1/-1)`; true = increase.
    ChangeJogSpeed(bool),
    /// Key released while a jog hotkey was active: `mCore.JogAbort()`.
    JogAbort,
    /// `EmulateCustomButtonDown(index)`: `EmulateMouseInside = true; PerformMouseDown(left)`.
    CustomButtonDown(usize),
    /// `EmulateCustomButtonUp` per pressed button: `PerformMouseUp; PerformClick; EmulateMouseInside = false`.
    CustomButtonUp(usize),
}

/// Result of [`HotKeysManager::manage_hotkeys`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HotKeyOutcome {
    /// C# return value: true when a binding matched (key is consumed).
    pub handled: bool,
    /// Effects to execute in order.
    pub effects: Vec<HotKeyEffect>,
}

/// Facts the C# code reads from forms/core while dispatching.
#[derive(Debug, Clone, Copy, Default)]
pub struct HotKeyContext {
    /// `mPreviewForm.CustomImageButtons.Count`.
    pub custom_button_count: usize,
    /// `mCore.JogEnabled`.
    pub jog_enabled: bool,
}

/// JSON row of the persisted list.
#[derive(Serialize, Deserialize)]
struct HotKeyRow {
    action: i32,
    combination: i32,
}

/// Hotkey list (C# `HotKeysManager : List<HotKey>`) plus the runtime state of `ManageHotKeys`.
#[derive(Debug, Clone, Default)]
pub struct HotKeysManager {
    list: Vec<HotKey>,
    custom_button_pressed: Vec<usize>,
    jog_key_requested: bool,
}

impl HotKeysManager {
    /// Empty list (C# ctor).
    pub fn new() -> Self {
        Self::default()
    }

    /// Manager with the full default table, ready to use (`new()` + [`init`](Self::init)).
    pub fn with_defaults() -> Self {
        let mut m = Self::new();
        m.init();
        m
    }

    /// Entries in list order.
    pub fn entries(&self) -> &[HotKey] {
        &self.list
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// C# `Init` (minus the form references): reset runtime state, add missing default
    /// bindings, sort by action value.
    pub fn init(&mut self) {
        self.custom_button_pressed.clear();
        self.add_all_features();
        // CompareKey: x.Action - y.Action
        self.list.sort_by_key(|h| h.action as i32);
    }

    /// C# `AddAllFeatures`: the default bindings, exactly in C# order.
    fn add_all_features(&mut self) {
        use Actions::*;
        use keys::*;
        let defaults = [
            (ConnectDisconnect, F12),
            (Connect, NONE),
            (Disconnect, NONE),
            (OpenFile, CONTROL | O),
            (ReopenLastFile, CONTROL | R),
            (SaveFile, CONTROL | S),
            (ExecuteFile, F5),
            (AbortFile, CONTROL | F5),
            (HelpOnline, CONTROL | F1),
            (Reset, CONTROL | X),
            (Homing, CONTROL | H),
            (Unlock, CONTROL | U),
            (PauseJob, F6),
            (ResumeJob, F7),
            (SetNewZero, CONTROL | Z),
            (JogHome, NUMPAD5),
            (JogN, NUMPAD8),
            (JogNE, NUMPAD9),
            (JogE, NUMPAD6),
            (JogSE, NUMPAD3),
            (JogS, NUMPAD2),
            (JogSW, NUMPAD1),
            (JogW, NUMPAD4),
            (JogNW, NUMPAD7),
            (JogUp, 107),
            (JogDown, 109),
            (JogStepIncrease, MULTIPLY),
            (JogStepDecrease, DIVIDE),
            (JogSpeedIncrease, NONE),
            (JogSpeedDecrease, NONE),
            (OverridePowerDefault, NONE),
            (OverridePowerUp, NONE),
            (OverridePowerDown, NONE),
            (OverrideLinearDefault, NONE),
            (OverrideLinearUp, NONE),
            (OverrideLinearDown, NONE),
            (OverrideRapidDefault, NONE),
            (OverrideRapidUp, NONE),
            (OverrideRapidDown, NONE),
            (CustomButton1, CONTROL | D1),
            (CustomButton2, CONTROL | D2),
            (CustomButton3, CONTROL | D3),
            (CustomButton4, CONTROL | D4),
            (CustomButton5, CONTROL | D5),
            (CustomButton6, CONTROL | D6),
            (CustomButton7, CONTROL | D7),
            (CustomButton8, CONTROL | D8),
            (CustomButton9, CONTROL | D9),
            (CustomButton10, CONTROL | D0),
            (AutoSizeDrawing, CONTROL | A),
            (ZoomInDrawing, CONTROL | ADD),
            (ZoomOutDrawing, CONTROL | SUBTRACT),
        ];
        for (a, k) in defaults {
            self.add_new(HotKey::new(a, k));
        }
    }

    /// C# `AddNew`: skip when the action exists; clear the shortcut when already used.
    ///
    /// `toadd`: candidate binding.
    fn add_new(&mut self, mut toadd: HotKey) {
        for hk in &self.list {
            if hk.action == toadd.action {
                return;
            }
            if toadd.combination != keys::NONE && hk.combination == toadd.combination {
                toadd.set_shortcut(keys::NONE);
            }
        }
        self.list.push(toadd);
    }

    /// C# `GetEditList`: deep copy for the editor form.
    pub fn get_edit_list(&self) -> Vec<HotKey> {
        self.list.clone()
    }

    /// C# `GrblCore.WriteHotkeys`: replace the list (does not save; call [`save_to_settings`](Self::save_to_settings)).
    ///
    /// `list`: new entries from the editor.
    pub fn write_hotkeys(&mut self, list: Vec<HotKey>) {
        self.list = list;
    }

    /// C# `GetHotKeyString`.
    ///
    /// `action`: action to look up. Returns the key text of the first binding with a key, or None.
    pub fn get_hotkey_string(&self, action: Actions) -> Option<String> {
        self.list.iter().find(|h| h.action == action && h.combination != keys::NONE).map(|h| keys::to_string(h.combination))
    }

    /// C# `ManageHotKeys(parent, keys)` as a pure decision.
    ///
    /// `key_data`: WinForms `Keys` value of the event (0 = all keys released). `ctx`: UI facts.
    /// Returns what to execute and whether the key is consumed.
    pub fn manage_hotkeys(&mut self, key_data: i32, ctx: HotKeyContext) -> HotKeyOutcome {
        let mut out = HotKeyOutcome::default();
        // key release: stop jog, release emulated custom buttons
        if key_data == keys::NONE {
            if self.jog_key_requested {
                out.effects.push(HotKeyEffect::JogAbort);
            }
            self.jog_key_requested = false;
            for idx in self.custom_button_pressed.drain(..) {
                if idx < ctx.custom_button_count {
                    out.effects.push(HotKeyEffect::CustomButtonUp(idx));
                }
            }
            return out;
        }
        // first exact match wins (list is sorted by action)
        let Some(action) = self.list.iter().find(|h| h.combination == key_data).map(|h| h.action) else {
            return out;
        };
        self.perform_action(action, ctx, &mut out.effects);
        out.handled = true;
        out
    }

    /// C# `PerformAction`.
    ///
    /// `action`: matched action. `ctx`: UI facts. `fx`: effect list to append to.
    fn perform_action(&mut self, action: Actions, ctx: HotKeyContext, fx: &mut Vec<HotKeyEffect>) {
        use Actions::*;
        use HotKeyEffect as E;
        let jog = |this: &mut Self, fx: &mut Vec<E>, d: JogDir| {
            this.jog_key_requested = true;
            fx.push(E::Jog(d));
        };
        match action {
            ConnectDisconnect | Connect | Disconnect | OpenFile | ReopenLastFile | SaveFile | ExecuteFile | AbortFile
            | HelpOnline | AutoSizeDrawing | ZoomInDrawing | ZoomOutDrawing | Reset | Homing | Unlock | PauseJob
            | ResumeJob | SetNewZero => fx.push(E::Core(action)),
            JogHome => jog(self, fx, JogDir::Home),
            JogN => jog(self, fx, JogDir::N),
            JogNE => jog(self, fx, JogDir::NE),
            JogE => jog(self, fx, JogDir::E),
            JogSE => jog(self, fx, JogDir::SE),
            JogS => jog(self, fx, JogDir::S),
            JogSW => jog(self, fx, JogDir::SW),
            JogW => jog(self, fx, JogDir::W),
            JogNW => jog(self, fx, JogDir::NW),
            JogUp => jog(self, fx, JogDir::Zup),
            JogDown => jog(self, fx, JogDir::Zdown),
            JogStepIncrease | JogStepDecrease => {
                if ctx.jog_enabled {
                    fx.push(E::ChangeJogStep(action == JogStepIncrease));
                }
            }
            JogSpeedIncrease | JogSpeedDecrease => {
                if ctx.jog_enabled {
                    fx.push(E::ChangeJogSpeed(action == JogSpeedIncrease));
                }
            }
            OverridePowerDefault | OverridePowerUp | OverridePowerDown | OverrideLinearDefault | OverrideLinearUp
            | OverrideLinearDown | OverrideRapidDefault | OverrideRapidUp | OverrideRapidDown => fx.push(E::Override(action)),
            CustomButton1 | CustomButton2 | CustomButton3 | CustomButton4 | CustomButton5 | CustomButton6
            | CustomButton7 | CustomButton8 | CustomButton9 | CustomButton10 => {
                let index = (action as i32 - CustomButton1 as i32) as usize;
                // EmulateCustomButtonDown
                if index < ctx.custom_button_count && !self.custom_button_pressed.contains(&index) {
                    self.custom_button_pressed.push(index);
                    fx.push(E::CustomButtonDown(index));
                }
            }
        }
    }

    /// Serialize to the settings JSON text form.
    ///
    /// Returns the JSON value `[{"action":..,"combination":..}]`.
    pub fn to_json(&self) -> serde_json::Value {
        let rows: Vec<HotKeyRow> = self.list.iter().map(|h| HotKeyRow { action: h.action as i32, combination: h.combination }).collect();
        serde_json::to_value(rows).unwrap_or_default()
    }

    /// Parse the settings JSON. Accepts our array form and the object form produced by the NRBF
    /// settings import (`{"_items":[{"mAction":..,"mCombination":..}],"_size":n}`).
    ///
    /// `v`: JSON value. Returns the manager (undefined actions dropped) or None if the shape is unknown.
    pub fn from_json(v: &serde_json::Value) -> Option<Self> {
        let mut m = Self::new();
        let push = |m: &mut Self, a: Option<i64>, c: Option<i64>| {
            if let (Some(a), Some(c)) = (a, c)
                && let Some(act) = Actions::from_i32(a as i32)
            {
                m.list.push(HotKey::new(act, c as i32));
            }
        };
        if let Some(arr) = v.as_array() {
            for r in arr {
                push(&mut m, r.get("action").and_then(|x| x.as_i64()), r.get("combination").and_then(|x| x.as_i64()));
            }
            return Some(m);
        }
        let items = v.get("_items")?.as_array()?;
        let size = v.get("_size").and_then(|s| s.as_u64()).unwrap_or(items.len() as u64) as usize;
        for r in items.iter().take(size) {
            push(&mut m, r.get("mAction").and_then(|x| x.as_i64()), r.get("mCombination").and_then(|x| x.as_i64()));
        }
        Some(m)
    }

    /// `GrblCore` ctor part: `if (!ExistObject) SetObject(new HotKeysManager()); get; Init`.
    ///
    /// Returns the manager with all default bindings added and sorted.
    pub fn load_from_settings() -> Self {
        use lg_base::settings;
        if !settings::exists(SETTING_KEY) {
            settings::set(SETTING_KEY, Self::new().to_json());
        }
        let stored: serde_json::Value = settings::get(SETTING_KEY, serde_json::Value::Null);
        let mut m = Self::from_json(&stored).unwrap_or_default();
        m.init();
        m
    }

    /// `Settings.SetObject("Hotkey Setup", manager, true)`.
    pub fn save_to_settings(&self) {
        lg_base::settings::set_force_save(SETTING_KEY, self.to_json());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn default_table_matches_csharp() {
        let m = HotKeysManager::with_defaults();
        let gold = std::fs::read_to_string(root().join("tests/golden/services/hotkeys_default.txt")).unwrap();
        let rows: Vec<Vec<&str>> = gold.lines().map(|l| l.split('\t').collect()).collect();
        assert_eq!(m.len(), rows.len());
        for (h, r) in m.entries().iter().zip(&rows) {
            assert_eq!(h.action_string(), r[0]);
            assert_eq!(h.action() as i32, r[1].parse::<i32>().unwrap());
            assert_eq!(h.combination(), r[2].parse::<i32>().unwrap(), "{}", r[0]);
        }
    }

    #[test]
    fn actions_roundtrip() {
        for a in Actions::ALL {
            assert_eq!(Actions::from_i32(*a as i32), Some(*a));
        }
        assert_eq!(Actions::from_i32(7), None);
        assert_eq!(Actions::ALL.len(), 52);
    }

    #[test]
    fn keys_text_matches_mono_where_names_agree() {
        let gold = std::fs::read_to_string(root().join("tests/golden/services/keys.txt")).unwrap();
        let mut checked = 0;
        for l in gold.lines() {
            let (code, txt) = l.split_once('\t').unwrap();
            let code: i32 = code.parse().unwrap();
            let ours = keys::to_string(code);
            // Mono prints raw enum names and Ctrl+Alt+Shift; compare only unambiguous cases
            let low = code & keys::KEY_CODE;
            let plain_name = !matches!(low, 13 | 33 | 34 | 45 | 46 | 48..=57) && (code & keys::MODIFIERS).count_ones() <= 1;
            if plain_name && low != 0 && !txt.is_empty() && keys::code_name(low).is_some() {
                assert_eq!(ours, txt, "code {code}");
                checked += 1;
            }
        }
        assert!(checked > 150);
    }

    #[test]
    fn key_strings_dotnet_framework() {
        assert_eq!(keys::to_string(keys::CONTROL | keys::O), "Ctrl+O");
        assert_eq!(keys::to_string(keys::CONTROL | keys::D1), "Ctrl+1");
        assert_eq!(keys::to_string(keys::CONTROL | keys::SHIFT | keys::ALT | keys::F5), "Ctrl+Shift+Alt+F5");
        assert_eq!(keys::to_string(keys::NEXT), "PgDn");
        assert_eq!(keys::to_string(0), "");
    }

    #[test]
    fn get_hotkey_string_and_valid_shortcut() {
        let m = HotKeysManager::with_defaults();
        assert_eq!(m.get_hotkey_string(Actions::SaveFile).as_deref(), Some("Ctrl+S"));
        assert_eq!(m.get_hotkey_string(Actions::Connect), None);
        assert!(HotKey::valid_shortcut(keys::CONTROL | keys::A));
        assert!(HotKey::valid_shortcut(keys::F12));
        assert!(!HotKey::valid_shortcut(keys::CONTROL));
        assert!(!HotKey::valid_shortcut(keys::DELETE));
        assert!(!HotKey::valid_shortcut(0));
    }

    #[test]
    fn manage_core_action() {
        let mut m = HotKeysManager::with_defaults();
        let o = m.manage_hotkeys(keys::F12, HotKeyContext::default());
        assert!(o.handled);
        assert_eq!(o.effects, vec![HotKeyEffect::Core(Actions::ConnectDisconnect)]);
        let o = m.manage_hotkeys(keys::F1, HotKeyContext::default());
        assert!(!o.handled && o.effects.is_empty());
    }

    #[test]
    fn manage_jog_and_release() {
        let mut m = HotKeysManager::with_defaults();
        let o = m.manage_hotkeys(keys::NUMPAD8, HotKeyContext::default());
        assert_eq!(o.effects, vec![HotKeyEffect::Jog(JogDir::N)]);
        let o = m.manage_hotkeys(0, HotKeyContext::default());
        assert!(!o.handled);
        assert_eq!(o.effects, vec![HotKeyEffect::JogAbort]);
        // second release: nothing left
        assert!(m.manage_hotkeys(0, HotKeyContext::default()).effects.is_empty());
    }

    #[test]
    fn manage_jog_step_needs_jog_enabled() {
        let mut m = HotKeysManager::with_defaults();
        let off = m.manage_hotkeys(keys::MULTIPLY, HotKeyContext::default());
        assert!(off.handled && off.effects.is_empty());
        let on = m.manage_hotkeys(keys::MULTIPLY, HotKeyContext { custom_button_count: 0, jog_enabled: true });
        assert_eq!(on.effects, vec![HotKeyEffect::ChangeJogStep(true)]);
    }

    #[test]
    fn manage_custom_buttons() {
        let mut m = HotKeysManager::with_defaults();
        let ctx = HotKeyContext { custom_button_count: 2, jog_enabled: false };
        let d = m.manage_hotkeys(keys::CONTROL | keys::D2, ctx);
        assert_eq!(d.effects, vec![HotKeyEffect::CustomButtonDown(1)]);
        // auto repeat while held: no second down
        assert!(m.manage_hotkeys(keys::CONTROL | keys::D2, ctx).effects.is_empty());
        // button 3 does not exist
        assert!(m.manage_hotkeys(keys::CONTROL | keys::D3, ctx).effects.is_empty());
        let u = m.manage_hotkeys(0, ctx);
        assert_eq!(u.effects, vec![HotKeyEffect::CustomButtonUp(1)]);
    }

    #[test]
    fn add_new_clears_duplicate_combination() {
        let mut m = HotKeysManager::new();
        m.list.push(HotKey::new(Actions::Reset, keys::F12));
        m.init();
        // ConnectDisconnect default F12 collides with the stored Reset binding
        let cd = m.entries().iter().find(|h| h.action() == Actions::ConnectDisconnect).unwrap();
        assert_eq!(cd.combination(), keys::NONE);
        let r = m.entries().iter().find(|h| h.action() == Actions::Reset).unwrap();
        assert_eq!(r.combination(), keys::F12);
    }

    #[test]
    fn json_roundtrip_and_nrbf_shape() {
        let m = HotKeysManager::with_defaults();
        let back = HotKeysManager::from_json(&m.to_json()).unwrap();
        assert_eq!(back.entries(), m.entries());
        let nrbf = serde_json::json!({"_items":[{"mAction":10,"mCombination":123},{"mAction":99,"mCombination":1},null],"_size":2});
        let n = HotKeysManager::from_json(&nrbf).unwrap();
        assert_eq!(n.len(), 1);
        assert_eq!(n.entries()[0].combination(), 123);
    }

    #[test]
    fn key_name_mapping() {
        assert_eq!(keys::from_name("A"), Some(65));
        assert_eq!(keys::from_name("KeyA"), Some(65));
        assert_eq!(keys::from_name("Num5"), Some(53));
        assert_eq!(keys::from_name("Numpad5"), Some(101));
        assert_eq!(keys::from_name("F24"), Some(135));
        assert_eq!(keys::from_name("F25"), None);
        assert_eq!(keys::from_name("ArrowUp"), Some(38));
        assert_eq!(keys::combine("O", true, false, false), Some(keys::CONTROL | keys::O));
        assert_eq!(keys::to_egui_name(53).as_deref(), Some("Num5"));
        assert_eq!(keys::to_egui_name(101), None);
    }
}
