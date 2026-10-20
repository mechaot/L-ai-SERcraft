//! Graph <-> JSON / typed helpers: settings import, `List<CustomButton>` read and build.
//!
//! JSON mapping (best effort, documented in PORTING.md):
//! bool/ints/floats -> numbers; `decimal` -> string; `char`/string/`Guid` -> string;
//! `TimeSpan` -> "c" string; `DateTime` -> `YYYY-MM-DDTHH:MM:SS.fffffff`; `Version` -> "a.b.c[.d]";
//! enums (class with only `value__`) -> integer; `byte[]` -> base64; arrays -> arrays;
//! other classes -> object of member name -> value.

use super::*;
use base64::Engine;
use serde_json::{Map, Value as J};
use std::collections::BTreeMap;
use std::path::Path;

/// Assembly version string written into library names (C# `AssemblyVersion`).
const ASM_VERSION: &str = "7.14.1.0";

/// Convert a value to JSON.
///
/// `g`: graph (for refs). `v`: value. Returns the JSON (see module doc for the mapping).
pub fn to_json(g: &Graph, v: &Value) -> J {
    to_json_d(g, v, 0)
}

/// Finite f64 to JSON number, null otherwise.
fn num(x: f64) -> J {
    serde_json::Number::from_f64(x).map_or(J::Null, J::Number)
}

fn to_json_d(g: &Graph, v: &Value, depth: usize) -> J {
    if depth > 64 {
        return J::Null;
    }
    match g.resolve(v) {
        Value::Null | Value::Ref(_) => J::Null,
        Value::Bool(b) => J::Bool(*b),
        Value::Byte(x) => J::from(*x),
        Value::SByte(x) => J::from(*x),
        Value::Char(c) => J::String(c.to_string()),
        Value::I16(x) => J::from(*x),
        Value::U16(x) => J::from(*x),
        Value::I32(x) => J::from(*x),
        Value::U32(x) => J::from(*x),
        Value::I64(x) => J::from(*x),
        Value::U64(x) => J::from(*x),
        // shortest f32 text keeps 1.5f as 1.5
        Value::F32(x) => num(x.to_string().parse::<f64>().unwrap_or(*x as f64)),
        Value::F64(x) => num(*x),
        Value::Decimal(d) => J::String(lg_dotnet::fmt_decimal(*d)),
        Value::TimeSpan(t) => J::String(lg_dotnet::TimeSpan::from_ticks(*t).to_string()),
        Value::DateTime(d) => J::String(datetime_text(*d)),
        Value::Str { s, .. } => J::String(s.to_string()),
        Value::Array(a) => match &a.data {
            ArrayData::Bytes(b) => J::String(base64::engine::general_purpose::STANDARD.encode(b)),
            ArrayData::Values(vs) => J::Array(vs.iter().map(|x| to_json_d(g, x, depth + 1)).collect()),
        },
        Value::Class(c) => class_to_json(g, c, depth),
    }
}

/// .NET DateTime raw (62 bit ticks) to text.
fn datetime_text(raw: u64) -> String {
    let ticks = (raw & 0x3fff_ffff_ffff_ffff) as i64;
    let unix_ticks = ticks - 621_355_968_000_000_000;
    let secs = unix_ticks.div_euclid(10_000_000);
    let frac = unix_ticks.rem_euclid(10_000_000);
    match chrono::DateTime::from_timestamp(secs, 0) {
        Some(d) => format!("{}.{:07}", d.naive_utc().format("%Y-%m-%dT%H:%M:%S"), frac),
        None => String::new(),
    }
}

/// Integer member helper.
fn gi(c: &ClassObj, name: &str) -> Option<i64> {
    match c.get(name)? {
        Value::I32(x) => Some(*x as i64),
        Value::I16(x) => Some(*x as i64),
        Value::U32(x) => Some(*x as i64),
        Value::I64(x) => Some(*x),
        Value::U64(x) => Some(*x as i64),
        Value::Byte(x) => Some(*x as i64),
        _ => None,
    }
}

