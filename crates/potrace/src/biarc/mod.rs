// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) László Domoszlai (Bezier2Biarc)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `CsPotrace/BezierToBiarc/*.cs` (bezier to biarc approximation, float precision like C#).

pub mod algorithm;
pub mod arc;
pub mod complex;
pub mod cubic_bezier;
pub mod line;
pub mod vector2;

pub use algorithm::approx_cubic_bezier;
pub use arc::{Arc, BiArc};
pub use complex::Complex;
pub use cubic_bezier::CubicBezier;
pub use line::Line;
pub use vector2::Vector2;
