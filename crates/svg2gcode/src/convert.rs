// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2015-2018 Sven Hasemann (GRBL-Plotter code)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `SvgConverter/GCodeFromSVG.cs` (SVG -> G-code). Floats stay `f32`, doubles `f64`.
//!
//! .NET exceptions of the original become `Err(SvgError)` with the same exception type name.

use crate::bezier::flatten_to;
use crate::color::from_html;
use crate::gcode::{Gcode, SvgConvOptions};
use crate::hatch::{HatchFill, fill_segments, flatten_arc};
use crate::matrix::{Matrix, Point};
use crate::SvgError;
use lg_dotnet::{f64_to_i32, fmt_float, is_number, parse_double};
use regex::Regex;
use roxmltree::Node;

/// Process-wide state of the static C# class `gcode` (checked out during a conversion).
static GCODE_STATE: std::sync::Mutex<Option<Gcode>> = std::sync::Mutex::new(None);

/// `ColorFilter` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorFilter {
    #[default]
    All,
    Red,
    Green,
    Blue,
    Black,
}

/// One selectable layer of an SVG (top-level `<g>` or the ungrouped root shapes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerInfo {
    /// Stable key: `#root`, `id:<id>` or `idx:<n>` (n = 1-based group index).
    pub key: String,
    /// Display label (`inkscape:label`, id, `Layer n` or `(ungrouped)`).
    pub label: String,
    /// Number of shape elements inside.
    pub shapes: usize,
}

/// Per-layer conversion values.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerParams {
    /// Layer is converted when true.
    pub enabled: bool,
    /// Repeat count of the layer body.
    pub passes: u32,
    /// G1 feed.
    pub border_speed: i32,
    /// S value in PWM mode.
    pub power_max: i32,
    /// Laser-on command (`M3`/`M4`).
    pub laser_on: String,
    /// Colour filter for this layer.
    pub filter: ColorFilter,
    /// Hatch fill of closed curves (`None` = off).
    pub fill: Option<HatchFill>,
}

impl LayerParams {
    /// Layer values equal to the global options. `o`: options. `filter`: colour filter. Returns params.
    pub fn from_options(o: &SvgConvOptions, filter: ColorFilter) -> Self {
        LayerParams { enabled: true, passes: 1, border_speed: o.border_speed, power_max: o.power_max, laser_on: o.laser_on.clone(), filter, fill: None }
    }
}

/// Per layer, its byte range in the generated G-code (`None` = layer disabled).
pub type LayerRanges = Vec<Option<std::ops::Range<usize>>>;

const SHAPE_NAMES: [&str; 8] = ["path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "text"];
const INKSCAPE_NS: &str = "http://www.inkscape.org/namespaces/inkscape";

/// Strip chars invalid in XML 1.0 (see `convert_from_text`). `text`: raw XML. Returns cleaned text.
fn clean_xml(text: &str) -> String {
    text.chars()
        .filter(|&c| matches!(c as u32, 0x09 | 0x0A | 0x0D | 0x20..=0xD7FF | 0xE000..=0xFFFD))
        .collect()
}

/// Default namespace of the root (as `parse_globals`). `svg`: root. Returns URI or "".
fn root_namespace(svg: Node) -> String {
    svg.namespaces().find(|ns| ns.name().is_none()).map(|ns| ns.uri().to_string()).unwrap_or_default()
}

/// Is `n` a shape element in namespace `ns`. `n`: node. `ns`: namespace URI. Returns bool.
fn is_shape(n: Node, ns: &str) -> bool {
    n.is_element()
        && (SHAPE_NAMES.contains(&n.tag_name().name()) || n.tag_name().name() == "image")
        && n.tag_name().namespace().unwrap_or("") == ns
}

/// Top-level `<g>` children in namespace `ns`. `svg`: root. `ns`: namespace URI. Returns nodes.
fn top_groups<'a, 'i>(svg: Node<'a, 'i>, ns: &str) -> Vec<Node<'a, 'i>> {
    svg.children().filter(|c| c.is_element() && c.tag_name().name() == "g" && c.tag_name().namespace().unwrap_or("") == ns).collect()
}

/// Does the root hold direct shapes. `svg`: root. `ns`: namespace URI. Returns bool.
fn root_has_shapes(svg: Node, ns: &str) -> bool {
    svg.children().any(|c| is_shape(c, ns))
}

/// List layers of a parsed document. `svg`: root element. Returns layers in conversion order.
fn list_layers_node(svg: Node) -> Vec<LayerInfo> {
    let ns = root_namespace(svg);
    let mut out = Vec::new();
    if root_has_shapes(svg, &ns) {
        let shapes = svg.children().filter(|c| is_shape(*c, &ns)).count();
        out.push(LayerInfo { key: "#root".into(), label: "(ungrouped)".into(), shapes });
    }
    for (i, g) in top_groups(svg, &ns).into_iter().enumerate() {
        let n = i + 1;
        let id = g.attribute("id").filter(|s| !s.is_empty());
        let label = g
            .attribute((INKSCAPE_NS, "label"))
            .filter(|s| !s.is_empty())
            .or(id)
            .map(str::to_string)
            .unwrap_or_else(|| format!("Layer {n}"));
        let key = id.map_or_else(|| format!("idx:{n}"), |s| format!("id:{s}"));
        let shapes = g.descendants().filter(|d| *d != g && is_shape(*d, &ns)).count();
        out.push(LayerInfo { key, label, shapes });
    }
    out
}

/// List layers of SVG text.
///
/// `text`: SVG XML. Returns layers (root pseudo-layer first when needed) or the XML error.
pub fn list_layers_from_text(text: &str) -> Result<Vec<LayerInfo>, SvgError> {
    let cleaned = clean_xml(text);
    let opt = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(&cleaned, opt)
        .map_err(|e| SvgError::new("System.Xml.XmlException", e.to_string()))?;
    Ok(list_layers_node(doc.root_element()))
}

/// List layers of an SVG file. `path`: file. Returns layers or error.
pub fn list_layers_from_file(path: &std::path::Path) -> Result<Vec<LayerInfo>, SvgError> {
    let bytes = std::fs::read(path).map_err(|e| SvgError::new("System.IO.IOException", e.to_string()))?;
    list_layers_from_text(&decode_read_all_text(&bytes))
}

/// `System.Drawing.PointF`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

const FACTOR_IN2PX: f32 = 96.0;
const FACTOR_MM2PX: f32 = 96.0f32 / 25.4f32;
const FACTOR_CM2PX: f32 = 96.0f32 / 2.54f32;
const FACTOR_PT2PX: f32 = 96.0f32 / 72.0f32;
const FACTOR_PC2PX: f32 = 12.0 * 96.0f32 / 72.0f32;
const FACTOR_EM2PX: f32 = 150.0;
const FILTER_COLOR_LIMIT_H_LOW: i32 = 20;
const FILTER_COLOR_LIMIT_HIGH: i32 = 127;
const SVG_STROKE_ATTRIBUTE: &str = "stroke";
const SVG_FILL_ATTRIBUTE: &str = "fill";

/// `Math.Max(float, float)` (NaN propagating). `a`,`b`: operands. Returns max.
fn cs_max(a: f32, b: f32) -> f32 {
    if a > b || a.is_nan() { a } else { b }
}

/// `Math.Min(float, float)` (NaN propagating). `a`,`b`: operands. Returns min.
fn cs_min(a: f32, b: f32) -> f32 {
    if a < b || a.is_nan() { a } else { b }
}

/// Array access with `IndexOutOfRangeException`. `s`: slice. `i`: index. Returns element.
fn at<T: Copy>(s: &[T], i: usize) -> Result<T, SvgError> {
    s.get(i).copied().ok_or_else(SvgError::index)
}

/// Attribute without namespace (`XElement.Attribute("name")?.Value`). `n`: element. `name`: key.
fn attr<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.attribute(name)
}

/// `str.Split(sep)` keeping empties. `s`: text. `sep`: separator char. Returns pieces.
fn split_c(s: &str, sep: char) -> Vec<&str> {
    s.split(sep).collect()
}

/// UTF-16 length of `s` (C# `string.Length`). `s`: text. Returns code unit count.
fn len16(s: &str) -> usize {
    s.encode_utf16().count()
}

/// `double.TryParse(s, NumberStyles.Float, InvariantInfo)`: Float has no thousands separator.
///
/// `s`: text. Returns parsed value or None.
fn try_parse_float_style(s: &str) -> Option<f64> {
    if s.contains(',') {
        return None;
    }
    parse_double(s)
}

/// `ConvertToPixel(str, ext)`: number with optional unit to px.
///
/// `s`: text like `10mm`, `50%`, `3`. `ext`: reference size for percent. Returns px value (0 on failure).
pub fn convert_to_pixel(s: &str, ext: f32) -> f32 {
    let mut percent = false;
    let mut factor = 1.0f32;
    let pos = |pat: &str| s.find(pat).is_some_and(|i| i > 0);
    if pos("mm") {
        factor = FACTOR_MM2PX;
    } else if pos("cm") {
        factor = FACTOR_CM2PX;
    } else if pos("in") {
        factor = FACTOR_IN2PX;
    } else if pos("pt") {
        factor = FACTOR_PT2PX;
    } else if pos("pc") {
        factor = FACTOR_PC2PX;
    } else if pos("em") {
        factor = FACTOR_EM2PX;
    } else if pos("%") {
        percent = true;
    }
    let s = s
        .replace("pt", "")
        .replace("pc", "")
        .replace("mm", "")
        .replace("cm", "")
        .replace("in", "")
        .replace("em ", "")
        .replace('%', "")
        .replace("px", "");
    if !s.is_empty()
        && let Some(test) = try_parse_float_style(&s)
    {
        return if percent { (test as f32) * ext / 100.0 } else { (test as f32) * factor };
    }
    0.0
}

