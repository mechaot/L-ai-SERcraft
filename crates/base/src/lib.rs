// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! `lg-base`: settings, paths, logger, tools, CSV dictionaries, NRBF, sound, test support.
//!
//! Foundation crate of the LaserGRBL Rust port (WP-C1a).

/// Product name shown in the UI, window title, logs and reports.
pub const APP_NAME: &str = "L-ai-SERcraft";

pub mod paths;
pub mod settings;
pub mod logger;
pub mod nrbf;
pub mod tools;
pub mod csv;
pub mod sound;
pub mod test_support;