fn class_to_json(g: &Graph, c: &ClassObj, depth: usize) -> J {
    let name = c.meta.name.as_str();
    // enum: only value__
    if c.meta.member_names.len() == 1 && c.meta.member_names[0] == "value__" {
        return to_json_d(g, &c.members[0], depth + 1);
    }
    match name {
        "System.Version" => {
            let p: Vec<i64> = ["_Major", "_Minor", "_Build", "_Revision"].iter().filter_map(|n| gi(c, n)).filter(|x| *x >= 0).collect();
            return J::String(p.iter().map(i64::to_string).collect::<Vec<_>>().join("."));
        }
        "System.TimeSpan" => {
            if let Some(t) = gi(c, "_ticks") {
                return J::String(lg_dotnet::TimeSpan::from_ticks(t).to_string());
            }
        }
        "System.DateTime" => {
            if let Some(Value::U64(d)) = c.get("dateData") {
                return J::String(datetime_text(*d));
            }
            if let Some(t) = gi(c, "ticks") {
                return J::String(datetime_text(t as u64));
            }
        }
        "System.Decimal" => {
            if let (Some(f), Some(hi), Some(lo), Some(mid)) = (gi(c, "flags"), gi(c, "hi"), gi(c, "lo"), gi(c, "mid")) {
                let d = Decimal::from_parts(lo as u32, mid as u32, hi as u32, f < 0, ((f >> 16) & 0xff) as u32);
                return J::String(lg_dotnet::fmt_decimal(d));
            }
        }
        "System.Guid" => {
            if let Some(s) = guid_text(c) {
                return J::String(s);
            }
        }
        _ => {}
    }
    let mut m = Map::new();
    for (n, v) in c.meta.member_names.iter().zip(&c.members) {
        m.insert(n.clone(), to_json_d(g, v, depth + 1));
    }
    J::Object(m)
}

