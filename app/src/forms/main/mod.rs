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
