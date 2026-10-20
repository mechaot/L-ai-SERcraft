//! Hand-written checks: clone-shared result, messages, SaveGCODE, drawing, svg glue, optimizer.

mod common;
use common::*;
use lg_gcode::*;

#[test]
fn clone_shares_result_and_clear_detaches() {
    let mut a = GrblCommand::new("g1 x1");
    let b = a.clone();
    assert!(a.shares_result_with(&b));
    assert_eq!(b.status(), CommandStatus::Queued);
    a.set_sending();
    assert_eq!(b.status(), CommandStatus::WaitingResponse);
    b.set_result(" ok ", false);
    assert_eq!(a.coded_result().as_deref(), Some("OK"));
    assert_eq!(a.status(), CommandStatus::ResponseGood);
    a.clear_result();
    assert!(!a.shares_result_with(&b));
    assert_eq!(a.status(), CommandStatus::Queued);
    assert_eq!(b.status(), CommandStatus::ResponseGood);
}

#[test]
fn display_list_link_is_invalidated() {
    let mut c = GrblCommand::new("G0 X1");
    let link = DisplayListLink::new();
    c.set_linked_display_list(Some(link.clone()));
    c.set_result("ok", false);
    c.clear_result();
    assert_eq!(link.generation(), 2);
}

#[test]
fn messages_classify() {
    let _g = sandbox();
    assert_eq!(GrblMessage::new("$110=500", false).message_type(), MessageType::Config);
    assert_eq!(GrblMessage::new("Grbl 1.1f ['$' for help]", false).message_type(), MessageType::Startup);
    assert_eq!(GrblMessage::new("ALARM:1", false).message_type(), MessageType::Alarm);
    assert_eq!(GrblMessage::new("<Idle|MPos:0,0,0>", false).message_type(), MessageType::Position);
    assert_eq!(GrblMessage::new("[MSG:x]", false).message_type(), MessageType::Feedback);
    let m = GrblMessage::new("ALARM:1", true);
    assert!(m.get_decoded_message().to_lowercase().contains("limit"), "{}", m.get_decoded_message());
    let mut bad = GrblCommand::new("G1 X");
    bad.set_result("error:3", true);
    assert_eq!(bad.status(), CommandStatus::ResponseBad);
    assert!(bad.get_result(true, false).unwrap() != "ERROR:3");
    assert_eq!(bad.image_index(), 2);
    let _ = bad.build_helper_report();
}

#[test]
fn save_gcode_header_footer_cycles() {
    let _g = sandbox();
    let mut f = GrblFile::new();
    f.commands_mut().push(GrblCommand::new("G1 X1"));
    f.commands_mut().push(GrblCommand::new("M5"));
    lg_base::settings::set("GCode.CustomHeader", "G90\r\n\r\n; comment [left]".to_string());
    let p = std::env::temp_dir().join(format!("lg-gcode-save-{}.nc", std::process::id()));
    f.save_gcode(&p, true, true, true, 2, true, &|l| if l.contains("comment") { String::new() } else { l.to_string() }).unwrap();
    let s = std::fs::read_to_string(&p).unwrap();
    let _ = std::fs::remove_file(&p);
    assert!(s.starts_with("G90\nG1 X1\nM5\n"), "{s:?}");
    // passes default has 4 lines, all start with ';' and are kept (evaluator echoes)
    assert!(s.contains(";G91 (use relative coordinates)\n"));
    assert!(s.ends_with("G0 X0 Y0 Z0 (move back to origin)\n"));
    assert_eq!(s.matches("G1 X1\n").count(), 2);
}

#[test]
fn optimize_paths_blocks_equal_sequential_blocks() {
    use lg_potrace::{Curve, CurveKind, DPoint};
    let mk = |x: f64, y: f64| vec![Curve::new(CurveKind::Line, DPoint::new(x, y), DPoint::new(x, y), DPoint::new(x, y), DPoint::new(x + 1.0, y + 0.5))];
    let mut list = Vec::new();
    for i in 0..5000u32 {
        let x = ((i * 7919) % 1000) as f64;
        let y = ((i * 104729) % 977) as f64;
        list.push(mk(x, y));
    }
    let par = parallel_optimize_paths(Some(list.clone()), 0.0).unwrap().unwrap();
    // reference: same blocks sequentially
    let blocknum = 3usize;
    let mut seq = Vec::new();
    for i in 0..blocknum {
        let from = i * list.len() / blocknum;
        let to = (i + 1) * list.len() / blocknum;
        seq.extend(optimize_paths(list[from..to].to_vec(), 0.0).unwrap());
    }
    assert_eq!(par, seq);
    assert_eq!(par.len(), 5000);
}

