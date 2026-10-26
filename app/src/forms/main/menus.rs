// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Ports the menu bar of `MainForm.Designer.cs` + `MainForm.AssignIcons/TimerUpdate/*_Click` (menu part).

use eframe::egui::{self, Button, Ui};
use lasergrbl_core::com::WrapperType;
use lasergrbl_core::GrblCore;
use lg_base::settings;

use crate::ui::i18n::cultures;
use crate::ui::icons::icons;
use crate::ui::theme::{Scheme, Theme};
use crate::ui::tr_plain;

/// Control names of the language items in menu order (same order as `cultures.json`).
const LANGUAGE_KEYS: [&str; 20] = [
    "MNEnglish",
    "MNItalian",
    "MNSpanish",
    "MNFrench",
    "MNGerman",
    "MNDanish",
    "MNBrazilian",
    "russianToolStripMenuItem",
    "chinexeToolStripMenuItem",
    "traditionalChineseToolStripMenuItem",
    "slovakianToolStripMenuItem",
    "hungarianToolStripMenuItem",
    "czechToolStripMenuItem",
    "polishToolStripMenuItem",
    "greekToolStripMenuItem",
    "turkishToolStripMenuItem",
    "romanianToolStripMenuItem",
    "dutchToolStripMenuItem",
    "ukrainianToolStripMenuItem",
    "japaneseToolStripMenuItem",
];

/// Scheme menu items: (scheme, control name).
const SCHEME_ITEMS: [(Scheme, &str); 7] = [
    (Scheme::CADStyle, "cadStyleToolStripMenuItem"),
    (Scheme::CADDark, "cadDarkToolStripMenuItem"),
    (Scheme::BlueLaser, "blueLaserToolStripMenuItem"),
    (Scheme::RedLaser, "redLaserToolStripMenuItem"),
    (Scheme::Dark, "darkToolStripMenuItem"),
    (Scheme::Hacker, "hackerToolStripMenuItem"),
    (Scheme::Nighty, "nightyToolStripMenuItem"),
];

/// A click on a menu item that the main window has to execute.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Connect,
    Disconnect,
    WifiDiscovery,
    GrblReset,
    GoHome,
    Unlock,
    GrblConfig,
    Settings,
    MaterialDb,
    LaserUsage,
    Hotkeys,
    Exit,
    FileOpen,
    FileAppend,
    ReopenFile,
    SaveProgram,
    AdvancedSave,
    SaveProject,
    FileSend,
    StartFromPosition,
    RunMulti,
    PowerVsSpeed,
    CuttingTest,
    AccuracyTest,
    ShakeTest,
    SetScheme(Scheme),
    AutoSize,
    ZoomIn,
    ZoomOut,
    /// `SetLanguage(culture)`.
    SetLanguage(String),
    InstallDriver,
    FlashFirmware,
    ConfigureWifi,
    OpenSessionLog,
    ToggleExtendedLog,
    ShowLicense,
}

/// Menu-only state (what `TimerUpdate` and the drop-down events kept in the form).
#[derive(Default)]
pub struct MenuState {
    /// `MnRunMulti.Visible`: more than one instance and SincroStart running.
    pub run_multi_visible: bool,
    /// `ComLogger.Enabled`, refreshed when the help menu opens.
    pub extended_log: bool,
    /// `Logger.ExistLog`.
    pub session_log_exists: bool,
}

/// One menu entry with optional icon, check mark and enable state.
///
/// `ui`: menu ui. `icon`: icon key (None = no image). `plain_icon`: draw without theme tint (flags).
/// `text`: caption. `enabled`: item enabled. `checked`: check mark state (None = not a check item).
/// Returns true when clicked (menu closes like WinForms).
fn item(ui: &mut Ui, icon: Option<&str>, plain_icon: bool, text: &str, enabled: bool, checked: Option<bool>) -> bool {
    let label = match checked {
        Some(true) => format!("\u{2714} {text}"),
        Some(false) => format!("    {text}"),
        None => text.to_owned(),
    };
    let button = match icon {
        Some(k) => {
            let img = if plain_icon { icons().image_plain(ui.ctx(), k, 16.0) } else { icons().image(ui.ctx(), k, 16.0) };
            Button::image_and_text(img, label)
        }
        None => Button::new(label),
    };
    ui.add_enabled(enabled, button).clicked()
}

