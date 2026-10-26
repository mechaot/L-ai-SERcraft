// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! NRBF reader / writer tests against StandardButtons.zbn and Mono-generated streams.

use lg_base::nrbf::*;
use std::io::Read;

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Raw NRBF bytes of a `.zbn` (header + raw deflate).
fn zbn_raw(path: &std::path::Path) -> Vec<u8> {
    let b = std::fs::read(path).unwrap();
    let marker = b"HERE-->\r\n";
    let pos = b.windows(marker.len()).position(|w| w == marker).unwrap() + marker.len();
    let mut raw = Vec::new();
    flate2::read::DeflateDecoder::new(&b[pos..]).read_to_end(&mut raw).unwrap();
    raw
}

#[test]
fn standard_buttons_roundtrip_byte_exact() {
    let raw = zbn_raw(&root().join("resources/misc/StandardButtons.zbn"));
    let g = read_graph(&raw).unwrap();
    assert_eq!(write_graph(&g).unwrap(), raw);
}

#[test]
fn standard_buttons_match_golden_json() {
    let raw = zbn_raw(&root().join("resources/misc/StandardButtons.zbn"));
    let g = read_graph(&raw).unwrap();
    let btns = custom_buttons_from_graph(&g).unwrap();
    let gold: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root().join("tests/golden/custom_buttons/standard_buttons.json")).unwrap()).unwrap();
    let gb = gold["buttons"].as_array().unwrap();
    assert_eq!(btns.len(), gb.len());
    for (b, j) in btns.iter().zip(gb) {
        assert_eq!(b.guid, j["guid"].as_str().unwrap());
        assert_eq!(b.caption.as_deref(), j["caption"].as_str());
        assert_eq!(b.tooltip.as_deref(), j["tooltip"].as_str());
        assert_eq!(b.gcode.as_deref(), j["gcode"].as_str());
        assert_eq!(b.gcode2.as_deref(), j["gcode2"].as_str());
        assert_eq!(b.enable_style as i64, j["enable_style_value"].as_i64().unwrap());
        assert_eq!(b.button_type as i64, j["button_type_value"].as_i64().unwrap());
        // golden PNG is re-encoded by GDI; compare PNG signature and size only
        let png = b.image.as_deref().unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()) as i64, j["image_width"].as_i64().unwrap());
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()) as i64, j["image_height"].as_i64().unwrap());
    }
}

#[test]
fn build_list_identical_to_dotnet() {
    let raw = zbn_raw(&root().join("resources/misc/StandardButtons.zbn"));
    let g = read_graph(&raw).unwrap();
    let btns = custom_buttons_from_graph(&g).unwrap();
    let built = custom_buttons_graph_ex(
        &btns,
        &g.libraries[&2],
        &g.libraries[&9],
        47,
    );
    let out = write_graph(&built).unwrap();
    assert_eq!(out.len(), raw.len());
    assert!(out == raw, "first diff at {:?}", (0..raw.len()).find(|&i| out[i] != raw[i]));
}

fn golden(name: &str) -> Vec<u8> {
    std::fs::read(root().join("tests/golden/base").join(name)).unwrap()
}

#[test]
fn mono_streams_roundtrip() {
    for n in ["buttons_0.nrbf", "buttons_2.nrbf", "settings_sample.bin"] {
        let raw = golden(n);
        let g = read_graph(&raw).unwrap_or_else(|e| panic!("{n}: {e}"));
        let out = write_graph(&g).unwrap();
        let d = (0..raw.len().min(out.len())).find(|&i| out[i] != raw[i]);
        assert!(out == raw, "{n}: first diff {d:?} lens {} {}", out.len(), raw.len());
    }
    let raw = zbn_raw(&root().join("tests/golden/base/buttons_std_reser.zbn"));
    let g = read_graph(&raw).unwrap();
    assert_eq!(write_graph(&g).unwrap(), raw);
    assert_eq!(custom_buttons_from_graph(&g).unwrap().len(), 5);
}

#[test]
fn settings_bin_import() {
    let m = import_settings_bin(&root().join("tests/golden/base/settings_sample.bin")).unwrap();
    println!("{m:#?}");
    assert_eq!(m["t.bool"], serde_json::json!(true));
    assert_eq!(m["t.int"], serde_json::json!(-42));
    assert_eq!(m["t.float"], serde_json::json!(1.5));
    assert_eq!(m["t.double"], serde_json::json!(0.1));
    assert_eq!(m["t.decimal"], serde_json::json!("12.50"));
    assert_eq!(m["t.string"], serde_json::json!("hello \u{e4}\u{20ac}"));
    assert_eq!(m["t.long"], serde_json::json!(1234567890123i64));
    assert_eq!(m["t.enum"], serde_json::json!(3));
    assert_eq!(m["t.timespan"], serde_json::json!("00:01:30"));
    assert_eq!(m["t.version"], serde_json::json!("1.2.3"));
    assert_eq!(m["t.objarray"], serde_json::json!([1, 2.5, "x", true]));
    assert_eq!(m["t.point"], serde_json::json!({"x": 3, "y": 4}));
    assert_eq!(m["t.datetime"], serde_json::json!("2024-05-06T07:08:09.0000000"));
}

#[test]
fn builder_vs_mono() {
    // Mono's BinaryFormatter may differ from .NET Framework; informational for the 0 / 2 button cases
    for (n, btns) in [("buttons_0.nrbf", 0usize), ("buttons_2.nrbf", 2)] {
        let raw = golden(n);
        let g = read_graph(&raw).unwrap();
        let b = custom_buttons_from_graph(&g).unwrap();
        assert_eq!(b.len(), btns);
        let lib_app = g.libraries.get(&2).cloned().unwrap_or_default();
        let lib_draw = g.libraries.values().find(|l| l.starts_with("System.Drawing")).cloned().unwrap_or_default();
        let built = write_graph(&custom_buttons_graph_ex(&b, &lib_app, &lib_draw, btns as i32)).unwrap();
        println!("{n}: mono {} bytes, built {} bytes, equal={}", raw.len(), built.len(), built == raw);
    }
}

#[test]
fn serializer_files() {
    let d = std::env::temp_dir().join(format!("lg_ser_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    // Windows zbn and Mono re-serialized zbn decode through the container reader
    for p in ["resources/misc/StandardButtons.zbn", "tests/golden/base/buttons_std_reser.zbn"] {
        let g = obj_from_file(&root().join(p)).unwrap().unwrap();
        assert_eq!(custom_buttons_from_graph(&g).unwrap().len(), 5);
    }
    // our writer: compressed container, read back
    let g = obj_from_file(&root().join("resources/misc/StandardButtons.zbn")).unwrap().unwrap();
    let out = d.join("out.zbn");
    obj_to_file(&g, &out, true).unwrap();
    let g2 = obj_from_file(&out).unwrap().unwrap();
    assert_eq!(write_graph(&g2).unwrap(), write_graph(&g).unwrap());
    // gzip wrapper accepted
    use std::io::Write;
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&write_graph(&g).unwrap()).unwrap();
    assert!(obj_from_bytes(&gz.finish().unwrap()).is_ok());
    // missing file
    assert!(obj_from_file(&d.join("nope")).unwrap().is_none());
    // orphan .bak restored
    std::fs::rename(&out, d.join("out2.zbn.bak")).unwrap();
    assert!(obj_from_file(&d.join("out2.zbn")).unwrap().is_some());
}
