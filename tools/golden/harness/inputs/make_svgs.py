#!/usr/bin/env python3
"""One-off generator for inputs/svg/*.svg (checked-in inputs; run.sh does not call this)."""
import pathlib
out = pathlib.Path(__file__).resolve().parent / "svg"
out.mkdir(exist_ok=True)
NS = 'xmlns="http://www.w3.org/2000/svg"'

def wrap(body, attrs='width="100mm" height="100mm" viewBox="0 0 100 100"', ns=True):
    return f'<?xml version="1.0" encoding="UTF-8"?>\n<svg {NS if ns else ""} {attrs}>\n{body}\n</svg>\n'

def save(name, body, attrs='width="100mm" height="100mm" viewBox="0 0 100 100"', ns=True):
    (out / f"{name}.svg").write_text(wrap(body, attrs, ns), encoding="utf-8")

def path(d, extra=""):
    return f'  <path d="{d}" {extra}/>'

# ---- every path command
save("path_abs_lines", path("M 10 10 L 50 10 L 50 50 L 10 50 Z"))
save("path_rel_lines", path("m 10 10 l 40 0 l 0 40 l -40 0 z"))
save("path_hv", path("M10 10 H 50 V 50 h -40 v -40") + "\n" + path("m 60 60 h 20 v 20 H 60 V 60 z"))
save("path_cubic_C", path("M 10 90 C 20 10 40 10 50 90 C 60 170 80 170 90 90"))
save("path_cubic_c", path("m 10 90 c 10 -80 30 -80 40 0 c 10 80 30 80 40 0"))
save("path_smooth_S", path("M 10 50 C 10 10 40 10 40 50 S 70 90 70 50 S 90 20 95 50"))
save("path_smooth_s", path("m 10 50 c 0 -40 30 -40 30 0 s 30 40 30 0 s 20 -30 25 0"))
save("path_smooth_S_after_nonC", path("M 10 10 L 30 30 S 60 10 80 30 s 10 20 10 40"))
save("path_quad_Q", path("M 10 50 Q 30 10 50 50 Q 70 90 90 50"))
save("path_quad_q", path("m 10 50 q 20 -40 40 0 q 20 40 40 0"))
save("path_quad_T", path("M 10 50 Q 30 10 50 50 T 90 50 T 95 10"))
save("path_quad_t", path("m 10 50 q 20 -40 40 0 t 40 0 t 5 -40"))
save("path_arc_A", path("M 10 50 A 20 20 0 0 1 50 50 A 20 20 0 0 0 90 50 A 30 15 30 1 1 50 90"))
save("path_arc_a", path("m 10 50 a 20 20 0 0 1 40 0 a 20 20 0 0 0 40 0 a 30 15 30 1 1 -40 40"))
save("path_arc_flags", path("M 20 50 A 25 25 0 0 0 70 50 M 20 50 A 25 25 0 0 1 70 50 M 20 50 A 25 25 0 1 0 70 50 M 20 50 A 25 25 0 1 1 70 50"))
save("path_arc_radius_scaleup", path("M 10 50 A 5 5 0 0 1 90 50"))
save("path_arc_zero_radius", path("M 10 50 A 0 0 0 0 1 90 50 L 90 90"))
save("path_arc_rotated_ellipse", path("M 20 50 A 40 15 45 0 1 80 50 A 40 15 -45 1 0 20 50"))
save("path_arc_compact_flags", path("M10 50a20 20 0 1140 0"))
save("path_closepaths", path("M 10 10 L 40 10 L 40 40 Z M 60 60 L 90 60 L 90 90 z L 70 95"))
save("path_close_then_rel", path("m 10 10 l 30 0 l 0 30 z m 50 50 l 30 0 l 0 30 z"))
save("path_implicit_repeats", path("M 10 10 20 20 30 10 40 20 L 50 10 60 20 70 10 l 5 5 5 -5 5 5 H 95 100 V 50 60 C 10 10 20 20 30 30 40 40 50 50 60 60 Q 1 1 2 2 3 3 4 4"))
save("path_implicit_rel", path("m 10 10 20 20 10 -10 10 10 l 5 5 5 -5 h 5 5 v 5 5"))
save("path_compact_numbers", path("M10-10L20-20L30.5.5L-.5-.5l1e1,1e1l.5.5.5.5"))
save("path_exponents", path("M 1e1 1E1 L 2.5e+1 2.5e-1 L 1.5e1 3e0"))
save("path_commas_spaces", path("M 10,10 L 20 , 20 ,30,10 C 1,2 3,4 5,6"))
save("path_newlines_tabs", path("M 10 10\n\t L 50 10\n\t L 50 50 Z"))
save("path_multi_elements", path("M 10 10 L 20 20") + "\n" + path("M 30 30 L 40 40") + "\n" + path("m 5 5 l 5 5"))
save("path_long_polyline", path("M 0 0 " + " ".join(f"L {i*3} {(i*37)%50}" for i in range(1, 30))))
save("path_unknown_command", path("M 10 10 L 20 20 X 30 30 L 40 40"))
save("path_empty_d", path("") + "\n" + path("M 10 10 L 30 30"))
save("path_only_move", path("M 10 10"))
save("path_odd_args", path("M 10 10 L 20"))
save("path_numbers_with_units", path("M 10mm 10mm L 20mm 20mm L 1in 1in"))
save("path_precision", path("M 0.0004 0.0005 L 10.00049 10.00051 L 33.333333333 66.666666666 L 1234567.891 0.1234"))
save("path_tiny_segments", path("M 10 10 L 10.0001 10.0001 L 10.0002 10 L 20 20"))
save("path_dup_points", path("M 10 10 L 10 10 L 10 10 L 20 20 L 20 20"))

