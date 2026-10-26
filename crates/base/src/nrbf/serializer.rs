// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Tools/Serializer.cs`: text header (`<!-- Tag: Value -->` lines) + optional raw-deflate +
//! NRBF payload.
//!
//! Not ported: the password mode (`SymmetricAlgorithm` = Rijndael-256/CBC with a `PasswordDeriveBytes` key).
//! The only C# caller with a password is the `UsageStats` news-message file (`"lasergrbl"`), which is
//! remote telemetry and not ported (PLAN section 6); the crate has no cipher dependency, so encrypted
//! files give [`SerializerError::MissingPassword`]. XML mode does not exist in the C# code either
//! (`CreateFormatterForMode` always returns a `BinaryFormatter`).
//!
//! Known deviations: no `.bak` + `File.Replace` dance (tmp file + rename instead); read errors are
//! logged but no `*.dam` copy of the broken file is made; a header-less (v0.0) file is read but not
//! re-saved in the v1.0 container.

use super::{read_graph, write_graph, Graph};
use flate2::read::{DeflateDecoder, GzDecoder};
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// C# `lock (ThreadLock)`: one serializer call at a time (also keeps the tmp file name unique).
static THREAD_LOCK: Mutex<()> = Mutex::new(());

const DATA_START: &str = "<!-- OBJECT DATA BEGIN HERE-->\r\n";
const SERIALIZER_VERSION: &str = "v1.0";

/// Serializer failure.
#[derive(Debug, Clone, PartialEq)]
pub enum SerializerError {
    /// File system error.
    Io(String),
    /// Bad header / payload.
    Format(String),
    /// File is encrypted (password mode not ported).
    MissingPassword,
}

impl std::fmt::Display for SerializerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(s) | Self::Format(s) => f.write_str(s),
            Self::MissingPassword => f.write_str("file is encrypted; password mode is not supported"),
        }
    }
}

impl std::error::Error for SerializerError {}

/// Value of a header tag (C# `GetTagValue`).
///
/// `s`: header text. `tag`: tag name. Returns the value or None.
fn tag_value(s: &str, tag: &str) -> Option<String> {
    let re = regex::Regex::new(r"<!-- (.+): (.+) -->").ok()?;
    re.captures_iter(s).find(|c| &c[1] == tag).map(|c| c[2].to_string())
}

/// Decode container bytes to a graph (C# `ObjFromFile` after reading the stream).
///
/// `bytes`: file content. Accepts: Serializer header (+ deflate), plain NRBF (v0.0), gzip-wrapped
/// anything. Returns the graph.
pub fn obj_from_bytes(bytes: &[u8]) -> Result<Graph, SerializerError> {
    // gzip (.gz) wrapper is accepted as convenience
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        GzDecoder::new(bytes).read_to_end(&mut out).map_err(|e| SerializerError::Format(e.to_string()))?;
        return obj_from_bytes(&out);
    }
    // find end of header within first 1024 bytes
    let marker = DATA_START.as_bytes();
    let limit = bytes.len().min(1024 + marker.len());
    let pos = bytes[..limit].windows(marker.len()).position(|w| w == marker);
    let (header, payload) = match pos {
        Some(p) => (String::from_utf8_lossy(&bytes[..p + marker.len()]).into_owned(), &bytes[p + marker.len()..]),
        None => (String::new(), bytes), // v0.0: no header
    };
    if pos.is_some() {
        let version = tag_value(&header, "RemotingBase SerializerVersion").unwrap_or_else(|| "v0.0".into());
        if version != SERIALIZER_VERSION {
            return Err(SerializerError::Format(format!("unsupported serializer version {version}")));
        }
        if tag_value(&header, "Encription").as_deref() == Some("Enabled") {
            return Err(SerializerError::MissingPassword);
        }
        if tag_value(&header, "Compression").as_deref() == Some("Enabled") {
            let mut out = Vec::new();
            DeflateDecoder::new(payload).read_to_end(&mut out).map_err(|e| SerializerError::Format(e.to_string()))?;
            return read_graph(&out).map_err(SerializerError::Format);
        }
    }
    read_graph(payload).map_err(SerializerError::Format)
}

