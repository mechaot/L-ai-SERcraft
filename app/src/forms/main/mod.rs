// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(clippy::collapsible_if)]
//! Main window pieces: `MainForm` menus and status bar, `ConnectLogForm`, `JogForm`, `CommandLog`,
//! `ExceptionManager`, and the dialog host used by `main_form.rs`.

pub mod command_log;
pub mod connect_log;
pub mod dialogs;
pub mod exception;
pub mod jog;
pub mod menus;
pub mod status_bar;
