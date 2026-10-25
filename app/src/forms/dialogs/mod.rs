// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

#![allow(dead_code)] // consumed by the main window wiring (WP-U1)
#![allow(clippy::collapsible_if, clippy::too_many_arguments, clippy::type_complexity, clippy::large_enum_variant)] // style lints off: mirrors C# shapes
//! WP-U4b dialogs: one module per C# form. Each form is a struct with
//! `open(..)` and `show(&mut self, &egui::Context) -> Option<Outcome>` (modal, call every frame while
//! open, drop the struct once `show` returned `Some`). Index of the API:
//!
//! * [`settings_form`]: `SettingsForm::open(&Arc<GrblCore>)`, `show -> Option<SettingsOutcome::{Cancelled, Saved{restart}}>`;
//!   `SettingsData::{load, save}` is the plain model. `restart == true`: restart the application.
//! * [`grbl_config`]: `GrblConfig::open(&Arc<GrblCore>)`, `show -> Option<GrblConfigOutcome::Closed>`.
//! * [`hotkey_manager_form`]: `HotkeyManagerForm::open(&HotKeysManager)`, `show -> Option<HotkeyOutcome::{Cancelled, Saved(list)}>`;
//!   apply with `apply_saved(&mut HotKeysManager, list)`.
//! * [`resume_job_form`]: `ResumeJobForm::open(ResumeInfo)`, `show -> Option<ResumeOutcome::{Cancelled, Ok{position, homing, wco}}>`
//!   (position 0 -> `run_program_from_start_user(homing)`, > 0 -> `continue_program(position, homing, wco)`).
//! * [`run_from_position_form`]: `RunFromPositionForm::open(&RunFromPositionInfo)`, `show -> Option<RunFromPositionOutcome::{Cancelled, Ok{position, homing}}>`
//!   (then `continue_program(position, homing, false)`).
//! * [`safety_countdown`]: `SafetyCountdown::needed()` (false = skip), `SafetyCountdown::open()`, `show -> Option<SafetyOutcome::{Go, Cancel}>`.
//! * [`laser_usage`]: `LaserUsage::open()`, `show -> Option<LaserUsageOutcome::Closed>` (embeds [`laser_life_edit`]).
//! * [`laser_life_edit`]: `LaserLifeEdit::open(&LaserLifeCounter)`, `show -> Option<LaserLifeEditOutcome::{Cancelled, Saved(counter)}>`.
//! * [`laser_selector`]: `LaserSelector::open()`, `show -> Option<LaserSelectorOutcome::{Cancelled, Selected(guid)}>` (then `laser_life::select`).
//! * [`issue_detector_form`]: `IssueDetectorForm::enabled()`, `IssueDetectorForm::open(DetectedIssue)`, `show -> Option<IssueOutcome::{Ok, Closed}>`.
//! * [`flash_grbl`]: `FlashGrbl::open()`, `show -> Option<FlashOutcome::{Cancelled, Flashed{retval}}>` (result message boxes included).
//! * [`emulator_ui`]: `EmulatorWindow::install() -> Arc<EmulatorWindow>` once, then `window.show(ctx)` every frame.
//! * [`discovery_form`]: `DiscoveryForm::open(&Arc<GrblCore>)`, `show -> Option<DiscoveryOutcome::{None, Address(String)}>`.
//! * [`ortur_wifi_config`]: `OrturWiFiConfig::open(&Arc<GrblCore>)`, `show -> Option<WifiConfigOutcome::{Cancelled, ConnectViaWifi{ip}}>`.
//! * [`ortur_wifi_config_prompt`]: `OrturWiFiConfigPrompt::suppressed()`, `OrturWiFiConfigPrompt::open(&Arc<GrblCore>)`, `show -> Option<WifiPromptOutcome::{Yes, Cancel}>`.

pub mod common;
pub mod discovery_form;
pub mod emulator_ui;
pub mod flash_grbl;
pub mod grbl_config;
pub mod hotkey_manager_form;
pub mod issue_detector_form;
pub mod laser_life_edit;
pub mod laser_selector;
pub mod laser_usage;
pub mod ortur_wifi_config;
pub mod ortur_wifi_config_prompt;
pub mod resume_job_form;
pub mod run_from_position_form;
pub mod safety_countdown;
pub mod settings_form;
pub mod gallery_tab;
