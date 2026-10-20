//! Replays `tests/golden/clipper/*.txt` (made by `tools/golden/clipper/run.sh` from the original
//! C# ClipperLib under Mono) and compares exact points, order and tree shape.
//!
//! Method: rebuild the whole case text (inputs echoed, outputs computed by the Rust port) and
//! compare line by line with the golden text. Rust panic where C# threw is accepted as `EXC`
//! (C# NullReference / IndexOutOfRange vs Rust index panic) and counted separately.

use lg_clipper::{
    Clipper, ClipperException, ClipperOffset, ClipType, EndType, IntPoint, JoinType, Path, Paths,
    PolyFillType, PolyTree, PolyType, f64_to_cint,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/clipper")
}

fn clip_type(v: i32) -> ClipType {
    [ClipType::Intersection, ClipType::Union, ClipType::Difference, ClipType::Xor][v as usize]
}

fn fill_type(v: i32) -> PolyFillType {
    [PolyFillType::EvenOdd, PolyFillType::NonZero, PolyFillType::Positive, PolyFillType::Negative]
        [v as usize]
}

fn join_type(v: i32) -> JoinType {
    [JoinType::Square, JoinType::Round, JoinType::Miter][v as usize]
}

fn end_type(v: i32) -> EndType {
    [
        EndType::ClosedPolygon,
        EndType::ClosedLine,
        EndType::OpenButt,
        EndType::OpenSquare,
        EndType::OpenRound,
    ][v as usize]
}

fn hex_f64(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).expect("hex double"))
}

fn fmt_hex(v: f64) -> String {
    format!("{:016X}", v.to_bits())
}

/// Format `<n> x y x y ...`.
fn fmt_path(p: &[IntPoint]) -> String {
    let mut s = p.len().to_string();
    for ip in p {
        s.push_str(&format!(" {} {}", ip.x, ip.y));
    }
    s
}

/// Parse `<n> x y ...` from tokens; returns (path, tokens consumed).
fn parse_path(tok: &[&str]) -> (Path, usize) {
    let n: usize = tok[0].parse().expect("count");
    let mut p = Path::with_capacity(n);
    for i in 0..n {
        p.push(IntPoint::new(
            tok[1 + 2 * i].parse().expect("x"),
            tok[2 + 2 * i].parse().expect("y"),
        ));
    }
    (p, 1 + 2 * n)
}

fn out_lines(sol: &Paths, out: &mut Vec<String>) {
    for p in sol {
        out.push(format!("OUT {}", fmt_path(p)));
    }
}

fn tree_lines(t: &PolyTree, out: &mut Vec<String>) {
    out.push(format!("TOTAL {}", t.total()));
    out.push(format!("CHILDCOUNT {}", t.child_count()));
    let mut n = t.get_first();
    while let Some(id) = n {
        let node = t.node(id);
        let mut d = 0;
        let mut p = node.parent();
        while p != Some(PolyTree::ROOT) {
            d += 1;
            p = t.node(p.unwrap()).parent();
        }
        out.push(format!(
            "NODE {} {} {} {}",
            d,
            u8::from(node.is_open()),
            u8::from(t.is_hole(id)),
            fmt_path(node.contour())
        ));
        n = t.get_next(id);
    }
    for p in Clipper::open_paths_from_poly_tree(t) {
        out.push(format!("OPEN {}", fmt_path(&p)));
    }
    for p in Clipper::closed_paths_from_poly_tree(t) {
        out.push(format!("CLOSED {}", fmt_path(&p)));
    }
    for p in Clipper::poly_tree_to_paths(t) {
        out.push(format!("ALL {}", fmt_path(&p)));
    }
}

#[derive(Default)]
struct Stats {
    cases: usize,
    failed: Vec<String>,
    panics_as_exc: usize,
}

/// Run `f`; Err(msg) = library exception, Panic = Rust panic.
enum Outcome<T> {
    Ok(T),
    Exc,
    Panic,
}

