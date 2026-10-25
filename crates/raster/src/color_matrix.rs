// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of GDI+ `ColorMatrix` / `ImageAttributes.SetColorMatrix` pixel math (used by ImageTransform.cs).
//!
//! Windows GDI+ computes `out = clamp(in * M)` in float and rounds to byte. libgdiplus
//! (Mono) truncates toward zero and wraps negatives (`(BYTE)int`), which is off-spec for
//! negative results (e.g. negative brightness). Both modes are provided; production uses
//! [`ColorMatrixMode::Gdi`], golden tests against Mono use [`ColorMatrixMode::Libgdiplus`].

use image::{Rgba, RgbaImage};

/// Rounding/clamp behaviour of the 5x5 matrix application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorMatrixMode {
    /// Windows GDI+ semantics: clamp to [0,255], round to nearest.
    #[default]
    Gdi,
    /// libgdiplus 6.1 semantics: truncate to int, >255 -> 255, negative wraps modulo 256.
    Libgdiplus,
}

/// 5x5 matrix, same layout as `System.Drawing.Imaging.ColorMatrix` (row = input, column = output).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorMatrix(pub [[f32; 5]; 5]);

impl ColorMatrix {
    /// Identity matrix. Returns matrix.
    pub fn identity() -> Self {
        let mut m = [[0f32; 5]; 5];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Self(m)
    }
}

/// Apply matrix to one straight-alpha pixel.
/// Args: `p` RGBA pixel; `cm` matrix; `mode` rounding mode. Returns new pixel.
fn apply_pixel(p: [u8; 4], cm: &ColorMatrix, mode: ColorMatrixMode) -> [u8; 4] {
    let m = &cm.0;
    let (r, g, b, a) = (p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32);
    let mut out = [0u8; 4];
    // output channel j (0=R,1=G,2=B,3=A) = sum over inputs + 255*row4
    for (o, j) in out.iter_mut().zip([0usize, 1, 2, 3]) {
        let v: f32 = r * m[0][j] + g * m[1][j] + b * m[2][j] + a * m[3][j] + (255.0 * m[4][j]);
        *o = match mode {
            ColorMatrixMode::Gdi => v.clamp(0.0, 255.0).round() as u8,
            ColorMatrixMode::Libgdiplus => {
                let i = v as i32;
                if i > 0xff { 0xff } else { i as u8 }
            }
        };
    }
    out
}

/// `g.Clear(Transparent); g.DrawImage(img, ..., imageAttributes)` with SourceOver.
/// Args: `img` source; `cm` matrix; `mode` rounding mode.
/// Returns new image; fully transparent results collapse to (0,0,0,0) like compositing onto empty dest.
pub fn apply_color_matrix(img: &RgbaImage, cm: &ColorMatrix, mode: ColorMatrixMode) -> RgbaImage {
    let mut out = RgbaImage::new(img.width(), img.height());
    for (d, s) in out.pixels_mut().zip(img.pixels()) {
        let q = apply_pixel(s.0, cm, mode);
        // SourceOver onto transparent dest: alpha 0 leaves dest empty, else color copied
        *d = if q[3] == 0 { Rgba([0, 0, 0, 0]) } else { Rgba(q) };
    }
    out
}
