// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `lg-potrace`: port of `CsPotrace/*` (Potrace tracer, exports, bezier-to-biarc, Clipper fill).

pub mod biarc;
mod biarc_free;
pub mod clipper;
pub mod export;
pub mod export_gcode;
pub mod gdi_path;
pub mod trace;

pub use biarc_free::Hilbert;
pub use clipper::{Direction, FillConf, build_filling};
pub use export::export_to_gdiplus;
pub use export_gcode::export_to_gcode;
pub use gdi_path::GraphicsPath;
pub use trace::{Curve, CurveKind, DPoint, PotraceParams, Tracer, TurnPolicy, potrace_trace};