/// `System.Guid` object to text.
fn guid_text(c: &ClassObj) -> Option<String> {
    let a = gi(c, "_a")? as u32;
    let b = gi(c, "_b")? as u16;
    let cc = gi(c, "_c")? as u16;
    let d: Vec<u8> = ["_d", "_e", "_f", "_g", "_h", "_i", "_j", "_k"].iter().map(|n| gi(c, n).map(|x| x as u8)).collect::<Option<_>>()?;
    Some(format!("{a:08x}-{b:04x}-{cc:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]))
}

/// Import a Windows `LaserGRBL.Settings.bin` (a `Dictionary<string,object>` in BinaryFormatter).
///
/// `p`: file path (plain NRBF, no Serializer header). Returns key -> JSON value (see module doc),
/// entries that cannot be decoded are skipped; Err when the file is not a dictionary.
pub fn import_settings_bin(p: &Path) -> Result<BTreeMap<String, J>, String> {
    let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
    let g = read_graph(&bytes)?;
    let root = g.root().as_class(&g).ok_or("root is not a class")?;
    if !root.meta.name.starts_with("System.Collections.Generic.Dictionary") {
        return Err("root is not a Dictionary".into());
    }
    let pairs = root.get("KeyValuePairs").and_then(|v| v.as_array(&g)).ok_or("no KeyValuePairs")?;
    let ArrayData::Values(items) = &pairs.data else { return Err("bad pairs".into()) };
    let mut out = BTreeMap::new();
    for it in items {
        let Some(kv) = it.as_class(&g) else { continue };
        let (Some(k), Some(v)) = (kv.get("key").and_then(|k| k.as_str(&g)), kv.get("value")) else { continue };
        out.insert(k.to_string(), to_json(&g, v));
    }
    Ok(out)
}

/// Fields of `LaserGRBL.CustomButton`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CustomButtonData {
    /// Guid text `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
    pub guid: String,
    /// Bytes of the serialized `System.Drawing.Bitmap` (`Data` member, normally a PNG).
    pub image: Option<Vec<u8>>,
    pub gcode: Option<String>,
    pub gcode2: Option<String>,
    pub caption: Option<String>,
    pub tooltip: Option<String>,
    /// `EnableStyles` value (0 Always, 1 Connected, 3 Idle, 4 Run, 10 IdleProgram).
    pub enable_style: i32,
    /// `ButtonTypes` value (0 Button, 1 TwoStateButton, 2 PushButton).
    pub button_type: i32,
}

/// Read a `List<CustomButton>` graph.
///
/// `g`: graph from [`read_graph`]. Returns the buttons or an error text.
pub fn custom_buttons_from_graph(g: &Graph) -> Result<Vec<CustomButtonData>, String> {
    let list = g.root().as_class(g).ok_or("root is not a class")?;
    if !list.meta.name.starts_with("System.Collections.Generic.List`1[[LaserGRBL.CustomButton") {
        return Err("root is not List<CustomButton>".into());
    }
    let size = gi(list, "_size").ok_or("no _size")? as usize;
    let items = list.get("_items").and_then(|v| v.as_array(g)).ok_or("no _items")?;
    let ArrayData::Values(vs) = &items.data else { return Err("bad _items".into()) };
    let mut out = Vec::new();
    for v in vs.iter().take(size) {
        let c = v.as_class(g).ok_or("button is not a class")?;
        let s = |n: &str| c.get(n).and_then(|x| x.as_str(g)).map(str::to_string);
        let image = c.get("Image").and_then(|v| v.as_class(g)).and_then(|b| b.get("Data")).and_then(|d| d.as_array(g)).and_then(|a| match &a.data {
            ArrayData::Bytes(b) => Some(b.clone()),
            _ => None,
        });
        let en = |n: &str| c.get(n).and_then(|v| v.as_class(g)).and_then(|e| gi(e, "value__")).unwrap_or(0) as i32;
        out.push(CustomButtonData {
            guid: c.get("guid").and_then(|v| v.as_class(g)).and_then(guid_text).unwrap_or_default(),
            image,
            gcode: s("GCode"),
            gcode2: s("GCode2"),
            caption: s("Caption"),
            tooltip: s("ToolTip"),
            enable_style: en("EnableStyle"),
            button_type: en("ButtonType"),
        });
    }
    Ok(out)
}

/// Id generator + node builder for [`custom_buttons_graph`].
struct B {
    next: i32,
    empty_id: Option<i32>,
}

impl B {
    /// Next positive id.
    fn pos(&mut self) -> i32 {
        self.next += 1;
        self.next
    }
    /// Next negative id (value types).
    fn neg(&mut self) -> i32 {
        self.next += 1;
        -self.next
    }
    /// String member: null -> Null, "" shared like String.Empty, else new inline string.
    fn string(&mut self, s: &Option<String>) -> Value {
        let id = self.pos();
        match s {
            None => Value::Null,
            Some(t) if t.is_empty() => match self.empty_id {
                Some(e) => Value::Ref(e),
                None => {
                    self.empty_id = Some(id);
                    Value::Str { id, s: "".into() }
                }
            },
            Some(t) => Value::Str { id, s: t.as_str().into() },
        }
    }
}

/// Parse guid text to the `System.Guid` field values.
///
/// `s`: guid text. Returns (a, b, c, [d..k]) or None.
fn parse_guid(s: &str) -> Option<(i32, i16, i16, [u8; 8])> {
    let h: String = s.chars().filter(|c| *c != '-').collect();
    if h.len() != 32 {
        return None;
    }
    let by: Vec<u8> = (0..16).map(|i| u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).ok()).collect::<Option<_>>()?;
    let a = u32::from_be_bytes(by[0..4].try_into().ok()?) as i32;
    let b = u16::from_be_bytes(by[4..6].try_into().ok()?) as i16;
    let c = u16::from_be_bytes(by[6..8].try_into().ok()?) as i16;
    Some((a, b, c, by[8..16].try_into().ok()?))
}

/// Register all inline objects of `top` into `g.objects`.
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

/// Build the graph .NET writes for a `List<CustomButton>` (same record layout, ids and ordering).
///
/// `buttons`: button data. Returns a [`Graph`] that [`write_graph`] turns into the stream.
pub fn custom_buttons_graph(buttons: &[CustomButtonData]) -> Graph {
    let lib_app = format!("LaserGRBL, Version={ASM_VERSION}, Culture=neutral, PublicKeyToken=null");
    let lib_draw = "System.Drawing, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b03f5f7f11d50a3a";
    custom_buttons_graph_ex(buttons, &lib_app, lib_draw, buttons.len() as i32)
}

