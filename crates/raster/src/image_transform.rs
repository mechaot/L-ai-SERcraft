//! Port of `RasterConverter/ImageTransform.cs` (pixel ops on straight-alpha `RgbaImage`).

use crate::color_matrix::{ColorMatrix, ColorMatrixMode, apply_color_matrix};
use crate::dithering::image_utilities::{get_pixels_from_32bit_argb_image, to_bitmap};
use crate::dithering::random::{RandomDithering, tick_count};
use crate::dithering::{ArgbColor, ErrorDiffusion, ErrorDiffusionDithering};
use image::{Rgba, RgbaImage};

pub use crate::resize::{InterpolationMode, resize_image};

/// C# `ImageTransform.Formula`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formula {
    SimpleAverage = 0,
    WeightAverage = 1,
    OpticalCorrect = 2,
    Custom = 3,
}

/// C# `ImageTransform.DitheringMode` (same order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DitheringMode {
    Atkinson,
    FloydSteinberg,
    Burks,
    Jarvis,
    Random,
    Sierra2,
    Sierra3,
    SierraLight,
    Stucki,
}

/// `System.Drawing.RotateFlipType` (the non-aliased values).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotateFlipType {
    RotateNoneFlipNone = 0,
    Rotate90FlipNone = 1,
    Rotate180FlipNone = 2,
    Rotate270FlipNone = 3,
    RotateNoneFlipX = 4,
    Rotate90FlipX = 5,
    Rotate180FlipX = 6,
    Rotate270FlipX = 7,
}

#[allow(non_upper_case_globals)]
impl RotateFlipType {
    /// .NET alias: `RotateNoneFlipY == Rotate180FlipX` (vertical flip).
    pub const RotateNoneFlipY: Self = Self::Rotate180FlipX;
    /// .NET alias: `Rotate180FlipXY == RotateNoneFlipNone`.
    pub const Rotate180FlipXY: Self = Self::RotateNoneFlipNone;
    /// .NET alias: `Rotate270FlipXY == Rotate90FlipNone`.
    pub const Rotate270FlipXY: Self = Self::Rotate90FlipNone;
    /// .NET alias: `RotateNoneFlipXY == Rotate180FlipNone`.
    pub const RotateNoneFlipXY: Self = Self::Rotate180FlipNone;
    /// .NET alias: `Rotate90FlipXY == Rotate270FlipNone`.
    pub const Rotate90FlipXY: Self = Self::Rotate270FlipNone;
    /// .NET alias: `Rotate180FlipY == RotateNoneFlipX`.
    pub const Rotate180FlipY: Self = Self::RotateNoneFlipX;
    /// .NET alias: `Rotate270FlipY == Rotate90FlipX`.
    pub const Rotate270FlipY: Self = Self::Rotate90FlipX;
    /// .NET alias: `Rotate90FlipY == Rotate270FlipX`.
    pub const Rotate90FlipY: Self = Self::Rotate270FlipX;
}