/// `ConvertToPixel(str)` with `ext = 1`. `s`: text. Returns px.
fn cp(s: &str) -> f32 {
    convert_to_pixel(s, 1.0)
}

/// `GetTextBetween(source, s1)`: text after `s1` up to first char not in `[number . , space - e]`.
///
/// `source`: full transform word. `s1`: prefix like `translate(`. Returns the argument text.
pub fn get_text_between(source: &str, s1: &str) -> Result<String, SvgError> {
    let chars: Vec<char> = source.chars().collect();
    let s1c: Vec<char> = s1.chars().collect();
    // IndexOf (ordinal), -1 when missing
    let idx: i64 = (0..chars.len())
        .find(|&i| chars[i..].starts_with(&s1c))
        .map_or(-1, |i| i as i64);
    let start = idx + s1c.len() as i64;
    let oor = || SvgError::new("System.ArgumentOutOfRangeException", "Index and length must refer to a location within the string.");
    let mut i = start;
    while i < chars.len() as i64 {
        let c = chars[i as usize];
        if !(is_number(c) || c == '.' || c == ',' || c == ' ' || c == '-' || c == 'e') {
            return Ok(chars[start as usize..i as usize].iter().collect());
        }
        i += 1;
    }
    let len = chars.len() as i64 - start - 1;
    if start < 0 || start > chars.len() as i64 || len < 0 {
        return Err(oor());
    }
    Ok(chars[start as usize..(start + len) as usize].iter().collect())
}

/// Split into coordinate parts like `coord.Split(',')`, fallback `Split(' ')` without comma.
///
/// `coord`: argument text. Returns pieces.
fn coord_split(coord: &str) -> Vec<&str> {
    if coord.contains(',') { split_c(coord, ',') } else { split_c(coord, ' ') }
}

/// `ParseTransform(string)`: one transform word to a matrix.
///
/// `transform`: word like `translate(10,20)`. Returns the matrix or an exception.
pub fn parse_transform_string(transform: &str) -> Result<Matrix, SvgError> {
    let mut tmp = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    if transform.contains("translate") {
        let coord = get_text_between(transform, "translate(")?;
        let split = coord_split(&coord);
        tmp.set_offset_x(cp(at(&split, 0)?) as f64);
        if split.len() > 1 {
            tmp.set_offset_y(cp(split[1].trim_end_matches(')')) as f64);
        }
    }
    if transform.contains("scale") {
        let coord = get_text_between(transform, "scale(")?;
        let split = coord_split(&coord);
        tmp.set_m11(cp(at(&split, 0)?) as f64);
        if split.len() > 1 {
            tmp.set_m22(cp(split[1]) as f64);
        } else {
            tmp.set_m11(cp(&coord) as f64);
            tmp.set_m22(cp(&coord) as f64);
        }
    }
    if transform.contains("rotate") {
        let coord = get_text_between(transform, "rotate(")?;
        let split = coord_split(&coord);
        // NOTE(upstream): arkypita change, rotation about (px, py)
        let angle = cp(at(&split, 0)?);
        let px = if split.len() == 3 { cp(split[1]) } else { 0.0 };
        let py = if split.len() == 3 { cp(split[2]) } else { 0.0 };
        tmp.rotate_at(angle as f64, px as f64, py as f64);
    }
    if transform.contains("matrix") {
        let coord = get_text_between(transform, "matrix(")?;
        let split = coord_split(&coord);
        tmp.set_m11(cp(at(&split, 0)?) as f64);
        tmp.set_m12(cp(at(&split, 1)?) as f64);
        tmp.set_m21(cp(at(&split, 2)?) as f64);
        tmp.set_m22(cp(at(&split, 3)?) as f64);
        tmp.set_offset_x(cp(at(&split, 4)?) as f64);
        tmp.set_offset_y(cp(at(&split, 5)?) as f64);
    }
    Ok(tmp)
}

/// `string.Split(string[], RemoveEmptyEntries)`: first listed separator wins at a position.
///
/// `s`: text. `seps`: separators. Returns non-empty pieces.
fn split_strings<'a>(s: &'a str, seps: &[&str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut last = 0;
    let mut i = 0;
    while i < s.len() {
        if let Some(sep) = seps.iter().find(|sep| s[i..].starts_with(**sep)) {
            if i > last {
                out.push(&s[last..i]);
            }
            i += sep.len();
            last = i;
        } else {
            i += s[i..].chars().next().map_or(1, |c| c.len_utf8());
        }
    }
    if s.len() > last {
        out.push(&s[last..]);
    }
    out
}

/// Parsed `style` attribute (`parseStyle`): None when attribute missing (C# null).
type StyleMap = Option<Vec<(String, String)>>;

/// SVG to G-code converter (class `GCodeFromSVG`). Public C# fields/properties are public fields here.
pub struct GCodeFromSvg {
    gcode_string: String,
    svg_bezier_accuracy: i32,
    /// `SvgScaleApply`: scale final G-code to `svg_max_size` when true (C# default false).
    pub svg_scale_apply: bool,
    /// `SvgMaxSize`: final size of the larger dimension if scaling (C# default 100).
    pub svg_max_size: f32,
    svg_close_path_extend: bool,
    svg_nodes_only: bool,
    svg_pause_element: bool,
    svg_pause_pen_down: bool,
    svg_comments: bool,
    gcode_reduce: bool,
    gcode_reduce_val: f32,
    /// `UserOffset`: added to the base matrix offset (C# default 0,0).
    pub user_offset: PointF,
    /// `GCodeXYFeed` (C# default 2000). NOTE(upstream): never read; feed comes from `border_speed` option.
    pub gcode_xy_feed: f32,
    scaled_error: f64,
    /// `svgConvertToMM` (C# default true): px -> mm, else inch.
    pub svg_convert_to_mm: bool,
    gcode_scale: f32,
    matrix_group: Vec<Matrix>,
    matrix_element: Matrix,
    old_matrix_element: Matrix,
    load_color_filter: ColorFilter,
    import_in_mm: bool,
    nspace: String,
    /// `UseLegacyBezier`: fixed-segment bezier instead of adaptive flattening (C# default false).
    pub use_legacy_bezier: bool,
    gcode: Gcode,
    pen_is_down: bool,
    start_sub_path: bool,
    count_sub_path: i32,
    start_path: bool,
    start_first_element: bool,
    svg_width_px: f32,
    svg_height_px: f32,
    offset_x: f32,
    offset_y: f32,
    current_x: f32,
    current_y: f32,
    first_x: Option<f32>,
    first_y: Option<f32>,
    last_x: f32,
    last_y: f32,
    cx_mirror: f32,
    cy_mirror: f32,
    second_move: String,
    is_reduce_ok: bool,
    reject_point: bool,
    last_gcx: f64,
    last_gcy: f64,
    last_set_gcx: f64,
    last_set_gcy: f64,
    distance: f64,
    fill_rec: Option<Vec<Vec<Point>>>,
    layers: Vec<LayerParams>,
    layer_ranges: Vec<Option<std::ops::Range<usize>>>,
}

impl Default for GCodeFromSvg {
    /// Field initialisers of the C# class.
    fn default() -> Self {
        Self::new()
    }
}

impl GCodeFromSvg {
    /// `new GCodeFromSVG()`. Returns a converter with C# defaults.
    pub fn new() -> Self {
        GCodeFromSvg {
            gcode_string: String::new(),
            svg_bezier_accuracy: 12,
            svg_scale_apply: false,
            svg_max_size: 100.0,
            svg_close_path_extend: true,
            svg_nodes_only: false,
            svg_pause_element: false,
            svg_pause_pen_down: false,
            svg_comments: false,
            gcode_reduce: false,
            gcode_reduce_val: 0.1,
            user_offset: PointF::default(),
            gcode_xy_feed: 2000.0,
            scaled_error: 1.0,
            svg_convert_to_mm: true,
            gcode_scale: 1.0,
            matrix_group: vec![Matrix::default(); 10],
            matrix_element: Matrix::default(),
            old_matrix_element: Matrix::default(),
            load_color_filter: ColorFilter::All,
            import_in_mm: false,
            nspace: String::new(),
            use_legacy_bezier: false,
            gcode: Gcode::default(),
            pen_is_down: true,
            start_sub_path: true,
            count_sub_path: 0,
            start_path: true,
            start_first_element: true,
            svg_width_px: 0.0,
            svg_height_px: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            first_x: None,
            first_y: None,
            last_x: 0.0,
            last_y: 0.0,
            cx_mirror: 0.0,
            cy_mirror: 0.0,
            second_move: String::new(),
            is_reduce_ok: false,
            reject_point: false,
            last_gcx: 0.0,
            last_gcy: 0.0,
            last_set_gcx: 0.0,
            last_set_gcy: 0.0,
            distance: 0.0,
            fill_rec: None,
            layers: Vec::new(),
            layer_ranges: Vec::new(),
        }
    }

    /// `convertFromText(text, core, importMM, filter)`.
    ///
    /// `text`: SVG XML. `opts`: settings snapshot (replaces `GrblCore core` and `Settings`).
    /// `import_mm`: stored only (unused upstream). `filter`: colour filter.
    /// Returns the G-code text, or the .NET exception the original would throw.
    pub fn convert_from_text(&mut self, text: &str, opts: &SvgConvOptions, import_mm: bool, filter: ColorFilter) -> Result<String, SvgError> {
        self.import_in_mm = import_mm;
        // strip chars not valid in XML 1.0 (upstream regex also drops astral chars, see NOTE)
        // NOTE(upstream): `က0-u10FFFF` in the regex is a typo; effective allowed set is 09 0A 0D 20-D7FF E000-FFFD.
        let cleaned = clean_xml(text);
        let opt = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
        let doc = roxmltree::Document::parse_with_options(&cleaned, opt)
            .map_err(|e| SvgError::new("System.Xml.XmlException", e.to_string()))?;
        self.load_color_filter = filter;
        self.convert_svg(doc.root_element(), opts)
    }

