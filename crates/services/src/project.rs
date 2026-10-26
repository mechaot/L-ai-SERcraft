// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Tools/Project.cs`: LaserGRBL project files (`.lps`).
//!
//! A `.lps` is a raw BinaryFormatter stream of `List<Dictionary<string,object>>` (no Serializer
//! header). Reading uses `lg_base::nrbf::read_graph`; writing builds the graph in this crate
//! (same record layout and id order as the .NET writer, verified byte-exact against Mono output).
//! Dictionary semantics: insertion order kept, keys compare ignoring case (`OrdinalIgnoreCase`).
//! `ProjectSettings` stays a process-global list like the C# static field.

use base64::Engine;
use lg_base::nrbf::{self, ArrayData, ArrayKind, ArrayObj, BinaryType, ClassMeta, ClassObj, Graph, Value};
use lg_dotnet::Decimal;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

const MSCORLIB: &str = "mscorlib, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089";
const LIB_APP: &str = "LaserGRBL, Version=7.14.1.0, Culture=neutral, PublicKeyToken=null";
const LIB_DRAW: &str = "System.Drawing, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b03f5f7f11d50a3a";

/// A value of the `Dictionary<string, object>` (the boxed types LaserGRBL stores).
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectValue {
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Decimal(Decimal),
    Str(String),
    /// Boxed enum: full type name (`LaserGRBL.RasterConverter.ImageProcessor+Tool`), assembly
    /// full name, numeric value.
    Enum { type_name: String, assembly: String, value: i32 },
}

impl ProjectValue {
    /// Enum value of a known LaserGRBL / System.Drawing type; the assembly is derived from the name.
    ///
    /// `type_name`: e.g. `LaserGRBL.RasterConverter.ImageProcessor+Tool`. `value`: numeric value.
    pub fn enum_of(type_name: &str, value: i32) -> Self {
        let assembly = if type_name.starts_with("System.Drawing") { LIB_DRAW } else { LIB_APP };
        ProjectValue::Enum { type_name: type_name.into(), assembly: assembly.into(), value }
    }

    /// `ImageProcessor.Tool` value.
    pub fn tool(v: i32) -> Self {
        Self::enum_of("LaserGRBL.RasterConverter.ImageProcessor+Tool", v)
    }

    /// `ImageProcessor.Direction` value.
    pub fn direction(v: i32) -> Self {
        Self::enum_of("LaserGRBL.RasterConverter.ImageProcessor+Direction", v)
    }

    /// `ImageTransform.Formula` value.
    pub fn formula(v: i32) -> Self {
        Self::enum_of("LaserGRBL.RasterConverter.ImageTransform+Formula", v)
    }

    /// `ImageTransform.DitheringMode` value.
    pub fn dithering_mode(v: i32) -> Self {
        Self::enum_of("LaserGRBL.RasterConverter.ImageTransform+DitheringMode", v)
    }

    /// `System.Drawing.Drawing2D.InterpolationMode` value.
    pub fn interpolation_mode(v: i32) -> Self {
        Self::enum_of("System.Drawing.Drawing2D.InterpolationMode", v)
    }

    /// Text of a `Str`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ProjectValue::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// `Dictionary<string, object>(StringComparer.OrdinalIgnoreCase)`, insertion ordered.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SettingsDict {
    entries: Vec<(String, ProjectValue)>,
}

impl SettingsDict {
    /// Empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// `Add(key, value)`.
    ///
    /// `key`: name. `value`: value. Returns Err(ArgumentException text) when the key exists (ignoring case).
    pub fn add(&mut self, key: &str, value: ProjectValue) -> Result<(), String> {
        if self.get(key).is_some() {
            return Err(format!("An item with the same key has already been added. Key: {key}"));
        }
        self.entries.push((key.to_string(), value));
        Ok(())
    }

    /// Indexer get (case-insensitive).
    ///
    /// `key`: name. Returns the value or None.
    pub fn get(&self, key: &str) -> Option<&ProjectValue> {
        self.entries.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }

