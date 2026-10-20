//! Port of `CustomButton.cs`: `CustomButton` and the `CustomButtons` list.
//!
//! Persistence differs from C#: the user list is JSON (`CustomButtons.json` in the data dir, same
//! row layout as `resources/misc/standard_buttons.json`), the first-run seed is that standard
//! file. Windows `.zbn` files (deflate + BinaryFormatter) are imported/exported through
//! `lg_base::nrbf`. The Windows `Form` dialogs of `Export`/`Import` are inverted: this module
//! returns data ([`ImportPlan`]) and the UI shows the questions.
//! Fields that C# may hold as `null` strings are plain `String` (null reads as empty).

use base64::Engine;
use lg_base::nrbf::{self, CustomButtonData};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};

/// Setting key of the pre-JSON button list (C# `Settings "Custom Buttons"`).
const LEGACY_SETTING: &str = "Custom Buttons";

/// C# `CustomButton.EnableStyles` (numeric values kept).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum EnableStyles {
    #[default]
    Always = 0,
    Connected = 1,
    Idle = 3,
    Run = 4,
    IdleProgram = 10,
}

impl EnableStyles {
    /// `(EnableStyles)int`.
    ///
    /// `v`: numeric value. Returns the style, unknown values give `Always` (C# keeps the raw
    /// value, which `EnabledNow` then treats as disabled; use [`CustomButton::enable_style_raw`]).
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Always),
            1 => Some(Self::Connected),
            3 => Some(Self::Idle),
            4 => Some(Self::Run),
            10 => Some(Self::IdleProgram),
            _ => None,
        }
    }

    /// `Enum.ToString()`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Always => "Always",
            Self::Connected => "Connected",
            Self::Idle => "Idle",
            Self::Run => "Run",
            Self::IdleProgram => "IdleProgram",
        }
    }
}

/// C# `CustomButton.ButtonTypes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ButtonTypes {
    #[default]
    Button = 0,
    TwoStateButton = 1,
    PushButton = 2,
}

impl ButtonTypes {
    /// `(ButtonTypes)int`.
    ///
    /// `v`: numeric value. Returns the type or None when undefined.
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Button),
            1 => Some(Self::TwoStateButton),
            2 => Some(Self::PushButton),
            _ => None,
        }
    }

    /// `Enum.ToString()`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Button => "Button",
            Self::TwoStateButton => "TwoStateButton",
            Self::PushButton => "PushButton",
        }
    }
}

/// The core facts `EnabledNow` reads (C# takes a `GrblCore`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonContext {
    /// `core.IsConnected`.
    pub is_connected: bool,
    /// `core.MachineStatus == MacStatus.Idle`.
    pub status_idle: bool,
    /// `core.MachineStatus == MacStatus.Run`.
    pub status_run: bool,
    /// `core.HasProgram`.
    pub has_program: bool,
}

/// C# `CustomButton`.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomButton {
    /// `guid` (lowercase hyphenated text).
    pub guid: String,
    /// `Image`: encoded image bytes (PNG), None = no image.
    pub image: Option<Vec<u8>>,
    pub gcode: String,
    pub gcode2: String,
    pub caption: String,
    pub tooltip: String,
    /// `EnableStyle` raw value (see [`EnableStyles`]).
    enable_style: i32,
    pub button_type: ButtonTypes,
}

impl Default for CustomButton {
    /// C# field initialisers: new random guid, everything else empty / default.
    fn default() -> Self {
        Self {
            guid: new_guid(),
            image: None,
            gcode: String::new(),
            gcode2: String::new(),
            caption: String::new(),
            tooltip: String::new(),
            enable_style: 0,
            button_type: ButtonTypes::Button,
        }
    }
}

impl CustomButton {
    /// New button (random guid) with the given enable style.
    ///
    /// `style`: `EnableStyle`. Returns the button.
    pub fn with_style(style: EnableStyles) -> Self {
        Self { enable_style: style as i32, ..Self::default() }
    }

    /// `EnableStyle` when it is a defined value.
    pub fn enable_style(&self) -> Option<EnableStyles> {
        EnableStyles::from_i32(self.enable_style)
    }

    /// Raw `EnableStyle` int (undefined values survive a round trip).
    pub fn enable_style_raw(&self) -> i32 {
        self.enable_style
    }