# ---- basic shapes
save("shape_rect", '  <rect x="10" y="10" width="40" height="30"/>')
save("shape_rect_rx", '  <rect x="10" y="10" width="40" height="30" rx="5"/>')
save("shape_rect_ry", '  <rect x="10" y="10" width="40" height="30" ry="6"/>')
save("shape_rect_rx_ry", '  <rect x="10" y="10" width="40" height="30" rx="5" ry="8"/>\n  <rect x="60" y="10" width="30" height="30" rx="8" ry="3"/>')
save("shape_rect_units", '  <rect x="1cm" y="1cm" width="2cm" height="10mm"/>\n  <rect x="0.5in" y="0.5in" width="10pt" height="10pc"/>')
save("shape_rect_percent", '  <rect x="10" y="10" width="50%" height="25%"/>')
save("shape_rect_noxy", '  <rect width="20" height="10"/>')
save("shape_circle", '  <circle cx="50" cy="50" r="20"/>')
save("shape_circle_units", '  <circle cx="2cm" cy="2cm" r="1cm"/>')
save("shape_ellipse", '  <ellipse cx="50" cy="50" rx="30" ry="15"/>')
save("shape_ellipse_circle_like", '  <ellipse cx="50" cy="50" rx="20" ry="20"/>')
save("shape_line", '  <line x1="10" y1="10" x2="90" y2="60"/>')
save("shape_polyline", '  <polyline points="10,10 20,40 30,10 40,40 50,10"/>')
save("shape_polygon", '  <polygon points="50,10 90,40 75,90 25,90 10,40"/>')
save("shape_polyline_spaces", '  <polyline points="10,10  20,40   30,10"/>\n  <polyline points=" 10,10 20,40 30,10 "/>')
save("shape_polyline_no_comma", '  <polyline points="10 10 20 40 30 10"/>')
save("shape_text_image", '  <text x="10" y="10">hello</text>\n  <image x="0" y="0" width="10" height="10" href="x.png"/>')
save("shape_mixed", '  <rect x="5" y="5" width="20" height="20"/>\n  <circle cx="50" cy="50" r="10"/>\n  <path d="M 70 70 L 90 90"/>\n  <line x1="0" y1="0" x2="100" y2="100"/>\n  <polygon points="10,80 30,80 20,95"/>')

