// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `lg-gcode`: G-code commands, state builder, `GrblFile` (analysis, raster/vector generators),
//! `GrblConfST`, `GrblVersionInfo`. Ports `GrblCommand.cs`, `StateBuilder.cs`, `GrblFile.cs` and
//! the config types of `Core/GrblCore.cs`.

pub mod command;
pub mod draw;
pub mod generators;
pub mod grbl_file;
pub mod l2l;
pub mod optimize;
pub mod vector;
pub mod config;
pub mod csvd;
pub mod error;
pub mod program_range;
pub mod state_builder;
pub mod text;

pub use command::*;
pub use draw::*;
pub use grbl_file::*;
pub use l2l::*;
pub use optimize::*;
pub use vector::{LayerInfo, LayerParams, list_layers_from_file, svg_conv_options};
pub use config::*;
pub use error::{GResult, GcodeError};
pub use program_range::*;
pub use state_builder::*;
pub use lg_potrace::Direction;