    /// Set `EnableStyle`.
    ///
    /// `style`: new style.
    pub fn set_enable_style(&mut self, style: EnableStyles) {
        self.enable_style = style as i32;
    }

    /// C# `EnabledNow(core)`.
    ///
    /// `ctx`: core facts. Returns true when the button may be used now (undefined style: false).
    pub fn enabled_now(&self, ctx: &ButtonContext) -> bool {
        match self.enable_style() {
            Some(EnableStyles::Always) => true,
            Some(EnableStyles::Connected) => ctx.is_connected,
            Some(EnableStyles::Idle) => ctx.status_idle,
            Some(EnableStyles::Run) => ctx.status_run,
            Some(EnableStyles::IdleProgram) => ctx.status_idle && ctx.has_program,
            None => false,
        }
    }

    /// Decode the image to RGBA pixels.
    ///
    /// Returns the image or None when absent / undecodable.
    pub fn image_rgba(&self) -> Option<image::RgbaImage> {
        image::load_from_memory(self.image.as_deref()?).ok().map(|i| i.to_rgba8())
    }

    /// Text shown in the import question (`ToolTip` or `[no name]`).
    pub fn import_name(&self) -> String {
        if self.tooltip.trim().is_empty() { "[no name]".into() } else { self.tooltip.clone() }
    }

    /// From NRBF data (null strings become empty).
    fn from_data(d: &CustomButtonData) -> Self {
        Self {
            guid: d.guid.clone(),
            image: d.image.clone(),
            gcode: d.gcode.clone().unwrap_or_default(),
            gcode2: d.gcode2.clone().unwrap_or_default(),
            caption: d.caption.clone().unwrap_or_default(),
            tooltip: d.tooltip.clone().unwrap_or_default(),
            enable_style: d.enable_style,
            button_type: ButtonTypes::from_i32(d.button_type).unwrap_or_default(),
        }
    }

    /// To NRBF data.
    fn to_data(&self) -> CustomButtonData {
        CustomButtonData {
            guid: self.guid.clone(),
            image: self.image.clone(),
            gcode: Some(self.gcode.clone()),
            gcode2: Some(self.gcode2.clone()),
            caption: Some(self.caption.clone()),
            tooltip: Some(self.tooltip.clone()),
            enable_style: self.enable_style,
            button_type: self.button_type as i32,
        }
    }
}

/// `Guid.NewGuid()` replacement: random v4 text without extra dependencies.
///
/// Returns lowercase `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`.
pub fn new_guid() -> String {
    let mut b = [0u8; 16];
    // two SipHash keys from the OS give 128 random bits
    for half in 0..2 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(half);
        b[half as usize * 8..half as usize * 8 + 8].copy_from_slice(&h.finish().to_le_bytes());
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = |r: std::ops::Range<usize>| b[r].iter().map(|x| format!("{x:02x}")).collect::<String>();
    format!("{}-{}-{}-{}-{}", h(0..4), h(4..6), h(6..8), h(8..10), h(10..16))
}

/// One row of the JSON files (`standard_buttons.json` layout; extra fields are ignored).
#[derive(Serialize, Deserialize, Default)]
struct ButtonRow {
    #[serde(default)]
    guid: String,
    #[serde(default)]
    caption: String,
    #[serde(default)]
    tooltip: String,
    #[serde(default)]
    gcode: String,
    #[serde(default)]
    gcode2: String,
    #[serde(default)]
    enable_style: String,
    #[serde(default)]
    enable_style_value: i32,
    #[serde(default)]
    button_type: String,
    #[serde(default)]
    button_type_value: i32,
    #[serde(default)]
    image_png_base64: Option<String>,
}

/// JSON file root.
#[derive(Serialize, Deserialize, Default)]
struct ButtonFile {
    #[serde(default)]
    buttons: Vec<ButtonRow>,
}

impl ButtonRow {
    fn from_button(b: &CustomButton) -> Self {
        Self {
            guid: b.guid.clone(),
            caption: b.caption.clone(),
            tooltip: b.tooltip.clone(),
            gcode: b.gcode.clone(),
            gcode2: b.gcode2.clone(),
            enable_style: b.enable_style().map(|s| s.name().to_string()).unwrap_or_default(),
            enable_style_value: b.enable_style,
            button_type: b.button_type.name().to_string(),
            button_type_value: b.button_type as i32,
            image_png_base64: b.image.as_ref().map(|i| base64::engine::general_purpose::STANDARD.encode(i)),
        }
    }

