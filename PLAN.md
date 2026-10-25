# LaserGRBL → Rust port: master plan

Source of truth: `../LaserGRBL/LaserGRBL` (C#, .NET Framework 4.0, WinForms, v7.14.1).
Target: `LaserGRBL_linux` (Rust 2024 workspace, egui/eframe UI, runs on Linux and Windows).

## 0. Ground rules (every work package)

1. **Behavioural fidelity first.** G-code generation, G-code parsing/analysis, time estimation and the
   GRBL streaming protocol must behave *exactly* like the C# code. Port algorithms line by line. Keep the
   same numeric types: C# `float` → `f32`, `double` → `f64`, `decimal` → `rust_decimal::Decimal`,
   `int` → `i32`, `long` → `i64`. Do not "improve" algorithms, do not fix upstream bugs silently. If you find
   an upstream bug, keep the behaviour and add a `// NOTE(upstream): ...` comment.
2. **Number formatting / parsing goes through `lg-dotnet`** (see §3). Never use `format!("{}", f64)` for
   anything that ends up in G-code or user-visible numbers that the original formats.
3. **Code style** (from AGENTS.md): readable structured Rust; brief caveman-style comments for senior devs;
   one-line comment per non-obvious block inside functions; every function docstring explains each argument
   and the return value, concisely. Each Rust module starts with a `//!` line naming the C# file(s) it ports.
4. **Safety:** never use `sudo`, never `git push`, never `git commit` (the user commits). Do not touch
   `../LaserGRBL` (read only). Temporary files go to the session scratchpad, not `/tmp`.
5. Only touch files of your own work package unless the package says otherwise. Run `cargo build -p <crate>`,
   `cargo test -p <crate>` and `cargo clippy -p <crate>` for your crate before reporting.
6. No new external dependencies beyond those in the root `Cargo.toml` `[workspace.dependencies]` without
   asking the planner.

## 1. Workspace layout

| crate | path | ports | deps |
|---|---|---|---|
| `lg-dotnet` | crates/dotnet | .NET Framework numeric semantics (formatting, parsing, Math.Round, decimal) | rust_decimal |
| `lg-clipper` | crates/clipper | `Clipper/Clipper.cs` (Clipper 6.4.2, `use_lines` on, int64 coords) | – |
| `lg-potrace` | crates/potrace | `CsPotrace/*` incl. `BezierToBiarc/*`, `PotraceClipper.cs`, GDI+ path flattening | dotnet, clipper, resvg(tiny-skia) |
| `lg-expr` | crates/expr | `Tools/ExpressionEvaluator.cs` | dotnet |
| `lg-hershey` | crates/hershey | `Hershey/Hershey.cs` (returns G-code lines as `String`) | dotnet |
| `lg-svg2gcode` | crates/svg2gcode | `SvgConverter/GCodeFromSVG.cs`, `gcodeRelated.cs`, `BezierTools.cs` | dotnet, roxmltree, regex |
| `lg-raster` | crates/raster | `RasterConverter/ImageTransform.cs`, `Dithering/*` (pixel ops on RGBA buffers) | dotnet, image, resvg |
| `lg-base` | crates/base | `Settings.cs` (+paths DataPath/TempPath/ExePath/resource dir), `Logger/*`, `Tools/{HiResTimer,ThreadClass,Cronometro,MathHelper,MobileAverageCalculator,WordWrap,RetainedSetting}.cs`, `Utils.TimeSpanToString`, `CSV/*` (+`CSVD`), NRBF (BinaryFormatter) reader/writer (`Tools/Serializer.cs` replacement), `Sound/SoundEvent.cs`, `Telegram.cs`, `UrlManager.cs` | dotnet |
| `lg-gcode` | crates/gcode | `GrblCommand.cs`, `StateBuilder.cs`, `GrblFile.cs` (+`ProgramRange`, L2L, generators, potrace/svg/centerline loaders), `GrblConfST`/`GrblConf`, `GrblVersionInfo`, `GPoint`, `Firmware` enum (from `Core/GrblCore.cs`) | dotnet, base, hershey, potrace, svg2gcode, raster |
| `lasergrbl-core` | crates/core | `GrblCore` engine (+Marlin/Smoothie/Vigo, `TimeProjection`, `LaserLifeHandler`, `ThreadingMode`, `ContinuousJog`), com wrappers, `Grblv11Emulator`/`WebsocketEmulator`, hotkeys model, custom buttons model, material DB, usage counters, sincro start, project files (.lps), firmware flasher (avrdude), wifi discovery, autotrace runner, `ImageProcessor` logic | all above |
| `laiserctrl` | app | egui UI: every Form / UserControl, preview renderer (glow + CPU fallback), icons, i18n | core + egui stack |

Leaf crates (dotnet, clipper, potrace, expr, hershey, svg2gcode, raster) never depend on base/gcode/core.
Things they need from settings or the GRBL configuration (e.g. `Settings.GetObject("Support Hardware PWM")`,
`GrblCore.Configuration.MaxPWM`, firmware type) are passed in as plain option structs whose fields are
named after the C# setting keys; the caller in `lg-gcode` fills them from `lg-base::settings` exactly where/when
the C# code reads them (respect C# `static` initialisation quirks, e.g. `gcodeRelated.firmwareType` is read
once per process).

