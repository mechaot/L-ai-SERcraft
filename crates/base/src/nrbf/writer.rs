// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! NRBF writer: re-emits a [`Graph`] byte-exact.

use super::*;
use std::collections::HashSet;

struct W {
    o: Vec<u8>,
    libs_written: HashSet<i32>,
    metas_written: HashSet<i32>,
}

impl W {
    fn i32(&mut self, v: i32) {
        self.o.extend_from_slice(&v.to_le_bytes());
    }
    /// 7-bit length + UTF-8.
    fn lps(&mut self, s: &str) {
        let mut n = s.len();
        loop {
            let b = (n & 0x7f) as u8;
            n >>= 7;
            if n == 0 {
                self.o.push(b);
                break;
            }
            self.o.push(b | 0x80);
        }
        self.o.extend_from_slice(s.as_bytes());
    }

    /// Raw primitive.
    fn prim(&mut self, v: &Value) -> Result<(), String> {
        match v {
            Value::Bool(b) => self.o.push(*b as u8),
            Value::Byte(b) => self.o.push(*b),
            Value::SByte(b) => self.o.push(*b as u8),
            Value::Char(c) => {
                let mut buf = [0u8; 4];
                self.o.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            Value::Decimal(d) => {
                let s = lg_dotnet::fmt_decimal(*d);
                self.lps(&s)
            }
            Value::F64(x) => self.o.extend_from_slice(&x.to_le_bytes()),
            Value::F32(x) => self.o.extend_from_slice(&x.to_le_bytes()),
            Value::I16(x) => self.o.extend_from_slice(&x.to_le_bytes()),
            Value::U16(x) => self.o.extend_from_slice(&x.to_le_bytes()),
            Value::I32(x) => self.o.extend_from_slice(&x.to_le_bytes()),
            Value::U32(x) => self.o.extend_from_slice(&x.to_le_bytes()),
            Value::I64(x) | Value::TimeSpan(x) => self.o.extend_from_slice(&x.to_le_bytes()),
            Value::U64(x) | Value::DateTime(x) => self.o.extend_from_slice(&x.to_le_bytes()),
            _ => return Err(format!("not a primitive: {v:?}")),
        }
        Ok(())
    }

    /// PrimitiveType code of a primitive value.
    fn prim_code(v: &Value) -> Option<u8> {
        Some(match v {
            Value::Bool(_) => 1,
            Value::Byte(_) => 2,
            Value::Char(_) => 3,
            Value::Decimal(_) => 5,
            Value::F64(_) => 6,
            Value::I16(_) => 7,
            Value::I32(_) => 8,
            Value::I64(_) => 9,
            Value::SByte(_) => 10,
            Value::F32(_) => 11,
            Value::TimeSpan(_) => 12,
            Value::DateTime(_) => 13,
            Value::U16(_) => 14,
            Value::U32(_) => 15,
            Value::U64(_) => 16,
            _ => return None,
        })
    }

    fn bt_code(t: &BinaryType) -> u8 {
        match t {
            BinaryType::Primitive(_) => 0,
            BinaryType::String => 1,
            BinaryType::Object => 2,
            BinaryType::SystemClass(_) => 3,
            BinaryType::Class(..) => 4,
            BinaryType::ObjectArray => 5,
            BinaryType::StringArray => 6,
            BinaryType::PrimitiveArray(_) => 7,
        }
    }

    /// AdditionalInfo.
    fn bt_info(&mut self, t: &BinaryType) {
        match t {
            BinaryType::Primitive(p) | BinaryType::PrimitiveArray(p) => self.o.push(*p),
            BinaryType::SystemClass(n) => self.lps(n),
            BinaryType::Class(n, l) => {
                self.lps(n);
                self.i32(*l);
            }
            _ => {}
        }
    }

    /// Emit BinaryLibrary if needed.
    fn library(&mut self, g: &Graph, id: i32) -> Result<(), String> {
        if self.libs_written.insert(id) {
            self.o.push(12);
            self.i32(id);
            let name = g.libraries.get(&id).ok_or("unknown library")?;
            self.lps(name);
        }
        Ok(())
    }

    /// Values in object position with null-run compression.
    fn seq(&mut self, g: &Graph, vals: &[Value], t: &BinaryType) -> Result<(), String> {
        let mut i = 0;
        while i < vals.len() {
            if let BinaryType::Primitive(_) = t {
                self.prim(&vals[i])?;
                i += 1;
            } else if vals[i] == Value::Null {
                let mut n = 0;
                while i + n < vals.len() && vals[i + n] == Value::Null {
                    n += 1;
                }
                i += n;
                self.nulls(n);
            } else {
                self.value(g, &vals[i])?;
                i += 1;
            }
        }
        Ok(())
    }

    /// Null run record(s): 1 -> ObjectNull, <=255 -> ObjectNullMultiple256, else ObjectNullMultiple.
    /// NOTE: matches Mono and the StandardButtons.zbn (3 nulls); MS for exactly 2 nulls is unverified.
    fn nulls(&mut self, n: usize) {
        if n == 1 {
            self.o.push(10);
        } else if n <= 255 {
            self.o.push(13);
            self.o.push(n as u8);
        } else {
            self.o.push(14);
            self.i32(n as i32);
        }
    }

    /// One record in object position.
    fn value(&mut self, g: &Graph, v: &Value) -> Result<(), String> {
        match v {
            Value::Null => self.o.push(10),
            Value::Ref(id) => {
                self.o.push(9);
                self.i32(*id);
            }
            Value::Str { id, s } => {
                self.o.push(6);
                self.i32(*id);
                self.lps(s);
            }
            Value::Class(c) => self.class(g, c)?,
            Value::Array(a) => self.array(g, a)?,
            p => {
                let code = Self::prim_code(p).ok_or("bad value")?;
                self.o.push(8);
                self.o.push(code);
                self.prim(p)?;
            }
        }
        Ok(())
    }

    /// Emit libraries needed by inline member classes before the parent record (.NET does this).
    fn prescan(&mut self, g: &Graph, c: &ClassObj) -> Result<(), String> {
        for v in &c.members {
            // referenced (queued) object: its library is registered now
            if let Value::Ref(id) = v {
                match g.objects.get(id) {
                    Some(Value::Class(t)) => {
                        if let Some(l) = t.meta.library_id {
                            self.library(g, l)?;
                        }
                        for ty in &t.meta.types {
                            if let BinaryType::Class(_, l) = ty {
                                self.library(g, *l)?;
                            }
                        }
                    }
                    Some(Value::Array(a)) => {
                        if let ArrayKind::Binary { elem: BinaryType::Class(_, l), .. } = &a.kind {
                            self.library(g, *l)?;
                        }
                    }
                    _ => {}
                }
            }
            if let Value::Class(ch) = v
                && !(ch.with_id && self.metas_written.contains(&ch.meta.object_id))
            {
                if let Some(l) = ch.meta.library_id {
                    self.library(g, l)?;
                }
                for t in &ch.meta.types {
                    if let BinaryType::Class(_, l) = t {
                        self.library(g, *l)?;
                    }
                }
                self.prescan(g, ch)?;
            }
        }
        Ok(())
    }

    fn class(&mut self, g: &Graph, c: &ClassObj) -> Result<(), String> {
        let m = &c.meta;
        self.prescan(g, c)?;
        if c.with_id && self.metas_written.contains(&m.object_id) {
            self.o.push(1);
            self.i32(c.object_id);
            self.i32(m.object_id);
        } else {
            // libraries must precede the first record that names them (also via member type info)
            if let Some(l) = m.library_id {
                self.library(g, l)?;
            }
            for t in &m.types {
                if let BinaryType::Class(_, l) = t {
                    self.library(g, *l)?;
                }
            }
            self.o.push(if m.library_id.is_some() { 5 } else { 4 });
            self.i32(c.object_id);
            self.lps(&m.name);
            self.i32(m.member_names.len() as i32);
            for n in &m.member_names {
                self.lps(n);
            }
            for t in &m.types {
                self.o.push(Self::bt_code(t));
            }
            for t in &m.types {
                self.bt_info(t);
            }
            if let Some(l) = m.library_id {
                self.i32(l);
            }
            self.metas_written.insert(m.object_id);
        }
        // members: primitives raw, others as records; null runs compressed
        let mut i = 0;
        while i < c.members.len() {
            let t = &m.types[i];
            if let BinaryType::Primitive(_) = t {
                self.prim(&c.members[i])?;
                i += 1;
            } else if c.members[i] == Value::Null {
                let mut n = 0;
                while i + n < c.members.len() && c.members[i + n] == Value::Null && !matches!(m.types[i + n], BinaryType::Primitive(_)) {
                    n += 1;
                }
                i += n;
                self.nulls(n);
            } else {
                self.value(g, &c.members[i])?;
                i += 1;
            }
        }
        Ok(())
    }

    fn array(&mut self, g: &Graph, a: &ArrayObj) -> Result<(), String> {
        match &a.kind {
            ArrayKind::SinglePrimitive(p) => {
                self.o.push(15);
                self.i32(a.object_id);
                self.i32(a.lengths[0]);
                self.o.push(*p);
                match &a.data {
                    ArrayData::Bytes(b) => self.o.extend_from_slice(b),
                    ArrayData::Values(v) => {
                        for x in v {
                            self.prim(x)?;
                        }
                    }
                }
            }
            ArrayKind::SingleObject | ArrayKind::SingleString => {
                let (rt, t) = if a.kind == ArrayKind::SingleObject { (16, BinaryType::Object) } else { (17, BinaryType::String) };
                self.o.push(rt);
                self.i32(a.object_id);
                self.i32(a.lengths[0]);
                if let ArrayData::Values(v) = &a.data {
                    self.seq(g, v, &t)?;
                }
            }
            ArrayKind::Binary { array_type, rank, lower, elem } => {
                if let BinaryType::Class(_, l) = elem {
                    self.library(g, *l)?;
                }
                self.o.push(7);
                self.i32(a.object_id);
                self.o.push(*array_type);
                self.i32(*rank);
                for l in &a.lengths {
                    self.i32(*l);
                }
                for l in lower {
                    self.i32(*l);
                }
                self.o.push(Self::bt_code(elem));
                self.bt_info(elem);
                if let ArrayData::Values(v) = &a.data {
                    self.seq(g, v, elem)?;
                }
            }
        }
        Ok(())
    }
}

/// Serialize a graph.
///
/// `g`: graph (as read or built). Returns the NRBF bytes, or error text for malformed graphs.
pub fn write_graph(g: &Graph) -> Result<Vec<u8>, String> {
    let mut w = W { o: Vec::new(), libs_written: HashSet::new(), metas_written: HashSet::new() };
    w.o.push(0);
    w.i32(g.root_id);
    w.i32(g.header_id);
    w.i32(g.major);
    w.i32(g.minor);
    for v in &g.top {
        w.value(g, v)?;
    }
    w.o.push(11);
    Ok(w.o)
}