# ---- groups and transforms
save("tf_translate", '  <g transform="translate(10,20)">\n' + path("M 0 0 L 30 0 L 30 30 Z") + '\n  </g>')
save("tf_translate_space", '  <g transform="translate(10 20)">\n' + path("M 0 0 L 30 0 L 30 30 Z") + '\n  </g>')
save("tf_translate_one", '  <g transform="translate(10)">\n' + path("M 0 0 L 30 0 L 30 30 Z") + '\n  </g>')
save("tf_scale", '  <g transform="scale(2,0.5)">\n' + path("M 5 5 L 30 5 L 30 30 Z") + '\n  </g>')
save("tf_scale_one", '  <g transform="scale(1.5)">\n' + path("M 5 5 L 30 5 L 30 30 Z") + '\n  </g>')
save("tf_rotate", '  <g transform="rotate(30)">\n' + path("M 30 0 L 60 0 L 60 30 Z") + '\n  </g>')
save("tf_rotate_center", '  <g transform="rotate(45,50,50)">\n' + path("M 30 30 L 70 30 L 70 70 L 30 70 Z") + '\n  </g>')
save("tf_rotate_center_space", '  <g transform="rotate(45 50 50)">\n' + path("M 30 30 L 70 30 L 70 70 L 30 70 Z") + '\n  </g>')
save("tf_matrix", '  <g transform="matrix(1,0.5,-0.5,1,20,10)">\n' + path("M 0 0 L 40 0 L 40 40 Z") + '\n  </g>')
save("tf_matrix_scale_neg", '  <g transform="matrix(-1,0,0,1,100,0)">\n' + path("M 10 10 L 40 10 L 40 40 Z") + '\n  </g>')
save("tf_skewX", '  <g transform="skewX(30)">\n' + path("M 10 10 L 40 10 L 40 40 Z") + '\n  </g>')
save("tf_skewY", '  <g transform="skewY(30)">\n' + path("M 10 10 L 40 10 L 40 40 Z") + '\n  </g>')
save("tf_chain", '  <g transform="translate(10,10) scale(2) rotate(20)">\n' + path("M 0 0 L 20 0 L 20 20 Z") + '\n  </g>')
save("tf_chain2", '  <g transform="rotate(20) translate(10,10) scale(2)">\n' + path("M 0 0 L 20 0 L 20 20 Z") + '\n  </g>')
save("tf_chain_matrix_translate", '  <g transform="matrix(1,0,0,1,5,5) translate(10,10)">\n' + path("M 0 0 L 20 0 L 20 20 Z") + '\n  </g>')
save("tf_nested_groups", '  <g transform="translate(10,10)">\n    <g transform="scale(2)">\n      <g transform="rotate(10)">\n' + path("M 0 0 L 10 0 L 10 10 Z") + '\n      </g>\n' + path("M 20 0 L 25 0 L 25 5 Z") + '\n    </g>\n' + path("M 50 0 L 60 0 L 60 10 Z") + '\n  </g>\n' + path("M 80 80 L 90 80 L 90 90 Z"))
save("tf_nested_deep", '  <g transform="translate(1,1)"><g transform="translate(1,1)"><g transform="translate(1,1)"><g transform="translate(1,1)"><g transform="translate(1,1)"><g transform="translate(1,1)">' + path("M 0 0 L 10 10") + '</g></g></g></g></g></g>')
save("tf_element_transform", path("M 0 0 L 20 0 L 20 20 Z", 'transform="translate(30,30) rotate(45)"') + "\n" + path("M 0 0 L 20 20"))
save("tf_group_and_element", '  <g transform="translate(10,10)">\n' + path("M 0 0 L 20 0 L 20 20 Z", 'transform="scale(2)"') + '\n  </g>')
save("tf_shapes_in_group", '  <g transform="translate(10,10) scale(0.5)">\n  <rect x="10" y="10" width="40" height="30" rx="5"/>\n  <circle cx="80" cy="50" r="20"/>\n  <ellipse cx="50" cy="90" rx="30" ry="10"/>\n  <line x1="0" y1="0" x2="100" y2="100"/>\n  <polygon points="10,10 30,10 20,30"/>\n  </g>')
save("tf_group_siblings", '  <g transform="translate(10,0)">' + path("M 0 0 L 10 10") + '</g>\n  <g transform="translate(0,10)">' + path("M 0 0 L 10 10") + '</g>\n  <g>' + path("M 0 0 L 10 10") + '</g>')