    /// Entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ProjectValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// `Project.ProjectSettings` (static).
static PROJECT_SETTINGS: Mutex<Vec<SettingsDict>> = Mutex::new(Vec::new());

fn settings_guard() -> std::sync::MutexGuard<'static, Vec<SettingsDict>> {
    PROJECT_SETTINGS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Copy of `Project.ProjectSettings`.
pub fn project_settings() -> Vec<SettingsDict> {
    settings_guard().clone()
}

/// `AddSettings(settings)`: adds `ImageName` / `ImageBase64` of the setting `Core.LastOpenFile`
/// and appends the dictionary.
///
/// `settings`: dictionary of the conversion options. Returns Err with the C# exception text
/// (duplicate key, unreadable image).
pub fn add_settings(mut settings: SettingsDict) -> Result<(), String> {
    let last: Option<String> = {
        let v: serde_json::Value = lg_base::settings::get("Core.LastOpenFile", serde_json::Value::Null);
        v.as_str().map(str::to_string)
    };
    add_settings_for(&mut settings, last.as_deref())?;
    settings_guard().push(settings);
    Ok(())
}

/// Body of `AddSettings` with an explicit image path.
///
/// `settings`: dictionary to extend. `image`: path of the opened image (None = C# null path).
/// Returns Ok, or the error text.
pub fn add_settings_for(settings: &mut SettingsDict, image: Option<&str>) -> Result<(), String> {
    let name = image.and_then(|p| Path::new(p).file_name()).map(|n| n.to_string_lossy().into_owned());
    settings.add("ImageName", name.map(ProjectValue::Str).unwrap_or(ProjectValue::Null))?;
    let b64 = convert_image_to_base64(image.ok_or("Value cannot be null. (path)")?)?;
    settings.add("ImageBase64", ProjectValue::Str(b64))
}

/// `ClearSettings()`.
pub fn clear_settings() {
    settings_guard().clear();
}

/// `StoreSettings(filename)`: write the global list. Empty name does nothing.
///
/// `filename`: target `.lps`. Returns Ok or the I/O / format error.
pub fn store_settings(filename: &str) -> Result<(), String> {
    if filename.is_empty() {
        return Ok(());
    }
    store_to(Path::new(filename), &settings_guard())
}

/// Write a list of dictionaries as `.lps`.
///
/// `path`: target. `list`: dictionaries. Returns Ok or the error text.
pub fn store_to(path: &Path, list: &[SettingsDict]) -> Result<(), String> {
    let bytes = nrbf::write_graph(&build_graph(list)?)?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// `LoadProject(filename)`: empty list for empty name / missing file.
///
/// `filename`: `.lps` path. Returns the dictionaries, or Err when the stream is not a project.
pub fn load_project(filename: &str) -> Result<Vec<SettingsDict>, String> {
    if filename.is_empty() || !Path::new(filename).exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(filename).map_err(|e| e.to_string())?;
    from_bytes(&bytes)
}

/// Decode `.lps` bytes.
///
/// `bytes`: raw NRBF. Returns the dictionaries or the error text.
pub fn from_bytes(bytes: &[u8]) -> Result<Vec<SettingsDict>, String> {
    let g = nrbf::read_graph(bytes)?;
    let list = g.root().as_class(&g).ok_or("root is not a class")?;
    if !list.meta.name.starts_with("System.Collections.Generic.List`1[[System.Collections.Generic.Dictionary`2[[System.String") {
        return Err("root is not List<Dictionary<string,object>>".into());
    }
    let size = match list.get("_size") {
        Some(Value::I32(n)) => *n as usize,
        _ => return Err("no _size".into()),
    };
    let items = list.get("_items").and_then(|v| v.as_array(&g)).ok_or("no _items")?;
    let ArrayData::Values(vs) = &items.data else { return Err("bad _items".into()) };
    let mut out = Vec::new();
    for v in vs.iter().take(size) {
        let d = v.as_class(&g).ok_or("entry is not a dictionary")?;
        let pairs = d.get("KeyValuePairs").and_then(|v| v.as_array(&g)).ok_or("no KeyValuePairs")?;
        let ArrayData::Values(kvs) = &pairs.data else { return Err("bad pairs".into()) };
        let mut dict = SettingsDict::new();
        for kv in kvs {
            let Some(kv) = kv.as_class(&g) else { continue };
            let Some(key) = kv.get("key").and_then(|k| k.as_str(&g)) else { continue };
            let val = kv.get("value").map(|v| convert_value(&g, v)).unwrap_or(ProjectValue::Null);
            dict.add(key, val)?;
        }
        out.push(dict);
    }
    Ok(out)
}

/// Graph value to [`ProjectValue`]; unsupported record kinds become `Null`.
fn convert_value(g: &Graph, v: &Value) -> ProjectValue {
    match g.resolve(v) {
        Value::Bool(b) => ProjectValue::Bool(*b),
        Value::I32(x) => ProjectValue::I32(*x),
        Value::I64(x) => ProjectValue::I64(*x),
        Value::F32(x) => ProjectValue::F32(*x),
        Value::F64(x) => ProjectValue::F64(*x),
        Value::Decimal(d) => ProjectValue::Decimal(*d),
        Value::Str { s, .. } => ProjectValue::Str(s.to_string()),
        Value::Class(c) => match (c.get("value__"), c.meta.library_id.and_then(|l| g.libraries.get(&l))) {
            (Some(Value::I32(x)), Some(lib)) => ProjectValue::Enum { type_name: c.meta.name.clone(), assembly: lib.clone(), value: *x },
            _ => ProjectValue::Null,
        },
        _ => ProjectValue::Null,
    }
}

/// `ConvertImageToBase64`: the image file bytes, re-encoded in its own format by GDI+.
/// Here the file bytes are used as they are (same format, no re-encode).
///
/// `path`: image file. Returns base64 text or the error.
pub fn convert_image_to_base64(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // validate that it is an image like Image.FromFile would
    image::guess_format(&bytes).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// `SaveImage(base64Image, imagePath)`: decode and write the image (format from the target extension
/// is not converted; bytes are written unchanged, which GDI+ `Save(path)` does for PNG).
///
/// `base64_image`: text. `image_path`: target file. Returns Ok or the error.
pub fn save_image(base64_image: &str, image_path: &str) -> Result<(), String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64_image).map_err(|e| e.to_string())?;
    std::fs::write(image_path, bytes).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ writer

/// Prime table of `HashHelpers` (enough for projects).
const PRIMES: [usize; 12] = [3, 7, 11, 17, 23, 29, 37, 47, 59, 71, 89, 107];

/// `Dictionary` bucket count after `n` adds starting from an empty dictionary.
///
/// `n`: entries. Returns the HashSize written by `GetObjectData`.
fn hash_size(n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut size = 3usize;
    let mut count = 0;
    while count < n {
        // full: resize to ExpandPrime(2 * size)
        if count == size {
            let want = size * 2;
            size = PRIMES.iter().copied().find(|p| *p >= want).unwrap_or(want | 1);
        }
        count += 1;
    }
    size as i32
}

fn dict_type() -> String {
    format!("System.Collections.Generic.Dictionary`2[[System.String, {MSCORLIB}],[System.Object, {MSCORLIB}]]")
}

fn kvp_type() -> String {
    format!("System.Collections.Generic.KeyValuePair`2[[System.String, {MSCORLIB}],[System.Object, {MSCORLIB}]]")
}

/// Register inline objects in `g.objects` (needed by the writer to find libraries of references).
fn index(g: &mut Graph) {
    fn walk(objs: &mut HashMap<i32, Value>, v: &Value) {
        match v {
            Value::Str { id, .. } => {
                objs.insert(*id, v.clone());
            }
            Value::Class(c) => {
                objs.insert(c.object_id, v.clone());
                c.members.iter().for_each(|m| walk(objs, m));
            }
            Value::Array(a) => {
                objs.insert(a.object_id, v.clone());
                if let ArrayData::Values(vs) = &a.data {
                    vs.iter().for_each(|m| walk(objs, m));
                }
            }
            _ => {}
        }
    }
    let mut objs = HashMap::new();
    for t in &g.top {
        walk(&mut objs, t);
    }
    g.objects = objs;
}

/// Build the NRBF graph .NET writes for the list. Ids follow the BinaryFormatter order: every
/// reference to an already known object still consumes an id (observed in Mono output).
///
/// `list`: dictionaries. Returns the graph or an error (never today).
pub fn build_graph(list: &[SettingsDict]) -> Result<Graph, String> {
    let n = list.len();
    let mut next = 0i32;
    let mut new_id = || {
        next += 1;
        next
    };
    let mut g = Graph { root_id: 1, header_id: -1, major: 1, minor: 0, ..Default::default() };

    // ids of list, items array, dictionaries
    let list_id = new_id();
    let items_id = new_id();
    let dict_ids: Vec<i32> = (0..n).map(|_| new_id()).collect();
    // comparer once, then one skipped id per extra dictionary, then the pair arrays
    let mut comparer_id = 0;
    let mut arr_ids = Vec::new();
    for k in 0..n {
        if k == 0 {
            comparer_id = new_id();
        } else {
            new_id();
        }
        arr_ids.push(new_id());
    }

    let dict_t = dict_type();
    let kvp_t = kvp_type();
    let list_meta = Rc::new(ClassMeta {
        object_id: list_id,
        name: format!("System.Collections.Generic.List`1[[{dict_t}, {MSCORLIB}]]"),
        member_names: vec!["_items".into(), "_size".into(), "_version".into()],
        types: vec![BinaryType::SystemClass(format!("{dict_t}[]")), BinaryType::Primitive(8), BinaryType::Primitive(8)],
        library_id: None,
    });
    let cap = if n == 0 { 0 } else { n.next_power_of_two().max(4) };
    let mut elems: Vec<Value> = dict_ids.iter().map(|i| Value::Ref(*i)).collect();
    elems.resize(cap, Value::Null);
    g.top.push(Value::Class(Rc::new(ClassObj {
        meta: list_meta,
        object_id: list_id,
        with_id: false,
        members: vec![Value::Ref(items_id), Value::I32(n as i32), Value::I32(n as i32)],
    })));
    g.top.push(Value::Array(Rc::new(ArrayObj {
        object_id: items_id,
        kind: ArrayKind::Binary { array_type: 0, rank: 1, lower: vec![], elem: BinaryType::SystemClass(dict_t.clone()) },
        lengths: vec![cap as i32],
        data: ArrayData::Values(elems),
    })));
    if n == 0 {
        index(&mut g);
        return Ok(g);
    }

    // dictionary records
    let dict_meta = Rc::new(ClassMeta {
        object_id: dict_ids[0],
        name: dict_t.clone(),
        member_names: vec!["Version".into(), "Comparer".into(), "HashSize".into(), "KeyValuePairs".into()],
        types: vec![BinaryType::Primitive(8), BinaryType::SystemClass("System.OrdinalComparer".into()), BinaryType::Primitive(8), BinaryType::SystemClass(format!("{kvp_t}[]"))],
        library_id: None,
    });
    for k in 0..n {
        g.top.push(Value::Class(Rc::new(ClassObj {
            meta: dict_meta.clone(),
            object_id: dict_ids[k],
            with_id: k > 0,
            members: vec![Value::I32(list[k].len() as i32), Value::Ref(comparer_id), Value::I32(hash_size(list[k].len())), Value::Ref(arr_ids[k])],
        })));
    }
    // comparer
    let cmp_meta = Rc::new(ClassMeta { object_id: comparer_id, name: "System.OrdinalComparer".into(), member_names: vec!["_ignoreCase".into()], types: vec![BinaryType::Primitive(1)], library_id: None });
    g.top.push(Value::Class(Rc::new(ClassObj { meta: cmp_meta, object_id: comparer_id, with_id: false, members: vec![Value::Bool(true)] })));

    // pair arrays (ids of pairs, strings, libraries, enum objects are allocated here)
    let mut kvp_meta: Option<Rc<ClassMeta>> = None;
    let mut key_ids: HashMap<String, i32> = HashMap::new();
    let mut lib_ids: HashMap<String, i32> = HashMap::new();
    let mut enum_metas: HashMap<String, Rc<ClassMeta>> = HashMap::new();
    let mut enum_objs: Vec<Value> = Vec::new();
    let mut arrays: Vec<Value> = Vec::new();
    for (k, d) in list.iter().enumerate() {
        let mut pairs = Vec::new();
        for (key, val) in d.iter() {
            let kid = new_id();
            // library of an enum value is registered before the key string
            let mut lib_id = 0;
            if let ProjectValue::Enum { assembly, .. } = val {
                lib_id = match lib_ids.get(assembly) {
                    Some(l) => *l,
                    None => {
                        let l = new_id();
                        lib_ids.insert(assembly.clone(), l);
                        g.libraries.insert(l, assembly.clone());
                        l
                    }
                };
            }
            let key_v = match key_ids.get(key) {
                Some(id) => {
                    // known string: reference, still burns an id
                    new_id();
                    Value::Ref(*id)
                }
                None => {
                    let id = new_id();
                    key_ids.insert(key.to_string(), id);
                    Value::Str { id, s: key.into() }
                }
            };
            let val_v = match val {
                ProjectValue::Null => Value::Null,
                ProjectValue::Bool(b) => Value::Bool(*b),
                ProjectValue::I32(x) => Value::I32(*x),
                ProjectValue::I64(x) => Value::I64(*x),
                ProjectValue::F32(x) => Value::F32(*x),
                ProjectValue::F64(x) => Value::F64(*x),
                ProjectValue::Decimal(x) => Value::Decimal(*x),
                // equal strings share one object (literals are interned in .NET)
                ProjectValue::Str(s) => match key_ids.get(s) {
                    Some(id) => {
                        new_id();
                        Value::Ref(*id)
                    }
                    None => {
                        let id = new_id();
                        key_ids.insert(s.clone(), id);
                        Value::Str { id, s: s.as_str().into() }
                    }
                },
                ProjectValue::Enum { type_name, value, .. } => {
                    let oid = new_id();
                    let meta = enum_metas
                        .entry(type_name.clone())
                        .or_insert_with(|| Rc::new(ClassMeta { object_id: oid, name: type_name.clone(), member_names: vec!["value__".into()], types: vec![BinaryType::Primitive(8)], library_id: Some(lib_id) }))
                        .clone();
                    enum_objs.push(Value::Class(Rc::new(ClassObj { meta: meta.clone(), object_id: oid, with_id: meta.object_id != oid, members: vec![Value::I32(*value)] })));
                    Value::Ref(oid)
                }
            };
            let meta = kvp_meta
                .get_or_insert_with(|| Rc::new(ClassMeta { object_id: -kid, name: kvp_t.clone(), member_names: vec!["key".into(), "value".into()], types: vec![BinaryType::String, BinaryType::Object], library_id: None }))
                .clone();
            pairs.push(Value::Class(Rc::new(ClassObj { with_id: meta.object_id != -kid, meta, object_id: -kid, members: vec![key_v, val_v] })));
        }
        arrays.push(Value::Array(Rc::new(ArrayObj {
            object_id: arr_ids[k],
            kind: ArrayKind::Binary { array_type: 0, rank: 1, lower: vec![], elem: BinaryType::SystemClass(kvp_t.clone()) },
            lengths: vec![pairs.len() as i32],
            data: ArrayData::Values(pairs),
        })));
    }
    g.top.extend(arrays);
    g.top.extend(enum_objs);
    index(&mut g);
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lg_dotnet::parse_decimal;

    fn root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn sample() -> Vec<SettingsDict> {
        let mut d1 = SettingsDict::new();
        d1.add("GrayScaleConversion.RasterConversionTool", ProjectValue::tool(2)).unwrap();
        d1.add("GrayScaleConversion.Line2LineOptions.Direction", ProjectValue::direction(4)).unwrap();
        d1.add("GrayScaleConversion.Line2LineOptions.Quality", ProjectValue::Decimal(parse_decimal("3.5").unwrap())).unwrap();
        d1.add("GrayScaleConversion.Line2LineOptions.Preview", ProjectValue::Bool(true)).unwrap();
        d1.add("GrayScaleConversion.Parameters.Interpolation", ProjectValue::interpolation_mode(7)).unwrap();
        d1.add("GrayScaleConversion.Parameters.R", ProjectValue::I32(100)).unwrap();
        d1.add("ImageName", ProjectValue::Str("first.png".into())).unwrap();
        d1.add("ImageBase64", ProjectValue::Str("iVBORw0KGgo=".into())).unwrap();
        let mut d2 = SettingsDict::new();
        d2.add("GrayScaleConversion.RasterConversionTool", ProjectValue::tool(0)).unwrap();
        d2.add("GrayScaleConversion.Line2LineOptions.Quality", ProjectValue::Decimal(parse_decimal("10.0").unwrap())).unwrap();
        d2.add("GrayScaleConversion.Parameters.Threshold.Enabled", ProjectValue::Bool(false)).unwrap();
        d2.add("GrayScaleConversion.Parameters.Brightness", ProjectValue::I32(-7)).unwrap();
        d2.add("ImageName", ProjectValue::Str("second.jpg".into())).unwrap();
        d2.add("ImageBase64", ProjectValue::Str("/9j/4AAQ".into())).unwrap();
        vec![d1, d2]
    }

    fn mk(name: &str) -> SettingsDict {
        let mut d = SettingsDict::new();
        d.add("k.tool", ProjectValue::tool(1)).unwrap();
        d.add("k.int", ProjectValue::I32(5)).unwrap();
        d.add("ImageName", ProjectValue::Str(name.into())).unwrap();
        d.add("ImageBase64", ProjectValue::Str("QQ==".into())).unwrap();
        d
    }

    #[test]
    fn mono_project_loads() {
        let l = load_project(root().join("tests/golden/services/project_sample.lps").to_str().unwrap()).unwrap();
        assert_eq!(l, sample());
        assert_eq!(l[0].get("imagename").unwrap().as_str(), Some("first.png"));
    }

    #[test]
    fn writer_is_byte_exact_with_mono() {
        let same = |list: &[SettingsDict], name: &str| {
            let ours = nrbf::write_graph(&build_graph(list).unwrap()).unwrap();
            let gold = std::fs::read(root().join("tests/golden/services").join(name)).unwrap();
            let diff = (0..ours.len().min(gold.len())).find(|&i| ours[i] != gold[i]);
            assert!(ours == gold, "{name}: first diff {diff:?}, lens {} {}", ours.len(), gold.len());
        };
        same(&sample(), "project_sample.lps");
        same(&[mk("a.png")], "project_one.lps");
        same(&[mk("a.png"), mk("b.png"), mk("c.png")], "project_three.lps");
        same(&[mk("a.png"), mk("b.png")][..0], "project_empty.lps");
    }

    #[test]
    fn empty_list_roundtrip() {
        let bytes = nrbf::write_graph(&build_graph(&[]).unwrap()).unwrap();
        assert!(from_bytes(&bytes).unwrap().is_empty());
    }

    #[test]
    fn hash_sizes() {
        assert_eq!((hash_size(1), hash_size(3), hash_size(4), hash_size(6), hash_size(7), hash_size(8)), (3, 3, 7, 7, 7, 17));
    }

    #[test]
    fn dict_semantics() {
        let mut d = SettingsDict::new();
        d.add("A", ProjectValue::I32(1)).unwrap();
        assert!(d.add("a", ProjectValue::I32(2)).is_err());
    }

    #[test]
    fn store_load_and_images() {
        let dir = lg_base::paths::test_dir("services_prj");
        // tiny png as the "opened image"
        let png = dir.join("pic.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255])).save(&png).unwrap();
        let mut d = mk("x");
        d.entries.retain(|(k, _)| k != "ImageName" && k != "ImageBase64");
        add_settings_for(&mut d, Some(png.to_str().unwrap())).unwrap();
        assert_eq!(d.get("ImageName").unwrap().as_str(), Some("pic.png"));
        let out = dir.join("t.lps");
        store_to(&out, &[d.clone()]).unwrap();
        let back = load_project(out.to_str().unwrap()).unwrap();
        assert_eq!(back, vec![d.clone()]);
        let b64 = back[0].get("ImageBase64").unwrap().as_str().unwrap();
        let restored = dir.join("restored.png");
        save_image(b64, restored.to_str().unwrap()).unwrap();
        assert_eq!(std::fs::read(&png).unwrap(), std::fs::read(&restored).unwrap());
        assert!(load_project("").unwrap().is_empty());
        assert!(load_project(dir.join("nope.lps").to_str().unwrap()).unwrap().is_empty());
        // static list semantics
        clear_settings();
        assert!(project_settings().is_empty());
    }
}