fn guarded<T>(f: impl FnOnce() -> Result<T, ClipperException>) -> Outcome<T> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(v)) => Outcome::Ok(v),
        Ok(Err(_)) => Outcome::Exc,
        Err(_) => Outcome::Panic,
    }
}

fn first_tag(l: &str) -> &str {
    l.split(' ').next().unwrap_or("")
}

/// Build actual text of one clip case (lines between CASE and END, inclusive).
fn run_clip(lines: &[&str], st: &mut Stats) -> Vec<String> {
    let mut out = Vec::new();
    let mut c: Option<Clipper> = None;
    let mut dead = false;
    let mut flags = 0;
    let mut adds: Vec<(PolyType, bool, Path)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        let tok: Vec<&str> = l.split(' ').collect();
        match tok[0] {
            "CLIP" => {
                flags = tok[1].parse().unwrap();
                out.push(l.to_string());
            }
            "ADD" => {
                let pt = if tok[1] == "0" { PolyType::Subject } else { PolyType::Clip };
                let (p, _) = parse_path(&tok[3..]);
                adds.push((pt, tok[2] == "1", p));
                out.push(l.to_string());
            }
            "EXEC" => {
                out.push(l.to_string());
                if c.is_none() {
                    // first exec: build the clipper from all ADD records
                    let mut cl = Clipper::new(flags);
                    let r = guarded(|| {
                        for (pt, closed, p) in &adds {
                            cl.add_path(p, *pt, *closed)?;
                        }
                        Ok(())
                    });
                    if !matches!(r, Outcome::Ok(())) {
                        dead = true;
                        if matches!(r, Outcome::Panic) {
                            st.panics_as_exc += 1;
                        }
                    }
                    c = Some(cl);
                }
                let tree = tok[1] == "tree";
                let (ct, sf, cf) = (
                    clip_type(tok[2].parse().unwrap()),
                    fill_type(tok[3].parse().unwrap()),
                    fill_type(tok[4].parse().unwrap()),
                );
                if dead {
                    out.push("EXC".into());
                } else {
                    let cl = c.as_mut().unwrap();
                    let mut body = Vec::new();
                    let r = if tree {
                        guarded(|| {
                            let mut t = PolyTree::new();
                            let ok = cl.execute_tree_fill(ct, &mut t, sf, cf)?;
                            body.push(format!("RES {}", u8::from(ok)));
                            if ok {
                                tree_lines(&t, &mut body);
                            }
                            Ok(())
                        })
                    } else {
                        guarded(|| {
                            let mut sol = Paths::new();
                            let ok = cl.execute_fill(ct, &mut sol, sf, cf)?;
                            body.push(format!("RES {}", u8::from(ok)));
                            out_lines(&sol, &mut body);
                            Ok(())
                        })
                    };
                    match r {
                        Outcome::Ok(()) => out.extend(body),
                        Outcome::Exc => {
                            out.push("EXC".into());
                            dead = true;
                        }
                        Outcome::Panic => {
                            out.push("EXC".into());
                            dead = true;
                            st.panics_as_exc += 1;
                        }
                    }
                }
                out.push("ENDEXEC".into());
                // skip golden output of this exec
                while lines[i] != "ENDEXEC" {
                    i += 1;
                }
            }
            _ => out.push(l.to_string()),
        }
        i += 1;
    }
    out
}