    /// `convertFromFile(file, core, filter)` for a real file (embedded resources: read them yourself
    /// and call [`Self::convert_from_text`]). `File.ReadAllText` BOM detection is replicated.
    ///
    /// `path`: SVG file. `opts`: settings snapshot. `filter`: colour filter. Returns G-code or exception.
    pub fn convert_from_file(&mut self, path: &std::path::Path, opts: &SvgConvOptions, filter: ColorFilter) -> Result<String, SvgError> {
        let bytes = std::fs::read(path).map_err(|e| SvgError::new("System.IO.IOException", e.to_string()))?;
        let xml = decode_read_all_text(&bytes);
        self.convert_from_text(&xml, opts, true, filter)
    }

    /// Layered `convertFromText`: per-layer feed, power, passes, enable and colour filter.
    ///
    /// `text`: SVG XML. `opts`: global options. `import_mm`: stored only. `layers`: params aligned with
    /// [`list_layers_from_text`] order (empty = plain conversion; missing entries use the globals).
    /// Returns the G-code and, per layer, its byte range in it (`None` = disabled), or the exception.
    pub fn convert_from_text_layered(
        &mut self,
        text: &str,
        opts: &SvgConvOptions,
        import_mm: bool,
        layers: &[LayerParams],
    ) -> Result<(String, LayerRanges), SvgError> {
        self.layers = layers.to_vec();
        self.layer_ranges.clear();
        // plain path for empty layer list
        let filter = layers.first().map_or(ColorFilter::All, |l| l.filter);
        // initial command uses the first converted layer's values, no instant mode switch
        let mut o = opts.clone();
        if let Some(first) = layers.iter().find(|l| l.enabled) {
            o.border_speed = first.border_speed;
            o.power_max = first.power_max;
            o.laser_on = first.laser_on.clone();
        }
        let r = self.convert_from_text(text, &o, import_mm, filter);
        self.layers.clear();
        let ranges = std::mem::take(&mut self.layer_ranges);
        r.map(|g| (g, ranges))
    }

    /// Layered `convertFromFile`. `path`: SVG file. Other args and return as [`Self::convert_from_text_layered`].
    pub fn convert_from_file_layered(
        &mut self,
        path: &std::path::Path,
        opts: &SvgConvOptions,
        layers: &[LayerParams],
    ) -> Result<(String, LayerRanges), SvgError> {
        let bytes = std::fs::read(path).map_err(|e| SvgError::new("System.IO.IOException", e.to_string()))?;
        let xml = decode_read_all_text(&bytes);
        self.convert_from_text_layered(&xml, opts, true, layers)
    }

    /// Layered body of `startConvert`: each layer in turn with its own values.
    ///
    /// `svg`: root. `opts`: global options. Records `layer_ranges`.
    fn convert_layers(&mut self, svg: Node, opts: &SvgConvOptions) -> Result<(), SvgError> {
        let ns = self.nspace.clone();
        let mut idx = 0usize;
        // one layer: params lookup, skip, set values, repeat body
        let mut run = |this: &mut Self, body: &dyn Fn(&mut Self) -> Result<(), SvgError>| -> Result<(), SvgError> {
            let lp = this.layers.get(idx).cloned().unwrap_or_else(|| LayerParams::from_options(opts, ColorFilter::All));
            idx += 1;
            if !lp.enabled {
                this.layer_ranges.push(None);
                return Ok(());
            }
            let start = this.gcode_string.len();
            this.gcode.set_layer(opts, &lp, &mut this.gcode_string);
            this.load_color_filter = lp.filter;
            let mut segs = Vec::new();
            for pass in 0..lp.passes.max(1) {
                // record output geometry in the first pass only
                if pass == 0 && lp.fill.is_some() {
                    this.fill_rec = Some(Vec::new());
                }
                let r = body(this);
                if pass == 0
                    && let Some(f) = lp.fill
                {
                    // hatch once from closed outlines, reuse for later passes
                    let polys = closed_polys(this.fill_rec.take().unwrap_or_default());
                    segs = fill_segments(&polys, &f);
                }
                r?;
                this.emit_fill(&segs);
            }
            this.layer_ranges.push(Some(start..this.gcode_string.len()));
            Ok(())
        };
        if root_has_shapes(svg, &ns) {
            run(self, &|t: &mut Self| {
                if !t.svg_nodes_only {
                    t.parse_basic_elements(svg, 1)?;
                }
                t.parse_path(svg, 1)
            })?;
        }
        for group in top_groups(svg, &ns) {
            run(self, &|t: &mut Self| t.parse_one_group(group, 1))?;
        }
        Ok(())
    }

