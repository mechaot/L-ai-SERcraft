// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2010-2017 Angus Johnson (Clipper)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Clipper parts: Boost Software License 1.0, see NOTICE.md

//! Port of `Clipper/Clipper.cs` (Angus Johnson's Clipper 6.4.2, C#, `use_lines`, int64 coords).
//!
//! Public names mirror the C# API in snake_case. C# overloads become distinct names:
//! `execute` / `execute_fill` (paths) and `execute_tree` / `execute_tree_fill` (PolyTree).

// line-by-line port: keep C# branch structure even where clippy would restructure it
#![allow(
    clippy::if_same_then_else,
    clippy::needless_range_loop,
    clippy::manual_clamp,
    clippy::vec_init_then_push,
    clippy::needless_late_init
)]

mod clipper;
mod offset;
mod polytree;
mod sort;
mod types;

pub use clipper::{
    Clipper, IO_PRESERVE_COLLINEAR, IO_REVERSE_SOLUTION, IO_STRICTLY_SIMPLE,
};
pub use offset::ClipperOffset;
pub use polytree::{NodeId, PolyNode, PolyTree};
pub use types::{
    CInt, ClipType, ClipperException, DoublePoint, EndType, HI_RANGE, IntPoint, IntRect, JoinType,
    LO_RANGE, Path, Paths, PolyFillType, PolyType, f64_to_cint, round,
};