fn run_offset(lines: &[&str], st: &mut Stats) -> Vec<String> {
    let mut out = Vec::new();
    let mut off: Option<ClipperOffset> = None;
    let mut tree = false;
    let (mut miter, mut arc) = (0.0, 0.0);
    let mut paths: Vec<(i32, i32, Path)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        let tok: Vec<&str> = l.split(' ').collect();
        match tok[0] {
            "OFFSET" => {
                tree = tok[1] == "tree";
                miter = hex_f64(tok[2]);
                arc = hex_f64(tok[3]);
                out.push(l.to_string());
            }
            "PATH" => {
                let (p, _) = parse_path(&tok[3..]);
                paths.push((tok[1].parse().unwrap(), tok[2].parse().unwrap(), p));
                out.push(l.to_string());
            }
            "EXEC" => {
                out.push(l.to_string());
                let co = off.get_or_insert_with(|| {
                    let mut c = ClipperOffset::new(miter, arc);
                    for (jt, et, p) in &paths {
                        c.add_path(p, join_type(*jt), end_type(*et));
                    }
                    c
                });
                let delta = hex_f64(tok[1]);
                let mut body = Vec::new();
                let r = guarded(|| {
                    if !tree {
                        let mut sol = Paths::new();
                        co.execute(&mut sol, delta)?;
                        out_lines(&sol, &mut body);
                        body.push("RES 1".into());
                    } else {
                        let mut t = PolyTree::new();
                        co.execute_tree(&mut t, delta)?;
                        body.push("RES 1".into());
                        tree_lines(&t, &mut body);
                    }
                    Ok(())
                });
                match r {
                    Outcome::Ok(()) => out.extend(body),
                    Outcome::Exc => out.push("EXC".into()),
                    Outcome::Panic => {
                        out.push("EXC".into());
                        st.panics_as_exc += 1;
                    }
                }
                out.push("ENDEXEC".into());
                while lines[i] != "ENDEXEC" {
                    i += 1;
                }
            }
            _ => out.push(l.to_string()),
        }
        i += 1;
    }
    out
}

/// simplify / clean / minkowski: echo inputs, compute outputs, skip the golden outputs.
fn run_misc_simple(lines: &[&str], st: &mut Stats) -> Vec<String> {
    let mut out = Vec::new();
    let mut header: Vec<&str> = Vec::new();
    let mut pattern = Path::new();
    let mut paths: Vec<Path> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        let tok: Vec<&str> = l.split(' ').collect();
        match tok[0] {
            "CASE" => out.push(l.to_string()),
            "SIMPLIFY" | "CLEAN" | "MINK" => {
                header = tok.clone();
                out.push(l.to_string());
            }
            "PATTERN" => {
                pattern = parse_path(&tok[1..]).0;
                out.push(l.to_string());
            }
            "PATH" => {
                paths.push(parse_path(&tok[3..]).0);
                out.push(l.to_string());
            }
            "END" => out.push(l.to_string()),
            _ => {} // golden output, skipped
        }
        i += 1;
    }
    // compute outputs before the final END
    let end = out.pop().expect("END");
    let mut body = Vec::new();
    match header[0] {
        "SIMPLIFY" => {
            let fill = fill_type(header[1].parse().unwrap());
            let single = header[2] == "1";
            let r = guarded(|| {
                if single {
                    Clipper::simplify_polygon(&paths[0], fill)
                } else {
                    Clipper::simplify_polygons(&paths, fill)
                }
            });
            match r {
                Outcome::Ok(res) => {
                    body.push("RES 1".to_string());
                    out_lines(&res, &mut body);
                }
                Outcome::Exc => body.push("EXC".into()),
                Outcome::Panic => {
                    body.push("EXC".into());
                    st.panics_as_exc += 1;
                }
            }
        }
        "CLEAN" => {
            let dist = hex_f64(header[1]);
            body.push("RES 1".into());
            out_lines(&Clipper::clean_polygons(&paths, dist), &mut body);
            body.push("EXEC1".into());
            body.push(format!("OUT {}", fmt_path(&Clipper::clean_polygon(&paths[0], dist))));
        }
        _ => {
            let kind: i32 = header[1].parse().unwrap();
            let closed = header[2] == "1";
            let r = guarded(|| match kind {
                3 => Clipper::minkowski_diff(&pattern, &paths[0]),
                2 => Clipper::minkowski_sum_paths(&pattern, &paths, closed),
                _ => Clipper::minkowski_sum(&pattern, &paths[0], closed),
            });
            match r {
                Outcome::Ok(res) => {
                    body.push("RES 1".to_string());
                    out_lines(&res, &mut body);
                }
                Outcome::Exc => body.push("EXC".into()),
                Outcome::Panic => {
                    body.push("EXC".into());
                    st.panics_as_exc += 1;
                }
            }
        }
    }
    out.extend(body);
    out.push(end);
    out
}