    /// `convertSVG`. `svg`: root element. `opts`: settings. Returns G-code.
    fn convert_svg(&mut self, svg: Node, opts: &SvgConvOptions) -> Result<String, SvgError> {
        // C# `gcode` is a static class: its state is shared by the whole process (e.g. `lastf` survives)
        self.gcode = GCODE_STATE.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or_default();
        let r = self.convert_svg_inner(svg, opts);
        *GCODE_STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::mem::take(&mut self.gcode));
        r
    }

    /// Body of `convertSVG` once the shared `gcode` state is checked out. Args as [`Self::convert_svg`].
    fn convert_svg_inner(&mut self, svg: Node, opts: &SvgConvOptions) -> Result<String, SvgError> {
        self.gcode.setup(opts);
        self.gcode.set_rapid_num(if opts.disable_g0_fast_skip { 1 } else { 0 });
        self.gcode.put_initial_command(&mut self.gcode_string);
        self.start_convert(svg, opts)?;
        self.gcode.put_final_command(&mut self.gcode_string);
        Ok(self.gcode_string.replace(',', "."))
    }

    /// `startConvert`: reset state, parse root. `svg`: root element. `opts`: global options (layered mode).
    fn start_convert(&mut self, svg: Node, opts: &SvgConvOptions) -> Result<(), SvgError> {
        self.count_sub_path = 0;
        self.start_first_element = true;
        self.gcode_scale = 1.0;
        self.svg_width_px = 0.0;
        self.svg_height_px = 0.0;
        self.current_x = 0.0;
        self.current_y = 0.0;
        self.offset_x = 0.0;
        self.offset_y = 0.0;
        self.first_x = None;
        self.first_y = None;
        self.last_x = 0.0;
        self.last_y = 0.0;
        self.matrix_element.set_identity();
        self.old_matrix_element.set_identity();
        for m in self.matrix_group.iter_mut() {
            m.set_identity();
        }
        self.parse_globals(svg)?;
        if !self.layers.is_empty() {
            return self.convert_layers(svg, opts);
        }
        if !self.svg_nodes_only {
            self.parse_basic_elements(svg, 1)?;
        }
        self.parse_path(svg, 1)?;
        self.parse_group(svg, 1)?;
        Ok(())
    }

    /// Child elements named `{nspace}name` (`Elements(nspace + name)`). `n`: parent. `name`: local name.
    fn elements<'a, 'i>(&self, n: Node<'a, 'i>, name: &'static str) -> Vec<Node<'a, 'i>> {
        let ns = self.nspace.clone();
        n.children()
            .filter(|c| c.is_element() && c.tag_name().name() == name && c.tag_name().namespace().unwrap_or("") == ns)
            .collect()
    }

    /// `parseGlobals`: viewBox / width / height to base matrix. `svg`: root element.
    fn parse_globals(&mut self, svg: Node) -> Result<(), SvgError> {
        let mut tmp = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        self.svg_width_px = 0.0;
        self.svg_height_px = 0.0;
        let mut vb_off_x = 0.0f32;
        let mut vb_off_y = 0.0f32;
        let mut vb_width = 0.0f32;
        let mut vb_height = 0.0f32;

        // `Attribute("xmlns")` = default namespace declared on root
        self.nspace = svg
            .namespaces()
            .find(|ns| ns.name().is_none())
            .map(|ns| ns.uri().to_string())
            .unwrap_or_default();

        if let Some(viewbox) = attr(svg, "viewBox") {
            // viewBox unit always in px
            let re = Regex::new(r"\s+").expect("re");
            let vb = re.replace_all(viewbox, " ").replace(' ', "|");
            let split = split_c(&vb, '|');
            vb_off_x = -cp(at(&split, 0)?);
            vb_off_y = -cp(at(&split, 1)?);
            vb_width = cp(at(&split, 2)?);
            vb_height = cp(at(&split, 3)?.trim_end_matches(')'));
            tmp.set_m11(1.0);
            tmp.set_m22(-1.0); // flip Y
            tmp.set_offset_y(vb_height as f64);
        }

        // back from px to mm / inch
        let scale: f32 = if self.svg_convert_to_mm { 1.0 / FACTOR_MM2PX } else { 1.0 / FACTOR_IN2PX };

        if let Some(w) = attr(svg, "width") {
            self.svg_width_px = cp(w);
            tmp.set_m11(scale as f64);
            if vb_width > 0.0 {
                self.scaled_error = (scale * self.svg_width_px / vb_width) as f64;
                tmp.set_m11(self.scaled_error);
                tmp.set_offset_x(((vb_off_x * self.svg_width_px / vb_width) * scale) as f64);
            }
        }

        if let Some(h) = attr(svg, "height") {
            self.svg_height_px = cp(h);
            tmp.set_m22((-scale) as f64); // scale and flip vertical
            tmp.set_offset_y((scale * self.svg_height_px) as f64);
            if vb_height > 0.0 {
                tmp.set_m22((-scale * self.svg_height_px / vb_height) as f64);
                tmp.set_offset_y(((-vb_off_y * self.svg_height_px / vb_height + self.svg_height_px) * scale) as f64);
            }
        }

        let new_width = cs_max(self.svg_width_px, vb_width);
        let new_height = cs_max(self.svg_height_px, vb_height);
        if new_width > 0.0 && new_height > 0.0 && self.svg_scale_apply {
            // factor to reach desired max size
            self.gcode_scale = self.svg_max_size / cs_max(new_width, new_height);
            tmp.scale(self.gcode_scale as f64, self.gcode_scale as f64);
            if self.svg_convert_to_mm {
                tmp.scale(FACTOR_MM2PX as f64, FACTOR_MM2PX as f64);
            } else {
                tmp.scale(FACTOR_IN2PX as f64, FACTOR_IN2PX as f64);
            }
        }

        // user offset (centerline)
        tmp.set_offset_x(tmp.offset_x() + self.user_offset.x as f64);
        tmp.set_offset_y(tmp.offset_y() + self.user_offset.y as f64);

        for m in self.matrix_group.iter_mut() {
            *m = tmp;
        }
        self.matrix_element = tmp;
        Ok(())
    }

    /// `parseGroup`: recurse over `<g>`. `svg`: parent. `level`: nesting level.
    fn parse_group(&mut self, svg: Node, level: usize) -> Result<(), SvgError> {
        for group in self.elements(svg, "g") {
            self.parse_one_group(group, level)?;
        }
        Ok(())
    }

    /// Body of the `parseGroup` loop. `group`: `<g>` node. `level`: nesting level.
    fn parse_one_group(&mut self, group: Node, level: usize) -> Result<(), SvgError> {
        if self.svg_comments
            && let Some(id) = attr(group, "id")
        {
            self.gcode_string.push_str(&format!("\r\n( Group level:{} id={} )\r\n", level, id));
        }
        self.parse_transform(group, true, level)?; // applied in gcodeMove
        if !self.svg_nodes_only {
            self.parse_basic_elements(group, level)?;
        }
        self.parse_path(group, level)?;
        self.parse_group(group, level + 1)?;
        Ok(())
    }

    /// Group matrix slot with bounds check. `level`: index. Returns the matrix copy.
    fn mg(&self, level: usize) -> Result<Matrix, SvgError> {
        self.matrix_group.get(level).copied().ok_or_else(SvgError::index)
    }

    /// `parseTransform`. `element`: node. `is_group`: group matrix mode. `level`: nesting. Returns the element's own matrix.
    fn parse_transform(&mut self, element: Node, is_group: bool, level: usize) -> Result<Matrix, SvgError> {
        let mut final_matrix = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        if let Some(transform) = attr(element, "transform") {
            // transform elements must be processed in given order
            let words = split_strings(transform, &["trans", "sca", "rot", "mat"]);
            for word in words {
                if word.starts_with("late") {
                    final_matrix = Matrix::multiply(parse_transform_string(&format!("trans{word}"))?, final_matrix);
                } else if word.starts_with("le") {
                    final_matrix = Matrix::multiply(parse_transform_string(&format!("sca{word}"))?, final_matrix);
                } else if word.starts_with("ate") {
                    final_matrix = Matrix::multiply(parse_transform_string(&format!("rot{word}"))?, final_matrix);
                } else if word.starts_with("rix") {
                    final_matrix = Matrix::multiply(parse_transform_string(&format!("mat{word}"))?, final_matrix);
                }
            }
        }
        if is_group {
            self.matrix_group.get_mut(level).ok_or_else(SvgError::index)?.set_identity();
            if level > 0 {
                for i in level..self.matrix_group.len() {
                    self.matrix_group[i] = Matrix::multiply(final_matrix, self.mg(level - 1)?);
                }
            } else {
                self.matrix_group[level] = final_matrix;
            }
            self.matrix_element = self.mg(level)?;
        } else {
            self.matrix_element = Matrix::multiply(final_matrix, self.mg(level)?);
        }
        Ok(final_matrix)
    }

    /// `getColor`: stroke colour text for comments. `el`: element. Returns text.
    fn get_color(&self, el: Node) -> String {
        let mut stroke_color = "000000".to_string();
        if let Some(style) = attr(el, "style") {
            if let Some(start) = style.find("stroke:#")
                && let Some(end) = style[start..].find(';').map(|e| e + start)
                && end > start
            {
                stroke_color = style[start + 8..end].to_string();
            }
            return stroke_color;
        }
        String::new()
    }

    /// `parseBasicElements`: rect, circle, ellipse, line, polyline, polygon, text, image.
    ///
    /// `svg`: parent element. `level`: nesting level.
    fn parse_basic_elements(&mut self, svg: Node, level: usize) -> Result<(), SvgError> {
        let forms = ["rect", "circle", "ellipse", "line", "polyline", "polygon", "text", "image"];
        for form in forms {
            for el in self.elements(svg, form) {
                if !self.filter_by_color(el, self.load_color_filter)? {
                    continue;
                }
                let my_color = self.get_color(el);
                if self.svg_comments {
                    if let Some(id) = attr(el, "id") {
                        self.gcode_string.push_str(&format!("\r\n( Basic shape level:{} id={} )\r\n", level, id));
                    }
                    self.gcode_string.push_str(&format!("( SVG color=#{})\r\n", my_color));
                }
                if self.start_first_element {
                    self.gcode_pen_up("1st shape");
                    self.start_first_element = false;
                }
                self.offset_x = 0.0;
                self.offset_y = 0.0;
                self.old_matrix_element = self.matrix_element;
                let avoid_g23 = false;
                self.parse_transform(el, false, level)?;

                let (mut x, mut y, mut x1, mut y1, mut x2, mut y2) = (0f32, 0f32, 0f32, 0f32, 0f32, 0f32);
                let (mut width, mut height, mut rx, mut ry, mut cx, mut cy, mut r) = (0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32);
                let mut points: Vec<&str> = vec![""];
                if let Some(v) = attr(el, "x") { x = cp(v); }
                if let Some(v) = attr(el, "y") { y = cp(v); }
                if let Some(v) = attr(el, "x1") { x1 = cp(v); }
                if let Some(v) = attr(el, "y1") { y1 = cp(v); }
                if let Some(v) = attr(el, "x2") { x2 = cp(v); }
                if let Some(v) = attr(el, "y2") { y2 = cp(v); }
                if let Some(v) = attr(el, "width") { width = convert_to_pixel(v, self.svg_width_px); }
                if let Some(v) = attr(el, "height") { height = convert_to_pixel(v, self.svg_height_px); }
                if let Some(v) = attr(el, "rx") { rx = cp(v); }
                if let Some(v) = attr(el, "ry") { ry = cp(v); }
                if let Some(v) = attr(el, "cx") { cx = cp(v); }
                if let Some(v) = attr(el, "cy") { cy = cp(v); }
                if let Some(v) = attr(el, "r") { r = cp(v); }
                if let Some(v) = attr(el, "points") { points = split_c(v, ' '); }

                if form == "rect" {
                    if ry == 0.0 {
                        ry = rx;
                    } else if rx == 0.0 {
                        rx = ry;
                    } else if rx != ry {
                        rx = cs_min(rx, ry);
                        ry = rx; // only same r for x and y are possible
                    }
                    if self.svg_comments {
                        self.gcode_string.push_str(&format!(
                            "( SVG-Rect x:{} y:{} width:{} height:{} rx:{} ry:{})\r\n",
                            fmt_float(x), fmt_float(y), fmt_float(width), fmt_float(height), fmt_float(rx), fmt_float(ry)
                        ));
                    }
                    x += self.offset_x;
                    y += self.offset_y;
                    self.gcode_start_path(x + rx, y + height, form);
                    self.gcode_move_to(x + width - rx, y + height, &format!("{form} a1"));
                    if rx > 0.0 { self.gcode_arc_to_ccw(x + width, y + height - ry, 0.0, -ry, form, avoid_g23); }
                    self.gcode_move_to(x + width, y + ry, &format!("{form} b1")); // upper right
                    if rx > 0.0 { self.gcode_arc_to_ccw(x + width - rx, y, -rx, 0.0, form, avoid_g23); }
                    self.gcode_move_to(x + rx, y, &format!("{form} a2")); // upper left
                    if rx > 0.0 { self.gcode_arc_to_ccw(x, y + ry, 0.0, ry, form, avoid_g23); }
                    self.gcode_move_to(x, y + height - ry, &format!("{form} b2")); // lower left
                    if rx > 0.0 {
                        self.gcode_arc_to_ccw(x + rx, y + height, rx, 0.0, form, avoid_g23);
                        self.gcode_move_to(x + rx, y + height, form); // repeat first point to avoid back draw after last G3
                    }
                    self.gcode_stop_path(form);
                } else if form == "circle" {
                    if self.svg_comments {
                        self.gcode_string.push_str(&format!("( circle cx:{} cy:{} r:{} )\r\n", fmt_float(cx), fmt_float(cy), fmt_float(r)));
                    }
                    cx += self.offset_x;
                    cy += self.offset_y;
                    self.gcode_start_path(cx + r, cy, form);
                    self.gcode_arc_to_ccw(cx + r, cy, -r, 0.0, form, avoid_g23);
                    self.gcode_stop_path(form);
                } else if form == "ellipse" {
                    if self.svg_comments {
                        // NOTE(upstream): format uses {2} twice (rx printed as ry)
                        self.gcode_string.push_str(&format!("( ellipse cx:{} cy:{} rx:{}  ry:{})\r\n", fmt_float(cx), fmt_float(cy), fmt_float(rx), fmt_float(rx)));
                    }
                    cx += self.offset_x;
                    cy += self.offset_y;
                    self.gcode_start_path(cx + rx, cy, form);
                    self.is_reduce_ok = true;
                    self.calc_arc(cx + rx, cy, rx, ry, 0.0, 1.0, 1.0, cx - rx, cy);
                    self.calc_arc(cx - rx, cy, rx, ry, 0.0, 1.0, 1.0, cx + rx, cy);
                    self.gcode_stop_path(form);
                } else if form == "line" {
                    if self.svg_comments {
                        self.gcode_string.push_str(&format!("( SVG-Line x1:{} y1:{} x2:{} y2:{} )\r\n", fmt_float(x1), fmt_float(y1), fmt_float(x2), fmt_float(y2)));
                    }
                    x1 += self.offset_x;
                    y1 += self.offset_y;
                    self.gcode_start_path(x1, y1, form);
                    self.gcode_move_to(x2, y2, form);
                    self.gcode_stop_path(form);
                } else if form == "polyline" || form == "polygon" {
                    self.offset_x = 0.0;
                    self.offset_y = 0.0;
                    if self.svg_comments {
                        self.gcode_string.push_str("( SVG-Polyline )\r\n");
                    }
                    let mut index = 0;
                    while index < points.len() {
                        if !points[index].is_empty() {
                            break;
                        }
                        index += 1;
                    }
                    let first = at(&points, index)?;
                    if first.contains(',') {
                        let coord = split_c(first, ',');
                        x = cp(at(&coord, 0)?);
                        y = cp(at(&coord, 1)?);
                        x1 = x;
                        y1 = y;
                        self.gcode_start_path(x, y, form);
                        self.is_reduce_ok = true;
                        for p in points.iter().skip(index + 1) {
                            if len16(p) > 3 {
                                let coord = split_c(p, ',');
                                x = cp(at(&coord, 0)?);
                                y = cp(at(&coord, 1)?);
                                x += self.offset_x;
                                y += self.offset_y;
                                self.gcode_move_to(x, y, form);
                            }
                        }
                        if form == "polygon" {
                            self.gcode_move_to(x1, y1, form);
                        }
                        self.gcode_stop_path(form);
                    } else {
                        self.gcode_string.push_str("( polygon coordinates - missing ',')\r\n");
                    }
                } else if form == "text" || form == "image" {
                    self.gcode_string.push_str("( +++++++++++++++++++++++++++++++++ )\r\n");
                    self.gcode_string.push_str(&format!("( ++++++ {form} is not supported ++++ )\r\n"));
                    if form == "text" {
                        self.gcode_string.push_str("( ++ Convert Object to Path first + )\r\n");
                    }
                    self.gcode_string.push_str("( +++++++++++++++++++++++++++++++++ )\r\n");
                } else if self.svg_comments {
                    self.gcode_string.push_str(&format!("( ++++++ Unknown Shape: {form} )"));
                }
                self.matrix_element = self.old_matrix_element;
            }
        }
        Ok(())
    }

    /// `parsePath`: all `<path>` children. `svg`: parent. `level`: nesting level.
    fn parse_path(&mut self, svg: Node, level: usize) -> Result<(), SvgError> {
        // pattern: split before each letter except lowercase 'e' (`(?=[A-Za-z-[e]])`)
        for el in self.elements(svg, "path") {
            if !self.filter_by_color(el, self.load_color_filter)? {
                continue;
            }
            self.offset_x = 0.0;
            self.offset_y = 0.0;
            self.current_x = self.offset_x;
            self.current_y = self.offset_x; // NOTE(upstream): uses offsetX for Y
            self.first_x = None;
            self.first_y = None;
            self.start_path = true;
            self.start_sub_path = true;
            self.last_x = self.offset_x;
            self.last_y = self.offset_y;
            let d = attr(el, "d").ok_or_else(SvgError::null_ref)?;
            let mut id: String = d.to_string();
            if len16(&id) > 20 {
                id = d.encode_utf16().take(20).map(|u| char::from_u32(u as u32).unwrap_or('\u{FFFD}')).collect();
            }
            let my_color = self.get_color(el);
            if self.svg_comments {
                if let Some(i) = attr(el, "id") {
                    self.gcode_string.push_str(&format!("\r\n( Path level:{} id={} )\r\n", level, i));
                } else {
                    self.gcode_string.push_str(&format!("\r\n( SVG path={} )\r\n", id));
                }
                self.gcode_string.push_str(&format!("\r\n(SVG color=#{})\r\n", my_color));
            }
            self.old_matrix_element = self.matrix_element;
            self.parse_transform(el, false, level)?; // applied in gcodeMove

            if !d.is_empty() {
                // split path into command tokens
                let mut tokens: Vec<&str> = Vec::new();
                let mut start = 0;
                for (i, c) in d.char_indices() {
                    if i > 0 && c.is_ascii_alphabetic() && c != 'e' {
                        tokens.push(&d[start..i]);
                        start = i;
                    }
                }
                tokens.push(&d[start..]);
                for token in tokens.into_iter().filter(|t| !t.is_empty()) {
                    self.parse_path_command(token)?;
                }
            }
            self.gcode_pen_up("End path");
            self.matrix_element = self.old_matrix_element;
        }
        Ok(())
    }

    /// `filterByColor`. `el`: element. `filter`: colour filter. Returns keep-flag or exception.
    fn filter_by_color(&self, el: Node, filter: ColorFilter) -> Result<bool, SvgError> {
        if filter == ColorFilter::All {
            return Ok(true);
        }
        let style_map = parse_style(el)?;
        let color_map = parse_color_attributes(el);
        let look = |m: &[(String, String)], k: &str| m.iter().find(|e| e.0 == k).map(|e| e.1.clone());
        // NOTE(upstream): styleMap is null without style attribute -> NullReferenceException
        let sm = style_map.as_ref().ok_or_else(SvgError::null_ref)?;
        let path_color = look(sm, SVG_STROKE_ATTRIBUTE)
            .or_else(|| look(sm, SVG_FILL_ATTRIBUTE))
            .or_else(|| look(&color_map, SVG_STROKE_ATTRIBUTE))
            .or_else(|| look(&color_map, SVG_FILL_ATTRIBUTE));
        let Some(path_color) = path_color else {
            return Ok(true);
        };
        let Some(rgb) = from_html(&path_color)? else {
            return Ok(true);
        };
        let (r, g, b) = (rgb[0] as i32, rgb[1] as i32, rgb[2] as i32);
        Ok(match filter {
            ColorFilter::Red => r >= FILTER_COLOR_LIMIT_HIGH && g <= FILTER_COLOR_LIMIT_H_LOW && b <= FILTER_COLOR_LIMIT_H_LOW,
            ColorFilter::Green => r <= FILTER_COLOR_LIMIT_H_LOW && g > FILTER_COLOR_LIMIT_HIGH && b <= FILTER_COLOR_LIMIT_H_LOW,
            ColorFilter::Blue => r <= FILTER_COLOR_LIMIT_H_LOW && g <= FILTER_COLOR_LIMIT_H_LOW && b > FILTER_COLOR_LIMIT_HIGH,
            ColorFilter::Black => r <= FILTER_COLOR_LIMIT_H_LOW && g <= FILTER_COLOR_LIMIT_H_LOW && b <= FILTER_COLOR_LIMIT_H_LOW,
            ColorFilter::All => true,
        })
    }

    /// `parsePathCommand`: one command token. `svg_path`: token text. Returns object count.
    fn parse_path_command(&mut self, svg_path: &str) -> Result<i32, SvgError> {
        let command = svg_path.chars().next().ok_or_else(|| SvgError::new("System.InvalidOperationException", "Sequence contains no elements"))?;
        let cmd = command.to_uppercase().next().unwrap_or(command);
        let absolute = cmd == command;
        let remaining = &svg_path[command.len_utf8()..];
        let re = Regex::new(r"((\-|)\d+(\.\d+|)((((E|e)(\-|\+|))|\.)\d+|)|((\-|)\.\d+))").expect("re");
        let fa: Vec<f32> = re.find_iter(remaining).map(|m| cp(m.as_str())).collect();
        let cmds = command.to_string();
        let mut obj_count = 0;
        let (ox, oy) = (self.offset_x, self.offset_y);

        match cmd {
            'M' => {
                // new sub-path
                let mut i = 0;
                while i < fa.len() {
                    obj_count += 1;
                    if absolute || self.start_path {
                        self.current_x = at(&fa, i)? + ox;
                        self.current_y = at(&fa, i + 1)? + oy;
                    } else {
                        self.current_x = at(&fa, i)? + self.last_x;
                        self.current_y = at(&fa, i + 1)? + self.last_y;
                    }
                    if self.start_sub_path {
                        if self.svg_comments {
                            self.gcode_string.push_str(&format!("( Start new subpath at {} {} )\r\n", fmt_float(fa[i]), fmt_float(fa[i + 1])));
                        }
                        let c = self.count_sub_path;
                        self.count_sub_path += 1;
                        if c > 0 {
                            self.gcode_stop_path("Stop Path");
                        }
                        if self.svg_nodes_only {
                            self.gcode_dot_only(self.current_x, self.current_y, &cmds);
                        } else {
                            self.gcode_start_path(self.current_x, self.current_y, &cmds);
                        }
                        self.is_reduce_ok = true;
                        self.first_x = Some(self.current_x);
                        self.first_y = Some(self.current_y);
                        self.start_path = false;
                        self.start_sub_path = false;
                    } else if self.svg_nodes_only {
                        self.gcode_dot_only(self.current_x, self.current_y, &cmds);
                    } else if i <= 1 {
                        self.gcode_start_path(self.current_x, self.current_y, &cmds);
                    } else {
                        self.gcode_move_to(self.current_x, self.current_y, &cmds);
                    }
                    if self.first_x.is_none() {
                        self.first_x = Some(self.current_x);
                    }
                    if self.first_y.is_none() {
                        self.first_y = Some(self.current_y);
                    }
                    self.last_x = self.current_x;
                    self.last_y = self.current_y;
                    i += 2;
                }
                self.cx_mirror = self.current_x;
                self.cy_mirror = self.current_y;
            }
            'Z' => {
                // close sub-path
                let inv = || SvgError::new("System.InvalidOperationException", "Nullable object must have a value.");
                if !self.svg_nodes_only {
                    if self.first_x.is_none() {
                        self.first_x = Some(self.current_x);
                    }
                    if self.first_y.is_none() {
                        self.first_y = Some(self.current_y);
                    }
                    let (fx, fy) = (self.first_x.ok_or_else(inv)?, self.first_y.ok_or_else(inv)?);
                    self.gcode_move_to(fx, fy, &cmds);
                }
                self.last_x = self.first_x.ok_or_else(inv)?;
                self.last_y = self.first_y.ok_or_else(inv)?;
                self.first_x = None;
                self.first_y = None;
                self.start_sub_path = true;
                if self.svg_close_path_extend && !self.svg_nodes_only {
                    let sm = self.second_move.clone();
                    self.gcode_string.push_str(&sm);
                }
                self.gcode_stop_path("Z");
            }
            'L' => {
                let mut i = 0;
                while i < fa.len() {
                    obj_count += 1;
                    if absolute {
                        self.current_x = at(&fa, i)? + ox;
                        self.current_y = at(&fa, i + 1)? + oy;
                    } else {
                        self.current_x = self.last_x + at(&fa, i)?;
                        self.current_y = self.last_y + at(&fa, i + 1)?;
                    }
                    self.emit_point(&cmds);
                    self.last_x = self.current_x;
                    self.last_y = self.current_y;
                    self.cx_mirror = self.current_x;
                    self.cy_mirror = self.current_y;
                    i += 2;
                }
                self.start_sub_path = true;
            }
            'H' => {
                for &v in &fa {
                    obj_count += 1;
                    if absolute {
                        self.current_x = v + ox;
                        self.current_y = self.last_y;
                    } else {
                        self.current_x = self.last_x + v;
                        self.current_y = self.last_y;
                    }
                    self.emit_point(&cmds);
                    self.last_x = self.current_x;
                    self.last_y = self.current_y;
                    self.cx_mirror = self.current_x;
                    self.cy_mirror = self.current_y;
                }
                self.start_sub_path = true;
            }
            'V' => {
                for &v in &fa {
                    obj_count += 1;
                    if absolute {
                        self.current_x = self.last_x;
                        self.current_y = v + oy;
                    } else {
                        self.current_x = self.last_x;
                        self.current_y = self.last_y + v;
                    }
                    self.emit_point(&cmds);
                    self.last_x = self.current_x;
                    self.last_y = self.current_y;
                    self.cx_mirror = self.current_x;
                    self.cy_mirror = self.current_y;
                }
                self.start_sub_path = true;
            }
            'A' => {
                if self.svg_comments {
                    self.gcode_string.push_str(&format!("( Command {} {} )\r\n", cmds, if absolute { "absolute" } else { "relative" }));
                }
                let mut rep = 0;
                while rep < fa.len() {
                    obj_count += 1;
                    if self.svg_comments {
                        self.gcode_string.push_str(&format!("( draw arc nr. {} )\r\n", 1 + rep / 6));
                    }
                    let rx = at(&fa, rep)?;
                    let ry = at(&fa, rep + 1)?;
                    let rot = at(&fa, rep + 2)?;
                    let large = at(&fa, rep + 3)?;
                    let sweep = at(&fa, rep + 4)?;
                    let (nx, ny) = if absolute {
                        (at(&fa, rep + 5)? + ox, at(&fa, rep + 6)? + oy)
                    } else {
                        (at(&fa, rep + 5)? + self.last_x, at(&fa, rep + 6)? + self.last_y)
                    };
                    if self.svg_nodes_only {
                        self.gcode_dot_only(self.current_x, self.current_y, &cmds);
                    } else {
                        self.calc_arc(self.last_x, self.last_y, rx, ry, rot, large, sweep, nx, ny);
                    }
                    self.last_x = nx;
                    self.last_y = ny;
                    rep += 7;
                }
                self.start_sub_path = true;
            }
            'C' => {
                if self.svg_comments {
                    self.gcode_string.push_str(&format!("( Command {} {} )\r\n", cmds, if absolute { "absolute" } else { "relative" }));
                }
                let mut rep = 0;
                while rep < fa.len() {
                    obj_count += 1;
                    if self.svg_comments {
                        self.gcode_string.push_str(&format!("( draw curve nr. {} )\r\n", 1 + rep / 6));
                    }
                    if rep + 5 < fa.len() {
                        let (cx1, cy1, cx2, cy2, cx3, cy3);
                        if absolute {
                            cx1 = fa[rep] + ox;
                            cy1 = fa[rep + 1] + oy;
                            cx2 = fa[rep + 2] + ox;
                            cy2 = fa[rep + 3] + oy;
                            cx3 = fa[rep + 4] + ox;
                            cy3 = fa[rep + 5] + oy;
                        } else {
                            cx1 = self.last_x + fa[rep];
                            cy1 = self.last_y + fa[rep + 1];
                            cx2 = self.last_x + fa[rep + 2];
                            cy2 = self.last_y + fa[rep + 3];
                            cx3 = self.last_x + fa[rep + 4];
                            cy3 = self.last_y + fa[rep + 5];
                        }
                        let points = [
                            Point::new(self.last_x as f64, self.last_y as f64),
                            Point::new(cx1 as f64, cy1 as f64),
                            Point::new(cx2 as f64, cy2 as f64),
                            Point::new(cx3 as f64, cy3 as f64),
                        ];
                        let b = self.get_bezier_approximation(&points, self.svg_bezier_accuracy);
                        self.emit_curve(&b, cx3, cy3, &cmds);
                        self.cx_mirror = cx3 - (cx2 - cx3);
                        self.cy_mirror = cy3 - (cy2 - cy3);
                        self.last_x = cx3;
                        self.last_y = cy3;
                    } else {
                        self.gcode_string.push_str(&format!("( Missing argument after {} )\r\n", rep));
                    }
                    rep += 6;
                }
                self.start_sub_path = true;
            }
            'S' => {
                if self.svg_comments {
                    self.gcode_string.push_str(&format!("( Command {} {} )\r\n", cmds, if absolute { "absolute" } else { "relative" }));
                }
                let mut rep = 0;
                while rep < fa.len() {
                    obj_count += 1;
                    if self.svg_comments {
                        self.gcode_string.push_str(&format!("( draw curve nr. {} )\r\n", 1 + rep / 4));
                    }
                    let (cx2, cy2, cx3, cy3);
                    if absolute {
                        cx2 = at(&fa, rep)? + ox;
                        cy2 = at(&fa, rep + 1)? + oy;
                        cx3 = at(&fa, rep + 2)? + ox;
                        cy3 = at(&fa, rep + 3)? + oy;
                    } else {
                        cx2 = self.last_x + at(&fa, rep)?;
                        cy2 = self.last_y + at(&fa, rep + 1)?;
                        cx3 = self.last_x + at(&fa, rep + 2)?;
                        cy3 = self.last_y + at(&fa, rep + 3)?;
                    }
                    let points = [
                        Point::new(self.last_x as f64, self.last_y as f64),
                        Point::new(self.cx_mirror as f64, self.cy_mirror as f64),
                        Point::new(cx2 as f64, cy2 as f64),
                        Point::new(cx3 as f64, cy3 as f64),
                    ];
                    let b = self.get_bezier_approximation(&points, self.svg_bezier_accuracy);
                    self.emit_curve(&b, cx3, cy3, &cmds);
                    self.cx_mirror = cx3 - (cx2 - cx3);
                    self.cy_mirror = cy3 - (cy2 - cy3);
                    self.last_x = cx3;
                    self.last_y = cy3;
                    rep += 4;
                }
                self.start_sub_path = true;
            }
            'Q' => {
                if self.svg_comments {
                    self.gcode_string.push_str(&format!("( Command {} {} )\r\n", cmds, if absolute { "absolute" } else { "relative" }));
                }
                let mut rep = 0;
                while rep < fa.len() {
                    obj_count += 1;
                    if self.svg_comments {
                        self.gcode_string.push_str(&format!("( draw curve nr. {} )\r\n", 1 + rep / 4));
                    }
                    let (cx2, cy2, cx3, cy3);
                    if absolute {
                        cx2 = at(&fa, rep)? + ox;
                        cy2 = at(&fa, rep + 1)? + oy;
                        cx3 = at(&fa, rep + 2)? + ox;
                        cy3 = at(&fa, rep + 3)? + oy;
                    } else {
                        cx2 = self.last_x + at(&fa, rep)?;
                        cy2 = self.last_y + at(&fa, rep + 1)?;
                        cx3 = self.last_x + at(&fa, rep + 2)?;
                        cy3 = self.last_y + at(&fa, rep + 3)?;
                    }
                    // shorten control points to 2/3 length to use cubic function
                    let qpx1 = (cx2 - self.last_x) * 2.0 / 3.0 + self.last_x;
                    let qpy1 = (cy2 - self.last_y) * 2.0 / 3.0 + self.last_y;
                    let qpx2 = (cx2 - cx3) * 2.0 / 3.0 + cx3;
                    let qpy2 = (cy2 - cy3) * 2.0 / 3.0 + cy3;
                    let points = [
                        Point::new(self.last_x as f64, self.last_y as f64),
                        Point::new(qpx1 as f64, qpy1 as f64),
                        Point::new(qpx2 as f64, qpy2 as f64),
                        Point::new(cx3 as f64, cy3 as f64),
                    ];
                    self.cx_mirror = cx3 - (cx2 - cx3);
                    self.cy_mirror = cy3 - (cy2 - cy3);
                    self.last_x = cx3;
                    self.last_y = cy3;
                    let b = self.get_bezier_approximation(&points, self.svg_bezier_accuracy);
                    self.emit_curve(&b, cx3, cy3, &cmds);
                    rep += 4;
                }
                self.start_sub_path = true;
            }
            'T' => {
                if self.svg_comments {
                    self.gcode_string.push_str(&format!("( Command {} {} )\r\n", cmds, if absolute { "absolute" } else { "relative" }));
                }
                let mut rep = 0;
                while rep < fa.len() {
                    obj_count += 1;
                    if self.svg_comments {
                        self.gcode_string.push_str(&format!("( draw curve nr. {} )\r\n", 1 + rep / 2));
                    }
                    let (cx3, cy3);
                    if absolute {
                        cx3 = at(&fa, rep)? + ox;
                        cy3 = at(&fa, rep + 1)? + oy;
                    } else {
                        cx3 = self.last_x + at(&fa, rep)?;
                        cy3 = self.last_y + at(&fa, rep + 1)?;
                    }
                    let qpx1 = (self.cx_mirror - self.last_x) * 2.0 / 3.0 + self.last_x;
                    let qpy1 = (self.cy_mirror - self.last_y) * 2.0 / 3.0 + self.last_y;
                    let qpx2 = (self.cx_mirror - cx3) * 2.0 / 3.0 + cx3;
                    let qpy2 = (self.cy_mirror - cy3) * 2.0 / 3.0 + cy3;
                    let points = [
                        Point::new(self.last_x as f64, self.last_y as f64),
                        Point::new(qpx1 as f64, qpy1 as f64),
                        Point::new(qpx2 as f64, qpy2 as f64),
                        Point::new(cx3 as f64, cy3 as f64),
                    ];
                    self.cx_mirror = cx3;
                    self.cy_mirror = cy3;
                    self.last_x = cx3;
                    self.last_y = cy3;
                    let b = self.get_bezier_approximation(&points, self.svg_bezier_accuracy);
                    self.emit_curve(&b, cx3, cy3, &cmds);
                    rep += 2;
                }
                self.start_sub_path = true;
            }
            _ => {
                if self.svg_comments {
                    self.gcode_string.push_str(&format!("( *********** unknown: {} ***** )\r\n", cmds));
                }
            }
        }
        Ok(obj_count)
    }

    /// Shared L/H/V tail: dot or line to (currentX, currentY). `cmt`: command text.
    fn emit_point(&mut self, cmt: &str) {
        if self.svg_nodes_only {
            self.gcode_dot_only(self.current_x, self.current_y, cmt);
        } else {
            self.gcode_move_to(self.current_x, self.current_y, cmt);
        }
    }

    /// Shared curve tail of C/S/Q/T. `b`: flattened points. `cx3`,`cy3`: end point. `cmt`: command text.
    fn emit_curve(&mut self, b: &[Point], cx3: f32, cy3: f32, cmt: &str) {
        if self.svg_nodes_only {
            self.gcode_dot_only(cx3, cy3, cmt);
        } else {
            for p in b.iter().skip(1) {
                self.gcode_move_to(p.x as f32, p.y as f32, cmt);
            }
        }
    }

    /// `calcArc`: SVG elliptical arc to cubic beziers (vvvv/SVG SvgArcSegment).
    ///
    /// Args: start, radii, x-axis rotation (deg), large-arc flag, sweep flag, end (all `float`).
    #[allow(clippy::too_many_arguments)]
    fn calc_arc(&mut self, start_x: f32, start_y: f32, radius_x: f32, radius_y: f32, angle: f32, size: f32, sweep: f32, end_x: f32, end_y: f32) {
        if radius_x == 0.0 && radius_y == 0.0 {
            return;
        }
        let pi = std::f64::consts::PI;
        let sin_phi = (angle as f64 * pi / 180.0).sin();
        let cos_phi = (angle as f64 * pi / 180.0).cos();
        let x1dash = cos_phi * (start_x - end_x) as f64 / 2.0 + sin_phi * (start_y - end_y) as f64 / 2.0;
        let y1dash = -sin_phi * (start_x - end_x) as f64 / 2.0 + cos_phi * (start_y - end_y) as f64 / 2.0;
        // float products first (C# float*float), then double
        let rx2 = radius_x * radius_x;
        let ry2 = radius_y * radius_y;
        let full = ((rx2 * radius_y) * radius_y) as f64;
        let numerator = full - (rx2 as f64 * y1dash) * y1dash - (ry2 as f64 * x1dash) * x1dash;
        let mut rx = radius_x;
        let mut ry = radius_y;
        let root = if numerator < 0.0 {
            let s = (1.0 - numerator / full).sqrt() as f32;
            rx *= s;
            ry *= s;
            0.0
        } else {
            let sign = if (size == 1.0 && sweep == 1.0) || (size == 0.0 && sweep == 0.0) { -1.0 } else { 1.0 };
            sign * (numerator / ((rx2 as f64 * y1dash) * y1dash + (ry2 as f64 * x1dash) * x1dash)).sqrt()
        };
        let (rxd, ryd) = (rx as f64, ry as f64);
        let cxdash = root * rxd * y1dash / ryd;
        let cydash = -root * ryd * x1dash / rxd;
        let cx = cos_phi * cxdash - sin_phi * cydash + (start_x + end_x) as f64 / 2.0;
        let cy = sin_phi * cxdash + cos_phi * cydash + (start_y + end_y) as f64 / 2.0;
        let mut theta1 = calculate_vector_angle(1.0, 0.0, (x1dash - cxdash) / rxd, (y1dash - cydash) / ryd);
        let mut dtheta = calculate_vector_angle(
            (x1dash - cxdash) / rxd,
            (y1dash - cydash) / ryd,
            (-x1dash - cxdash) / rxd,
            (-y1dash - cydash) / ryd,
        );
        if sweep == 0.0 && dtheta > 0.0 {
            dtheta -= 2.0 * pi;
        } else if sweep == 1.0 && dtheta < 0.0 {
            dtheta += 2.0 * pi;
        }
        let segments = f64_to_i32((dtheta / (pi / 2.0)).abs().ceil());
        let delta = dtheta / segments as f64;
        let t = 8.0 / 3.0 * (delta / 4.0).sin() * (delta / 4.0).sin() / (delta / 2.0).sin();

        let mut sx = start_x as f64;
        let mut sy = start_y as f64;
        for _ in 0..segments.max(0) {
            let cos_t1 = theta1.cos();
            let sin_t1 = theta1.sin();
            let theta2 = theta1 + delta;
            let cos_t2 = theta2.cos();
            let sin_t2 = theta2.sin();
            let endpoint_x = cos_phi * rxd * cos_t2 - sin_phi * ryd * sin_t2 + cx;
            let endpoint_y = sin_phi * rxd * cos_t2 + cos_phi * ryd * sin_t2 + cy;
            let dx1 = t * (-cos_phi * rxd * sin_t1 - sin_phi * ryd * cos_t1);
            let dy1 = t * (-sin_phi * rxd * sin_t1 + cos_phi * ryd * cos_t1);
            let dxe = t * (cos_phi * rxd * sin_t2 + sin_phi * ryd * cos_t2);
            let dye = t * (sin_phi * rxd * sin_t2 - cos_phi * ryd * cos_t2);
            let points = [
                Point::new(sx, sy),
                Point::new(sx + dx1, sy + dy1),
                Point::new(endpoint_x + dxe, endpoint_y + dye),
                Point::new(endpoint_x, endpoint_y),
            ];
            let b = self.get_bezier_approximation(&points, self.svg_bezier_accuracy);
            for p in b.iter().skip(1) {
                self.gcode_move_to_pt(*p, "arc");
            }
            theta1 = theta2;
            sx = (endpoint_x as f32) as f64;
            sy = (endpoint_y as f32) as f64;
        }
    }

    /// `GetBezierApproximation`. `cp`: 4 control points. `n`: segments for the legacy mode. Returns polyline.
    fn get_bezier_approximation(&self, cp: &[Point; 4], n: i32) -> Vec<Point> {
        if self.use_legacy_bezier {
            self.get_bezier_approximation_old(cp, n)
        } else {
            flatten_to(cp, 0.2 / self.scaled_error, 20)
        }
    }

    /// `GetBezierApproximationOld`. `cp`: control points. `n`: output segment count. Returns n+1 points.
    fn get_bezier_approximation_old(&self, cp: &[Point], n: i32) -> Vec<Point> {
        bezier_approximation_old(cp, n)
    }

    /// `translateXY(Point)`. `p`: input. Returns point through the element matrix.
    fn translate_xy(&self, p: Point) -> Point {
        self.matrix_element.transform(p)
    }

    /// `translateIJ(float, float)`: rotate/scale only, no offset. `i`,`j`: arc center offset. Returns mapped offset.
    fn translate_ij(&self, i: f32, j: f32) -> Point {
        let (ti, tj) = (i as f64, j as f64);
        let m = &self.matrix_element;
        Point::new(ti * m.m11() + tj * m.m21(), ti * m.m12() + tj * m.m22())
    }

    /// `gcodeDotOnly`. `x`,`y`: point. `cmt`: comment.
    fn gcode_dot_only(&mut self, x: f32, y: f32, cmt: &str) {
        self.gcode_start_path(x, y, cmt);
        self.gcode_pen_down(cmt);
        self.gcode_pen_up(cmt);
    }

    /// `gcodeStartPath`: rapid to start, pen up. `x`,`y`: point. `cmt`: comment.
    fn gcode_start_path(&mut self, x: f32, y: f32, cmt: &str) {
        let coord = self.translate_xy(Point::new(x as f64, y as f64));
        self.last_gcx = coord.x;
        self.last_gcy = coord.y;
        self.last_set_gcx = coord.x;
        self.last_set_gcy = coord.y;
        self.gcode_pen_up(cmt);
        self.gcode.move_to_rapid(&mut self.gcode_string, coord, cmt);
        self.pen_is_down = false;
        self.is_reduce_ok = false;
        // new polyline for fill
        if let Some(rec) = self.fill_rec.as_mut() {
            rec.push(vec![coord]);
        }
    }

    /// Emit hatch segments as burn lines (pen up, rapid, pen down, G1). `segs`: (start, end) in output coords.
    fn emit_fill(&mut self, segs: &[(Point, Point)]) {
        for &(a, b) in segs {
            self.gcode_pen_up("");
            self.gcode.move_to_rapid(&mut self.gcode_string, a, "");
            self.gcode_pen_down("");
            self.gcode.move_to(&mut self.gcode_string, b, "");
            self.last_gcx = b.x;
            self.last_gcy = b.y;
            self.last_set_gcx = b.x;
            self.last_set_gcy = b.y;
        }
        self.gcode_pen_up("");
        self.is_reduce_ok = false;
    }

    /// `gcodeStopPath`. `cmt`: comment.
    fn gcode_stop_path(&mut self, cmt: &str) {
        if self.gcode_reduce && (self.last_set_gcx != self.last_gcx || self.last_set_gcy != self.last_gcy) {
            // restore last skipped point
            self.gcode.move_to(&mut self.gcode_string, Point::new(self.last_gcx, self.last_gcy), "restore Point");
        }
        self.gcode_pen_up(cmt);
    }

    /// `gcodeMoveTo(float, float, string)`. `x`,`y`: point. `cmt`: comment.
    fn gcode_move_to(&mut self, x: f32, y: f32, cmt: &str) {
        self.gcode_move_to_pt(Point::new(x as f64, y as f64), cmt);
    }

    /// `gcodeMoveTo(Point, string)`: G1 with optional point reduction. `orig`: point. `cmt`: comment.
    fn gcode_move_to_pt(&mut self, orig: Point, cmt: &str) {
        let coord = self.translate_xy(orig);
        self.reject_point = false;
        self.gcode_pen_down(cmt);
        if self.gcode_reduce && self.is_reduce_ok {
            self.distance = ((coord.x - self.last_set_gcx) * (coord.x - self.last_set_gcx) + (coord.y - self.last_set_gcy) * (coord.y - self.last_set_gcy)).sqrt();
            if self.distance < self.gcode_reduce_val as f64 {
                self.reject_point = true;
            } else {
                self.last_set_gcx = coord.x;
                self.last_set_gcy = coord.y;
            }
        }
        if !self.gcode_reduce || !self.reject_point {
            self.gcode.move_to(&mut self.gcode_string, coord, cmt);
        }
        self.last_gcx = coord.x;
        self.last_gcy = coord.y;
        // record geometry even when reduction dropped the point
        if let Some(rec) = self.fill_rec.as_mut() {
            match rec.last_mut() {
                Some(poly) => poly.push(coord),
                None => rec.push(vec![coord]),
            }
        }
    }

    /// `gcodeArcToCCW`: G3 arc. `x`,`y`: end. `i`,`j`: center offset. `cmt`: comment. `avoid_g23`: split arc.
    fn gcode_arc_to_ccw(&mut self, x: f32, y: f32, i: f32, j: f32, cmt: &str, avoid_g23: bool) {
        let xy = self.translate_xy(Point::new(x as f64, y as f64));
        let ij = self.translate_ij(i, j);
        self.gcode_pen_down(cmt);
        if self.gcode_reduce && self.is_reduce_ok && (self.last_set_gcx != self.last_gcx || self.last_set_gcy != self.last_gcy) {
            self.gcode.move_to(&mut self.gcode_string, Point::new(self.last_gcx, self.last_gcy), cmt);
        }
        self.gcode.arc(&mut self.gcode_string, 3, xy, ij, cmt, avoid_g23);
        // record flattened arc from last recorded point
        let (lx, ly) = (self.last_gcx, self.last_gcy);
        if let Some(rec) = self.fill_rec.as_mut() {
            if rec.is_empty() {
                rec.push(vec![Point::new(lx, ly)]);
            }
            let poly = rec.last_mut().expect("poly");
            let last = *poly.last().expect("pt");
            poly.extend(flatten_arc(last, xy, Point::new(last.x + ij.x, last.y + ij.y), true));
        }
    }

    /// `gcodePenUp`. `cmt`: comment.
    fn gcode_pen_up(&mut self, cmt: &str) {
        if self.pen_is_down {
            self.gcode.pen_up(&mut self.gcode_string, cmt);
        }
        self.pen_is_down = false;
    }

    /// `gcodePenDown`. `cmt`: comment.
    fn gcode_pen_down(&mut self, cmt: &str) {
        if !self.pen_is_down {
            self.gcode.pen_down(&mut self.gcode_string, cmt);
        }
        self.pen_is_down = true;
    }

    /// Unused flags kept for 1:1 field parity (pause options, reduce value).
    #[allow(dead_code)]
    fn unused_flags(&self) -> (bool, bool, f32) {
        (self.svg_pause_element, self.svg_pause_pen_down, self.gcode_reduce_val)
    }
}

