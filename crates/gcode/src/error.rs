// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! .NET exception stand-ins for the C# paths that throw (OverflowException etc.).

use std::fmt;

/// Error raised where the C# code would throw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GcodeError {
    /// decimal / TimeSpan / int overflow (`OverflowException`).
    Overflow,
    /// Argument out of range (`ArgumentOutOfRangeException`, `ArgumentException`).
    Argument(String),
    /// Anything else (message only).
    Other(String),
}

impl fmt::Display for GcodeError {
    /// Human readable text.
    ///
    /// `f`: formatter. Returns fmt result.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GcodeError::Overflow => write!(f, "Overflow"),
            GcodeError::Argument(m) => write!(f, "Argument: {m}"),
            GcodeError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for GcodeError {}

/// Result alias used across the crate.
pub type GResult<T> = Result<T, GcodeError>;