/// Named misc cases with one-line records.
fn run_misc_named(name: &str, lines: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut paths: Vec<Path> = Vec::new();
    for l in lines {
        let tok: Vec<&str> = l.split(' ').collect();
        match tok[0] {
            "AREA" => {
                let (p, used) = parse_path(&tok[1..]);
                let _ = used;
                out.push(format!(
                    "AREA {} {} {}",
                    fmt_path(&p),
                    fmt_hex(Clipper::area(&p)),
                    u8::from(Clipper::orientation(&p))
                ));
            }
            "PIP" => {
                let q = IntPoint::new(tok[1].parse().unwrap(), tok[2].parse().unwrap());
                let (p, _) = parse_path(&tok[3..]);
                out.push(format!(
                    "PIP {} {} {} {}",
                    q.x,
                    q.y,
                    fmt_path(&p),
                    Clipper::point_in_polygon(q, &p)
                ));
            }
            "DCTOR" => {
                let ip = IntPoint::from_f64(hex_f64(tok[1]), hex_f64(tok[2]));
                out.push(format!("DCTOR {} {} {} {}", tok[1], tok[2], ip.x, ip.y));
            }
            "PATH" => {
                paths.push(parse_path(&tok[3..]).0);
                out.push((*l).to_string());
            }
            "OUT" | "BOUNDS" => {} // regenerated below
            _ => out.push((*l).to_string()),
        }
    }
    if name == "reverse_paths_bounds" {
        let end = out.pop().expect("END");
        let mut copy = paths.clone();
        Clipper::reverse_paths(&mut copy);
        out_lines(&copy, &mut out);
        let b = Clipper::get_bounds(&paths);
        out.push(format!("BOUNDS {} {} {} {}", b.left, b.top, b.right, b.bottom));
        let b0 = Clipper::get_bounds(&[]);
        out.push(format!("BOUNDS {} {} {} {}", b0.left, b0.top, b0.right, b0.bottom));
        out.push(end);
    }
    out
}

fn run_file(file: &str, st: &mut Stats) {
    let text = std::fs::read_to_string(golden_dir().join(file)).expect("golden file");
    let all: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < all.len() {
        assert!(all[i].starts_with("CASE "), "bad golden structure at line {i}");
        let start = i;
        while all[i] != "END" {
            i += 1;
        }
        let case = &all[start..=i];
        i += 1;
        let name = case[0].trim_start_matches("CASE ").to_string();
        let kind = first_tag(case[1]);
        let actual = match kind {
            "CLIP" => run_clip(case, st),
            "OFFSET" => run_offset(case, st),
            "SIMPLIFY" | "CLEAN" | "MINK" => run_misc_simple(case, st),
            _ => run_misc_named(&name, case),
        };
        st.cases += 1;
        let expected: Vec<String> = case.iter().map(|s| (*s).to_string()).collect();
        if actual != expected {
            // first differing line for the report
            let n = actual.iter().zip(&expected).position(|(a, b)| a != b);
            let detail = match n {
                Some(k) => format!(
                    "line {k}: expected `{}` got `{}`",
                    expected[k].chars().take(200).collect::<String>(),
                    actual[k].chars().take(200).collect::<String>()
                ),
                None => format!("length expected {} got {}", expected.len(), actual.len()),
            };
            st.failed.push(format!("{file}:{name}: {detail}"));
        }
    }
}

#[test]
fn golden_all() {
    let mut st = Stats::default();
    for f in ["clip.txt", "potrace_clip.txt", "offset.txt", "misc.txt"] {
        run_file(f, &mut st);
    }
    eprintln!(
        "golden: {} cases, {} passed, {} failed, {} panics accepted as exception",
        st.cases,
        st.cases - st.failed.len(),
        st.failed.len(),
        st.panics_as_exc
    );
    for f in st.failed.iter().take(40) {
        eprintln!("FAIL {f}");
    }
    assert!(st.cases >= 300, "too few golden cases: {}", st.cases);
    assert!(st.failed.is_empty(), "{} golden cases differ", st.failed.len());
    let _ = f64_to_cint(0.0);
}
