// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Andreas Pösch

//! `<text>` to `<path>` pre-pass via usvg + system fonts. Not in C# (it skips text).

use resvg::usvg::{self, Node};
use resvg::usvg::tiny_skia_path::PathSegment;
use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};

const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// Shared system font db, loaded once (OnceLock). Returns the db.
pub fn system_fontdb() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        // map generic families to installed ones (fontdb defaults are Windows names)
        let has = |db: &usvg::fontdb::Database, n: &str| db.faces().any(|f| f.families.iter().any(|(fam, _)| fam == n));
        let pick = |db: &usvg::fontdb::Database, c: &[&str]| c.iter().find(|n| has(db, n)).map(|n| n.to_string());
        if let Some(f) = pick(&db, &["DejaVu Sans", "Liberation Sans", "Noto Sans", "Arial", "Helvetica", "Ubuntu", "Cantarell"]) {
            db.set_sans_serif_family(f);
        }
        if let Some(f) = pick(&db, &["DejaVu Serif", "Liberation Serif", "Noto Serif", "Times New Roman"]) {
            db.set_serif_family(f);
        }
        if let Some(f) = pick(&db, &["DejaVu Sans Mono", "Liberation Mono", "Noto Sans Mono", "Courier New"]) {
            db.set_monospace_family(f);
        }
        Arc::new(db)
    })
    .clone()
}

/// Start tag facts of one text element.
struct StartTag {
    /// Element start (byte, at `<`).
    start: usize,
    /// End of qname (byte).
    name_end: usize,
    /// Byte of closing `>`.
    close: usize,
}

/// Locate start tag. `xml`: source. `start`: byte of `<`. Returns tag or None if self-closing/broken.
fn start_tag(xml: &str, start: usize) -> Option<StartTag> {
    let b = xml.as_bytes();
    // qname runs to whitespace, `/` or `>`
    let mut i = start + 1;
    while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'/' && b[i] != b'>' {
        i += 1;
    }
    let name_end = i;
    // find closing `>`, skip quoted values
    let mut quote = 0u8;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == quote {
                quote = 0;
            }
        } else if c == b'"' || c == b'\'' {
            quote = c;
        } else if c == b'>' {
            // self-closing: leave alone
            if b[i - 1] == b'/' {
                return None;
            }
            return Some(StartTag { start, name_end, close: i });
        }
        i += 1;
    }
    None
}

/// Append glyph outlines of a group as SVG path data.
///
/// `g`: flattened text group. `ts`: accumulated transform (colour glyph groups). `d`: output buffer.
fn path_data(g: &usvg::Group, ts: usvg::Transform, d: &mut String) {
    // map point, f32 Display, space separated
    let pt = |p: usvg::tiny_skia_path::Point| {
        let mut p = p;
        ts.map_point(&mut p);
        format!("{} {}", p.x, p.y)
    };
    for n in g.children() {
        match n {
            Node::Path(p) if p.is_visible() => {
                for s in p.data().segments() {
                    let _ = match s {
                        PathSegment::MoveTo(a) => write!(d, "M{} ", pt(a)),
                        PathSegment::LineTo(a) => write!(d, "L{} ", pt(a)),
                        PathSegment::QuadTo(a, b) => write!(d, "Q{} {} ", pt(a), pt(b)),
                        PathSegment::CubicTo(a, b, c) => write!(d, "C{} {} {} ", pt(a), pt(b), pt(c)),
                        PathSegment::Close => write!(d, "Z "),
                    };
                }
            }
            // colour glyph groups carry own transform
            Node::Group(sub) => path_data(sub, ts.pre_concat(sub.transform()), d),
            _ => {}
        }
    }
}

/// Replace each renderable `<text>` with a `<path>` of its glyph outlines.
///
/// `xml`: SVG source. Returns rewritten SVG; input copy when no text, parse error or nothing laid out.
pub fn text_to_paths(xml: &str) -> String {
    let opt = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let Ok(doc) = roxmltree::Document::parse_with_options(xml, opt) else {
        return xml.to_string();
    };
    // text elements in SVG namespace, with usable start tags
    let mut texts = Vec::new();
    for n in doc.descendants() {
        if n.is_element()
            && n.tag_name().name() == "text"
            && n.tag_name().namespace() == Some(SVG_NS)
            && let Some(tag) = start_tag(xml, n.range().start)
        {
            texts.push((n, tag));
        }
    }
    if texts.is_empty() {
        return xml.to_string();
    }
    // probe copy: unique id on every text, edits from the back
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for (i, (n, tag)) in texts.iter().enumerate() {
        let id = format!("__lg_text_{i}");
        match n.attribute_node("id") {
            Some(a) => {
                let r = a.range_value();
                edits.push((r.start, r.end, id));
            }
            None => edits.push((tag.name_end, tag.name_end, format!(" id=\"{id}\""))),
        }
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut probe = xml.to_string();
    for (s, e, t) in &edits {
        probe.replace_range(*s..*e, t);
    }
    let options = usvg::Options { fontdb: system_fontdb(), ..Default::default() };
    let Ok(tree) = usvg::Tree::from_str(&probe, &options) else {
        return xml.to_string();
    };
    // build replacements per laid-out text
    let mut reps: Vec<(usize, usize, String)> = Vec::new();
    for (i, (n, tag)) in texts.iter().enumerate() {
        let Some(Node::Text(t)) = tree.node_by_id(&format!("__lg_text_{i}")) else {
            // blank text lays out to nothing: drop it; other misses stay untouched
            let blank = n.descendants().filter(|c| c.is_text()).all(|c| c.text().is_none_or(|x| x.trim().is_empty()));
            if blank {
                let r = n.range();
                reps.push((r.start, r.end, String::new()));
            }
            continue;
        };
        let mut d = String::new();
        path_data(t.flattened(), usvg::Transform::identity(), &mut d);
        let d = d.trim_end();
        let r = n.range();
        // empty outline: drop element
        if d.is_empty() {
            reps.push((r.start, r.end, String::new()));
            continue;
        }
        let qname = &xml[tag.start + 1..tag.name_end];
        let prefix = qname.rfind(':').map_or("", |p| &qname[..=p]);
        let attrs = &xml[tag.name_end..tag.close];
        reps.push((r.start, r.end, format!("<{prefix}path{attrs} d=\"{d}\"/>")));
    }
    reps.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = xml.to_string();
    for (s, e, t) in &reps {
        out.replace_range(*s..*e, t);
    }
    out
}