/// `System.Drawing.Color` (ARGB, straight alpha).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const WHITE: Color = Color { a: 255, r: 255, g: 255, b: 255 };
    pub const BLACK: Color = Color { a: 255, r: 0, g: 0, b: 0 };
    pub const TRANSPARENT: Color = Color { a: 0, r: 255, g: 255, b: 255 };
    pub const LIGHT_PINK: Color = Color { a: 255, r: 255, g: 182, b: 193 };

    /// `Color.FromArgb(a,r,g,b)`. Returns color.
    pub fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { a, r, g, b }
    }
    /// `Color.FromArgb(int)`. Args: `v` 0xAARRGGBB. Returns color.
    pub fn from_argb_i32(v: i32) -> Self {
        let u = v as u32;
        Self { a: (u >> 24) as u8, r: (u >> 16) as u8, g: (u >> 8) as u8, b: u as u8 }
    }
    /// `Color.ToArgb()`. Returns 0xAARRGGBB as int.
    pub fn to_argb(self) -> i32 {
        (((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32) as i32
    }
}

/// `System.Drawing.Point`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Args: coords. Returns point.
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// `System.Drawing.Rectangle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// `Bitmap.Clone(Rectangle, PixelFormat)` crop helper.
/// Args: `img` source; `r` crop rect.
/// Returns `None` if rect empty or outside the image (C# throws OutOfMemory), else the crop.
pub fn clone_rect(img: &RgbaImage, r: Rect) -> Option<RgbaImage> {
    if r.width <= 0 || r.height <= 0 || r.x < 0 || r.y < 0 {
        return None;
    }
    if r.x as i64 + r.width as i64 > img.width() as i64 || r.y as i64 + r.height as i64 > img.height() as i64 {
        return None;
    }
    Some(image::imageops::crop_imm(img, r.x as u32, r.y as u32, r.width as u32, r.height as u32).to_image())
}

/// `Bitmap.RotateFlip(RotateFlipType)` (returns new image instead of in-place).
/// Args: `img` source; `t` rotate/flip type (rotate first, then flip, as GDI+).
/// Returns transformed image.
pub fn rotate_flip(img: &RgbaImage, t: RotateFlipType) -> RgbaImage {
    use image::imageops::{flip_horizontal, rotate180, rotate270, rotate90};
    use RotateFlipType::*;
    match t {
        RotateNoneFlipNone => img.clone(),
        Rotate90FlipNone => rotate90(img),
        Rotate180FlipNone => rotate180(img),
        Rotate270FlipNone => rotate270(img),
        RotateNoneFlipX => flip_horizontal(img),
        Rotate90FlipX => flip_horizontal(&rotate90(img)),
        Rotate180FlipX => flip_horizontal(&rotate180(img)),
        Rotate270FlipX => flip_horizontal(&rotate270(img)),
    }
}

/// `g.Clear(White); g.DrawImage(img, 0, 0)`: composite SourceOver on opaque white.
/// Args: `img` source. Returns opaque image (alpha killed).
pub fn flatten_on_white(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let a = p[3] as u32;
        if a != 255 {
            for c in 0..3 {
                // round(c*a/255 + 255*(255-a)/255)
                p[c] = ((p[c] as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
            }
            p[3] = 255;
        }
    }
    out
}

/// C# `Threshold(Image, float threshold, bool apply)`.
/// Args: `img` source; `threshold` 0..1; `apply` false only flattens alpha onto white.
/// Returns opaque image; each channel is 255 when `> round(threshold*255)` else 0 (libgdiplus rule).
pub fn threshold(img: &RgbaImage, threshold: f32, apply: bool) -> RgbaImage {
    let mut bmp = flatten_on_white(img);
    if apply {
        let cutoff = (threshold as f64 * 255.0).round() as u8;
        for p in bmp.pixels_mut() {
            for c in 0..3 {
                p[c] = if p[c] > cutoff { 255 } else { 0 };
            }
        }
    }
    bmp
}

/// C# private `draw_adjusted_image`.
/// Args: `img` source; `cm` matrix; `mode` rounding mode. Returns adjusted image.
fn draw_adjusted_image(img: &RgbaImage, cm: &ColorMatrix, mode: ColorMatrixMode) -> RgbaImage {
    apply_color_matrix(img, cm, mode)
}

/// C# `InvertingImage`.
/// Args: `img` source. Returns image with RGB inverted (alpha kept; alpha 0 pixels become 0,0,0,0).
pub fn inverting_image(img: &RgbaImage) -> RgbaImage {
    let cm = ColorMatrix([
        [-1.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, -1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, 0.0],
        [1.0, 1.0, 1.0, 0.0, 1.0],
    ]);
    apply_color_matrix(img, &cm, ColorMatrixMode::Gdi)
}

/// C# private `GetDitheringMode`.
/// Args: `mode` dithering mode; `seed` RNG seed for `Random` (None = tick count like C#).
/// Returns boxed ditherer.
fn get_dithering_mode(mode: DitheringMode, seed: Option<i32>) -> Box<dyn ErrorDiffusion> {
    match mode {
        DitheringMode::FloydSteinberg => Box::new(ErrorDiffusionDithering::floyd_steinberg()),
        DitheringMode::Atkinson => Box::new(ErrorDiffusionDithering::atkinson()),
        DitheringMode::Burks => Box::new(ErrorDiffusionDithering::burks()),
        DitheringMode::Jarvis => Box::new(ErrorDiffusionDithering::jarvis_judice_ninke()),
        DitheringMode::Random => Box::new(RandomDithering::new(seed.unwrap_or_else(tick_count))),
        DitheringMode::Sierra2 => Box::new(ErrorDiffusionDithering::sierra2()),
        DitheringMode::Sierra3 => Box::new(ErrorDiffusionDithering::sierra3()),
        DitheringMode::SierraLight => Box::new(ErrorDiffusionDithering::sierra_lite()),
        DitheringMode::Stucki => Box::new(ErrorDiffusionDithering::stucki()),
    }
}

/// C# `DitherImage(Bitmap, DitheringMode)`. `Random` mode seeds from the tick count (non reproducible).
/// Args: `img` source; `dithering` mode. Returns black/white image (alpha kept, except Random: opaque).
pub fn dither_image(img: &RgbaImage, dithering: DitheringMode) -> RgbaImage {
    dither_image_inner(img, dithering, None)
}

/// Like [`dither_image`] but with explicit RNG seed (for `Random`), reproducible.
/// Args: `img` source; `dithering` mode; `seed` `System.Random` seed. Returns dithered image.
pub fn dither_image_with_seed(img: &RgbaImage, dithering: DitheringMode, seed: i32) -> RgbaImage {
    dither_image_inner(img, dithering, Some(seed))
}

/// Shared body of the two public dither entry points.
/// Args: as above, `seed` optional. Returns dithered image.
fn dither_image_inner(img: &RgbaImage, dithering: DitheringMode, seed: Option<i32>) -> RgbaImage {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let mut data = get_pixels_from_32bit_argb_image(img);
    let mut dither = get_dithering_mode(dithering, seed);
    for row in 0..h {
        for col in 0..w {
            let index = (row * w + col) as usize;
            let current = data[index];
            // threshold transform, then diffuse error
            let transformed = transform_pixel(current);
            data[index] = transformed;
            dither.diffuse(&mut data, current, transformed, col, row, w, h);
        }
    }
    to_bitmap(&data, w as u32, h as u32)
}

/// C# private `TransformPixel`.
/// Args: `pixel` input. Returns black or white with same alpha.
fn transform_pixel(pixel: ArgbColor) -> ArgbColor {
    let gray = (0.299 * pixel.r as f64 + 0.587 * pixel.g as f64 + 0.114 * pixel.b as f64) as u8;
    if gray < 128 {
        ArgbColor::new(pixel.a as i32, 0, 0, 0)
    } else {
        ArgbColor::new(pixel.a as i32, 255, 255, 255)
    }
}

/// Build the GrayScale color matrix exactly as C# `GrayScale`.
/// Args: as [`gray_scale`]. Returns the matrix.
pub fn gray_scale_matrix(r: f32, g: f32, b: f32, brightness: f32, contrast: f32, formula: Formula) -> ColorMatrix {
    // https://www.had2know.com/technology/rgb-to-gray-scale-converter.html (OpticalCorrect)
    let (mut rf, mut gf, mut bf) = match formula {
        Formula::SimpleAverage => (0.333f32, 0.333f32, 0.333f32),
        Formula::WeightAverage => (0.333, 0.444, 0.222),
        Formula::OpticalCorrect => (0.299, 0.587, 0.114),
        Formula::Custom => (0.333 * r, 0.333 * g, 0.333 * b),
    };
    rf *= contrast;
    gf *= contrast;
    bf *= contrast;
    ColorMatrix([
        [rf, rf, rf, 0.0, 0.0],
        [gf, gf, gf, 0.0, 0.0],
        [bf, bf, bf, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, 0.0],
        [brightness, brightness, brightness, 0.0, 1.0],
    ])
}

/// C# `GrayScale(Image, R, G, B, brightness, contrast, formula)` with Windows GDI+ rounding.
/// Args: `img` source; `r`,`g`,`b` custom weights (Custom only); `brightness` matrix offset (-1..1);
/// `contrast` factor; `formula` gray formula. Returns gray image.
pub fn gray_scale(img: &RgbaImage, r: f32, g: f32, b: f32, brightness: f32, contrast: f32, formula: Formula) -> RgbaImage {
    gray_scale_mode(img, r, g, b, brightness, contrast, formula, ColorMatrixMode::Gdi)
}

/// [`gray_scale`] with selectable matrix rounding mode (Libgdiplus mode for Mono oracle tests).
/// Args: as [`gray_scale`] plus `mode`. Returns gray image.
#[allow(clippy::too_many_arguments)]
pub fn gray_scale_mode(
    img: &RgbaImage,
    r: f32,
    g: f32,
    b: f32,
    brightness: f32,
    contrast: f32,
    formula: Formula,
    mode: ColorMatrixMode,
) -> RgbaImage {
    let cm = gray_scale_matrix(r, g, b, brightness, contrast, formula);
    draw_adjusted_image(img, &cm, mode)
}

/// C# `Whitenize(Bitmap, int threshold, bool demo)`.
/// Args: `src` source; `threshold` tolerance around white; `demo` paint LightPink instead of transparent.
/// Returns image with near-white pixels substituted.
pub fn whitenize(src: &RgbaImage, threshold: i32, demo: bool) -> RgbaImage {
    let new_color = if demo { Color::LIGHT_PINK } else { Color::TRANSPARENT };
    color_substitution(src, threshold, Color::WHITE, new_color)
}

/// C# private `ColorSubstitution`.
/// Args: `src` source; `th` threshold; `source` color to match; `new` replacement. Returns result.
fn color_substitution(src: &RgbaImage, th: i32, source: Color, new: Color) -> RgbaImage {
    let mut out = src.clone();
    for p in out.pixels_mut() {
        if p[3] == 0 {
            continue;
        }
        let (sr, sg, sb) = (p[0] as i32, p[1] as i32, p[2] as i32);
        let near = |s: i32, f: u8| s < f as i32 + th && s > f as i32 - th;
        if near(sb, source.b) && near(sg, source.g) && near(sr, source.r) {
            // shift by distance to source, clamp
            let nb = (source.b as i32 - sb + new.b as i32).clamp(0, 255);
            let ng = (source.g as i32 - sg + new.g as i32).clamp(0, 255);
            let nr = (source.r as i32 - sr + new.r as i32).clamp(0, 255);
            *p = Rgba([nr as u8, ng as u8, nb as u8, new.a]);
        }
    }
    out
}

/// C# private `Format32bppArgbCopy` (unused upstream). Args: `src`. Returns exact copy.
pub fn format32bpp_argb_copy(src: &RgbaImage) -> RgbaImage {
    src.clone()
}

/// C# nested `DirectBitmap`: ARGB ints, premultiplied-format backing (see `fill`).
#[derive(Clone, Debug)]
pub struct DirectBitmap {
    pub bits: Vec<i32>,
    pub width: i32,
    pub height: i32,
}

impl DirectBitmap {
    /// C# ctor. Args: size. Returns zeroed bitmap.
    pub fn new(width: i32, height: i32) -> Self {
        Self { bits: vec![0; (width * height) as usize], width, height }
    }
    /// Args: `x`,`y` pos; `c` color. Returns nothing. Panics out of range (C# throws).
    pub fn set_pixel(&mut self, x: i32, y: i32, c: Color) {
        self.bits[(x + y * self.width) as usize] = c.to_argb();
    }
    /// Args: `x`,`y` pos. Returns raw color. Panics out of range.
    pub fn get_pixel(&self, x: i32, y: i32) -> Color {
        Color::from_argb_i32(self.bits[(x + y * self.width) as usize])
    }

    /// Build from straight image the way `Graphics.DrawImage` into a PArgb bitmap does (premultiply, rounded).
    /// Args: `img` source. Returns bitmap.
    pub fn from_image(img: &RgbaImage) -> Self {
        let mut d = Self::new(img.width() as i32, img.height() as i32);
        for (i, p) in img.pixels().enumerate() {
            let a = p[3] as u32;
            let m = |c: u8| ((c as u32 * a + 127) / 255) as u8;
            d.bits[i] = Color::from_argb(p[3], m(p[0]), m(p[1]), m(p[2])).to_argb();
        }
        d
    }

    /// Convert PArgb back to straight alpha. Returns image.
    pub fn to_image(&self) -> RgbaImage {
        let mut out = RgbaImage::new(self.width as u32, self.height as u32);
        for (p, &v) in out.pixels_mut().zip(self.bits.iter()) {
            let c = Color::from_argb_i32(v);
            *p = if c.a == 0 {
                Rgba([0, 0, 0, 0])
            } else {
                let u = |x: u8| ((x as u32 * 255 + c.a as u32 / 2) / c.a as u32).min(255) as u8;
                Rgba([u(c.r), u(c.g), u(c.b), c.a])
            };
        }
        out
    }
}

/// C# `Fill`. Operates on the premultiplied ARGB representation like `DirectBitmap`.
/// Args: `img` source; `location` seed pixel; `color` fill color (raw ARGB, opaque expected); `v` tolerance.
/// Returns filled image, or `None` if `location` is outside the image (C# throws).
pub fn fill(img: &RgbaImage, location: Point, color: Color, v: i32) -> Option<RgbaImage> {
    let mut nb = DirectBitmap::from_image(img);
    if location.x < 0 || location.y < 0 || location.x >= nb.width || location.y >= nb.height {
        return None;
    }
    fill4(&mut nb, location, color, v);
    Some(nb.to_image())
}

/// C# `Fill4`: 4-connected stack flood fill.
/// Args: `bmp` target; `pt` seed; `c1` new color; `v` tolerance (strict `<`).
/// Returns touched map `tchd[x][y]`.
pub fn fill4(bmp: &mut DirectBitmap, pt: Point, c1: Color, v: i32) -> Vec<Vec<bool>> {
    let mut tchd = vec![vec![false; bmp.height as usize]; bmp.width as usize];
    let c0 = bmp.get_pixel(pt.x, pt.y);
    let mut stack = vec![pt];
    while let Some(p) = stack.pop() {
        // bounds guard (Rectangle.Contains)
        if p.x < 0 || p.y < 0 || p.x >= bmp.width || p.y >= bmp.height {
            continue;
        }
        tchd[p.x as usize][p.y as usize] = true;
        let cx = bmp.get_pixel(p.x, p.y);
        if fill_color_compare(c0, cx, v) {
            bmp.set_pixel(p.x, p.y, c1);
            for q in [Point::new(p.x, p.y + 1), Point::new(p.x, p.y - 1), Point::new(p.x + 1, p.y), Point::new(p.x - 1, p.y)] {
                if fill_chk_in(&tchd, q) {
                    stack.push(q);
                }
            }
        }
    }
    tchd
}

/// C# `FillColorCompare`: strict per-channel `|c0-cx| < v`.
/// Args: `c0` seed color; `cx` candidate; `v` tolerance. Returns match flag.
fn fill_color_compare(c0: Color, cx: Color, v: i32) -> bool {
    let ok = |a: u8, b: u8| a as i32 > b as i32 - v && (a as i32) < b as i32 + v;
    ok(c0.r, cx.r) && ok(c0.g, cx.g) && ok(c0.b, cx.b)
}

/// C# `FillChkIn`: in range and untouched.
/// Args: `tchd` touched map; `p` point. Returns flag.
fn fill_chk_in(tchd: &[Vec<bool>], p: Point) -> bool {
    if p.x < 0 || p.y < 0 || p.x as usize >= tchd.len() || p.y as usize >= tchd[p.x as usize].len() {
        return false;
    }
    !tchd[p.x as usize][p.y as usize]
}

/// C# `Outliner`.
/// Args: `img` source; `location` seed pixel. Returns outlined image, `None` if seed outside.
pub fn outliner(img: &RgbaImage, location: Point) -> Option<RgbaImage> {
    let mut nb = DirectBitmap::from_image(img);
    if location.x < 0 || location.y < 0 || location.x >= nb.width || location.y >= nb.height {
        return None;
    }
    outline(&mut nb, location);
    Some(nb.to_image())
}

/// C# `Outline`: fill with white (tol 127) then blacken everything untouched.
/// Args: `bmp` target; `pt` seed. Returns nothing.
pub fn outline(bmp: &mut DirectBitmap, pt: Point) {
    let tchd = fill4(bmp, pt, Color::WHITE, 127);
    for y in 0..bmp.height {
        for x in 0..bmp.width {
            if !tchd[x as usize][y as usize] {
                bmp.set_pixel(x, y, Color::BLACK);
            }
        }
    }
}