/// Closed outlines of recorded polylines. `rec`: polylines. Returns polygons.
///
/// Closed = some vertex at index >= 2 equals vertex 0 within 1e-3 (covers close-path extension past the start).
fn closed_polys(rec: Vec<Vec<Point>>) -> Vec<Vec<Point>> {
    rec.into_iter()
        .filter_map(|mut pts| {
            let p0 = *pts.first()?;
            let i = (2..pts.len()).find(|&i| (pts[i].x - p0.x).abs() < 1e-3 && (pts[i].y - p0.y).abs() < 1e-3)?;
            pts.truncate(i + 1);
            Some(pts)
        })
        .collect()
}

/// `GetBezierApproximationOld` as a free function. `cp`: control points. `n`: output segment count. Returns n+1 points.
pub fn bezier_approximation_old(cp: &[Point], n: i32) -> Vec<Point> {
    (0..=n).map(|i| get_bezier_point(i as f64 / n as f64, cp, 0, cp.len())).collect()
}

/// `CalculateVectorAngle`. `ux`,`uy`,`vx`,`vy`: vectors. Returns angle u->v in [0, 2pi).
fn calculate_vector_angle(ux: f64, uy: f64, vx: f64, vy: f64) -> f64 {
    let ta = uy.atan2(ux);
    let tb = vy.atan2(vx);
    if tb >= ta {
        return tb - ta;
    }
    std::f64::consts::PI * 2.0 - (ta - tb)
}