    fn to_button(&self) -> CustomButton {
        CustomButton {
            guid: if self.guid.is_empty() { new_guid() } else { self.guid.clone() },
            image: self.image_png_base64.as_ref().and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok()),
            gcode: self.gcode.clone(),
            gcode2: self.gcode2.clone(),
            caption: self.caption.clone(),
            tooltip: self.tooltip.clone(),
            enable_style: self.enable_style_value,
            button_type: ButtonTypes::from_i32(self.button_type_value).unwrap_or_default(),
        }
    }
}

/// Read a button JSON file.
///
/// `path`: file. Returns the buttons, None when missing or not parseable.
fn read_json(path: &Path) -> Option<Vec<CustomButton>> {
    let text = std::fs::read_to_string(path).ok()?;
    let f: ButtonFile = serde_json::from_str(&text).ok()?;
    Some(f.buttons.iter().map(ButtonRow::to_button).collect())
}

/// Migration of the old settings entry (BinaryFormatter list imported as JSON by lg-base:
/// `{"_items":[{"GCode":..,"Caption":..,"Image":{"Data":b64},"guid":{"_a":..},..}],"_size":n}`).
///
/// `v`: JSON value of the setting. Returns the buttons or None when the shape is unknown.
fn from_legacy_json(v: &serde_json::Value) -> Option<Vec<CustomButton>> {
    let items = v.get("_items")?.as_array()?;
    let size = v.get("_size").and_then(|s| s.as_u64()).unwrap_or(items.len() as u64) as usize;
    let s = |o: &serde_json::Value, k: &str| o.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let i = |o: &serde_json::Value, k: &str| o.get(k).and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    let mut out = Vec::new();
    for o in items.iter().take(size).filter(|o| o.is_object()) {
        out.push(CustomButton {
            guid: new_guid(),
            image: o.get("Image").and_then(|im| im.get("Data")).and_then(|d| d.as_str()).and_then(|d| base64::engine::general_purpose::STANDARD.decode(d).ok()),
            gcode: s(o, "GCode"),
            gcode2: s(o, "GCode2"),
            caption: s(o, "Caption"),
            tooltip: s(o, "ToolTip"),
            enable_style: i(o, "EnableStyle"),
            button_type: ButtonTypes::from_i32(i(o, "ButtonType")).unwrap_or_default(),
        });
    }
    Some(out)
}

/// One button of an import file waiting for the user's answer.
#[derive(Debug, Clone)]
pub struct ImportCandidate {
    /// Text of the "Import "{0}"?" question.
    pub display_name: String,
    pub button: CustomButton,
}

/// What `CustomButtons.Import` would ask (C# shows the message boxes, the UI does it now).
#[derive(Debug, Clone)]
pub struct ImportPlan {
    /// True when current buttons exist: ask `Strings.BoxImportCustomButtonClearText`
    /// (Yes = clear first, No = keep, Cancel = abort import). False: behave like "No".
    pub ask_clear: bool,
    /// Buttons of the file; ask `Import "{name}"?` for each (YesNo).
    pub candidates: Vec<ImportCandidate>,
}

/// C# static class `CustomButtons`, as an instance that owns its file paths.
#[derive(Debug, Clone)]
pub struct CustomButtons {
    buttons: Vec<CustomButton>,
    user_file: PathBuf,
    standard_file: PathBuf,
}

impl CustomButtons {
    /// Buttons using the default paths: `<data>/CustomButtons.json` and
    /// `<resources>/misc/standard_buttons.json`. Not loaded yet, see [`load_file`](Self::load_file).
    pub fn new() -> Self {
        Self::with_paths(lg_base::paths::data_path().join("CustomButtons.json"), lg_base::paths::resources_dir().join("misc").join("standard_buttons.json"))
    }

    /// Buttons with explicit files (tests).
    ///
    /// `user_file`: JSON list of the user. `standard_file`: seed copied on first run.
    pub fn with_paths(user_file: PathBuf, standard_file: PathBuf) -> Self {
        Self { buttons: Vec::new(), user_file, standard_file }
    }

