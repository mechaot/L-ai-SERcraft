//! Small unit tests: GDI+ path emulation, Hilbert, GDI+ export smoke test.

use lg_potrace::{Curve, CurveKind, DPoint, GraphicsPath, Hilbert, export_to_gdiplus};
use resvg::tiny_skia::{Color, Pixmap};

#[test]
fn flatten_line_only_is_noop_and_compresses_start() {
    let mut p = GraphicsPath::new();
    p.add_line((0.0, 0.0), (1.0, 0.0));
    p.add_line((1.0, 0.0), (1.0, 1.0));
    p.flatten();
    assert_eq!(p.path_points(), &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
}

#[test]
fn flatten_bezier_ends_on_endpoint() {
    let mut p = GraphicsPath::new();
    p.add_bezier((0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0));
    p.flatten();
    let pts = p.path_points();
    assert!(pts.len() > 4);
    assert_eq!(pts[0], (0.0, 0.0));
    assert_eq!(*pts.last().unwrap(), (10.0, 0.0));
}

#[test]
fn hilbert_point_count() {
    // depth n has 4^n - 1 segments
    assert_eq!(Hilbert::execute(3, 1.0).len(), 64);
}

#[test]
fn gdiplus_export_fills_square() {
    let d = DPoint::new;
    let l = |a, b| Curve::new(CurveKind::Line, a, a, b, b);
    let sq = vec![
        l(d(2.0, 2.0), d(8.0, 2.0)),
        l(d(8.0, 2.0), d(8.0, 8.0)),
        l(d(8.0, 8.0), d(2.0, 8.0)),
        l(d(2.0, 8.0), d(2.0, 2.0)),
    ];
    let mut pm = Pixmap::new(10, 10).unwrap();
    export_to_gdiplus(&[sq], &mut pm, Some(Color::BLACK), None, 0.0);
    assert_eq!(pm.pixel(5, 5).unwrap().alpha(), 255);
    assert_eq!(pm.pixel(0, 0).unwrap().alpha(), 0);
}
