# Source map

Where things live. Start with the **task index** to jump straight to the code for a kind of change; the
**file table** at the bottom lists every Rust source file with its purpose (generated from the `//!` module
headers — regenerate with `python3 -I tools/gen_source_map.py` after adding/renaming files or editing headers).
Architecture and data flow: [`ARCHITECTURE.md`](ARCHITECTURE.md). C# member → Rust location: `../PORTING.md`.

## Task index

### G-code generation (byte-exact — golden tests in `crates/gcode/tests`)
| change | go to |
|---|---|
| G-code line parsing, `SerialData`, status of a sent line | `crates/gcode/src/command.rs` (`GrblCommand::build_helper`, `serial_data`, `status`) |
| modal state / position tracking / time estimate / arcs | `crates/gcode/src/state_builder.rs` (`analyze_command`, `compute_execution_time`, `G2G3Helper`) |
| load / analyse / save a `.nc` file, header/footer/passes on save | `crates/gcode/src/grbl_file.rs` (`load_file`, `analyze`, `save_gcode`, `spawn_load`) |
| raster line-to-line / dithering output | `crates/gcode/src/l2l.rs` (`load_image_l2l`, segments, `optimize_line2line`) |
| vectorize (potrace) output, SVG and centerline loaders | `crates/gcode/src/vector.rs`; path ordering `crates/gcode/src/optimize.rs` |
| cutting / power-speed / shake test patterns | `crates/gcode/src/generators.rs` (+ text via `crates/hershey`) |
| potrace tracing, fill patterns, curve→G-code (incl. G2/G3 biarcs) | `crates/potrace/src/{trace,clipper,export_gcode,biarc/*}.rs` |
| SVG parsing / transforms / G-code writer | `crates/svg2gcode/src/{convert,gcode,matrix,bezier}.rs` |
| image filters before generation (resize, grayscale, threshold, dithering) | `crates/raster/src/{image_transform,resize,color_matrix,dithering/*}.rs` |
| raster dialog pipeline (preview + generation parameters) | `crates/services/src/image_processor/{mod,job,overlay}.rs` |
| number text in G-code | `crates/dotnet` (`fmt_double_custom`, `fmt_decimal`, …) — never `format!` floats |
| GRBL `$` config parsing, MaxPWM/table size used by generators | `crates/gcode/src/config.rs` (`GrblConfST`, `configuration()`) |

### Machine control / protocol (golden tests in `crates/core/tests/golden_protocol.rs`)
| change | go to |
|---|---|
| connect / disconnect, threads, public engine API | `crates/core/src/grbl_core/mod.rs` (`open_com`, `close_com`, `run_program`, `continue_program`, `open_file`, `refresh_config`, `write_config`) |
| streaming, buffer accounting, reply/status parsing, overrides, hang detection, program end | `crates/core/src/grbl_core/state.rs` (`thread_tx`, `send_line`, `has_space_in_buffer`, `manage_received_line`, `manage_command_response`, `parse_machine_status`, `manage_overrides`, `detect_hang`, `on_program_end`) |
| jog commands (V0.9/V1.1/continuous) | `crates/core/src/grbl_core/jog.rs` |
| custom-button code, `[expressions]`, immediate bytes | `crates/core/src/grbl_core/expr.rs` (+ evaluator `crates/expr`) |
| job counters / projected time | `crates/core/src/grbl_core/time_projection.rs` |
| enums, `CoreEvent`, `RunDecision`, `OpenFileOutcome` | `crates/core/src/grbl_core/types.rs` |
| serial / telnet / websocket / emulator links, port listing | `crates/core/src/com/{serial,telnet,websocket,emulator_link,line_queue,mod}.rs` |
| built-in GRBL emulator | `crates/core/src/emulator/grbl_v11.rs`, websocket server `websocket_emulator.rs` |
| laser module usage tracking | `crates/core/src/laser_life.rs` |
| multi-instance "start all" | `crates/core/src/sincro_start.rs` |

### Settings, files, platform services
| change | go to |
|---|---|
| settings store (keys, defaults, debounce, import of Windows `.bin`) | `crates/base/src/settings.rs`; data/temp/resource dirs `crates/base/src/paths.rs` |
| session log / com log | `crates/base/src/logger/*` |
| GRBL setting/alarm/error descriptions (CSV) | `crates/base/src/csv/*` + `resources/csv/` |
| BinaryFormatter files (`.zbn`, `.lps`, `.bin`) | `crates/base/src/nrbf/*` |
| sounds | `crates/base/src/sound.rs` |
| hotkey table, default bindings, key matching | `crates/services/src/hotkeys.rs`; execution in `app/src/main_form.rs` (`hotkeys`, `hotkey_effect`) |
| custom buttons model, import/export | `crates/services/src/custom_button.rs`; UI in `app/src/forms/preview_form/` |
| projects (`.lps`) | `crates/services/src/project.rs`; open flow in `app/src/main_form.rs` (`pump_project`) |
| named layer presets (save / load / delete) | `crates/services/src/layer_presets.rs`; UI `app/src/forms/material/`, dock `app/src/forms/params_dock.rs` |
| avrdude flashing, autotrace, wifi scan | `crates/services/src/{flash_grbl,autotrace,wifi_discovery}.rs` |