    /// C# `LoadFile`: seed from the standard file on first run, read the user file, fall back to
    /// the legacy setting, save when nothing could be read.
    pub fn load_file(&mut self) {
        // first run: copy standard list
        if !self.user_file.exists() && self.standard_file.exists() {
            let _ = std::fs::copy(&self.standard_file, &self.user_file);
        }
        let mut list = read_json(&self.user_file);
        if list.is_none() {
            // migration from the old settings entry
            let legacy: serde_json::Value = lg_base::settings::get_and_delete(LEGACY_SETTING, serde_json::Value::Null);
            list = Some(from_legacy_json(&legacy).unwrap_or_default());
            self.buttons = list.unwrap_or_default();
            self.save_file();
        } else {
            self.buttons = list.unwrap_or_default();
        }
    }

    /// C# `SaveFile`. Errors are logged, like `Serializer.ObjToFile`.
    pub fn save_file(&self) {
        let file = ButtonFile { buttons: self.buttons.iter().map(ButtonRow::from_button).collect() };
        let res = serde_json::to_string_pretty(&file).map_err(|e| e.to_string()).and_then(|t| {
            // tmp + rename like the C# serializer
            let tmp = self.user_file.with_extension("json.tmp");
            std::fs::write(&tmp, t).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &self.user_file).map_err(|e| e.to_string())
        });
        if let Err(e) = res {
            lg_base::logger::log_exception("Serialization", &format!("error writing {}: {e}", self.user_file.display()));
        }
    }

    /// C# `Reorder(oldindex, newindex)`: move a button and save.
    ///
    /// `oldindex`: current position. `newindex`: drop position (before removal indexing).
    /// Returns false when `oldindex` is out of range (C# throws).
    pub fn reorder(&mut self, oldindex: usize, newindex: i32) -> bool {
        if oldindex >= self.buttons.len() {
            return false;
        }
        let item = self.buttons.remove(oldindex);
        let mut newindex = newindex;
        // removing the element shifts the later indexes
        if (oldindex as i32) < newindex && newindex > 0 {
            newindex -= 1;
        }
        if newindex < 0 || newindex as usize > self.buttons.len() {
            self.buttons.push(item);
        } else {
            self.buttons.insert(newindex as usize, item);
        }
        self.save_file();
        true
    }

    /// C# `Add` (does not save).
    ///
    /// `cb`: button to append.
    pub fn add(&mut self, cb: CustomButton) {
        self.buttons.push(cb);
    }

    /// C# `Buttons`.
    pub fn buttons(&self) -> &[CustomButton] {
        &self.buttons
    }

    /// Mutable access (the editor form changes a button in place, then calls `save_file`).
    pub fn buttons_mut(&mut self) -> &mut Vec<CustomButton> {
        &mut self.buttons
    }

    /// C# `Remove` (does not save; reference equality becomes guid equality).
    ///
    /// `guid`: guid of the button. Returns true when one was removed.
    pub fn remove(&mut self, guid: &str) -> bool {
        match self.buttons.iter().position(|b| b.guid == guid) {
            Some(i) => {
                self.buttons.remove(i);
                true
            }
            None => false,
        }
    }

    /// C# `GetButton(index)`.
    ///
    /// `index`: position. Returns the button or None when out of range.
    pub fn get_button(&self, index: usize) -> Option<&CustomButton> {
        self.buttons.get(index)
    }

    /// C# `Count`.
    pub fn count(&self) -> usize {
        self.buttons.len()
    }

    /// C# `Export` after the file dialog: write all buttons as `.zbn` (deflate + BinaryFormatter).
    ///
    /// `path`: target file (absolute). Returns Ok or the error text.
    pub fn export_zbn(&self, path: &Path) -> Result<(), String> {
        let data: Vec<CustomButtonData> = self.buttons.iter().map(CustomButton::to_data).collect();
        let graph = nrbf::custom_buttons_graph(&data);
        nrbf::obj_to_file(&graph, path, true).map_err(|e| e.to_string())
    }

    /// First half of C# `Import(parent, filename)`: read the file and describe the questions.
    ///
    /// `path`: `.zbn` / `.gz` (or plain NRBF). Returns the plan, `Ok(None)` when the file does not
    /// exist or holds no button (C# returns false), Err(`Error importing custom buttons from ..`)
    /// when it cannot be decoded.
    pub fn import_plan(&self, path: &Path) -> Result<Option<ImportPlan>, String> {
        if !path.exists() {
            return Ok(None);
        }
        let err = || format!("Error importing custom buttons from {}", path.display());
        let graph = nrbf::obj_from_file(path).map_err(|_| err())?.ok_or_else(err)?;
        let data = nrbf::custom_buttons_from_graph(&graph).map_err(|_| err())?;
        if data.is_empty() {
            return Ok(None);
        }
        let candidates = data
            .iter()
            .map(CustomButton::from_data)
            .map(|b| ImportCandidate { display_name: b.import_name(), button: b })
            .collect();
        Ok(Some(ImportPlan { ask_clear: !self.buttons.is_empty(), candidates }))
    }

    /// Second half of `Import`: apply the answers. The caller saves afterwards
    /// ([`save_file`](Self::save_file)), like `PreviewForm` does.
    ///
    /// `plan`: from [`import_plan`](Self::import_plan). `clear`: answer "Yes" to the clear question
    /// (false = "No"). `accepted`: per candidate answer of `Import "{name}"?`.
    /// Returns true (C# returns true for every non-cancelled import).
    pub fn apply_import(&mut self, plan: &ImportPlan, clear: bool, accepted: &[bool]) -> bool {
        if clear {
            self.buttons.clear();
        }
        for (c, ok) in plan.candidates.iter().zip(accepted) {
            if *ok {
                self.buttons.push(c.button.clone());
            }
        }
        true
    }
}