# ---- viewBox / size / units
sq = path("M 10 10 L 60 10 L 60 40 L 10 40 Z")
for name, attrs in [
    ("dim_mm_viewbox", 'width="50mm" height="30mm" viewBox="0 0 100 60"'),
    ("dim_mm_viewbox_offset", 'width="50mm" height="30mm" viewBox="10 20 100 60"'),
    ("dim_px", 'width="200px" height="100px" viewBox="0 0 200 100"'),
    ("dim_px_nounit", 'width="200" height="100" viewBox="0 0 200 100"'),
    ("dim_in", 'width="2in" height="1in" viewBox="0 0 200 100"'),
    ("dim_cm", 'width="5cm" height="3cm" viewBox="0 0 100 60"'),
    ("dim_pt", 'width="100pt" height="50pt" viewBox="0 0 100 50"'),
    ("dim_pc", 'width="10pc" height="5pc" viewBox="0 0 100 50"'),
    ("dim_percent", 'width="100%" height="100%" viewBox="0 0 100 100"'),
    ("dim_percent_half", 'width="50%" height="50%" viewBox="0 0 100 100"'),
    ("dim_em", 'width="10em" height="5em" viewBox="0 0 100 50"'),
    ("dim_no_viewbox", 'width="100mm" height="50mm"'),
    ("dim_viewbox_only", 'viewBox="0 0 100 50"'),
    ("dim_none", ''),
    ("dim_width_only", 'width="100mm" viewBox="0 0 100 50"'),
    ("dim_height_only", 'height="50mm" viewBox="0 0 100 50"'),
    ("dim_viewbox_commas", 'width="100mm" height="50mm" viewBox="0,0,100,50"'),
    ("dim_viewbox_multispace", 'width="100mm" height="50mm" viewBox="0   0   100    50"'),
    ("dim_viewbox_nonuniform", 'width="100mm" height="100mm" viewBox="0 0 100 50"'),
    ("dim_viewbox_neg_origin", 'width="100mm" height="100mm" viewBox="-50 -50 100 100"'),
    ("dim_mixed_units", 'width="1in" height="20mm" viewBox="0 0 96 75.59"'),
    ("dim_viewbox_scaleup", 'width="200mm" height="100mm" viewBox="0 0 100 50"'),
    ("dim_decimal", 'width="22.535355mm" height="24.070175mm" viewBox="0 0 22.535355 24.070175"'),
]:
    save(name, sq + "\n" + '  <circle cx="35" cy="25" r="10"/>', attrs)