/// Section title with icon for a top-level menu (`questionMark` shows "?" with legacy icons).
///
/// `ui`: menu bar ui. `icon`: icon key. `text`: caption (empty = icon only). `content`: drop-down body.
fn top_menu(ui: &mut Ui, icon: Option<&str>, text: &str, content: impl FnOnce(&mut Ui)) {
    let mut atoms = egui::Atoms::new(());
    if let Some(k) = icon {
        atoms.push_right(icons().image(ui.ctx(), k, 16.0));
    }
    if !text.is_empty() {
        atoms.push_right(text.to_owned());
    }
    let _ = egui::containers::menu::MenuButton::new(atoms).ui(ui, content);
}

/// Draw the menu bar.
///
/// `ui`: top panel ui. `core`: the engine. `st`: menu state. Returns the clicked actions.
pub fn show_menu_bar(ui: &mut Ui, core: &GrblCore, st: &mut MenuState) -> Vec<MenuAction> {
    let mut out: Vec<MenuAction> = Vec::new();
    let legacy = icons().is_legacy();
    let connected = core.is_connected();
    let wrapper: WrapperType = settings::get("ComWrapper Protocol", WrapperType::UsbSerial);
    let serial = matches!(wrapper, WrapperType::UsbSerial | WrapperType::UsbSerial2 | WrapperType::RJCPSerial);
    let lucky_wifi = core.grbl_version().is_some_and(|v| v.is_lucky_wifi());
    let show_wifi_cfg = lucky_wifi && serial;
    let th = Theme::current();
    let _ = th;
    let ico = |k: &'static str| if legacy { None } else { Some(k) };

    egui::MenuBar::new().ui(ui, |ui| {
        // ---- MnGrbl (title = firmware name) ----
        top_menu(ui, ico("mdi-hammer-wrench"), lasergrbl_core::NamedEnum::variant_name(&core.firmware()), |ui| {
            if !connected && item(ui, Some("mdi-power-plug"), false, &tr_plain("MainForm.MnConnect.Text"), true, None) {
                out.push(MenuAction::Connect);
            }
            if connected && item(ui, Some("mdi-power-plug-off"), false, &tr_plain("MainForm.MnDisconnect.Text"), true, None) {
                out.push(MenuAction::Disconnect);
            }
            if matches!(wrapper, WrapperType::LaserWebESP8266 | WrapperType::Telnet)
                && item(ui, Some("mdi-wifi"), false, &tr_plain("MainForm.MnWiFiDiscovery.Text"), !connected, None)
            {
                out.push(MenuAction::WifiDiscovery);
            }
            ui.separator();
            if item(ui, Some("mdi-lightning-bolt"), false, &tr_plain("MainForm.MnGrblReset.Text"), core.can_reset_grbl(), None) {
                out.push(MenuAction::GrblReset);
            }
            if lasergrbl_core::GrblCore::configuration().homing_enabled() && item(ui, Some("mdi-home"), false, &tr_plain("MainForm.MnGoHome.Text"), core.can_do_homing(), None) {
                out.push(MenuAction::GoHome);
            }
            if core.ui_show_unlock_buttons() && item(ui, Some("mdi-lock-open"), false, &tr_plain("MainForm.MnUnlock.Text"), core.can_unlock(), None) {
                out.push(MenuAction::Unlock);
            }
            ui.separator();
            if core.ui_show_grbl_config() && item(ui, Some("mdi-cogs"), false, &tr_plain("MainForm.MnGrblConfig.Text"), true, None) {
                out.push(MenuAction::GrblConfig);
            }
            if item(ui, Some("mdi-wrench"), false, &tr_plain("MainForm.settingsToolStripMenuItem.Text"), true, None) {
                out.push(MenuAction::Settings);
            }
            if item(ui, Some("mdi-database"), false, &tr_plain("MainForm.MnMaterialDB.Text"), true, None) {
                out.push(MenuAction::MaterialDb);
            }
            if item(ui, Some("mdi-chart-bar"), false, &tr_plain("MainForm.laserUsageStatsToolStripMenuItem.Text"), true, None) {
                out.push(MenuAction::LaserUsage);
            }
            ui.separator();
            if item(ui, Some("mdi-keyboard"), false, &tr_plain("MainForm.MnHotkeys.Text"), true, None) {
                out.push(MenuAction::Hotkeys);
            }
            ui.separator();
            if item(ui, Some("mdi-close-box"), false, &tr_plain("MainForm.MnExit.Text"), true, None) {
                out.push(MenuAction::Exit);
            }
        });

        // ---- File ----
        top_menu(ui, ico("mdi-file-document-outline"), &tr_plain("MainForm.fileToolStripMenuItem.Text"), |ui| {
            let can_load = core.can_load_new_file();
            if item(ui, Some("mdi-folder"), false, &tr_plain("MainForm.MnFileOpen.Text"), can_load, None) {
                out.push(MenuAction::FileOpen);
            }
            if item(ui, Some("mdi-folder-plus"), false, &tr_plain("MainForm.MnFileAppend.Text"), can_load, None) {
                out.push(MenuAction::FileAppend);
            }
            if item(ui, Some("mdi-reload"), false, &tr_plain("MainForm.MnReOpenFile.Text"), core.can_reopen_file(), None) {
                out.push(MenuAction::ReopenFile);
            }
            let has = core.has_program();
            if item(ui, Some("mdi-content-save"), false, &tr_plain("MainForm.MnSaveProgram.Text"), has, None) {
                out.push(MenuAction::SaveProgram);
            }
            if item(ui, Some("mdi-content-save-edit"), false, &tr_plain("MainForm.MnAdvancedSave.Text"), has, None) {
                out.push(MenuAction::AdvancedSave);
            }
            ui.separator();
            if item(ui, Some("mdi-content-save-cog"), false, &tr_plain("MainForm.MnSaveProject.Text"), has, None) {
                out.push(MenuAction::SaveProject);
            }
            ui.separator();
            let can_send = core.can_send_file();
            if item(ui, Some("mdi-play-circle-outline"), false, &tr_plain("MainForm.MnFileSend.Text"), can_send, None) {
                out.push(MenuAction::FileSend);
            }
            if item(ui, Some("mdi-motion-play-outline"), false, &tr_plain("MainForm.MnStartFromPosition.Text"), can_send, None) {
                out.push(MenuAction::StartFromPosition);
            }
            if st.run_multi_visible {
                ui.separator();
                let en = can_send || core.can_resume_hold() || core.can_feed_hold();
                if item(ui, Some("mdi-lan-connect"), false, &tr_plain("MainForm.MnRunMulti.Text"), en, None) {
                    out.push(MenuAction::RunMulti);
                }
            }
        });

        // ---- Generate ----
        top_menu(ui, ico("mdi-flask"), &tr_plain("MainForm.MnGenerate.Text"), |ui| {
            let en = core.can_load_new_file();
            ui.add_enabled_ui(en, |ui| {
                if item(ui, Some("mdi-speedometer"), false, &tr_plain("MainForm.MnPowerVsSpeed.Text"), true, None) {
                    out.push(MenuAction::PowerVsSpeed);
                }
                if item(ui, Some("mdi-content-cut"), false, &tr_plain("MainForm.MnCuttingTest.Text"), true, None) {
                    out.push(MenuAction::CuttingTest);
                }
                if item(ui, Some("mdi-details"), false, &tr_plain("MainForm.MnAccuracyTest.Text"), true, None) {
                    out.push(MenuAction::AccuracyTest);
                }
                if item(ui, Some("mdi-pulse"), false, &tr_plain("MainForm.shakeTestToolStripMenuItem.Text"), true, None) {
                    out.push(MenuAction::ShakeTest);
                }
            });
        });

        // ---- Color scheme ----
        top_menu(ui, ico("mdi-palette"), &tr_plain("MainForm.schemaToolStripMenuItem.Text"), |ui| {
            let cur = Theme::current_scheme();
            for (scheme, name) in SCHEME_ITEMS {
                if item(ui, None, false, &tr_plain(&format!("MainForm.{name}.Text")), true, Some(cur == scheme)) {
                    out.push(MenuAction::SetScheme(scheme));
                }
            }
        });

        // ---- Preview (hidden in GDI mode) ----
        if settings::current_graphic_mode() != settings::GraphicMode::Gdi {
            top_menu(ui, ico("mdi-image-filter-center-focus-strong"), &tr_plain("MainForm.previewToolStripMenuItem.Text"), |ui| {
                if item(ui, Some("mdi-magnify-scan"), false, &tr_plain("MainForm.autoSizeToolStripMenuItem.Text"), true, None) {
                    out.push(MenuAction::AutoSize);
                }
                if item(ui, Some("mdi-magnify-plus-outline"), false, &tr_plain("MainForm.zoomInToolStripMenuItem.Text"), true, None) {
                    out.push(MenuAction::ZoomIn);
                }
                if item(ui, Some("mdi-magnify-minus-outline"), false, &tr_plain("MainForm.zoomOutToolStripMenuItem.Text"), true, None) {
                    out.push(MenuAction::ZoomOut);
                }
                ui.separator();
                let mut v = core.view_options();
                let t = |s: &str| tr_plain(&format!("MainForm.{s}.Text"));
                let nv = !*v.show_laser_off_movements.value();
                if item(ui, None, false, &t("showLaserOffMovementsToolStripMenuItem"), true, Some(!nv)) {
                    v.show_laser_off_movements.set(nv);
                }
                let nv = !*v.show_executed_commands.value();
                if item(ui, None, false, &t("showExecutedCommandsToolStripMenuItem"), true, Some(!nv)) {
                    v.show_executed_commands.set(nv);
                }
                let nv = !*v.show_bounding_box.value();
                if item(ui, None, false, &t("showBoundingBoxToolStripMenuItem"), true, Some(!nv)) {
                    v.show_bounding_box.set(nv);
                }
                let nv = !*v.cross_cursor.value();
                if item(ui, None, false, &t("crossCursorToolStripMenuItem"), true, Some(!nv)) {
                    v.cross_cursor.set(nv);
                }
                // autosize mode sub menu
                ui.menu_button(
                    egui::Atoms::new((icons().image(ui.ctx(), "mdi-resize", 16.0), t("autosizeModeToolStripMenuItem"))),
                    |ui| {
                        let drawing = *v.auto_size_on_drawing.value();
                        let mut changed = false;
                        if item(ui, None, false, &t("drawingAreaToolStripMenuItem"), true, Some(drawing)) {
                            v.auto_size_on_drawing.set(true);
                            changed = true;
                        }
                        if item(ui, None, false, &t("movingAreaToolStripMenuItem"), true, Some(!drawing)) {
                            v.auto_size_on_drawing.set(false);
                            changed = true;
                        }
                        if changed {
                            out.push(MenuAction::AutoSize);
                        }
                    },
                );
                // line size sub menu: 1px..5px
                ui.menu_button(egui::Atoms::new((icons().image(ui.ctx(), "mdi-gesture", 16.0), t("lineSizeToolStripMenuItem"))), |ui| {
                    let cur = *v.preview_line_size.value();
                    for n in 1..=5 {
                        if item(ui, None, false, &t(&format!("pxToolStripMenuItem{n}")), true, Some((cur - n as f32).abs() < f32::EPSILON)) {
                            v.preview_line_size.set(n as f32);
                        }
                    }
                });
                ui.separator();
                let nv = !*v.show_performance_diagnostic.value();
                if item(ui, None, false, &t("showDiagnosticDataToolStripMenuItem"), true, Some(!nv)) {
                    v.show_performance_diagnostic.set(nv);
                }
            });
        }

        // ---- Language ----
        top_menu(ui, ico("mdi-flag-variant"), &tr_plain("MainForm.linguaToolStripMenuItem.Text"), |ui| {
            for (c, key) in cultures().iter().zip(LANGUAGE_KEYS) {
                if item(ui, Some(&c.flag), true, &tr_plain(&format!("MainForm.{key}.Text")), true, None) {
                    out.push(MenuAction::SetLanguage(c.culture.clone()));
                }
            }
        });

        // ---- Tools ----
        top_menu(ui, ico("mdi-tools"), &tr_plain("MainForm.toolsToolStripMenuItem.Text"), |ui| {
            if item(ui, Some("mdi-usb"), false, &tr_plain("MainForm.installCH340DriverToolStripMenuItem.Text"), true, None) {
                out.push(MenuAction::InstallDriver);
            }
            let disconnected = core.machine_status() == lasergrbl_core::MacStatus::Disconnected;
            if item(ui, Some("mdi-chip"), false, &tr_plain("MainForm.flashGrblFirmwareToolStripMenuItem.Text"), disconnected, None) {
                out.push(MenuAction::FlashFirmware);
            }
            // NOTE: configurationWizardToolStripMenuItem is Visible=false and has no handler in C#
        });

        // ---- Longer / Ortur (shown by board detection) ----
        // vendor link entries (support, YouTube, manuals, firmware) were upstream partner pages: dropped, only the WiFi setup stays
        if core.is_longer_board() && show_wifi_cfg {
            top_menu(ui, ico("mdi-cube-scan"), &tr_plain("MainForm.MnLonger.Text"), |ui| {
                if item(ui, None, false, &tr_plain("MainForm.MnConfigureLongerWiFi.Text"), core.can_read_write_config(), None) {
                    out.push(MenuAction::ConfigureWifi);
                }
            });
        }
        if core.is_ortur_board() && show_wifi_cfg {
            top_menu(ui, ico("mdi-opera"), &tr_plain("MainForm.MnOrtur.Text"), |ui| {
                if item(ui, Some("mdi-wifi"), false, &tr_plain("MainForm.MnConfigureOrturWiFi.Text"), core.can_read_write_config(), None) {
                    out.push(MenuAction::ConfigureWifi);
                }
            });
        }

        // ---- Help ("?" with legacy icons, icon only otherwise) ----
        top_menu(ui, ico("mdi-help"), if legacy { "?" } else { "" }, |ui| {
            // toggled when the menu opens in C# (DropDownOpening)
            st.session_log_exists = lg_base::logger::exist_log();
            st.extended_log = lg_base::logger::com::enabled();
            if item(ui, Some("mdi-file-account-outline"), false, &tr_plain("MainForm.openSessionLogToolStripMenuItem.Text"), st.session_log_exists, None) {
                out.push(MenuAction::OpenSessionLog);
            }
            if item(ui, Some("mdi-file-chart"), false, &tr_plain("MainForm.activateExtendedLogToolStripMenuItem.Text"), true, Some(st.extended_log)) {
                out.push(MenuAction::ToggleExtendedLog);
            }
            ui.separator();
            if item(ui, Some("mdi-license"), false, &tr_plain("MainForm.licenseToolStripMenuItem.Text"), true, None) {
                out.push(MenuAction::ShowLicense);
            }
        });
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_table_matches_cultures() {
        assert_eq!(cultures().len(), LANGUAGE_KEYS.len());
        assert_eq!(cultures()[0].culture, "en");
        assert_eq!(cultures()[9].culture, "zh-TW");
    }

    #[test]
    fn every_language_key_has_text() {
        for k in LANGUAGE_KEYS {
            assert!(crate::ui::i18n::catalog().has(&format!("MainForm.{k}.Text")), "{k}");
        }
    }
}