/// `GetBezierPoint`: recursive De Casteljau. `t`: parameter. `cp`: control points. `index`,`count`: sub-range.
fn get_bezier_point(t: f64, cp: &[Point], index: usize, count: usize) -> Point {
    if count == 1 {
        return cp[index];
    }
    let p0 = get_bezier_point(t, cp, index, count - 1);
    let p1 = get_bezier_point(t, cp, index + 1, count - 1);
    let x = (1.0 - t) * p0.x + t * p1.x;
    Point::new(x, (1.0 - t) * p0.y + t * p1.y)
}

/// `parseStyle`: `style="a:b;c:d"` to a dictionary.
///
/// `el`: element. Returns None without style attribute; Err(ArgumentException) on duplicate keys.
fn parse_style(el: Node) -> Result<StyleMap, SvgError> {
    let Some(style) = attr(el, "style") else {
        return Ok(None);
    };
    let mut map: Vec<(String, String)> = Vec::new();
    let s = style.replace(' ', "");
    for part in s.split(';').filter(|p| !p.is_empty()) {
        let kv: Vec<&str> = part.split(':').collect();
        if kv.len() == 2 && kv[1] != "none" {
            if map.iter().any(|e| e.0 == kv[0]) {
                return Err(SvgError::new("System.ArgumentException", "An item with the same key has already been added."));
            }
            map.push((kv[0].to_string(), kv[1].to_string()));
        }
    }
    Ok(Some(map))
}

