// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2015 Cyotek Ltd. (dithering classes)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Cyotek parts: MIT License, see NOTICE.md

//! Port of `RasterConverter/Dithering/ArgbColor.cs` (Cyotek, MIT).

/// 32-bit ARGB color; field order mirrors the C# explicit layout (B,G,R,A in memory).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArgbColor {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl ArgbColor {
    /// C# ctor `ArgbColor(int alpha, int red, int green, int blue)`.
    /// Args: channel values; `(byte)` cast wraps like unchecked C#.
    /// Returns the color.
    pub fn new(alpha: i32, red: i32, green: i32, blue: i32) -> Self {
        Self { a: alpha as u8, r: red as u8, g: green as u8, b: blue as u8 }
    }

    /// C# `FromArgb(byte a, byte r, byte g, byte b)`.
    /// Args: channel bytes. Returns the color.
    pub fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { a, r, g, b }
    }
}
