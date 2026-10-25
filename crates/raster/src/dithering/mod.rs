// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2015 Cyotek Ltd. (dithering classes)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Cyotek parts: MIT License, see NOTICE.md

//! Port of `RasterConverter/Dithering/*` (Cyotek dithering classes).

pub mod argb_color;
pub mod error_diffusion;
pub mod image_utilities;
pub mod random;

pub use argb_color::ArgbColor;
pub use error_diffusion::{ErrorDiffusion, ErrorDiffusionDithering};
pub use random::{DotNetRandom, RandomDithering};