impl Default for CustomButtons {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lg_services_cb_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn enabled_now_table() {
        let mut b = CustomButton::with_style(EnableStyles::Always);
        let none = ButtonContext::default();
        assert!(b.enabled_now(&none));
        b.set_enable_style(EnableStyles::Connected);
        assert!(!b.enabled_now(&none));
        assert!(b.enabled_now(&ButtonContext { is_connected: true, ..none }));
        b.set_enable_style(EnableStyles::Idle);
        assert!(b.enabled_now(&ButtonContext { status_idle: true, ..none }));
        b.set_enable_style(EnableStyles::Run);
        assert!(b.enabled_now(&ButtonContext { status_run: true, ..none }));
        b.set_enable_style(EnableStyles::IdleProgram);
        assert!(!b.enabled_now(&ButtonContext { status_idle: true, ..none }));
        assert!(b.enabled_now(&ButtonContext { status_idle: true, has_program: true, ..none }));
        // undefined style is never enabled
        b.enable_style = 99;
        assert!(!b.enabled_now(&ButtonContext { is_connected: true, status_idle: true, status_run: true, has_program: true }));
    }

    #[test]
    fn guid_format() {
        let g = new_guid();
        assert_eq!(g.len(), 36);
        assert_eq!(&g[14..15], "4");
        assert_ne!(g, new_guid());
    }