/// Like [`custom_buttons_graph`] with explicit assembly names and `List._version` (for byte-exact tests).
///
/// `buttons`: data. `lib_app`: full LaserGRBL assembly name. `lib_draw`: full System.Drawing assembly name.
/// `list_version`: value of the list's `_version` field. Returns the graph.
pub fn custom_buttons_graph_ex(buttons: &[CustomButtonData], lib_app: &str, lib_draw: &str, list_version: i32) -> Graph {
    let (lib_app, lib_draw) = (lib_app.to_string(), lib_draw.to_string());
    let n = buttons.len();
    let mut g = Graph { root_id: 1, header_id: -1, major: 1, minor: 0, ..Default::default() };
    g.libraries.insert(2, lib_app.clone());
    let mut b = B { next: 2, empty_id: None };

    // List<T> + items array
    let list_meta = Rc::new(ClassMeta {
        object_id: 1,
        name: format!("System.Collections.Generic.List`1[[LaserGRBL.CustomButton, {lib_app}]]"),
        member_names: vec!["_items".into(), "_size".into(), "_version".into()],
        types: vec![BinaryType::Class("LaserGRBL.CustomButton[]".into(), 2), BinaryType::Primitive(8), BinaryType::Primitive(8)],
        library_id: None,
    });
    let items_id = b.pos();
    let ids: Vec<i32> = (0..n).map(|_| b.pos()).collect();
    // List<T> growth: 0 -> 4 -> 8 -> 16 ...
    let cap = if n == 0 { 0 } else { n.next_power_of_two().max(4) };
    let mut elems: Vec<Value> = ids.iter().map(|i| Value::Ref(*i)).collect();
    elems.resize(cap, Value::Null);
    let list = Value::Class(Rc::new(ClassObj {
        meta: list_meta,
        object_id: 1,
        with_id: false,
        members: vec![Value::Ref(items_id), Value::I32(n as i32), Value::I32(list_version)],
    }));
    let items = Value::Array(Rc::new(ArrayObj {
        object_id: items_id,
        kind: ArrayKind::Binary { array_type: 0, rank: 1, lower: vec![], elem: BinaryType::Class("LaserGRBL.CustomButton".into(), 2) },
        lengths: vec![cap as i32],
        data: ArrayData::Values(elems),
    }));
    g.top.push(list);
    g.top.push(items);

    // buttons
    let mut btn_meta: Option<Rc<ClassMeta>> = None;
    let mut guid_meta: Option<Rc<ClassMeta>> = None;
    let mut es_meta: Option<Rc<ClassMeta>> = None;
    let mut bt_meta: Option<Rc<ClassMeta>> = None;
    let mut bmp_meta: Option<Rc<ClassMeta>> = None;
    let mut bitmaps: Vec<(Value, Vec<u8>)> = Vec::new();
    let mut tops: Vec<Value> = Vec::new();
    let mut draw_lib: i32 = 0;
    for (bd, id) in buttons.iter().zip(&ids) {
        if draw_lib == 0 {
            // System.Drawing library id is taken when the first button metadata is written
            let l = b.pos();
            draw_lib = l;
            g.libraries.insert(l, lib_draw.clone());
        }
        let first = btn_meta.is_none();
        // guid
        let gid = b.neg();
        let guid_m = guid_meta.get_or_insert_with(|| {
            Rc::new(ClassMeta {
                object_id: gid,
                name: "System.Guid".into(),
                member_names: "_a _b _c _d _e _f _g _h _i _j _k".split(' ').map(String::from).collect(),
                types: vec![
                    BinaryType::Primitive(8),
                    BinaryType::Primitive(7),
                    BinaryType::Primitive(7),
                    BinaryType::Primitive(2),
                    BinaryType::Primitive(2),
                    BinaryType::Primitive(2),
                    BinaryType::Primitive(2),
                    BinaryType::Primitive(2),
                    BinaryType::Primitive(2),
                    BinaryType::Primitive(2),
                    BinaryType::Primitive(2),
                ],
                library_id: None,
            })
        });
        let (a, bb, cc, d) = parse_guid(&bd.guid).unwrap_or((0, 0, 0, [0; 8]));
        let mut gm = vec![Value::I32(a), Value::I16(bb), Value::I16(cc)];
        gm.extend(d.iter().map(|x| Value::Byte(*x)));
        let guid_v = Value::Class(Rc::new(ClassObj { meta: guid_m.clone(), object_id: gid, with_id: guid_m.object_id != gid, members: gm }));
        // image (bitmap object id, data array assigned later)
        let image_v = match &bd.image {
            Some(data) => {
                let bid = b.pos();
                bitmaps.push((Value::Null, data.clone()));
                if let Some(last) = bitmaps.last_mut() {
                    last.0 = Value::Ref(bid);
                }
                Value::Ref(bid)
            }
            None => Value::Null,
        };
        let gcode = b.string(&bd.gcode);
        let gcode2 = b.string(&bd.gcode2);
        let caption = b.string(&bd.caption);
        let tooltip = b.string(&bd.tooltip);
        let eid = b.neg();
        let es = es_meta.get_or_insert_with(|| {
            Rc::new(ClassMeta { object_id: eid, name: "LaserGRBL.CustomButton+EnableStyles".into(), member_names: vec!["value__".into()], types: vec![BinaryType::Primitive(8)], library_id: Some(2) })
        });
        let es_v = Value::Class(Rc::new(ClassObj { meta: es.clone(), object_id: eid, with_id: es.object_id != eid, members: vec![Value::I32(bd.enable_style)] }));
        let tid = b.neg();
        let bt = bt_meta.get_or_insert_with(|| {
            Rc::new(ClassMeta { object_id: tid, name: "LaserGRBL.CustomButton+ButtonTypes".into(), member_names: vec!["value__".into()], types: vec![BinaryType::Primitive(8)], library_id: Some(2) })
        });
        let bt_v = Value::Class(Rc::new(ClassObj { meta: bt.clone(), object_id: tid, with_id: bt.object_id != tid, members: vec![Value::I32(bd.button_type)] }));
        let meta = btn_meta.get_or_insert_with(|| {
            Rc::new(ClassMeta {
                object_id: *id,
                name: "LaserGRBL.CustomButton".into(),
                member_names: "guid Image GCode GCode2 Caption ToolTip EnableStyle ButtonType".split(' ').map(String::from).collect(),
                types: vec![
                    BinaryType::SystemClass("System.Guid".into()),
                    BinaryType::Class("System.Drawing.Bitmap".into(), draw_lib),
                    BinaryType::String,
                    BinaryType::String,
                    BinaryType::String,
                    BinaryType::String,
                    BinaryType::Class("LaserGRBL.CustomButton+EnableStyles".into(), 2),
                    BinaryType::Class("LaserGRBL.CustomButton+ButtonTypes".into(), 2),
                ],
                library_id: Some(2),
            })
        });
        let _ = first;
        tops.push(Value::Class(Rc::new(ClassObj {
            meta: meta.clone(),
            object_id: *id,
            with_id: meta.object_id != *id,
            members: vec![guid_v, image_v, gcode, gcode2, caption, tooltip, es_v, bt_v],
        })));
    }
    g.top.extend(tops);

    // bitmaps then their byte arrays
    let mut arrays = Vec::new();
    for (r, data) in bitmaps {
        let Value::Ref(bid) = r else { continue };
        let aid = b.pos();
        let meta = bmp_meta.get_or_insert_with(|| {
            Rc::new(ClassMeta { object_id: bid, name: "System.Drawing.Bitmap".into(), member_names: vec!["Data".into()], types: vec![BinaryType::PrimitiveArray(2)], library_id: Some(draw_lib) })
        });
        g.top.push(Value::Class(Rc::new(ClassObj { meta: meta.clone(), object_id: bid, with_id: meta.object_id != bid, members: vec![Value::Ref(aid)] })));
        arrays.push(Value::Array(Rc::new(ArrayObj { object_id: aid, kind: ArrayKind::SinglePrimitive(2), lengths: vec![data.len() as i32], data: ArrayData::Bytes(data) })));
    }
    g.top.extend(arrays);
    index(&mut g);
    g
}