/// Encode a graph into container bytes (C# `ObjToFile` stream part, no password).
///
/// `g`: graph. `compression`: raw deflate the payload. Returns the bytes.
pub fn obj_to_bytes(g: &Graph, compression: bool) -> Result<Vec<u8>, SerializerError> {
    let payload = write_graph(g).map_err(SerializerError::Format)?;
    let mut out = String::new();
    let mut tag = |t: &str, v: &str| out.push_str(&format!("<!-- {t}: {v} -->\r\n"));
    tag("RemotingBase SerializerVersion", SERIALIZER_VERSION);
    tag("Serialization Mode", "Binary");
    tag("Encription", "Disabled");
    tag("Compression", if compression { "Enabled" } else { "Disabled" });
    out.push_str(DATA_START);
    let mut bytes = out.into_bytes();
    if compression {
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&payload).map_err(|e| SerializerError::Io(e.to_string()))?;
        bytes.extend(enc.finish().map_err(|e| SerializerError::Io(e.to_string()))?);
    } else {
        bytes.extend(payload);
    }
    Ok(bytes)
}

/// Resolve a file name against the data dir (absolute paths stay).
fn full(p: &Path) -> PathBuf {
    crate::paths::data_path().join(p)
}

/// Read a serialized object file (C# `ObjFromFile`).
///
/// `path`: absolute, or relative to `data_path()`. An orphan `.bak` (no main file) is renamed back
/// first. Returns Ok(None) when the file does not exist; read errors are logged to the session log
/// and returned.
pub fn obj_from_file(path: &Path) -> Result<Option<Graph>, SerializerError> {
    let _lock = THREAD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let file = full(path);
    let bak = PathBuf::from(format!("{}.bak", file.display()));
    // orphan backup: restore
    if bak.exists() && !file.exists() {
        crate::logger::log_message("Serialization", &format!("orphan .bak found {}", bak.display()));
        let _ = std::fs::rename(&bak, &file);
    }
    if !file.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&file).map_err(|e| SerializerError::Io(e.to_string()))?;
    match obj_from_bytes(&bytes) {
        Ok(g) => Ok(Some(g)),
        Err(e) => {
            crate::logger::log_exception("Serialization", &format!("error reading {}: {e}", file.display()));
            Err(e)
        }
    }
}

/// Write a serialized object file via tmp file + rename (C# `ObjToFile`).
///
/// `g`: graph. `path`: absolute, or relative to `data_path()`. `compression`: deflate payload.
/// Returns Ok on success; errors are also logged.
pub fn obj_to_file(g: &Graph, path: &Path, compression: bool) -> Result<(), SerializerError> {
    let _lock = THREAD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let file = full(path);
    let res = (|| {
        let bytes = obj_to_bytes(g, compression)?;
        let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
        let tmp = dir.join(format!("tmp_{}", std::process::id()));
        std::fs::write(&tmp, bytes).map_err(|e| SerializerError::Io(e.to_string()))?;
        std::fs::rename(&tmp, &file).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            SerializerError::Io(e.to_string())
        })
    })();
    if let Err(e) = &res {
        crate::logger::log_exception("Serialization", &format!("error writing {}: {e}", file.display()));
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_roundtrip() {
        let g = super::super::custom_buttons_graph(&[]);
        for c in [false, true] {
            let b = obj_to_bytes(&g, c).unwrap();
            assert!(b.starts_with(b"<!-- RemotingBase SerializerVersion: v1.0 -->\r\n"));
            let g2 = obj_from_bytes(&b).unwrap();
            assert_eq!(write_graph(&g2).unwrap(), write_graph(&g).unwrap());
        }
        // plain NRBF
        let raw = write_graph(&g).unwrap();
        assert!(obj_from_bytes(&raw).is_ok());
        assert!(obj_from_bytes(b"junk").is_err());
    }

    #[test]
    fn concurrent_writers_do_not_clash() {
        // error paths log to the session log: keep it out of the real config dir
        crate::paths::isolate_for_tests("base");
        // C# lock + random tmp name: parallel saves into one dir must all succeed
        let dir = std::env::temp_dir().join(format!("lg_ser_conc_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let p = dir.join(format!("f{i}.bin"));
                // graph holds Rc: build it inside the thread
                std::thread::spawn(move || {
                    let g = super::super::custom_buttons_graph(&[]);
                    (0..20).all(|_| obj_to_file(&g, &p, false).is_ok())
                })
            })
            .collect();
        for h in handles {
            assert!(h.join().unwrap());
        }
        for i in 0..8 {
            assert!(obj_from_bytes(&std::fs::read(dir.join(format!("f{i}.bin"))).unwrap()).is_ok());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn encrypted_flagged() {
        let b = b"<!-- RemotingBase SerializerVersion: v1.0 -->\r\n<!-- Encription: Enabled -->\r\n<!-- OBJECT DATA BEGIN HERE-->\r\nxx";
        assert_eq!(obj_from_bytes(b).unwrap_err(), SerializerError::MissingPassword);
    }
}