### UI (mirrors the WinForms forms; captions via i18n keys)
| change | go to |
|---|---|
| startup, CLI args, window geometry, panic dialog | `app/src/main.rs`, `app/src/forms/main/exception.rs` |
| main window flows (open/save/run/connect, dialogs, hotkeys, event handling) | `app/src/main_form.rs` (`pump_events`, `open_outcome`, `request_run`, `execute_run`, `do_menu`, `overlay_closed`) |
| menu bar, status bar | `app/src/forms/main/{menus,status_bar}.rs` |
| connection panel, command log, jog pad | `app/src/forms/main/{connect_log,command_log,jog}.rs` |
| right panel: preview + custom-button bar | `app/src/forms/preview_form/mod.rs` |
| preview rendering (GL / CPU), camera, rulers | `app/src/ui/preview/{panel_gl,gl,scene,camera,panel_cpu,overlay}.rs` |
| a specific dialog | `app/src/forms/dialogs/<form>.rs`, `forms/raster/*`, `forms/params_dock.rs`, `forms/generators/*`, … (one file per C# form; see table) |
| colours / color schemes | `app/src/ui/theme.rs` |
| icons | `app/src/ui/icons.rs` + `resources/svgicons`, `resources/icons` |
| translations | `app/src/ui/i18n.rs` (`tr`), data `resources/i18n/*.json` generated by `tools/resx2json.py` |
| reusable controls (sliders, numeric inputs, progress bars, buttons) | `app/src/ui/widgets/*` (API doc `app/src/ui/README.md`) |
| modal/message-box/file-dialog framework | `app/src/ui/dialog.rs`; main-window modal host `app/src/forms/main/dialogs.rs` |
| visual check of a widget/dialog | `cargo run -p lasergrbl -- --gallery` (`app/src/ui/gallery.rs`) |

### Tests, goldens, tooling
| what | where |
|---|---|
| golden reference data (from the original C# via Mono) | `tests/golden/<area>/` (+ `FORMAT.md` each) |
| golden generators | `tools/golden/<area>/` (`run.sh`; protocol harness `tools/golden/protocol/`) |
| member-level porting checklist | `PORTING.md` (regenerate skeleton: `tools/gen_porting_checklist.py`) |
| audit results / known deviations | `docs/AUDIT_NONUI.md`, `docs/AUDIT_UI_MAIN.md`, `docs/AUDIT_UI_DIALOGS.md`, `crates/dotnet/README.md` |
| AppImage packaging | `packaging/appimage/build-appimage.sh` |
| resources inventory | `resources/README.md` |

## File table

<!-- BEGIN GENERATED FILE TABLE (tools/gen_source_map.py) -->


### `app`

| file | lines | purpose |
|---|---:|---|
| [`app/src/app_state.rs`](../app/src/app_state.rs) | 90 | Shared application state: the engine, its event channel, custom buttons and hotkeys. |
| [`app/src/forms/dialogs/common.rs`](../app/src/forms/dialogs/common.rs) | 230 | Shared helpers of the WP-U4b dialogs. No C# counterpart: replaces WinForms boilerplate (`ShowDialog`, `TableLayoutPanel` rows, caption lookups for forms without `.resx` entries). |
| [`app/src/forms/dialogs/discovery_form.rs`](../app/src/forms/dialogs/discovery_form.rs) | 274 | Ports `WiFiDiscovery/DiscoveryForm.cs` + `DiscoveryForm.Designer.cs`: find the engraver on the LAN. |
| [`app/src/forms/dialogs/emulator_ui.rs`](../app/src/forms/dialogs/emulator_ui.rs) | 176 | Ports `GrblEmulator/EmulatorUI.cs` + `EmulatorUI.Designer.cs`: log window of the built-in grbl emulator. |
| [`app/src/forms/dialogs/flash_grbl.rs`](../app/src/forms/dialogs/flash_grbl.rs) | 387 | Ports `Firmware/FlashGrbl.cs` + `FlashGrbl.Designer.cs`: flash a precompiled grbl `.hex` with avrdude. |
| [`app/src/forms/dialogs/gallery_tab.rs`](../app/src/forms/dialogs/gallery_tab.rs) | 230 | Gallery tab "Dialogs2": buttons opening every WP-U4b dialog against a core on the built-in emulator. |
| [`app/src/forms/dialogs/grbl_config.rs`](../app/src/forms/dialogs/grbl_config.rs) | 607 | Ports `GrblConfig.cs` + `GrblConfig.Designer.cs`: grbl `$$` configuration editor. |
| [`app/src/forms/dialogs/hotkey_manager_form.rs`](../app/src/forms/dialogs/hotkey_manager_form.rs) | 216 | Ports `HotkeyManagerForm.cs` + `HotkeyManagerForm.Designer.cs`: shortcut editor. |
| [`app/src/forms/dialogs/issue_detector_form.rs`](../app/src/forms/dialogs/issue_detector_form.rs) | 115 | Ports `IssueDetectorForm.cs` + `IssueDetectorForm.Designer.cs`: "Problem detected!" box. |
| [`app/src/forms/dialogs/laser_life_edit.rs`](../app/src/forms/dialogs/laser_life_edit.rs) | 247 | Ports `LaserLifeEdit.cs` + `LaserLifeEdit.Designer.cs`: "Edit Laser" dialog. |
| [`app/src/forms/dialogs/laser_selector.rs`](../app/src/forms/dialogs/laser_selector.rs) | 126 | Ports `LaserSelector.cs` + `LaserSelector.Designer.cs`: "Select Laser" dialog shown on connect when more than one laser module is alive (`laser_life::on_connect` -> `NeedSelection`). |
| [`app/src/forms/dialogs/laser_usage.rs`](../app/src/forms/dialogs/laser_usage.rs) | 331 | Ports `LaserUsage.cs` + `LaserUsage.Designer.cs`: "Laser Lifespan - Monitoring Tool". |
| [`app/src/forms/dialogs/mod.rs`](../app/src/forms/dialogs/mod.rs) | 43 | WP-U4b dialogs: one module per C# form. Each form is a struct with `open(..)` and `show(&mut self, &egui::Context) -> Option<Outcome>` (modal, call every frame while open, drop the struct once `show` returned `Some`). Index of the API: |
| [`app/src/forms/dialogs/ortur_wifi_config.rs`](../app/src/forms/dialogs/ortur_wifi_config.rs) | 255 | Ports `WiFiConfigurator/OrturWiFiConfig.cs` + `OrturWiFiConfig.Designer.cs`: program SSID / password into an Ortur or Longer engraver and wait for its IP. |
| [`app/src/forms/dialogs/ortur_wifi_config_prompt.rs`](../app/src/forms/dialogs/ortur_wifi_config_prompt.rs) | 116 | Ports `WiFiConfigurator/OrturWiFiConfigPrompt.cs` (class `WiFiConfigPrompt`) + Designer: "Configure machine for WiFi?" question shown after connecting to an Ortur/Longer board. |
| [`app/src/forms/dialogs/resume_job_form.rs`](../app/src/forms/dialogs/resume_job_form.rs) | 277 | Ports `ResumeJobForm.cs` + `ResumeJobForm.Designer.cs`: "resume interrupted job" dialog. |
| [`app/src/forms/dialogs/run_from_position_form.rs`](../app/src/forms/dialogs/run_from_position_form.rs) | 128 | Ports `RunFromPositionForm.cs` + `RunFromPositionForm.Designer.cs`: "run from line N" dialog. |
| [`app/src/forms/dialogs/safety_countdown.rs`](../app/src/forms/dialogs/safety_countdown.rs) | 172 | Ports `SafetyCountdown.cs` + `SafetyCountdown.Designer.cs`: "Safety first!" 5 s countdown before a job. |
| [`app/src/forms/dialogs/settings_form.rs`](../app/src/forms/dialogs/settings_form.rs) | 748 | Ports `SettingsForm.cs` + `SettingsForm.Designer.cs`: tabbed settings dialog. |
| [`app/src/forms/form_util.rs`](../app/src/forms/form_util.rs) | 324 | Shared helpers of the WP-U4a forms. No C# counterpart: replaces WinForms boilerplate (ShowDialog, ComboboxItem, tooltips, `GrblCore.Configuration` limit lookups). |
| [`app/src/forms/gallery_tab.rs`](../app/src/forms/gallery_tab.rs) | 257 | Gallery tab "Forms" (WP-U4a): opens each form with a throwaway `GrblFile` and logs the outcome. |
| [`app/src/forms/generators/cutting_test.rs`](../app/src/forms/generators/cutting_test.rs) | 355 | Ports `Generator/CuttingTest.cs` (+ Designer): cutting test generator form. |
| [`app/src/forms/generators/mod.rs`](../app/src/forms/generators/mod.rs) | 53 | Test-pattern generator forms (`Generator/CuttingTest.cs`, `PowerVsSpeedForm.cs`, `ShakeTest.cs`). |
| [`app/src/forms/generators/power_vs_speed.rs`](../app/src/forms/generators/power_vs_speed.rs) | 379 | Ports `Generator/PowerVsSpeedForm.cs` (+ Designer): power vs speed (greyscale) test generator form. |
| [`app/src/forms/generators/shake_test.rs`](../app/src/forms/generators/shake_test.rs) | 230 | Ports `Generator/ShakeTest.cs` (+ Designer): shake test generator form. |
| [`app/src/forms/legal_disclaimer.rs`](../app/src/forms/legal_disclaimer.rs) | 126 | Ports `LegalDisclaimer.cs` (+ Designer): legal disclaimer shown on first start. |
| [`app/src/forms/license.rs`](../app/src/forms/license.rs) | 136 | Ports `LicenseForm.cs` (+ Designer): "LaserGRBL License" dialog (GPL text + donate link). |
| [`app/src/forms/main/command_log.rs`](../app/src/forms/main/command_log.rs) | 174 | Ports `UserControls/CommandLog.cs`: virtualised list of sent commands and board messages. |
| [`app/src/forms/main/connect_log.rs`](../app/src/forms/main/connect_log.rs) | 374 | Ports `ConnectLogForm.cs` (+Designer): connection, file/progress and manual command / log group boxes. |
| [`app/src/forms/main/dialogs.rs`](../app/src/forms/main/dialogs.rs) | 123 | Dialog host of the main window: one `Overlay` per modal form, shown one at a time (C# `ShowDialog`). |
| [`app/src/forms/main/exception.rs`](../app/src/forms/main/exception.rs) | 164 | Ports `ExceptionManager.cs` (+Designer): panic hook that logs and queues a report, and the dialog. |
| [`app/src/forms/main/jog.rs`](../app/src/forms/main/jog.rs) | 243 | Ports `JogForm.cs` (+Designer): direction pad, Z buttons, speed and step sliders, and the `StepBar`, `DirectionButton`, `DirectionStepButton` helper controls. |
| [`app/src/forms/main/menus.rs`](../app/src/forms/main/menus.rs) | 467 | Ports the menu bar of `MainForm.Designer.cs` + `MainForm.AssignIcons/TimerUpdate/*_Click` (menu part). |
| [`app/src/forms/main/mod.rs`](../app/src/forms/main/mod.rs) | 11 | Main window pieces: `MainForm` menus and status bar, `ConnectLogForm`, `JogForm`, `CommandLog`, `ExceptionManager`, and the dialog host used by `main_form.rs`. |
| [`app/src/forms/main/status_bar.rs`](../app/src/forms/main/status_bar.rs) | 219 | Ports the `StatusBar` of `MainForm` (labels, buffer bar, overrides) and the status part of `TimerUpdate`. |
| [`app/src/forms/material/mod.rs`](../app/src/forms/material/mod.rs) | 26 | Named layer presets: shared handle `SharedPresetDb`, process wide `default_presets()` (replaces `PSHelper/*`, intentional deviation). |
| [`app/src/forms/material/preset_editor.rs`](../app/src/forms/material/preset_editor.rs) | 127 | List editor of the layer presets: read-only table, immediate delete (replaces `PSEditorForm.cs`). |
| [`app/src/forms/material/preset_picker.rs`](../app/src/forms/material/preset_picker.rs) | 103 | Preset picker of the raster import P/S helper button (replaces `PSHelperForm.cs`). |
| [`app/src/forms/mod.rs`](../app/src/forms/mod.rs) | 20 | Forms of the application (one module per C# form). |
| [`app/src/forms/params_dock.rs`](../app/src/forms/params_dock.rs) | 1545 | "Parameters" dock (right side of the main window). Replaces the modal `SvgToGCodeForm` (`SvgConverter/ConvertSizeAndOptionForm.cs`): SVG is loaded at once, power/speed are tweaked here and every committed edit regenerates the G-code in the background. Settings are kept per SVG layer (top-level `<g>`). Not part of the C# app (intentional deviation). |
| [`app/src/forms/preview_form/custom_button_form.rs`](../app/src/forms/preview_form/custom_button_form.rs) | 384 | Ports `CustomButtonForm.cs` + Designer: modal editor to create / edit one custom button. |
| [`app/src/forms/preview_form/custom_button_ib.rs`](../app/src/forms/preview_form/custom_button_ib.rs) | 353 | UI side of `CustomButton.cs` / `PreviewForm.CustomButtonIB`: click/press/release semantics, icon choice, PNG image rendering and the drag-reorder helpers. |
| [`app/src/forms/preview_form/gallery_demo.rs`](../app/src/forms/preview_form/gallery_demo.rs) | 81 | Gallery tab "PreviewForm": the form with a core on the built-in emulator and the standard buttons. |
| [`app/src/forms/preview_form/mod.rs`](../app/src/forms/preview_form/mod.rs) | 685 | Ports `PreviewForm.cs` + Designer: right-hand panel of the main window. Job preview on top, toolbar below (reset, homing, unlock, zero, custom buttons flow, resume, stop). Custom button editor: [`custom_button_form`]; button behaviour helpers: [`custom_button_ib`]. |
| [`app/src/forms/raster/convert_size.rs`](../app/src/forms/raster/convert_size.rs) | 479 | Ports `RasterConverter/ConvertSizeAndOptionForm.cs` (+ Designer): target size, offset, speeds and laser options of the raster import (second page). |
| [`app/src/forms/raster/gallery_demo.rs`](../app/src/forms/raster/gallery_demo.rs) | 121 | Gallery tab "Raster import": opens [`RasterImport`] on a bundled test image with a throwaway `GrblFile` and prints the resulting G-code line count (dev tool, no C# counterpart). |
| [`app/src/forms/raster/mod.rs`](../app/src/forms/raster/mod.rs) | 148 | Raster import dialogs (WP-U3): `RasterConverter/RasterToLaserForm.cs`, `ConvertSizeAndOptionForm.cs`, `ResolutionHelperForm.cs`. |
| [`app/src/forms/raster/raster_to_laser.rs`](../app/src/forms/raster/raster_to_laser.rs) | 1339 | Ports `RasterConverter/RasterToLaserForm.cs` (+ Designer): the image import dialog (tool selection, conversion parameters, crop / fill / outline tools, live preview) which hands over to [`ConvertSizeForm`] and finally drives `ImageProcessor::generate_gcode`. |
| [`app/src/forms/raster/resolution_helper.rs`](../app/src/forms/raster/resolution_helper.rs) | 166 | Ports `RasterConverter/ResolutionHelperForm.cs` (+ Designer): computes a line resolution that matches the stepper resolution (`$100`). |
| [`app/src/forms/save_options.rs`](../app/src/forms/save_options.rs) | 127 | Ports `SaveOptionForm.cs` (+ Designer): "Save G-code options" dialog. |
| [`app/src/forms/splash.rs`](../app/src/forms/splash.rs) | 80 | Ports `SplashScreenForm.cs` (+ Designer): borderless, top-most splash with logo, version and 2 s timer. |
| [`app/src/forms/svg_mode.rs`](../app/src/forms/svg_mode.rs) | 113 | Ports `SvgConverter/SvgModeForm.cs` (+ Designer): choose "Load as Vector" / "Load as Image" for an SVG. |
| [`app/src/main.rs`](../app/src/main.rs) | 184 | LaserGRBL egui application entry (ports `Program.cs` startup + `MainForm.RefreshFormTitle`). |
| [`app/src/main_form.rs`](../app/src/main_form.rs) | 1202 | Ports `MainForm.cs`: menu bar, status bar, left column (connect/log + jog), preview, and the dialog flows (open/save/run/connect) that the C# `GrblCore` ran with blocking dialogs. |
| [`app/src/ui/dialog.rs`](../app/src/ui/dialog.rs) | 415 | Modal-form framework: WinForms `ShowDialog`, `MessageBox.Show`, `OpenFileDialog/SaveFileDialog`. |
| [`app/src/ui/fonts.rs`](../app/src/ui/fonts.rs) | 113 | Font setup: egui default fonts plus a system CJK fallback font when one exists. |
| [`app/src/ui/gallery.rs`](../app/src/ui/gallery.rs) | 434 | Widget gallery: shows every theme color, icon, widget and dialog for visual checks. Start with `lasergrbl --gallery`. |
| [`app/src/ui/i18n.rs`](../app/src/ui/i18n.rs) | 279 | Localization (replaces .resx `ResourceManager` + WinForms `ApplyResources`). |
| [`app/src/ui/icons.rs`](../app/src/ui/icons.rs) | 352 | Ports `Icons/IconsMgr.cs`, `Icons/IIconsLoader.cs`, `Icons/LegacyIcons.cs`, `SvgIcons/SvgIcons.cs`. |
| [`app/src/ui/mod.rs`](../app/src/ui/mod.rs) | 17 | UI foundation (WP-U0): theme, icons, i18n, fonts, dialogs, widgets, gallery. |
| [`app/src/ui/preview/camera.rs`](../app/src/ui/preview/camera.rs) | 362 | Orthographic camera + ruler math of `GrblPanel3D.cs` (SetWorldPosition, resize, AutoSizeDrawing, CenterZoom, mouse wheel, `GetRulerStep`, `HumanReadableLength`). Pure math, unit tested. |
| [`app/src/ui/preview/demo.rs`](../app/src/ui/preview/demo.rs) | 241 | Gallery "Preview" tab: both renderers side by side with sample programs, a fake moving cursor and a fake "program run" that marks commands as executed (exercises the invalidation path). |
| [`app/src/ui/preview/gl.rs`](../app/src/ui/preview/gl.rs) | 399 | OpenGL renderer of the job preview (replaces SharpGL display lists of `Objects3D.cs`). |
| [`app/src/ui/preview/mod.rs`](../app/src/ui/preview/mod.rs) | 333 | Job preview panels (WP-U2a): ports of `GrblPanel3D.cs` (OpenGL), `GrblPanel.cs` (GDI fallback), `IGrblPanel.cs` and `Obj3D/*`. See `README.md` in this folder. |
| [`app/src/ui/preview/overlay.rs`](../app/src/ui/preview/overlay.rs) | 73 | Text overlays shared by both panels (C# `DrawOverlay`, `MeasureText`, `FormatCoord`). |
| [`app/src/ui/preview/panel_cpu.rs`](../app/src/ui/preview/panel_cpu.rs) | 504 | Port of `UserControls/GrblPanel.cs`: CPU (GDI fallback) preview. |
| [`app/src/ui/preview/panel_gl.rs`](../app/src/ui/preview/panel_gl.rs) | 779 | Port of `UserControls/GrblPanel3D.cs`: OpenGL job preview as an egui widget. |
| [`app/src/ui/preview/scene.rs`](../app/src/ui/preview/scene.rs) | 506 | Job geometry for the GL preview: port of `Objects3D.cs` `Grbl3D.Draw` / `Invalidate`. |
| [`app/src/ui/theme.rs`](../app/src/ui/theme.rs) | 610 | Ports `ColorScheme.cs` (7 schemes) and the intent of `UserControls/ThemeMgr.cs` (apply dark/light look to the whole window; here: egui `Visuals`). |
| [`app/src/ui/widgets/color_slider.rs`](../app/src/ui/widgets/color_slider.rs) | 421 | Ports `UserControls/ColorSlider.cs`: themed gradient slider (raster params; also vertical). |
| [`app/src/ui/widgets/combo.rs`](../app/src/ui/widgets/combo.rs) | 97 | Ports `FlatComboBox.cs` and `ComboBoxEnumItem.cs` (`EnumComboBox`). |
| [`app/src/ui/widgets/data_grid.rs`](../app/src/ui/widgets/data_grid.rs) | 63 | Replacement for `MyDatagridView.cs` (a `DataGridView` with a WinForms crash workaround that has no egui equivalent) -- here a small themed table helper on top of `egui_extras::TableBuilder`. |
| [`app/src/ui/widgets/graph.rs`](../app/src/ui/widgets/graph.rs) | 149 | Ports `Tools/Graph.cs` (rounded rect) plus small GDI+-like drawing helpers shared by widgets. |
| [`app/src/ui/widgets/grbl_button.rs`](../app/src/ui/widgets/grbl_button.rs) | 72 | Ports `UserControls/GrblButton.cs`: flat themed button, optional icon at left. |
| [`app/src/ui/widgets/group_box.rs`](../app/src/ui/widgets/group_box.rs) | 38 | Ports `UserControls/GrblGroupBox.cs`: group box with bold title in a themed border. |
| [`app/src/ui/widgets/image_button.rs`](../app/src/ui/widgets/image_button.rs) | 182 | Ports `UserControls/ImageButton.cs`: icon button with themed border, hover/pressed look, caption. |
| [`app/src/ui/widgets/input_box.rs`](../app/src/ui/widgets/input_box.rs) | 90 | Ports `Tools/InputBox.cs`: modal "prompt + text + OK/Cancel" dialog with optional validator. |
| [`app/src/ui/widgets/label_tb.rs`](../app/src/ui/widgets/label_tb.rs) | 118 | Ports `UserControls/LabelTB.cs`: override slider (Rapid / Linear / Power) with label and reset button. |
| [`app/src/ui/widgets/mod.rs`](../app/src/ui/widgets/mod.rs) | 32 | egui equivalents of the custom WinForms controls (see `ui/README.md`). |
| [`app/src/ui/widgets/numeric.rs`](../app/src/ui/widgets/numeric.rs) | 521 | Ports `UserControls/NumericInput/*`: `DecimalInputRanged`, `IntegerInputRanged`, `NumericUpDown`. |
| [`app/src/ui/widgets/picture_box.rs`](../app/src/ui/widgets/picture_box.rs) | 181 | Ports `UserControls/MyPictureBox.cs`: image on a "paper" with a draggable selection fence. Adds optional wheel zoom (egui extra). |
| [`app/src/ui/widgets/progress.rs`](../app/src/ui/widgets/progress.rs) | 432 | Ports `ColorProgressBar.cs`, `DoubleProgressBar.cs`, `WaitingProgressBar.cs`. |
| [`app/src/ui/widgets/text.rs`](../app/src/ui/widgets/text.rs) | 195 | Ports `PlaceholderTextbox.cs`, `TextInput.cs`, `GrblTextBox.cs` and `ColoredBorderControl.cs`. |
| [`app/src/ui/widgets/track_bar.rs`](../app/src/ui/widgets/track_bar.rs) | 182 | Themed port of the WinForms `TrackBar` (used by `JogForm` speed and `LabelTB` overrides) with ticks. |
| [`app/src/ui/widgets/usage_class_preview.rs`](../app/src/ui/widgets/usage_class_preview.rs) | 104 | Ports `UserControls/UsageClassPreview.cs`: 10-bar histogram of laser usage classes. |

### `crates/base`

| file | lines | purpose |
|---|---:|---|
| [`crates/base/src/csv/csvd.rs`](../crates/base/src/csv/csvd.rs) | 168 | Static `CSVD` from `GrblCommand.cs`: Settings / Alarms / Errors dictionaries and version selection. |
| [`crates/base/src/csv/dictionary.rs`](../crates/base/src/csv/dictionary.rs) | 60 | Port of `CSV/CsvDictionary.cs`. |
| [`crates/base/src/csv/formatted_message.rs`](../crates/base/src/csv/formatted_message.rs) | 394 | Port of `CSV/FormattedMessage.cs` (`FormattedMessage`, `StringList`). |
| [`crates/base/src/csv/mod.rs`](../crates/base/src/csv/mod.rs) | 9 | Port of `CSV/CsvDictionary.cs`, `CSV/FormattedMessage.cs` and static `CSVD` (from `GrblCommand.cs`). |
| [`crates/base/src/lib.rs`](../crates/base/src/lib.rs) | 11 | `lg-base`: settings, paths, logger, tools, CSV dictionaries, NRBF, sound. |
| [`crates/base/src/logger/async_file.rs`](../crates/base/src/logger/async_file.rs) | 120 | Port of `Logger/AsyncLogFile.cs`: queue + writer thread. |
| [`crates/base/src/logger/com.rs`](../crates/base/src/logger/com.rs) | 104 | Port of `Logger/ComLogger.cs`: TX/RX recording to a file. |
| [`crates/base/src/logger/mod.rs`](../crates/base/src/logger/mod.rs) | 98 | Port of `Logger/Logger.cs` (session log) and `Logger/ComLogger.cs` (TX/RX log). |
| [`crates/base/src/nrbf/convert.rs`](../crates/base/src/nrbf/convert.rs) | 449 | Graph <-> JSON / typed helpers: settings import, `List<CustomButton>` read and build. |
| [`crates/base/src/nrbf/mod.rs`](../crates/base/src/nrbf/mod.rs) | 188 | MS-NRBF (.NET `BinaryFormatter`) reader / writer and the `Tools/Serializer.cs` file container. |
| [`crates/base/src/nrbf/reader.rs`](../crates/base/src/nrbf/reader.rs) | 319 | NRBF reader. |
| [`crates/base/src/nrbf/serializer.rs`](../crates/base/src/nrbf/serializer.rs) | 220 | Port of `Tools/Serializer.cs`: text header (`<!-- Tag: Value -->` lines) + optional raw-deflate + NRBF payload. |
| [`crates/base/src/nrbf/writer.rs`](../crates/base/src/nrbf/writer.rs) | 341 | NRBF writer: re-emits a [`Graph`] byte-exact. |
| [`crates/base/src/paths.rs`](../crates/base/src/paths.rs) | 91 | Port of `GrblCore.DataPath`, `GrblCore.TempPath`, `GrblCore.ExePath` (Core/GrblCore.cs) + resource dir lookup. |
| [`crates/base/src/settings.rs`](../crates/base/src/settings.rs) | 350 | Port of `Settings.cs`: global key/value store with debounced save. |
| [`crates/base/src/sound.rs`](../crates/base/src/sound.rs) | 146 | Port of `Sound/SoundEvent.cs`: play event sounds without blocking the caller. |
| [`crates/base/src/tools/cronometro.rs`](../crates/base/src/tools/cronometro.rs) | 264 | Port of `Tools/Cronometro.cs`: `TimingBase`, `ElapsedFromEvent`, `SimpleCrono`, `AdvancedCrono`, `PeriodicEventTimer`. |
| [`crates/base/src/tools/hires_timer.rs`](../crates/base/src/tools/hires_timer.rs) | 66 | Port of `Tools/HiResTimer.cs` (class `HiResTimer`). |
| [`crates/base/src/tools/math_helper.rs`](../crates/base/src/tools/math_helper.rs) | 113 | Port of `Tools/MathHelper.cs`: `MathHelper` and `RulerStepCalculator`. |
| [`crates/base/src/tools/mobile_average.rs`](../crates/base/src/tools/mobile_average.rs) | 212 | Port of `Tools/MobileAverageCalculator.cs`: `MobileAverageCalculator` (long) and `MobileDAverageCalculator` (double). |
| [`crates/base/src/tools/mod.rs`](../crates/base/src/tools/mod.rs) | 10 | Port of `Tools/*.cs` helpers (timers, threads, math, wordwrap, utils). |
| [`crates/base/src/tools/retained_setting.rs`](../crates/base/src/tools/retained_setting.rs) | 50 | Port of `Tools/RetainedSetting.cs`. |
| [`crates/base/src/tools/thread_class.rs`](../crates/base/src/tools/thread_class.rs) | 318 | Port of `Tools/ThreadClass.cs` (`ThreadClass` + `ThreadObject`). |
| [`crates/base/src/tools/utils.rs`](../crates/base/src/tools/utils.rs) | 197 | Port of class `Utils` in `Tools/HiResTimer.cs` (TimeSpanToString, HumanReadableDateDiff, OpenLink). |
| [`crates/base/src/tools/word_wrap.rs`](../crates/base/src/tools/word_wrap.rs) | 96 | Port of `Tools/WordWrap.cs`. Works on UTF-16 units like C# so lengths match. |
| [`crates/base/tests/golden_timespan.rs`](../crates/base/tests/golden_timespan.rs) | 49 | Compare Utils.HumanReadableDateDiff / TimeSpanToString with Mono goldens. |
| [`crates/base/tests/logger.rs`](../crates/base/tests/logger.rs) | 63 | Logger + ComLogger output tests. |
| [`crates/base/tests/nrbf.rs`](../crates/base/tests/nrbf.rs) | 146 | NRBF reader / writer tests against StandardButtons.zbn and Mono-generated streams. |
| [`crates/base/tests/settings.rs`](../crates/base/tests/settings.rs) | 122 | Settings store tests (one process, one test fn: the store is global). |

### `crates/clipper`

| file | lines | purpose |
|---|---:|---|
| [`crates/clipper/src/clipper.rs`](../crates/clipper/src/clipper.rs) | 4233 | Port of `ClipperBase` + `Clipper` from `Clipper/Clipper.cs` (Clipper 6.4.2, `use_lines`). C# linked object graph (TEdge / OutRec / OutPt / LocalMinima / Maxima) is held in index arenas. `NIL` is the C# `null`. Traversal order is kept identical to the C# code. |
| [`crates/clipper/src/lib.rs`](../crates/clipper/src/lib.rs) | 29 | Port of `Clipper/Clipper.cs` (Angus Johnson's Clipper 6.4.2, C#, `use_lines`, int64 coords). |
| [`crates/clipper/src/offset.rs`](../crates/clipper/src/offset.rs) | 545 | Port of `ClipperOffset` from `Clipper/Clipper.cs`. |
| [`crates/clipper/src/polytree.rs`](../crates/clipper/src/polytree.rs) | 159 | Port of `PolyTree` / `PolyNode` from `Clipper/Clipper.cs`. C# object graph becomes an arena: node 0 is the tree (root) itself, others are `PolyNode`s. |
| [`crates/clipper/src/sort.rs`](../crates/clipper/src/sort.rs) | 156 | .NET `List<T>.Sort(IComparer)` (introsort from referencesource `ArraySortHelper`). Not stable, so tie order must match .NET exactly: port the algorithm. |
| [`crates/clipper/src/types.rs`](../crates/clipper/src/types.rs) | 264 | Port of the basic value types of `Clipper/Clipper.cs`: `IntPoint`, `IntRect`, `DoublePoint`, `Int128` and the enums. |
| [`crates/clipper/tests/golden.rs`](../crates/clipper/tests/golden.rs) | 499 | Replays `tests/golden/clipper/*.txt` (made by `tools/golden/clipper/run.sh` from the original C# ClipperLib under Mono) and compares exact points, order and tree shape. |

### `crates/core`

| file | lines | purpose |
|---|---:|---|
| [`crates/core/src/com/emulator_link.rs`](../crates/core/src/com/emulator_link.rs) | 123 | Port of `ComWrapper/Emulator.cs`: the in-process emulator as a transport. |
| [`crates/core/src/com/fake.rs`](../crates/core/src/com/fake.rs) | 204 | Fake link for tests: records everything the core writes and lets the test inject received lines. |
| [`crates/core/src/com/ip_helper.rs`](../crates/core/src/com/ip_helper.rs) | 97 | Port of `IPHelper` (in `ComWrapper/Telnet.cs`): parse `host[:port]`, `[v6]:port` and bare IPv6. |
| [`crates/core/src/com/line_queue.rs`](../crates/core/src/com/line_queue.rs) | 175 | Shared line buffer between a reader thread and `read_line_blocking` (replaces the blocking C# reads). |
| [`crates/core/src/com/mod.rs`](../crates/core/src/com/mod.rs) | 149 | Port of `ComWrapper/IComWrapper.cs` (interface + `WrapperType`) and the wrapper factory used by `GrblCore.Configure`. |
| [`crates/core/src/com/serial.rs`](../crates/core/src/com/serial.rs) | 329 | Port of `ComWrapper/UsbSerial.cs`, `UsbSerial2.cs` and `RJCPSerial.cs`. |
| [`crates/core/src/com/telnet.rs`](../crates/core/src/com/telnet.rs) | 248 | Port of `ComWrapper/Telnet.cs` (plain TCP line stream, no telnet option negotiation, like C#). |
| [`crates/core/src/com/websocket.rs`](../crates/core/src/com/websocket.rs) | 276 | Port of `ComWrapper/LaserWebESP8266.cs` on top of `tungstenite` (client). |
| [`crates/core/src/emulator/grbl_v11.rs`](../crates/core/src/emulator/grbl_v11.rs) | 548 | Port of `GrblEmulator/Grblv11Emulator.cs`: an in-process Grbl 1.1 look-alike. |
| [`crates/core/src/emulator/mod.rs`](../crates/core/src/emulator/mod.rs) | 44 | Port of `GrblEmulator/*` (the `EmulatorUI` form is UI work, only its hook is here). |
| [`crates/core/src/emulator/websocket_emulator.rs`](../crates/core/src/emulator/websocket_emulator.rs) | 164 | Port of `GrblEmulator/WebsocketEmulator.cs`: a websocket server that serves one [`Grblv11Emulator`] per connection (tungstenite server instead of websocket-sharp). |
| [`crates/core/src/grbl_core/expr.rs`](../crates/core/src/grbl_core/expr.rs) | 198 | Custom code evaluation of `GrblCore` (`ExecuteCustomCode` helpers, `EvaluateExpression`, `EvaluateCB`, `IsImmediate`, `GetImmediate`, `FormatNumber`). |
| [`crates/core/src/grbl_core/jog.rs`](../crates/core/src/grbl_core/jog.rs) | 249 | Jog command construction of `GrblCore` (`EnqueueJogV09/V11`, `HandleContinuosJog`, `ContinuousJog`). |
| [`crates/core/src/grbl_core/mod.rs`](../crates/core/src/grbl_core/mod.rs) | 1349 | Port of `Core/GrblCore.cs` (+ `MarlinCore.cs`, `SmoothieCore.cs`, `VigoCore.cs`): the GRBL streaming engine. |
| [`crates/core/src/grbl_core/state.rs`](../crates/core/src/grbl_core/state.rs) | 1739 | Mutable state of `GrblCore` (everything the C# code guards with `lock(this)`) and the logic that runs under that lock. |
| [`crates/core/src/grbl_core/testing.rs`](../crates/core/src/grbl_core/testing.rs) | 135 | Test support for the protocol golden tests (not part of the C# code). |
| [`crates/core/src/grbl_core/time_projection.rs`](../crates/core/src/grbl_core/time_projection.rs) | 398 | Port of `TimeProjection` (in `Core/GrblCore.cs`): job counters, timing and pause accounting. |
| [`crates/core/src/grbl_core/types.rs`](../crates/core/src/grbl_core/types.rs) | 471 | Enums, event and result types of `Core/GrblCore.cs` (nested types of `GrblCore`, delegates, events). |
| [`crates/core/src/laser_life.rs`](../crates/core/src/laser_life.rs) | 633 | Port of `LaserLifeHandler` (+ `LaserLifeCounter`, `ListLLC`) from `Core/GrblCore.cs`. |
| [`crates/core/src/lib.rs`](../crates/core/src/lib.rs) | 14 | `lasergrbl-core`: GRBL streaming engine, communication links, emulator and support services. |
| [`crates/core/src/sincro_start.rs`](../crates/core/src/sincro_start.rs) | 188 | Port of `SincroStart.cs`: "start all instances" signal between LaserGRBL processes. |
| [`crates/core/src/usage_stats.rs`](../crates/core/src/usage_stats.rs) | 188 | Port of `UsageStats.cs`, local part only (PLAN section 6: no upload to the upstream server). |
| [`crates/core/src/util.rs`](../crates/core/src/util.rs) | 161 | Small helpers shared by the modules (GUID generation, TimeSpan setting codec, UTF-16 style substrings). |
| [`crates/core/tests/engine.rs`](../crates/core/tests/engine.rs) | 577 | Engine tests against a recording fake link (deterministic replies) and against the in-process emulator. Expected command strings were derived by reading `Core/GrblCore.cs`. |
| [`crates/core/tests/golden_protocol.rs`](../crates/core/tests/golden_protocol.rs) | 1062 | Protocol golden test: replays every `tests/golden/protocol/*.scn` scenario on the Rust core and compares the trace byte for byte with `*.txt`, which the ORIGINAL C# `GrblCore` (Mono) produced (`tools/golden/protocol/run.sh`). |

### `crates/dotnet`

| file | lines | purpose |
|---|---:|---|
| [`crates/dotnet/src/chars.rs`](../crates/dotnet/src/chars.rs) | 57 | `System.Char` classification (UTF-16 code unit semantics) and string helpers. |
| [`crates/dotnet/src/decimal.rs`](../crates/dotnet/src/decimal.rs) | 278 | `System.Decimal` formatting, conversions and rounding (.NET Fx semantics). |
| [`crates/dotnet/src/fmt.rs`](../crates/dotnet/src/fmt.rs) | 610 | Standard + custom numeric format engine (port of .NET Fx `Number.Formatting`, InvariantCulture). |
| [`crates/dotnet/src/lib.rs`](../crates/dotnet/src/lib.rs) | 209 | `lg-dotnet`: bit-exact .NET Framework 4.x number/time/char semantics for the LaserGRBL port. |
| [`crates/dotnet/src/number.rs`](../crates/dotnet/src/number.rs) | 154 | Digit-string number model shared by all formatters (mirrors .NET Fx `NumberBuffer`). |
| [`crates/dotnet/src/parse.rs`](../crates/dotnet/src/parse.rs) | 307 | Number parsing (.NET Fx `Number.ParseNumber` + `NumberToDouble/Decimal`), InvariantCulture. |
| [`crates/dotnet/src/timespan.rs`](../crates/dotnet/src/timespan.rs) | 197 | `System.TimeSpan` (.NET Fx 4.x semantics). |
| [`crates/dotnet/src/unicode_tables.rs`](../crates/dotnet/src/unicode_tables.rs) | 511 | GENERATED by tools/golden/dotnet/run.sh from Mono Char.IsLetter/IsDigit/IsNumber/IsWhiteSpace. Do not edit. |
| [`crates/dotnet/tests/golden.rs`](../crates/dotnet/tests/golden.rs) | 309 | Golden tests: compare against tables produced by Mono (tools/golden/dotnet/run.sh). |
| [`crates/dotnet/tests/semantics.rs`](../crates/dotnet/tests/semantics.rs) | 251 | Hand-computed expectations from .NET Framework 4.x behaviour (not Mono-derived). |

### `crates/expr`

| file | lines | purpose |
|---|---:|---|
| [`crates/expr/src/expand.rs`](../crates/expr/src/expand.rs) | 236 | Infix text normalisation (`Expand`, `ScientificNotationCorrection`) of `Tools/ExpressionEvaluator.cs`. Regexes are hand-coded to keep .NET semantics and avoid dependencies. |
| [`crates/expr/src/lib.rs`](../crates/expr/src/lib.rs) | 581 | Port of `Tools/ExpressionEvaluator.cs`: infix expression evaluator for custom-button G-code. |
| [`crates/expr/src/num.rs`](../crates/expr/src/num.rs) | 30 | .NET number glue for `Tools/ExpressionEvaluator.cs`: `double.Parse` / `double.ToString` wrappers. |
| [`crates/expr/tests/golden.rs`](../crates/expr/tests/golden.rs) | 98 | Golden test: `Expression` vs original C# (Mono) output in tests/golden/expr/cases.tsv. |

### `crates/gcode`

| file | lines | purpose |
|---|---:|---|
| [`crates/gcode/src/command.rs`](../crates/gcode/src/command.rs) | 839 | Port of `GrblCommand.cs`: `Element`, `GrblCommand`, `GrblMessage`, `IGrblRow`. |
| [`crates/gcode/src/config.rs`](../crates/gcode/src/config.rs) | 776 | Types from `Core/GrblCore.cs`: `Firmware`, `GrblVersionInfo`, `GrblConfST`, obsolete `GrblConf`, `GPoint`, the static `GrblCore.Configuration` property and the `GCODE_STD_*` constants. |
| [`crates/gcode/src/csvd.rs`](../crates/gcode/src/csvd.rs) | 40 | Port of the `CSVD` static class of `GrblCommand.cs` on top of `lg_base::csv`. |
| [`crates/gcode/src/draw.rs`](../crates/gcode/src/draw.rs) | 354 | Port of the GDI preview drawing of `GrblFile.cs` (`DrawOnGraphics`, `DrawJobPreview`, `DrawJobRange`, `ScaleAndPosition`, `DrawString`). |
| [`crates/gcode/src/error.rs`](../crates/gcode/src/error.rs) | 32 | .NET exception stand-ins for the C# paths that throw (OverflowException etc.). |
| [`crates/gcode/src/generators.rs`](../crates/gcode/src/generators.rs) | 278 | Port of the test-pattern generators of `GrblFile.cs`: cutting test, greyscale (power/speed) test, shake test. |
| [`crates/gcode/src/grbl_file.rs`](../crates/gcode/src/grbl_file.rs) | 400 | Port of `GrblFile.cs` (core part): command list, load/save, analysis, quadrant, events. |
| [`crates/gcode/src/l2l.rs`](../crates/gcode/src/l2l.rs) | 417 | Port of the raster line-to-line part of `GrblFile.cs`: `L2LConf`, `ColorSegment` family, `LoadImageL2L`, `ImageLine2Line`, `OptimizeLine2Line`, `GetSegments`, `ExtractSegment`, `GetColor`. |
| [`crates/gcode/src/lib.rs`](../crates/gcode/src/lib.rs) | 29 | `lg-gcode`: G-code commands, state builder, `GrblFile` (analysis, raster/vector generators), `GrblConfST`, `GrblVersionInfo`. Ports `GrblCommand.cs`, `StateBuilder.cs`, `GrblFile.cs` and the config types of `Core/GrblCore.cs`. |
| [`crates/gcode/src/optimize.rs`](../crates/gcode/src/optimize.rs) | 116 | Port of `OptimizePaths` / `ParallelOptimizePaths` of `GrblFile.cs`: nearest-neighbour ordering of potrace path lists. |
| [`crates/gcode/src/program_range.rs`](../crates/gcode/src/program_range.rs) | 184 | Port of `ProgramRange` (bottom of `GrblFile.cs`). |
| [`crates/gcode/src/state_builder.rs`](../crates/gcode/src/state_builder.rs) | 799 | Port of `StateBuilder.cs`: `StatePositionBuilder`, `StateBuilder`, `ModalElement`, `G2G3Helper`. |
| [`crates/gcode/src/text.rs`](../crates/gcode/src/text.rs) | 102 | .NET string helpers (`Trim()`, `ToUpper()`, `ToLower()`) with Fx semantics. |
| [`crates/gcode/src/vector.rs`](../crates/gcode/src/vector.rs) | 291 | Port of the vector loaders of `GrblFile.cs`: `LoadImagePotrace`, `LoadImportedSVG`, `LoadImageCenterline`. |
| [`crates/gcode/tests/common/mod.rs`](../crates/gcode/tests/common/mod.rs) | 142 | Shared helpers for the golden tests: settings sandbox, conf parsing, escaping. |
| [`crates/gcode/tests/golden_analyze.rs`](../crates/gcode/tests/golden_analyze.rs) | 151 | gcode_analyze + state_builder goldens. |
| [`crates/gcode/tests/golden_config.rs`](../crates/gcode/tests/golden_config.rs) | 120 | config golden: `GrblConfST` parsing + derived properties, `GrblVersionInfo` comparisons. |
| [`crates/gcode/tests/golden_generators.rs`](../crates/gcode/tests/golden_generators.rs) | 69 | generators golden: cutting test, greyscale test, shake test. |
| [`crates/gcode/tests/golden_l2l.rs`](../crates/gcode/tests/golden_l2l.rs) | 76 | l2l golden: `LoadImageL2L` over synthetic bitmaps. |
| [`crates/gcode/tests/golden_parse.rs`](../crates/gcode/tests/golden_parse.rs) | 106 | gcode_parse golden: byte compare of the dump produced from `GrblCommand`. |
| [`crates/gcode/tests/golden_potrace.rs`](../crates/gcode/tests/golden_potrace.rs) | 133 | potrace golden: `LoadImagePotrace` (vector fills; raster fills are smoke-tested only because GDI+ resampling is not reproducible). |
| [`crates/gcode/tests/misc.rs`](../crates/gcode/tests/misc.rs) | 176 | Hand-written checks: clone-shared result, messages, SaveGCODE, drawing, svg glue, optimizer. |

### `crates/hershey`

| file | lines | purpose |
|---|---:|---|
| [`crates/hershey/src/data.rs`](../crates/hershey/src/data.rs) | 197 | Glyph tables copied verbatim from Hershey/Hershey.cs (`hor` and `ver`). |
| [`crates/hershey/src/lib.rs`](../crates/hershey/src/lib.rs) | 163 | Port of `Hershey/Hershey.cs`: Hershey single-stroke text as G-code lines. |
| [`crates/hershey/tests/golden.rs`](../crates/hershey/tests/golden.rs) | 64 | Golden test: `create_string` vs original C# (Mono) output in tests/golden/hershey/cases.txt. |

### `crates/potrace`

| file | lines | purpose |
|---|---:|---|
| [`crates/potrace/src/biarc/algorithm.rs`](../crates/potrace/src/biarc/algorithm.rs) | 104 | Port of `BezierToBiarc/Algorithm.cs`. |
| [`crates/potrace/src/biarc/arc.rs`](../crates/potrace/src/biarc/arc.rs) | 135 | Port of `BezierToBiarc/Arc.cs` and `BiArc.cs`. |
| [`crates/potrace/src/biarc/complex.rs`](../crates/potrace/src/biarc/complex.rs) | 133 | Port of `BezierToBiarc/Complex.cs` (arithmetic, Abs, Sqrt and friends; formatting and the implicit conversions are not needed in Rust). |
| [`crates/potrace/src/biarc/cubic_bezier.rs`](../crates/potrace/src/biarc/cubic_bezier.rs) | 68 | Port of `BezierToBiarc/CubicBezier.cs`. |
| [`crates/potrace/src/biarc/line.rs`](../crates/potrace/src/biarc/line.rs) | 61 | Port of `BezierToBiarc/Line.cs`: line in point-slope form, f32. |
| [`crates/potrace/src/biarc/mod.rs`](../crates/potrace/src/biarc/mod.rs) | 15 | Port of `CsPotrace/BezierToBiarc/*.cs` (bezier to biarc approximation, float precision like C#). |
| [`crates/potrace/src/biarc/vector2.rs`](../crates/potrace/src/biarc/vector2.rs) | 81 | Port of `BezierToBiarc/Vector2.cs`: f32 2D vector. Only members used by the port are ported. C# `Tuple.cs` is replaced by Rust tuples. |
| [`crates/potrace/src/biarc_free.rs`](../crates/potrace/src/biarc_free.rs) | 37 | `PotraceClipper.Hilbert`: Hilbert curve generator (f32 like C#). |
| [`crates/potrace/src/clipper.rs`](../crates/potrace/src/clipper.rs) | 328 | Port of `CsPotrace/PotraceClipper.cs`: vector filling of a traced shape with Clipper. |
| [`crates/potrace/src/export.rs`](../crates/potrace/src/export.rs) | 73 | Port of `CsPotrace/CsPotraceExport.cs`: `Export2GDIPlus` rendered with tiny-skia. |
| [`crates/potrace/src/export_gcode.rs`](../crates/potrace/src/export_gcode.rs) | 144 | Port of `CsPotrace/CsPotraceExportGCODE.cs`: curve lists to G-code lines (arcs via biarcs). |
| [`crates/potrace/src/gdi_path.rs`](../crates/potrace/src/gdi_path.rs) | 196 | Emulation of the parts of GDI+ `GraphicsPath` used by `CsPotrace` (AddLine, AddBezier, Flatten, PathPoints), all in f32. |
| [`crates/potrace/src/lib.rs`](../crates/potrace/src/lib.rs) | 16 | `lg-potrace`: port of `CsPotrace/*` (Potrace tracer, exports, bezier-to-biarc, Clipper fill). |
| [`crates/potrace/src/trace.rs`](../crates/potrace/src/trace.rs) | 1447 | Port of `CsPotrace/CsPotrace.cs`: Potrace bitmap tracer (bitmap -> paths -> curves). |
| [`crates/potrace/tests/golden_curves.rs`](../crates/potrace/tests/golden_curves.rs) | 90 | Exact comparison of `Tracer::trace` against `tests/golden/potrace_curves` (Mono oracle). |
| [`crates/potrace/tests/golden_pipeline.rs`](../crates/potrace/tests/golden_pipeline.rs) | 146 | Whole-pipeline pieces of `tests/golden/potrace` (full `LoadImagePotrace` G-code, Mono oracle). |
| [`crates/potrace/tests/unit.rs`](../crates/potrace/tests/unit.rs) | 46 | Small unit tests: GDI+ path emulation, Hilbert, GDI+ export smoke test. |

### `crates/raster`

| file | lines | purpose |
|---|---:|---|
| [`crates/raster/src/color_matrix.rs`](../crates/raster/src/color_matrix.rs) | 66 | Port of GDI+ `ColorMatrix` / `ImageAttributes.SetColorMatrix` pixel math (used by ImageTransform.cs). |
| [`crates/raster/src/dithering/argb_color.rs`](../crates/raster/src/dithering/argb_color.rs) | 25 | Port of `RasterConverter/Dithering/ArgbColor.cs` (Cyotek, MIT). |
| [`crates/raster/src/dithering/error_diffusion.rs`](../crates/raster/src/dithering/error_diffusion.rs) | 150 | Port of `IErrorDiffusion.cs`, `ErrorDiffusionDithering.cs` and the matrix based dithering classes (Atkinson, Burkes, FloydSteinberg, JarvisJudiceNinke, Sierra2, Sierra3, SierraLite, Stucki). |
| [`crates/raster/src/dithering/image_utilities.rs`](../crates/raster/src/dithering/image_utilities.rs) | 27 | Port of `RasterConverter/Dithering/ImageUtilities.cs`. |
| [`crates/raster/src/dithering/mod.rs`](../crates/raster/src/dithering/mod.rs) | 10 | Port of `RasterConverter/Dithering/*` (Cyotek dithering classes). |
| [`crates/raster/src/dithering/random.rs`](../crates/raster/src/dithering/random.rs) | 183 | Port of `RandomDithering.cs` plus .NET Framework `System.Random` (Knuth subtractive). |
| [`crates/raster/src/image_transform.rs`](../crates/raster/src/image_transform.rs) | 468 | Port of `RasterConverter/ImageTransform.cs` (pixel ops on straight-alpha `RgbaImage`). |
| [`crates/raster/src/lib.rs`](../crates/raster/src/lib.rs) | 9 | `lg-raster`: port of `RasterConverter/ImageTransform.cs` and `RasterConverter/Dithering/*`. |
| [`crates/raster/src/resize.rs`](../crates/raster/src/resize.rs) | 171 | Port of `ImageTransform.ResizeImage` (GDI+ `Graphics.DrawImage` scaling). |
| [`crates/raster/tests/golden.rs`](../crates/raster/tests/golden.rs) | 186 | Golden tests against the original LaserGRBL run under Mono + libgdiplus. Data: `tests/golden/raster/{input,output,manifest.txt}` (see `tools/golden/raster/run.sh`). |

### `crates/services`

| file | lines | purpose |
|---|---:|---|
| [`crates/services/src/autotrace.rs`](../crates/services/src/autotrace.rs) | 189 | Port of `Autotrace/Autotrace.cs`: run the external `autotrace` binary on a bitmap. |
| [`crates/services/src/custom_button.rs`](../crates/services/src/custom_button.rs) | 682 | Port of `CustomButton.cs`: `CustomButton` and the `CustomButtons` list. |
| [`crates/services/src/flash_grbl.rs`](../crates/services/src/flash_grbl.rs) | 270 | Port of the logic of `Firmware/FlashGrbl.cs` (the form itself is UI). |
| [`crates/services/src/hotkeys.rs`](../crates/services/src/hotkeys.rs) | 884 | Port of `HotKeysManager.cs`: hotkey model, default bindings, key event dispatch (pure). |
| [`crates/services/src/image_processor/file_dpi.rs`](../crates/services/src/image_processor/file_dpi.rs) | 159 | `Bitmap.HorizontalResolution` of the file (GDI+ reads it from the image header). |
| [`crates/services/src/image_processor/job.rs`](../crates/services/src/image_processor/job.rs) | 384 | Worker side of `ImageProcessor`: `CreatePreview`, `DoTrueWork` and the bitmap pipeline. A [`Job`] is the C# `Clone()` of the processor that a thread works on. |
| [`crates/services/src/image_processor/mod.rs`](../crates/services/src/image_processor/mod.rs) | 723 | Port of `RasterConverter/ImageProcessor.cs`. |
| [`crates/services/src/image_processor/overlay.rs`](../crates/services/src/image_processor/overlay.rs) | 255 | Pixel drawing helpers of the previews (GDI+ `Graphics` calls reproduced by direct pixel writes). |
| [`crates/services/src/image_processor/tests.rs`](../crates/services/src/image_processor/tests.rs) | 342 | Unit tests of the image processor (preview + generator wiring). |
| [`crates/services/src/layer_presets.rs`](../crates/services/src/layer_presets.rs) | 341 | Named layer presets (passes, speed, power %, laser mode, filter) stored in `LayerPresets.json`; replaces `PSHelper/MaterialDB.cs` (intentional deviation). |
| [`crates/services/src/lib.rs`](../crates/services/src/lib.rs) | 49 | `lg-services`: non-UI services of LaserGRBL that do not need the `GrblCore` engine. |
| [`crates/services/src/project.rs`](../crates/services/src/project.rs) | 592 | Port of `Tools/Project.cs`: LaserGRBL project files (`.lps`). |
| [`crates/services/src/wifi_discovery.rs`](../crates/services/src/wifi_discovery.rs) | 897 | Port of `WiFiDiscovery/IPAddressHelper.cs` and the non-UI parts of `DiscoveryForm.cs`. |

### `crates/svg2gcode`

| file | lines | purpose |
|---|---:|---|
| [`crates/svg2gcode/src/bezier.rs`](../crates/svg2gcode/src/bezier.rs) | 72 | Port of `SvgConverter/BezierTools.cs` (adaptive cubic bezier flattening). |
| [`crates/svg2gcode/src/color.rs`](../crates/svg2gcode/src/color.rs) | 97 | `System.Drawing.ColorTranslator.FromHtml` (+ `ColorConverter` string rules) as used by `parseRgb` in GCodeFromSVG.cs. |
| [`crates/svg2gcode/src/color_table.rs`](../crates/svg2gcode/src/color_table.rs) | 211 | Colour name tables dumped from System.Drawing (Mono) KnownColor / HTML system colours: (lower-case name, r, g, b). |
| [`crates/svg2gcode/src/convert.rs`](../crates/svg2gcode/src/convert.rs) | 1683 | Port of `SvgConverter/GCodeFromSVG.cs` (SVG -> G-code). Floats stay `f32`, doubles `f64`. |
| [`crates/svg2gcode/src/gcode.rs`](../crates/svg2gcode/src/gcode.rs) | 680 | Port of `SvgConverter/gcodeRelated.cs` (static class `gcode`: G-code writer with modal compression). |
| [`crates/svg2gcode/src/lib.rs`](../crates/svg2gcode/src/lib.rs) | 53 | `lg-svg2gcode`: port of LaserGRBL `SvgConverter/{GCodeFromSVG,gcodeRelated,BezierTools}.cs` plus the WPF `Matrix`/`Point`/`Vector` and `ColorTranslator.FromHtml` pieces they rely on. |
| [`crates/svg2gcode/src/matrix.rs`](../crates/svg2gcode/src/matrix.rs) | 438 | Port of WPF `System.Windows.Media.Matrix`, `Point`, `Vector` (WindowsBase) used by GCodeFromSVG.cs / BezierTools.cs. |
| [`crates/svg2gcode/tests/golden_svg.rs`](../crates/svg2gcode/tests/golden_svg.rs) | 93 | Golden test: full SVG conversions vs the original (Mono) in tests/golden/svg/ (WP-H). |
| [`crates/svg2gcode/tests/layers.rs`](../crates/svg2gcode/tests/layers.rs) | 123 | Layered conversion tests: layer listing, regression vs plain conversion, per-layer values. |
| [`crates/svg2gcode/tests/unit_convert.rs`](../crates/svg2gcode/tests/unit_convert.rs) | 78 | Extra full conversions vs Mono (tools/golden/svg2gcode -> tests/golden/svg_unit/convert/). |
| [`crates/svg2gcode/tests/unit_oracle.rs`](../crates/svg2gcode/tests/unit_oracle.rs) | 218 | Unit oracle tests vs Mono (tools/golden/svg2gcode -> tests/golden/svg_unit/). |

<!-- END GENERATED FILE TABLE -->
