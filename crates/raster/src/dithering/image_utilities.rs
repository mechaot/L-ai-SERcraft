// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2015 Cyotek Ltd. (dithering classes)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Cyotek parts: MIT License, see NOTICE.md

//! Port of `RasterConverter/Dithering/ImageUtilities.cs`.

use super::argb_color::ArgbColor;
use image::{Rgba, RgbaImage};

/// C# `Copy(Image)`: copy into fresh transparent 32bpp ARGB bitmap (straight alpha, exact).
/// Args: `image` source. Returns copy.
pub fn copy(image: &RgbaImage) -> RgbaImage {
    image.clone()
}

/// C# `ToBitmap(ArgbColor[], Size)`.
/// Args: `data` row-major pixels; `width`,`height` size. Returns image.
pub fn to_bitmap(data: &[ArgbColor], width: u32, height: u32) -> RgbaImage {
    let mut out = RgbaImage::new(width, height);
    for (i, p) in out.pixels_mut().enumerate() {
        let c = data[i];
        *p = Rgba([c.r, c.g, c.b, c.a]);
    }
    out
}

/// C# `GetPixelsFrom32BitArgbImage`.
/// Args: `bitmap` source. Returns row-major pixel vec.
pub fn get_pixels_from_32bit_argb_image(bitmap: &RgbaImage) -> Vec<ArgbColor> {
    bitmap.pixels().map(|p| ArgbColor::from_argb(p[3], p[0], p[1], p[2])).collect()
}