## 2. Runtime architecture

### 2.1 Core threading (mirrors C#)
* `GrblCore` is shared as `Arc<GrblCore>`. All mutable state that C# guards with `lock(this)` lives in one
  `Mutex<CoreState>`; methods that C# runs under `lock(this)` take that mutex. TX and RX are background
  threads (`ThreadObject` port: loop + sleep time + must-exit flag), exactly like `Tools.ThreadObject`.
* Com wrappers: `trait ComWrapper: Send + Sync` with interior mutability:
  `configure(ComConfig)`, `open()`, `close(auto)`, `is_open()`, `write_byte(u8)`, `write_bytes(&[u8])`,
  `write_str(&str)`, `read_line_blocking() -> Result<Option<String>>` (returns `Ok(None)` on read timeout so the
  RX loop can re-check `is_open`, `Err` on I/O error), `has_data()`.
  Reader and writer must not block each other (serial: `try_clone()` handles; telnet: cloned `TcpStream`).
* Firmware variants (C# subclasses `MarlinCore`, `SmoothieCore`, `VigoCore` overriding virtual methods) are a
  `Firmware` enum stored in the core; each C# `virtual` method becomes a Rust method containing a `match` on
  the firmware type, so overriding behaviour lives next to the base behaviour.
* Events (`MachineStatusChanged`, `OnFileLoading/OnFileLoaded`, `OnOverrideChange`, `OnLoopCountChange`,
  `IssueDetected`, `JogStateChange`, `OnAutoSizeDrawing`, `OnZoomIn/OutDrawing`, `OnProgramEnded`) are sent as
  `CoreEvent` values over a `crossbeam_channel::Sender`. The UI drains the channel every frame. Additionally the
  UI polls getters every frame (like `UpdateTimer_Tick`, 200 ms in C#; egui requests repaint every 100 ms).
* Blocking dialogs the C# core opens itself (`ResumeJobForm`, `RunFromPositionForm`, `SafetyCountdown`,
  `LaserSelector`, `MessageBox` in `HKDisconnect`/`OpenFile`/`OpenCom`, `RasterToLaserForm`,
  `SvgToGCodeForm`) are inverted: the core exposes the *decision function* and the *continuation*; the UI shows
  the dialog and calls the continuation. Example: `run_program()` → `RunDecision::{Start, AskResume{..}}`;
  UI then calls `run_program_from_start(homing)` or `continue_program_from_known(pos, homing, setwco)`.
  `SafetyCountdown::can_go()` becomes a UI step executed before those continuations (same order as C#).
* Long jobs (`GrblFile::load_*`) run on worker threads exactly like `SafeLoadFile` / ImageProcessor threads.
  `GrblFile` lives in `Arc<RwLock<GrblFile>>` inside the core; `InUse`/loading flag semantics preserved.

### 2.2 Persistence (paths)
* `DataPath` = `dirs::config_dir()/L-ai-SERctrl` (Linux: `~/.config/L-ai-SERctrl`; Windows: `%APPDATA%\L-ai-SERctrl`; old LaserGRBL folder migrated once,
  same folder as the original). `TempPath` = `std::env::temp_dir()/LaserGRBL`.
* Settings: same keys as C#, stored as JSON (`LaserGRBL.Settings.json`), typed get/set with default,
  debounced save 2 s after the last change, flush on exit (C# `Settings.cs` semantics incl. `IsNewFile`,
  `PrevVersion`). Values are a serde enum covering every type C# stores (bool, i32, f32, f64, decimal, string,
  enums by name, TimeSpan, Version, object[] for window geometry, ThreadingMode, GrblVersionInfo, GrblConfST,
  HotKeysManager list, Point/Size/Color, …). If `LaserGRBL.Settings.bin` (C# BinaryFormatter) exists and the
  JSON file does not, import what can be decoded with the NRBF reader (WP-C6).
* Custom buttons / laser life / usage stats / emulator config: JSON files in DataPath (C# used
  BinaryFormatter). Import of Windows `.zbn` (gzip + BinaryFormatter) and `.lps` project files via NRBF
  reader. Export writes `.zbn` as **gzip + NRBF** (writer for the fixed `List<CustomButton>` shape) so files
  stay interchangeable with Windows LaserGRBL.

### 2.3 UI (egui/eframe, glow backend)
* One `eframe::App`. Main window = `MainForm`: menu bar, status bar, left column (`ConnectLogForm` on top,
  `JogForm` below, splitter width persisted as "MainForm Splitter Position"), right = `PreviewForm`.
* Every C# `Form` becomes a Rust struct with `show(&mut self, ctx) -> Option<Outcome>` rendered as an
  `egui::Modal`/`Window`. Layout, captions, tooltips, enable/visible rules, default values and setting keys
  must mirror the `*.Designer.cs` + `*.cs` code of that form.
* Preview: `GrblPanel3D` (OpenGL in C#) → `egui::PaintCallback` with `eframe::glow` (vertex buffers, same
  colors/zoom/pan/rulers/grid/cursor/bounding box/executed-commands coloring). `GrblPanel` (GDI mode)
  → CPU rasteriser using tiny-skia into a texture (port of `GrblFile.DrawOnGraphics`). Graphic mode setting
  chooses (`GDI` → CPU, others → GL).
* Color schemes: port `ColorScheme.cs` (7 schemes) and apply to egui `Visuals` + custom painting.
* Icons: `SvgIcons` (`resources/svgicons/*.data/.header`, rendered with resvg, tinted per `IconsMgr` color
  tables) and `LegacyIcons` (base64 PNG). Setting `LegacyIcons` selects loader.
* i18n: `tools/resx2json.py` converts every `*.resx` (Strings + every form) into
  `resources/i18n/<culture>.json` (`{"Strings.Key": "...", "MainForm.MnConnect.Text": "..."}`), embedded with
  `include_str!`. `tr(key)` falls back to invariant (`resources/i18n/en.json`). Language menu like C#
  (stored as setting "UserLanguage", restart required as in C#).
* Fonts: egui default fonts + at runtime try to load a system CJK font (Noto Sans CJK / WenQuanYi / Droid
  Sans Fallback) when present, for zh/ja languages.

## 3. .NET semantics cheat sheet (`lg-dotnet` API — all crates use it)

```rust
pub use rust_decimal::Decimal;
pub fn fmt_double(v: f64) -> String;                 // double.ToString(InvariantCulture): .NET Fx "G" = 15 significant digits, "E+XX" form when exp < -5 or >= 15
pub fn fmt_float(v: f32) -> String;                  // float.ToString(InvariantCulture): 7 significant digits
pub fn fmt_double_custom(v: f64, fmt: &str) -> String; // custom numeric formats used by LaserGRBL: "0", "0.0", "0.00", "0.000", "0.###", "#0.00", "0.0#", "#,##0" ... (.NET Fx: value first converted to 15 sig. digits, then rounded half-away-from-zero on the decimal digit string)
pub fn fmt_float_custom(v: f32, fmt: &str) -> String;  // same with 7 significant digits
pub fn fmt_decimal(v: Decimal) -> String;            // decimal.ToString(): keeps scale ("1000.000")
pub fn fmt_decimal_custom(v: Decimal, fmt: &str) -> String;
pub fn fmt_fixed(v: f64, decimals: usize) -> String; // ToString("F2") / "{0:F2}"
pub fn round_double(v: f64, digits: i32) -> f64;     // Math.Round(double,int): .NET Fx InternalRound = scale by 10^d, round-half-even, unscale
pub fn round_double_away(v: f64, digits: i32) -> f64; // Math.Round(.., MidpointRounding.AwayFromZero)
pub fn round_half_even(v: f64) -> f64;               // Math.Round(double)
pub fn parse_decimal(s: &str) -> Option<Decimal>;    // decimal.Parse(s, InvariantCulture) NumberStyles.Number (lead/trail white, leading sign, decimal point, thousands)
pub fn parse_double(s: &str) -> Option<f64>;         // double.Parse(s, InvariantCulture) NumberStyles.Float|AllowThousands
pub fn parse_float(s: &str) -> Option<f32>;
pub fn parse_int(s: &str) -> Option<i32>;            // int.Parse (NumberStyles.Integer)
pub fn f64_to_i32(v: f64) -> i32;                    // C# (int)double unchecked cast (truncate; NaN/overflow -> i32::MIN)
pub fn f32_to_i32(v: f32) -> i32;
pub fn decimal_to_f64(d: Decimal) -> f64;            // (double)decimal
pub fn f64_to_decimal(v: f64) -> Decimal;            // (decimal)double: .NET rounds to 15 significant digits
pub fn f32_to_decimal(v: f32) -> Decimal;            // (decimal)float: 7 significant digits
pub struct TimeSpan(i64 /*ticks of 100ns*/);          // FromMinutes/FromSeconds/FromMilliseconds (.NET Fx rounds to whole ms!), TotalX, Add/Sub, Display "c" format
```
Other rules: C# `string.Split(char)` keeps empty entries; `Trim()` trims Unicode whitespace;
`ToUpper()/ToLower()` → `to_uppercase()/to_lowercase()`; `Char.IsLetter` → `char::is_alphabetic`;
`Char.IsNumber` → `char::is_numeric`; `Char.IsDigit` → `is_ascii_digit` is NOT equal (Unicode Nd) →
use `char::is_numeric` restricted to Nd via helper `dotnet::is_digit`. Integer `/` and `%` behave the same.
`String.Format("{0}", x)` for `int` = decimal digits, for `float/double` → `fmt_float/fmt_double`, for
`decimal` → `fmt_decimal`. Interpolated strings `$"..{x}.."` follow the same rule. NOTE: the C# app formats
some non-invariant interpolations with the user's culture (bug on e.g. Italian Windows); the port always
uses invariant culture.

## 4. Golden testing (reference = the original C# compiled with Mono)

* `../.mono-build/env.sh` sets up user-space Mono 6.12 + libgdiplus (no sudo) and `LASERGRBL_EXE`
  (original built from an untouched copy; only JPEG resources stripped and one unused `using` removed).
* `tools/golden/` holds a C# harness (`Golden.cs`) compiled with `mcs -r:$LASERGRBL_EXE ...` that calls the
  original code (reflection for non-public members) and writes `tests/golden/**` files. Run with
  `HOME=<scratch>` so the original's settings go to a throw-away folder.
* Rust integration tests (`crates/*/tests/golden_*.rs`) compare byte-for-byte against those files.
* Known non-oracle areas (documented, not byte-compared): GDI+ bitmap resampling (`HighQualityBicubic`),
  anti-aliased GDI+ fills, font rendering. Mono's `Math.Round(double,int)` differs from .NET Framework on
  exact-binary midpoints; harness avoids or documents those cases.

## 5. Work packages

### Phase 1 (parallel)
* **WP-A dotnet**: implement `lg-dotnet` per §3 with exhaustive unit tests; golden values for formatting
  generated with Mono (`tests/golden/dotnet/*.txt`).
* **WP-B clipper**: faithful port of Clipper 6.4.2 C# (`Clipper`, `ClipperOffset`, `PolyTree/PolyNode`,
  `IntPoint` with `(cInt)double` truncation constructor, `Execute` overloads, `OpenPathsFromPolyTree`,
  `ClosedPathsFromPolyTree`, `PolyTreeToPaths`, `SimplifyPolygon(s)`, `CleanPolygon(s)`, `MinkowskiSum/Diff`,
  `Orientation`, `Area`, `PointInPolygon`, `ReversePaths`). API names in snake_case mirroring C#.
* **WP-D expr+hershey**: `Tools/ExpressionEvaluator.cs` → `lg-expr` (`Expression::new(text)`,
  `add_set_variable(name, f64)`, `exist_token(name)`, `evaluate_d() -> Result<f64>`), `Hershey.cs` →
  `lg-hershey` (`create_string(text, x, y, f, s, center, laser_on) -> Vec<String>` returning raw G-code
  lines; caller wraps them in `GrblCommand`).
* **WP-F raster**: `ImageTransform.cs` + `Dithering/*` → `lg-raster` operating on `image::RgbaImage`
  (ColorMatrix maths reproduced exactly; resize = high quality bicubic approximation of GDI+
  HighQualityBicubic; `Fill` flood fill; `Outliner`; `Invert`; `Threshold`; `Whitenize`; `GrayScale` with all
  `Formula`s; `DitherImage` with every `DitheringMode`; rotate/flip helpers).
* **WP-G i18n+resources**: `tools/resx2json.py` (stdlib only), generated `resources/i18n/*.json`, culture list
  as in MainForm language menu; `resources/README.md` describing every resource and its origin.
* **WP-H golden harness**: `tools/golden/` C# harness + `tools/golden/run.sh` + committed outputs under
  `tests/golden/` for: dotnet formatting table; `GrblCommand` parsing (BuildHelper, SerialData, IsXxx flags);
  `GrblFile.LoadFile` analysis (command list, per-command TimeOffset ticks, EstimatedTime ticks, ranges,
  quadrant) for several `.nc` files incl. arcs/G92/relative; `LoadImageL2L` for synthetic bitmaps × directions
  (Horizontal/Vertical/Diagonal) × pwm on/off × unidirectional × firmware Grbl/Smoothie × G0-skip setting;
  `LoadImagePotrace` (vectorize) for synthetic bitmaps × filling modes (None, New*), spot removal, smoothing,
  optimize, optimize-fast; `GCodeFromSVG.convertFromText` for a set of SVGs (paths with every command, shapes,
  transforms, groups, viewBox/units, colors filter) incl. `resources/generator/LaserGRBL-accuracy-test-file.svg`;
  `GenerateCuttingTest`, `GenerateGreyscaleTest`, `GenerateShakeTest`; Hershey strings; ExpressionEvaluator;
  `ImageTransform` ops that are pure pixel maths (GrayScale via ColorMatrix, Threshold, Whitenize, Invert,
  dithering); Clipper intersection/offset cases; Potrace curve lists; `GrblConfST` parsing; `StatePositionBuilder`
  (`GetSettledModals`, resume command construction); `G2G3Helper` bbox; `EvaluateExpression` for custom
  button code; `TimeSpanToString`.

### Phase 2 (after Phase 1)
* **WP-C1 core-foundation**: settings, tools (HiResTimer, ThreadObject, PeriodicEventTimer, ElapsedFromEvent,
  MobileAverageCalculator, Cronometro, RulerStepCalculator, Utils.TimeSpanToString, MathHelper, WordWrap,
  RetainedSetting), logger (Logger, AsyncLogFile, ComLogger), CSV dictionaries + `CSVD`, `FormattedMessage`,
  `GrblCommand`/`Element`/`GrblMessage`/`IGrblRow`/`JogCommand`, `StateBuilder`/`StatePositionBuilder`/
  `G2G3Helper`, `GrblFile` (+`ProgramRange`, L2L, generators, analyze, save), `GrblConfST`, `GPoint`,
  `GrblVersionInfo`, `ColorScheme`.
* **WP-B2 potrace**: `CsPotrace.cs`, `CsPotraceExport.cs` (GDI exports → tiny-skia), `CsPotraceExportGCODE.cs`,
  `BezierToBiarc/*`, `PotraceClipper.cs`, GraphicsPath.Flatten port (libgdiplus algorithm). Then wire
  `GrblFile::load_image_potrace` + `load_image_centerline` in core.
* **WP-E svg2gcode**: GCodeFromSVG + gcodeRelated + BezierTools (+ WPF `Matrix`, `Point`, `Vector` emulation).
* **WP-C2 core-engine**: `GrblCore` (+ Marlin/Smoothie/Vigo), `TimeProjection`, `LaserLifeHandler`, com
  wrappers (UsbSerial/UsbSerial2/RJCPSerial → one serialport impl keeping the three protocol ids; Telnet;
  LaserWebESP8266 websocket; Emulator), `Grblv11Emulator` + `WebsocketEmulator`, `HotKeysManager` model,
  `CustomButtons` model, `SincroStart` (Unix socket / named pipe replacement), `Telegram` (ureq),
  `SoundEvent` (spawn `paplay`/`pw-play`/`aplay`; Windows: PowerShell SoundPlayer), `UsageStats` (local
  counters only), `Project` (.lps), NRBF reader/writer, `MaterialDB` (psh XML), `FlashGrbl` (avrdude runner),
  `IPAddressHelper` (wifi discovery scan), `Autotrace` runner (system `autotrace` binary), `ImageProcessor`
  logic (threads, preview generation, `DoTrueWork`).

### Phase 3 (after Phase 2): UI
* **WP-U1 shell**: main window, menus (all items, enable/visible rules from `TimerUpdate`), status bar,
  ConnectLogForm (+CommandLog control), JogForm (+Move2DControl), theming/ColorScheme, icons, i18n, hotkey
  dispatch, splash, legal disclaimer, license, about, exception manager, issue detector, multi-instance.
* **WP-U2 preview**: PreviewForm (+custom-buttons bar, overrides, progress, loop count), GrblPanel3D (glow),
  GrblPanel (CPU), CustomButtonForm, import/export buttons.
* **WP-U3 raster import**: RasterToLaserForm, ConvertSizeAndOptionForm (raster), ResolutionHelperForm,
  ImageTransform user control, PSHelper forms (material DB).
* **WP-U4 settings & dialogs**: SettingsForm, GrblConfig, HotkeyManagerForm, SvgToGCodeForm/ConvertSize (SVG),
  SaveOptionForm, ResumeJobForm, RunFromPositionForm, SafetyCountdown, generators (CuttingTest,
  PowerVsSpeedForm, ShakeTest), LaserUsage, LaserLifeEdit, LaserSelector, FlashGrbl, EmulatorUI,
  WiFi DiscoveryForm, OrturWiFiConfig(+Prompt), NewVersionForm (disabled, see §6), InputBox.

### Phase 4: audit & hardening
* Per-file audit agents walk `PORTING.md` (one row per C# file and per public/private member) and mark
  each item as ported (Rust path) or N/A (reason). Gaps get fixed.
* Full golden test run, clippy clean, manual run with the built-in emulator.

## 6. Intentionally not ported (justification)
* `SharpGL/*`, `SvgLibrary/*` (Svg.NET), `ComWrapper/WebSocket/*` (websocket-sharp), `ComWrapper/RJCP/*`,
  `ComWrapper/MySerial/*`: third-party libraries, replaced by glow, resvg/roxmltree, tungstenite, serialport.
* Windows-only: `Driver/CH341SER.EXE` + menu item (Linux kernel has ch341 driver; menu shows info instead),
  `Tools/TaskScheduler.cs` (NtSetTimerResolution), `Tools/WinAPI.cs` (sleep prevention → no-op /
  documented), `Tools/OSHelper.cs` (Windows version probing → `std::env::consts` + `/etc/os-release` for
  logs), `Tools/DataProtector.cs` (DPAPI → plain value in settings file with 0600 permissions),
  `Program.cs` NvAPI hacks, `GetCH340Version` WMI query.
* `AutoUpdate/GitHub.cs` + `NewVersionForm`: upstream releases are Windows installers; menu kept, check
  disabled with explanation.
* Remote telemetry (`UsageStats` upload, `LaserLifeHandler.RealDoSend`, toolbar news messages): this is an
  unofficial port and must not post to the upstream author's servers; local counters kept.
* `Autotrace/autotrace.exe`, `Firmware/avrdude.exe`, `libusb0.dll`: use system `autotrace` / `avrdude`.