    #[test]
    fn zbn_equals_standard_json() {
        let d = scratch("zbn");
        let cb = CustomButtons::with_paths(d.join("u.json"), d.join("s.json"));
        let plan = cb.import_plan(&root().join("resources/misc/StandardButtons.zbn")).unwrap().unwrap();
        assert!(!plan.ask_clear);
        let gold: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root().join("tests/golden/custom_buttons/standard_buttons.json")).unwrap()).unwrap();
        let gb = gold["buttons"].as_array().unwrap();
        assert_eq!(plan.candidates.len(), gb.len());
        for (c, j) in plan.candidates.iter().zip(gb) {
            let b = &c.button;
            assert_eq!(b.guid, j["guid"].as_str().unwrap());
            assert_eq!(b.caption, j["caption"].as_str().unwrap());
            assert_eq!(b.tooltip, j["tooltip"].as_str().unwrap());
            assert_eq!(b.gcode, j["gcode"].as_str().unwrap());
            assert_eq!(b.gcode2, j["gcode2"].as_str().unwrap());
            assert_eq!(b.enable_style_raw() as i64, j["enable_style_value"].as_i64().unwrap());
            assert_eq!(b.button_type as i64, j["button_type_value"].as_i64().unwrap());
            let img = b.image_rgba().unwrap();
            assert_eq!(img.width() as i64, j["image_width"].as_i64().unwrap());
            assert_eq!(img.height() as i64, j["image_height"].as_i64().unwrap());
        }
    }

    #[test]
    fn standard_resource_json_loads_on_first_run() {
        let d = scratch("first");
        let mut cb = CustomButtons::with_paths(d.join("CustomButtons.json"), root().join("resources/misc/standard_buttons.json"));
        cb.load_file();
        assert_eq!(cb.count(), 5);
        assert!(d.join("CustomButtons.json").exists());
        assert_eq!(cb.get_button(0).unwrap().caption, "Center");
        assert_eq!(cb.get_button(3).unwrap().button_type, ButtonTypes::TwoStateButton);
        // second load reads the user copy
        let mut again = CustomButtons::with_paths(d.join("CustomButtons.json"), d.join("missing.json"));
        again.load_file();
        assert_eq!(again.buttons(), cb.buttons());
    }

    #[test]
    fn reorder_semantics() {
        let d = scratch("reorder");
        let mut cb = CustomButtons::with_paths(d.join("u.json"), d.join("none.json"));
        for n in ["a", "b", "c", "d"] {
            let mut b = CustomButton::default();
            b.caption = n.into();
            cb.add(b);
        }
        let order = |c: &CustomButtons| c.buttons().iter().map(|b| b.caption.clone()).collect::<String>();
        // drop "a" before "c" (index 2): removal shifts, lands between b and c
        assert!(cb.reorder(0, 2));
        assert_eq!(order(&cb), "bacd");
        // moving back to the front
        assert!(cb.reorder(1, 0));
        assert_eq!(order(&cb), "abcd");
        // drop beyond the end appends
        assert!(cb.reorder(0, 99));
        assert_eq!(order(&cb), "bcda");
        assert!(!cb.reorder(9, 0));
        assert!(d.join("u.json").exists());
    }

    #[test]
    fn export_import_roundtrip() {
        let d = scratch("exp");
        let mut cb = CustomButtons::with_paths(d.join("u.json"), root().join("resources/misc/standard_buttons.json"));
        cb.load_file();
        let out = d.join("out.zbn");
        cb.export_zbn(&out).unwrap();
        let mut other = CustomButtons::with_paths(d.join("u2.json"), d.join("none.json"));
        other.add(CustomButton::default());
        let plan = other.import_plan(&out).unwrap().unwrap();
        assert!(plan.ask_clear);
        assert_eq!(plan.candidates.len(), 5);
        assert_eq!(plan.candidates[2].display_name, "Framing - Laser ON - Low Power (3%)");
        // keep existing, accept 1st and 3rd
        assert!(other.apply_import(&plan, false, &[true, false, true, false, false]));
        assert_eq!(other.count(), 3);
        assert_eq!(other.get_button(1).unwrap().guid, cb.get_button(0).unwrap().guid);
        assert_eq!(other.get_button(2).unwrap().image, cb.get_button(2).unwrap().image);
        // clear + accept all
        assert!(other.apply_import(&plan, true, &[true; 5]));
        assert_eq!(other.count(), 5);
    }

    #[test]
    fn import_missing_or_bad() {
        let d = scratch("bad");
        let cb = CustomButtons::with_paths(d.join("u.json"), d.join("none.json"));
        assert!(cb.import_plan(&d.join("nope.zbn")).unwrap().is_none());
        std::fs::write(d.join("junk.zbn"), b"junk").unwrap();
        assert!(cb.import_plan(&d.join("junk.zbn")).unwrap_err().starts_with("Error importing custom buttons from"));
    }

    #[test]
    fn legacy_settings_shape() {
        let v = serde_json::json!({"_items":[{"GCode":"G0","GCode2":"","Caption":"c","ToolTip":"t","EnableStyle":3,"ButtonType":1,"Image":{"Data":"AAEC"}},null],"_size":1});
        let l = from_legacy_json(&v).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].enable_style(), Some(EnableStyles::Idle));
        assert_eq!(l[0].image.as_deref(), Some(&[0u8, 1, 2][..]));
    }

    #[test]
    fn remove_by_guid() {
        let d = scratch("rm");
        let mut cb = CustomButtons::with_paths(d.join("u.json"), d.join("none.json"));
        let b = CustomButton::default();
        let g = b.guid.clone();
        cb.add(b);
        assert!(cb.remove(&g));
        assert!(!cb.remove(&g));
    }
}
