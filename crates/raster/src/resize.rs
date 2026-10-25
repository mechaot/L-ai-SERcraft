// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `ImageTransform.ResizeImage` (GDI+ `Graphics.DrawImage` scaling).
//!
//! APPROXIMATION: GDI+ resampling is not bit-reproducible. This is a separable
//! resampler on premultiplied float RGBA with mirrored edges (`WrapMode.TileFlipXY`),
//! half-pixel centers (`PixelOffsetMode.HighQuality`):
//! * NearestNeighbor: point sample.
//! * Default/Low/Bilinear: triangle kernel, no prefilter (aliases when shrinking, like GDI+).
//! * Bicubic: Catmull-Rom (a=-0.5) kernel, no prefilter.
//! * HighQualityBilinear: triangle kernel widened by the scale factor (area prefilter) when shrinking.
//! * HighQualityBicubic/High: a=-0.5 cubic widened by the scale factor when shrinking.

use image::{Rgba, RgbaImage};

/// `System.Drawing.Drawing2D.InterpolationMode` (same names and values).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum InterpolationMode {
    Invalid = -1,
    #[default]
    Default = 0,
    Low = 1,
    High = 2,
    Bilinear = 3,
    Bicubic = 4,
    NearestNeighbor = 5,
    HighQualityBilinear = 6,
    HighQualityBicubic = 7,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Nearest,
    Triangle,
    Cubic,
}

/// Cubic convolution kernel, a = -0.5.
/// Args: `x` distance. Returns weight.
fn cubic(x: f32) -> f32 {
    let a = -0.5f32;
    let x = x.abs();
    if x < 1.0 {
        (a + 2.0) * x * x * x - (a + 3.0) * x * x + 1.0
    } else if x < 2.0 {
        a * x * x * x - 5.0 * a * x * x + 8.0 * a * x - 4.0 * a
    } else {
        0.0
    }
}

/// Mirror index into `0..n` (TileFlipXY).
/// Args: `i` any index; `n` length. Returns mirrored index.
fn reflect(i: i64, n: i64) -> usize {
    let p = i.rem_euclid(2 * n);
    (if p >= n { 2 * n - 1 - p } else { p }) as usize
}

/// One resampling pass along an axis.
/// Args: `src` premult RGBA floats; `sw`,`sh` source size; `dst_len` new length on axis;
/// `horizontal` axis; `kind` kernel; `prefilter` widen kernel when shrinking.
/// Returns resampled buffer.
fn pass(src: &[f32], sw: usize, sh: usize, dst_len: usize, horizontal: bool, kind: Kind, prefilter: bool) -> Vec<f32> {
    let src_len = if horizontal { sw } else { sh };
    let scale = src_len as f32 / dst_len as f32;
    let fs = if prefilter && scale > 1.0 { scale } else { 1.0 };
    let radius = match kind {
        Kind::Nearest => 0.5,
        Kind::Triangle => 1.0,
        Kind::Cubic => 2.0,
    } * fs;
    // per destination index: list of (src index, weight)
    let mut taps: Vec<Vec<(usize, f32)>> = Vec::with_capacity(dst_len);
    for d in 0..dst_len {
        let center = (d as f32 + 0.5) * scale - 0.5;
        let mut v = Vec::new();
        if kind == Kind::Nearest {
            let s = (((d as f32 + 0.5) * scale).floor() as i64).clamp(0, src_len as i64 - 1);
            v.push((s as usize, 1.0));
        } else {
            let lo = (center - radius).floor() as i64;
            let hi = (center + radius).ceil() as i64;
            let mut sum = 0.0;
            for i in lo..=hi {
                let dist = (i as f32 - center) / fs;
                let w = if kind == Kind::Triangle { (1.0 - dist.abs()).max(0.0) } else { cubic(dist) };
                if w != 0.0 {
                    v.push((reflect(i, src_len as i64), w));
                    sum += w;
                }
            }
            for t in &mut v {
                t.1 /= sum;
            }
        }
        taps.push(v);
    }
    let (ow, oh) = if horizontal { (dst_len, sh) } else { (sw, dst_len) };
    let mut out = vec![0f32; ow * oh * 4];
    for y in 0..oh {
        for x in 0..ow {
            let t = &taps[if horizontal { x } else { y }];
            let mut acc = [0f32; 4];
            for &(s, w) in t {
                let (sx, sy) = if horizontal { (s, y) } else { (x, s) };
                let i = (sy * sw + sx) * 4;
                for c in 0..4 {
                    acc[c] += src[i + c] * w;
                }
            }
            let o = (y * ow + x) * 4;
            out[o..o + 4].copy_from_slice(&acc);
        }
    }
    out
}

/// C# `ImageTransform.ResizeImage(Image, Size, bool killalfa, InterpolationMode)`.
/// Args: `image` source; `width`,`height` target size; `killalfa` composite over white
/// (else straight alpha kept); `interpolation` mode.
/// Returns resized image (a clone when the size is unchanged).
pub fn resize_image(image: &RgbaImage, width: u32, height: u32, killalfa: bool, interpolation: InterpolationMode) -> RgbaImage {
    // same size short circuit
    if image.width() == width && image.height() == height {
        return image.clone();
    }
    if width == 0 || height == 0 || image.width() == 0 || image.height() == 0 {
        return RgbaImage::new(width, height);
    }
    let (kind, prefilter) = match interpolation {
        InterpolationMode::NearestNeighbor => (Kind::Nearest, false),
        InterpolationMode::Bicubic => (Kind::Cubic, false),
        InterpolationMode::HighQualityBilinear => (Kind::Triangle, true),
        InterpolationMode::HighQualityBicubic | InterpolationMode::High => (Kind::Cubic, true),
        _ => (Kind::Triangle, false),
    };
    let (sw, sh) = (image.width() as usize, image.height() as usize);
    // premultiply
    let mut buf = Vec::with_capacity(sw * sh * 4);
    for p in image.pixels() {
        let a = p[3] as f32 / 255.0;
        buf.extend_from_slice(&[p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a, p[3] as f32]);
    }
    let h = pass(&buf, sw, sh, width as usize, true, kind, prefilter);
    let v = pass(&h, width as usize, sh, height as usize, false, kind, prefilter);
    let mut out = RgbaImage::new(width, height);
    #[allow(clippy::chunks_exact_to_as_chunks)]
    for (d, s) in out.pixels_mut().zip(v.chunks_exact(4)) {
        let a = s[3].clamp(0.0, 255.0);
        if killalfa {
            // SourceOver on white: premult color + white * (1 - a)
            let k = 255.0 * (1.0 - a / 255.0);
            *d = Rgba([
                (s[0] + k).clamp(0.0, 255.0).round() as u8,
                (s[1] + k).clamp(0.0, 255.0).round() as u8,
                (s[2] + k).clamp(0.0, 255.0).round() as u8,
                255,
            ]);
        } else if a <= 0.0 {
            *d = Rgba([0, 0, 0, 0]);
        } else {
            let f = 255.0 / a;
            *d = Rgba([
                (s[0] * f).clamp(0.0, 255.0).round() as u8,
                (s[1] * f).clamp(0.0, 255.0).round() as u8,
                (s[2] * f).clamp(0.0, 255.0).round() as u8,
                a.round() as u8,
            ]);
        }
    }
    out
}
