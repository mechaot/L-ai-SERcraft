// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! NRBF reader.

use super::*;

struct R<'a> {
    b: &'a [u8],
    p: usize,
    g: Graph,
    metas: HashMap<i32, Rc<ClassMeta>>,
    pending_nulls: usize,
    depth: usize,
}

/// One decoded record in "value position".
enum Item {
    V(Value),
    Nulls(usize),
    End,
}

type Res<T> = Result<T, String>;

impl<'a> R<'a> {
    /// Read `n` bytes.
    fn take(&mut self, n: usize) -> Res<&'a [u8]> {
        let end = self.p.checked_add(n).filter(|e| *e <= self.b.len()).ok_or("unexpected end of stream")?;
        let s = &self.b[self.p..end];
        self.p = end;
        Ok(s)
    }
    fn u8(&mut self) -> Res<u8> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Res<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "i32")?))
    }
    fn i64(&mut self) -> Res<i64> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().map_err(|_| "i64")?))
    }
    /// 7-bit encoded length + UTF-8.
    fn lps(&mut self) -> Res<String> {
        let mut len = 0usize;
        let mut shift = 0;
        loop {
            let b = self.u8()?;
            len |= ((b & 0x7f) as usize) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift > 35 {
                return Err("bad string length".into());
            }
        }
        Ok(String::from_utf8_lossy(self.take(len)?).into_owned())
    }

    /// Primitive of type `t` (raw, no header).
    fn prim(&mut self, t: u8) -> Res<Value> {
        Ok(match t {
            1 => Value::Bool(self.u8()? != 0),
            2 => Value::Byte(self.u8()?),
            3 => {
                // UTF-8 char: lead byte decides length
                let lead = self.u8()?;
                let n = if lead < 0x80 { 0 } else if lead >> 5 == 0b110 { 1 } else if lead >> 4 == 0b1110 { 2 } else { 3 };
                let mut v = vec![lead];
                v.extend_from_slice(self.take(n)?);
                Value::Char(std::str::from_utf8(&v).ok().and_then(|s| s.chars().next()).unwrap_or('\u{fffd}'))
            }
            5 => {
                let s = self.lps()?;
                Value::Decimal(lg_dotnet::parse_decimal(&s).ok_or_else(|| format!("bad decimal {s}"))?)
            }
            6 => Value::F64(f64::from_le_bytes(self.take(8)?.try_into().map_err(|_| "f64")?)),
            7 => Value::I16(i16::from_le_bytes(self.take(2)?.try_into().map_err(|_| "i16")?)),
            8 => Value::I32(self.i32()?),
            9 => Value::I64(self.i64()?),
            10 => Value::SByte(self.u8()? as i8),
            11 => Value::F32(f32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "f32")?)),
            12 => Value::TimeSpan(self.i64()?),
            13 => Value::DateTime(self.i64()? as u64),
            14 => Value::U16(u16::from_le_bytes(self.take(2)?.try_into().map_err(|_| "u16")?)),
            15 => Value::U32(self.i32()? as u32),
            16 => Value::U64(self.i64()? as u64),
            _ => return Err(format!("bad primitive type {t}")),
        })
    }

    /// AdditionalInfo for a BinaryType byte.
    fn binary_type(&mut self, bt: u8) -> Res<BinaryType> {
        Ok(match bt {
            0 => BinaryType::Primitive(self.u8()?),
            1 => BinaryType::String,
            2 => BinaryType::Object,
            3 => BinaryType::SystemClass(self.lps()?),
            4 => {
                let n = self.lps()?;
                BinaryType::Class(n, self.i32()?)
            }
            5 => BinaryType::ObjectArray,
            6 => BinaryType::StringArray,
            7 => BinaryType::PrimitiveArray(self.u8()?),
            _ => return Err(format!("bad binary type {bt}")),
        })
    }

    /// Member / element of given type.
    fn typed(&mut self, t: &BinaryType) -> Res<Value> {
        if let BinaryType::Primitive(p) = t {
            return self.prim(*p);
        }
        self.value_item()
    }

    /// Next value in object position, expanding null runs.
    fn value_item(&mut self) -> Res<Value> {
        if self.pending_nulls > 0 {
            self.pending_nulls -= 1;
            return Ok(Value::Null);
        }
        match self.record()? {
            Item::V(v) => Ok(v),
            Item::Nulls(n) => {
                self.pending_nulls = n.saturating_sub(1);
                Ok(Value::Null)
            }
            Item::End => Err("unexpected MessageEnd".into()),
        }
    }

    /// Register inline object.
    fn reg(&mut self, id: i32, v: &Value) {
        self.g.objects.insert(id, v.clone());
    }

    /// Read members of a class.
    fn members(&mut self, meta: &Rc<ClassMeta>, id: i32, with_id: bool) -> Res<Value> {
        let mut members = Vec::with_capacity(meta.types.len());
        for t in &meta.types {
            members.push(self.typed(t)?);
        }
        let v = Value::Class(Rc::new(ClassObj { meta: meta.clone(), object_id: id, with_id, members }));
        self.reg(id, &v);
        Ok(v)
    }

    /// ClassInfo.
    fn class_info(&mut self) -> Res<(i32, String, Vec<String>)> {
        let id = self.i32()?;
        let name = self.lps()?;
        let n = self.i32()?;
        if !(0..=100_000).contains(&n) {
            return Err("bad member count".into());
        }
        let mut names = Vec::new();
        for _ in 0..n {
            names.push(self.lps()?);
        }
        Ok((id, name, names))
    }

    /// MemberTypeInfo for `n` members.
    fn member_types(&mut self, n: usize) -> Res<Vec<BinaryType>> {
        let kinds: Vec<u8> = (0..n).map(|_| self.u8()).collect::<Res<_>>()?;
        kinds.into_iter().map(|k| self.binary_type(k)).collect()
    }

    /// Read one record.
    fn record(&mut self) -> Res<Item> {
        self.depth += 1;
        if self.depth > 500 {
            return Err("nesting too deep".into());
        }
        let r = self.record_inner();
        self.depth -= 1;
        r
    }

    fn record_inner(&mut self) -> Res<Item> {
        let rt = self.u8()?;
        Ok(match rt {
            0 => return Err("unexpected header".into()),
            1 => {
                let id = self.i32()?;
                let mid = self.i32()?;
                let meta = self.metas.get(&mid).cloned().ok_or("unknown metadata id")?;
                Item::V(self.members(&meta, id, true)?)
            }
            2 | 3 => return Err("class records without member types are not supported".into()),
            4 | 5 => {
                let (id, name, names) = self.class_info()?;
                let types = self.member_types(names.len())?;
                let library_id = if rt == 5 { Some(self.i32()?) } else { None };
                let meta = Rc::new(ClassMeta { object_id: id, name, member_names: names, types, library_id });
                self.metas.insert(id, meta.clone());
                Item::V(self.members(&meta, id, false)?)
            }
            6 => {
                let id = self.i32()?;
                let s = self.lps()?;
                let v = Value::Str { id, s: s.into() };
                self.reg(id, &v);
                Item::V(v)
            }
            7 => Item::V(self.binary_array()?),
            8 => {
                let p = self.u8()?;
                Item::V(self.prim(p)?)
            }
            9 => Item::V(Value::Ref(self.i32()?)),
            10 => Item::Nulls(1),
            11 => Item::End,
            12 => {
                let id = self.i32()?;
                let name = self.lps()?;
                self.g.libraries.insert(id, name);
                return self.record_inner();
            }
            13 => Item::Nulls(self.u8()? as usize),
            14 => Item::Nulls(self.i32()?.max(0) as usize),
            15 => {
                let id = self.i32()?;
                let len = self.i32()?;
                let p = self.u8()?;
                let v = self.single_primitive(id, len, p)?;
                Item::V(v)
            }
            16 | 17 => {
                let id = self.i32()?;
                let len = self.i32()?;
                if len < 0 || len as usize > self.b.len() {
                    return Err("bad array length".into());
                }
                let vals = self.elements(len as usize, &if rt == 16 { BinaryType::Object } else { BinaryType::String })?;
                let kind = if rt == 16 { ArrayKind::SingleObject } else { ArrayKind::SingleString };
                let v = Value::Array(Rc::new(ArrayObj { object_id: id, kind, lengths: vec![len], data: ArrayData::Values(vals) }));
                self.reg(id, &v);
                Item::V(v)
            }
            x => return Err(format!("unknown record type {x} at {}", self.p - 1)),
        })
    }

    /// `n` elements of type `t`.
    fn elements(&mut self, n: usize, t: &BinaryType) -> Res<Vec<Value>> {
        let mut v = Vec::with_capacity(n.min(1 << 20));
        for _ in 0..n {
            v.push(self.typed(t)?);
        }
        Ok(v)
    }

    /// ArraySinglePrimitive body.
    fn single_primitive(&mut self, id: i32, len: i32, p: u8) -> Res<Value> {
        if len < 0 {
            return Err("negative array length".into());
        }
        let n = len as usize;
        let data = if p == 2 {
            ArrayData::Bytes(self.take(n)?.to_vec())
        } else {
            ArrayData::Values(self.elements(n, &BinaryType::Primitive(p))?)
        };
        let v = Value::Array(Rc::new(ArrayObj { object_id: id, kind: ArrayKind::SinglePrimitive(p), lengths: vec![len], data }));
        self.reg(id, &v);
        Ok(v)
    }

    /// BinaryArray record.
    fn binary_array(&mut self) -> Res<Value> {
        let id = self.i32()?;
        let array_type = self.u8()?;
        let rank = self.i32()?;
        if !(1..=32).contains(&rank) {
            return Err("bad rank".into());
        }
        let lengths: Vec<i32> = (0..rank).map(|_| self.i32()).collect::<Res<_>>()?;
        let lower: Vec<i32> = if array_type >= 3 { (0..rank).map(|_| self.i32()).collect::<Res<_>>()? } else { vec![] };
        let bt = self.u8()?;
        let elem = self.binary_type(bt)?;
        let mut total: usize = 1;
        for l in &lengths {
            if *l < 0 {
                return Err("negative length".into());
            }
            total = total.checked_mul(*l as usize).ok_or("array too large")?;
        }
        if total > self.b.len() * 4 + 1024 {
            return Err("array too large".into());
        }
        let vals = self.elements(total, &elem)?;
        let v = Value::Array(Rc::new(ArrayObj { object_id: id, kind: ArrayKind::Binary { array_type, rank, lower, elem }, lengths, data: ArrayData::Values(vals) }));
        self.reg(id, &v);
        Ok(v)
    }
}

/// Parse an NRBF stream.
///
/// `bytes`: raw stream (no Serializer header). Returns the graph or an error text.
pub fn read_graph(bytes: &[u8]) -> Result<Graph, String> {
    let mut r = R { b: bytes, p: 0, g: Graph::default(), metas: HashMap::new(), pending_nulls: 0, depth: 0 };
    if r.u8()? != 0 {
        return Err("missing SerializedStreamHeader".into());
    }
    r.g.root_id = r.i32()?;
    r.g.header_id = r.i32()?;
    r.g.major = r.i32()?;
    r.g.minor = r.i32()?;
    loop {
        match r.record()? {
            Item::End => break,
            Item::V(v) => r.g.top.push(v),
            Item::Nulls(_) => r.g.top.push(Value::Null),
        }
    }
    Ok(r.g)
}
