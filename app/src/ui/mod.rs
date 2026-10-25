// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code, unused_imports)] // foundation API, consumed by later work packages
#![allow(clippy::collapsible_if, clippy::field_reassign_with_default, clippy::chunks_exact_to_as_chunks, clippy::enum_variant_names, clippy::too_many_arguments)] // style lints off: mirrors C# shapes
//! UI foundation (WP-U0): theme, icons, i18n, fonts, dialogs, widgets, gallery.


pub mod dialog;
pub mod fonts;
pub mod gallery;
pub mod i18n;
pub mod icons;
pub mod preview;
pub mod theme;
pub mod widgets;

pub use i18n::{tr, tr_fmt, tr_plain};
pub use icons::{icon, icons};
pub use theme::{LogRowColor, Scheme, Theme};
