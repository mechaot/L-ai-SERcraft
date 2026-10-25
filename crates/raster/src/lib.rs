// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `lg-raster`: port of `RasterConverter/ImageTransform.cs` and `RasterConverter/Dithering/*`.

pub mod color_matrix;
pub mod dithering;
pub mod image_transform;
pub mod resize;

pub use color_matrix::{ColorMatrix, ColorMatrixMode};
pub use image_transform::*;