#[test]
fn draw_list_for_square() {
    let _g = sandbox();
    let root = golden_root().join("gcode_analyze");
    let mut f = GrblFile::new();
    f.load_file(root.join("square.nc").to_str().unwrap(), false).unwrap();
    let dl = f.draw_on_graphics((400, 300), &|t, em| (t.len() as f32 * em * 0.6, em)).unwrap().unwrap();
    assert!(dl.zoom > 0.0);
    assert!(dl.prims.iter().any(|p| matches!(p, DrawPrim::Line { .. })));
    assert!(dl.prims.iter().any(|p| matches!(p, DrawPrim::Text { .. })));
    let empty = GrblFile::new();
    assert!(empty.draw_on_graphics((400, 300), &|_, _| (1.0, 1.0)).unwrap().is_none());
}

#[test]
fn svg_load_smoke() {
    let _g = sandbox();
    let svg = golden_root().join("svg/inputs/LaserGRBL-accuracy-test-file.svg");
    let mut f = GrblFile::new();
    f.load_imported_svg(svg.to_str().unwrap(), false, lg_svg2gcode::ColorFilter::All).unwrap();
    assert!(f.count() > 100);
    assert!(f.range().moving_range.valid_range());
}

#[test]
fn loading_flag_via_spawn() {
    let _g = sandbox();
    let root = golden_root().join("gcode_analyze");
    let f = std::sync::Arc::new(std::sync::RwLock::new(GrblFile::new()));
    let path = root.join("square.nc").to_str().unwrap().to_string();
    let h = GrblFile::spawn_load(&f, move |g| g.load_file(&path, false)).unwrap();
    h.join().unwrap();
    assert!(f.read().unwrap().count() > 0);
    assert!(!f.read().unwrap().check_in_use());
}

#[test]
fn truncate_list_keeps_head_only() {
    let mut f = GrblFile::new();
    for l in ["G0 X1", "G1 X2", "G1 X3"] {
        f.commands_mut().push(GrblCommand::new(l));
    }
    f.truncate_list(1);
    assert_eq!(f.count(), 1);
    assert_eq!(f.get(0).map(|c| c.command()), Some("G0 X1"));
    // longer than list: no-op
    f.truncate_list(5);
    assert_eq!(f.count(), 1);
    f.truncate_list(0);
    assert_eq!(f.count(), 0);
}

#[test]
fn svg_layered_ranges_cover_layers() {
    use lg_svg2gcode::{ColorFilter, LayerParams};
    let _g = sandbox();
    let dir = std::env::temp_dir().join(format!("lg_layers_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let svg = dir.join("two.svg");
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="50mm" height="50mm" viewBox="0 0 50 50">
<g id="a"><rect x="1" y="1" width="10" height="10"/></g>
<g id="b"><circle cx="30" cy="30" r="5"/></g></svg>"#,
    )
    .unwrap();
    let o = lg_gcode::svg_conv_options();
    let base = LayerParams::from_options(&o, ColorFilter::All);
    let path = svg.to_str().unwrap();
    let mut f = GrblFile::new();
    f.commands_mut().push(GrblCommand::new("G0 X1"));
    let r = f.load_imported_svg_layered(path, true, &[base.clone(), base.clone()]).unwrap();
    let (a, b) = (r[0].clone().unwrap(), r[1].clone().unwrap());
    // append offset: first layer starts after head + initial command
    assert!(a.start >= 2 && a.end == b.start && b.end as usize <= f.count());
    assert!(f.commands_mut()[b.start as usize..b.end as usize].iter().any(|c| c.command().contains("G3")));
    assert!(!f.commands_mut()[a.start as usize..a.end as usize].iter().any(|c| c.command().contains("G3")));
    // disabled layer -> None
    let mut off = base.clone();
    off.enabled = false;
    let r = f.load_imported_svg_layered(path, false, &[off, base]).unwrap();
    assert!(r[0].is_none() && r[1].is_some());
    std::fs::remove_dir_all(&dir).ok();
}
