// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2015 Cyotek Ltd. (dithering classes)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Cyotek parts: MIT License, see NOTICE.md

//! Port of `IErrorDiffusion.cs`, `ErrorDiffusionDithering.cs` and the matrix based
//! dithering classes (Atkinson, Burkes, FloydSteinberg, JarvisJudiceNinke, Sierra2,
//! Sierra3, SierraLite, Stucki).

use super::argb_color::ArgbColor;

/// C# `IErrorDiffusion`.
pub trait ErrorDiffusion {
    /// Spread error of one pixel into `data`.
    /// Args: `data` pixel buffer (row major); `original` pixel before transform;
    /// `transformed` pixel after transform; `x`,`y` pixel position; `width`,`height` image size.
    /// Returns nothing, mutates `data`.
    #[allow(clippy::too_many_arguments)]
    fn diffuse(
        &mut self,
        data: &mut [ArgbColor],
        original: ArgbColor,
        transformed: ArgbColor,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    );
}

/// Matrix based error diffusion (C# `ErrorDiffusionDithering`).
#[derive(Clone, Debug)]
pub struct ErrorDiffusionDithering {
    matrix: Vec<Vec<u8>>,
    divisor: u8,
    matrix_width: u8,
    matrix_height: u8,
    starting_offset: u8,
    use_shifting: bool,
}

impl ErrorDiffusionDithering {
    /// C# protected ctor.
    /// Args: `matrix` coefficient rows; `divisor` shift count or divisor; `use_shifting` pick `>>` over `/`.
    /// Returns the ditherer.
    pub fn new(matrix: &[&[u8]], divisor: u8, use_shifting: bool) -> Self {
        let matrix_height = matrix.len() as u8;
        let matrix_width = matrix[0].len() as u8;
        let mut starting_offset = 0u8;
        // first non-zero coef in row 0 sets the offset (byte wrap kept)
        for (i, &c) in matrix[0].iter().enumerate() {
            if c != 0 {
                starting_offset = (i as i32 - 1) as u8;
                break;
            }
        }
        Self {
            matrix: matrix.iter().map(|r| r.to_vec()).collect(),
            divisor,
            matrix_width,
            matrix_height,
            starting_offset,
            use_shifting,
        }
    }

    /// C# `AtkinsonDithering`. Returns ditherer.
    pub fn atkinson() -> Self {
        Self::new(&[&[0, 0, 1, 1], &[1, 1, 1, 0], &[0, 1, 0, 0]], 3, true)
    }
    /// C# `BurksDithering`. Returns ditherer.
    pub fn burks() -> Self {
        Self::new(&[&[0, 0, 0, 8, 4], &[2, 4, 8, 4, 2]], 5, true)
    }
    /// C# `FloydSteinbergDithering`. Returns ditherer.
    pub fn floyd_steinberg() -> Self {
        Self::new(&[&[0, 0, 7], &[3, 5, 1]], 4, true)
    }
    /// C# `JarvisJudiceNinkeDithering`. Returns ditherer.
    pub fn jarvis_judice_ninke() -> Self {
        Self::new(&[&[0, 0, 0, 7, 5], &[3, 5, 7, 5, 3], &[1, 3, 5, 3, 1]], 48, false)
    }
    /// C# `Sierra2Dithering`. Returns ditherer.
    pub fn sierra2() -> Self {
        Self::new(&[&[0, 0, 0, 4, 3], &[1, 2, 3, 2, 1]], 4, true)
    }
    /// C# `Sierra3Dithering`. Returns ditherer.
    pub fn sierra3() -> Self {
        Self::new(&[&[0, 0, 0, 5, 3], &[2, 4, 5, 4, 2], &[0, 2, 3, 2, 0]], 5, true)
    }
    /// C# `SierraLiteDithering`. Returns ditherer.
    pub fn sierra_lite() -> Self {
        Self::new(&[&[0, 0, 2], &[1, 1, 0]], 2, true)
    }
    /// C# `StuckiDithering`. Returns ditherer.
    pub fn stucki() -> Self {
        Self::new(&[&[0, 0, 0, 8, 4], &[2, 4, 8, 4, 2], &[1, 2, 4, 2, 1]], 42, false)
    }
}

/// C# `ToByte(int)`: clamp to 0..=255.
/// Args: `value` any int. Returns clamped byte.
pub fn to_byte(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

impl ErrorDiffusion for ErrorDiffusionDithering {
    fn diffuse(
        &mut self,
        data: &mut [ArgbColor],
        original: ArgbColor,
        transformed: ArgbColor,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        // NOTE(upstream): blue/green error names are swapped, kept as is
        let red_error = original.r as i32 - transformed.r as i32;
        let blue_error = original.g as i32 - transformed.g as i32;
        let green_error = original.b as i32 - transformed.b as i32;

        for row in 0..self.matrix_height as i32 {
            let offset_y = y + row;
            for col in 0..self.matrix_width as i32 {
                let coefficient = self.matrix[row as usize][col as usize] as i32;
                let offset_x = x + (col - self.starting_offset as i32);
                // NOTE(upstream): `> 0` skips column 0 and row 0 targets
                if coefficient != 0 && offset_x > 0 && offset_x < width && offset_y > 0 && offset_y < height {
                    let idx = (offset_y * width + offset_x) as usize;
                    let px = data[idx];
                    let (nr, ng, nb) = if self.use_shifting {
                        (
                            (red_error * coefficient) >> self.divisor,
                            (green_error * coefficient) >> self.divisor,
                            (blue_error * coefficient) >> self.divisor,
                        )
                    } else {
                        (
                            (red_error * coefficient) / self.divisor as i32,
                            (green_error * coefficient) / self.divisor as i32,
                            (blue_error * coefficient) / self.divisor as i32,
                        )
                    };
                    data[idx] = ArgbColor::from_argb(
                        px.a,
                        to_byte(px.r as i32 + nr),
                        to_byte(px.g as i32 + ng),
                        to_byte(px.b as i32 + nb),
                    );
                }
            }
        }
    }
}