/// `parseColorAttributes`: `stroke` / `fill` attributes that are not `none`. `el`: element.
fn parse_color_attributes(el: Node) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for key in [SVG_STROKE_ATTRIBUTE, SVG_FILL_ATTRIBUTE] {
        if let Some(v) = attr(el, key)
            && v != "none"
        {
            result.push((key.to_string(), v.to_string()));
        }
    }
    result
}

/// `File.ReadAllText` decoding: UTF-8 (default), UTF-8/16/32 BOMs.
///
/// `b`: file bytes. Returns text (invalid sequences become U+FFFD).
fn decode_read_all_text(b: &[u8]) -> String {
    if b.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&b[3..]).into_owned();
    }
    if b.starts_with(&[0xFF, 0xFE, 0, 0]) {
        let v: Vec<u32> = b[4..].as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
        return v.into_iter().map(|u| char::from_u32(u).unwrap_or('\u{FFFD}')).collect();
    }
    if b.starts_with(&[0xFF, 0xFE]) {
        let v: Vec<u16> = b[2..].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
        return String::from_utf16_lossy(&v);
    }
    if b.starts_with(&[0xFE, 0xFF]) {
        let v: Vec<u16> = b[2..].as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect();
        return String::from_utf16_lossy(&v);
    }
    String::from_utf8_lossy(b).into_owned()
}