save("dim_no_namespace", sq, 'width="100mm" height="100mm" viewBox="0 0 100 100"', ns=False)
(out / "dim_prefixed_namespace.svg").write_text('<?xml version="1.0"?>\n<svg:svg xmlns:svg="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><svg:path d="M 10 10 L 50 50"/></svg:svg>\n', encoding="utf-8")
(out / "dim_extra_ns_default.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg" xmlns:x="urn:x" width="10mm" height="10mm" viewBox="0 0 10 10"><x:path d="M 1 1 L 5 5"/><path d="M 2 2 L 8 8"/></svg>\n', encoding="utf-8")

# ---- colors / style / filter
colored = "\n".join([
    path("M 10 10 L 30 10", 'stroke="#ff0000" fill="none"'),
    path("M 10 20 L 30 20", 'stroke="#00ff00"'),
    path("M 10 30 L 30 30", 'stroke="#0000ff"'),
    path("M 10 40 L 30 40", 'stroke="#000000"'),
    path("M 10 50 L 30 50", 'style="stroke:#ff0000;stroke-width:1"'),
    path("M 10 60 L 30 60", 'style="fill:none;stroke:#00ff00;opacity:1"'),
    path("M 10 70 L 30 70", 'style="stroke:#0000ff"'),
    path("M 10 80 L 30 80", 'style="fill:#000000;stroke:none"'),
    path("M 10 90 L 30 90", 'fill="#ff0000"'),
    path("M 40 10 L 60 10", 'fill="red"'),
    path("M 40 20 L 60 20", 'stroke="green"'),
    path("M 40 30 L 60 30", 'stroke="blue"'),
    path("M 40 40 L 60 40", 'stroke="black"'),
    path("M 40 50 L 60 50", 'stroke="#f00"'),
    path("M 40 60 L 60 60", 'stroke="none" fill="none"'),
    path("M 40 70 L 60 70"),
    path("M 40 80 L 60 80", 'stroke="#808080"'),
    path("M 40 90 L 60 90", 'stroke="#7f0000"'),
    path("M 70 10 L 90 10", 'stroke="#ff1414"'),
    path("M 70 20 L 90 20", 'stroke="#ff1500"'),
    path("M 70 30 L 90 30", 'style="stroke: #0000ff; fill: none"'),
    path("M 70 40 L 90 40", 'stroke="#00800a"'),
    '  <rect x="70" y="50" width="10" height="10" stroke="#ff0000"/>',
    '  <circle cx="85" cy="65" r="5" style="stroke:#0000ff"/>',
    '  <line x1="70" y1="80" x2="90" y2="80" stroke="#00ff00"/>',
    '  <g transform="translate(0,0)">' + path("M 5 95 L 95 95", 'stroke="#ff0000"') + path("M 5 97 L 95 97", 'stroke="#000000"') + '</g>',
])
save("color_mixed", colored)
save("color_invalid_rgb_func", path("M 10 10 L 30 30", 'stroke="rgb(255,0,0)"') + "\n" + path("M 20 20 L 40 40", 'stroke="#ff0000"'))
save("color_invalid_name", path("M 10 10 L 30 30", 'stroke="notacolor"') + "\n" + path("M 20 20 L 40 40", 'stroke="#ff0000"'))
save("color_style_fill_only", path("M 10 10 L 30 30", 'style="fill:#ff0000"') + "\n" + path("M 20 20 L 40 40", 'style="stroke:none;fill:#0000ff"'))
save("color_style_malformed", path("M 10 10 L 30 30", 'style="stroke:#ff0000;;fill"') + "\n" + path("M 20 20 L 40 40", 'style="stroke:#ff0000:x"'))


# ---- color filter needs a style attribute on EVERY element (parseStyle returns null otherwise -> NullReferenceException upstream)
def sp(d, style): return path(d, f'style="{style}"')
save("color_style_all_valid", "\n".join([
    sp("M 10 10 L 30 10", "stroke:#ff0000;fill:none"),
    sp("M 10 20 L 30 20", "stroke:#00ff00"),
    sp("M 10 30 L 30 30", "stroke:#0000ff"),
    sp("M 10 40 L 30 40", "stroke:#000000"),
    sp("M 10 50 L 30 50", "fill:#ff0000"),
    sp("M 10 60 L 30 60", "fill:#00ff00;stroke:none"),
    sp("M 10 70 L 30 70", "stroke:none;fill:#0000ff"),
    sp("M 10 80 L 30 80", "fill:none;stroke:none"),
    sp("M 10 90 L 30 90", "opacity:1"),
    sp("M 40 10 L 60 10", "stroke:#ff1414"),
    sp("M 40 20 L 60 20", "stroke:#ff1500"),
    sp("M 40 30 L 60 30", "stroke:#7f0000"),
    sp("M 40 40 L 60 40", "stroke:#80ffff"),
    sp("M 40 50 L 60 50", "stroke:#14ff14"),
    sp("M 40 60 L 60 60", "stroke:#1480ff"),
    sp("M 40 70 L 60 70", "stroke:#141414"),
    sp("M 40 80 L 60 80", "stroke:#151515"),
    sp("M 40 90 L 60 90", "stroke:#808080"),
    sp("M 70 10 L 90 10", "stroke:red"),
    sp("M 70 20 L 90 20", "stroke:Lime"),
    sp("M 70 30 L 90 30", "stroke:blue"),
    sp("M 70 40 L 90 40", "stroke:black"),
    sp("M 70 50 L 90 50", "stroke:#f00"),
    sp("M 70 60 L 90 60", "stroke:#0f0"),
    sp("M 70 70 L 90 70", "stroke: #00f ; fill: none"),
    sp("M 70 80 L 90 80", "stroke:white"),
    '  <rect x="72" y="85" width="10" height="10" style="stroke:#ff0000"/>',
    '  <circle cx="86" cy="90" r="4" style="stroke:#0000ff"/>',
    '  <line x1="0" y1="95" x2="30" y2="95" style="stroke:#00ff00"/>',
    '  <g transform="translate(0,1)">' + sp("M 5 5 L 95 5", "stroke:#ff0000") + sp("M 5 7 L 95 7", "stroke:#000000") + '</g>',
]))
save("color_style_invalid_name", sp("M 10 10 L 30 30", "stroke:notacolor") + "\n" + sp("M 20 20 L 40 40", "stroke:#ff0000"))
save("color_style_dup_keys", sp("M 10 10 L 30 30", "stroke:#ff0000;stroke:#00ff00"))
save("color_style_attr_mix", '  <path d="M 10 10 L 30 30" style="opacity:1" stroke="#ff0000"/>\n  <path d="M 20 20 L 40 40" style="opacity:1" fill="#0000ff"/>\n  <path d="M 30 30 L 50 50" style="opacity:1" stroke="none" fill="#00ff00"/>')

# ---- misc structure
save("struct_comments_title", '  <title>t</title>\n  <desc>d</desc>\n  <!-- c -->\n' + path("M 1 1 L 9 9") + '\n  <defs>' + path("M 50 50 L 60 60") + '</defs>')
save("struct_unicode_ids", path("M 1 1 L 9 9", 'id="päth-日本"'))
(out / "struct_invalid_chars.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm" viewBox="0 0 10 10"><path d="M 1 1 L 9 9\x01\x02"/></svg>\n', encoding="utf-8")
(out / "struct_not_xml.svg").write_text("this is not xml\n", encoding="utf-8")
(out / "struct_empty_svg.svg").write_text('<svg xmlns="http://www.w3.org/2000/svg"/>\n', encoding="utf-8")
print(len(list(out.glob("*.svg"))), "svgs")
