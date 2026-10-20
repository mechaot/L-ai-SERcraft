//! MS-NRBF (.NET `BinaryFormatter`) reader / writer and the `Tools/Serializer.cs` file container.
//!
//! The graph keeps the record tree exactly as in the stream (inline objects, `MemberReference`s,
//! class metadata, library ids) so that [`write_graph`] reproduces the bytes .NET wrote.
//! References can be resolved with [`Graph::resolve`].

mod convert;
mod reader;
mod serializer;
mod writer;

pub use convert::{custom_buttons_from_graph, custom_buttons_graph, custom_buttons_graph_ex, import_settings_bin, to_json, CustomButtonData};
pub use reader::read_graph;
pub use serializer::{obj_from_bytes, obj_from_file, obj_to_bytes, obj_to_file, SerializerError};
pub use writer::write_graph;

use lg_dotnet::Decimal;
use std::collections::HashMap;
use std::rc::Rc;

/// BinaryType of a member / array element.
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryType {
    Primitive(u8),
    String,
    Object,
    SystemClass(String),
    Class(String, i32),
    ObjectArray,
    StringArray,
    PrimitiveArray(u8),
}

/// Class metadata (shared by `ClassWithId` instances).
#[derive(Debug, Clone, PartialEq)]
pub struct ClassMeta {
    /// ObjectId of the defining record.
    pub object_id: i32,
    pub name: String,
    pub member_names: Vec<String>,
    pub types: Vec<BinaryType>,
    /// `Some` for `ClassWithMembersAndTypes`, `None` for system classes.
    pub library_id: Option<i32>,
}

/// Class instance.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassObj {
    pub meta: Rc<ClassMeta>,
    pub object_id: i32,
    /// True when stream used `ClassWithId` (metadata defined earlier).
    pub with_id: bool,
    pub members: Vec<Value>,
}

impl ClassObj {
    /// Member by name.
    ///
    /// `name`: member name. Returns the value or None.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.meta.member_names.iter().position(|n| n == name).and_then(|i| self.members.get(i))
    }
}

/// Array record kind.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayKind {
    SinglePrimitive(u8),
    SingleObject,
    SingleString,
    Binary { array_type: u8, rank: i32, lower: Vec<i32>, elem: BinaryType },
}

/// Array payload.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayData {
    Bytes(Vec<u8>),
    Values(Vec<Value>),
}

/// Array instance.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayObj {
    pub object_id: i32,
    pub kind: ArrayKind,
    pub lengths: Vec<i32>,
    pub data: ArrayData,
}

/// A value in the graph.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Byte(u8),
    SByte(i8),
    Char(char),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Decimal(Decimal),
    /// Ticks.
    TimeSpan(i64),
    /// Raw 64 bit (62 bit ticks + 2 bit kind).
    DateTime(u64),
    /// `BinaryObjectString`.
    Str { id: i32, s: Rc<str> },
    Class(Rc<ClassObj>),
    Array(Rc<ArrayObj>),
    /// `MemberReference` to an object id.
    Ref(i32),
}

/// Parsed stream.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    pub root_id: i32,
    pub header_id: i32,
    pub major: i32,
    pub minor: i32,
    /// Library id -> name.
    pub libraries: HashMap<i32, String>,
    /// Top level records in stream order (first = root object).
    pub top: Vec<Value>,
    /// Object id -> value (inline objects).
    pub objects: HashMap<i32, Value>,
}

impl Graph {
    /// Follow `Ref`s.
    ///
    /// `v`: value. Returns the referenced inline value (or `v` itself).
    pub fn resolve<'a>(&'a self, v: &'a Value) -> &'a Value {
        let mut cur = v;
        for _ in 0..64 {
            match cur {
                Value::Ref(id) => match self.objects.get(id) {
                    Some(n) => cur = n,
                    None => return &Value::Null,
                },
                _ => return cur,
            }
        }
        cur
    }

    /// Root object.
    pub fn root(&self) -> &Value {
        self.objects.get(&self.root_id).or(self.top.first()).unwrap_or(&Value::Null)
    }
}

impl Value {
    /// String content (after resolving).
    ///
    /// `g`: graph for refs. Returns the text for strings.
    pub fn as_str<'a>(&'a self, g: &'a Graph) -> Option<&'a str> {
        match g.resolve(self) {
            Value::Str { s, .. } => Some(s),
            _ => None,
        }
    }

    /// Class object (after resolving).
    ///
    /// `g`: graph for refs.
    pub fn as_class<'a>(&'a self, g: &'a Graph) -> Option<&'a ClassObj> {
        match g.resolve(self) {
            Value::Class(c) => Some(c),
            _ => None,
        }
    }

    /// Array object (after resolving).
    ///
    /// `g`: graph for refs.
    pub fn as_array<'a>(&'a self, g: &'a Graph) -> Option<&'a ArrayObj> {
        match g.resolve(self) {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
}
