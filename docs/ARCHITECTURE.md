# Architecture

L-ai-SERcraft (binary `laisercraft`, package `laisercraft`) is a Rust/egui port of LaserGRBL 7.14.1 by Diego Settimi (author of the port: Andreas Pösch). Internal crates keep the names `lasergrbl-core` and `lg-*`.

Orientation for anyone (human or agent) changing this codebase. Read this first, then use
[`SOURCE_MAP.md`](SOURCE_MAP.md) to jump to the right file. `PORTING.md` maps every member of the original C#
code to its Rust location; `PLAN.md` records the original design decisions.

## 1. Ground rules that shape the code

1. **Behavioural fidelity to LaserGRBL 7.14.1 (C#).** Every module is a port of a named C# file (its `//!`
   header says which). Algorithms are ported line by line, keeping C# numeric types: `float`→`f32`,
   `double`→`f64`, `decimal`→`rust_decimal::Decimal`, `int`→`i32`. Upstream bugs are kept and marked
   `NOTE(upstream)`.
2. **Numbers that reach G-code, the serial line or the UI go through `lg-dotnet`** (`fmt_double`,
   `fmt_double_custom(v, "0.###")`, `fmt_decimal`, `parse_decimal`, `round_double`, `TimeSpan`, …), never
   through Rust `format!` on floats. Decimal arithmetic whose scale matters uses `dec_add/dec_sub/dec_mul/dec_div`.
3. **Golden tests are the contract.** Reference outputs in `tests/golden/**` were produced by running the
   original C# under Mono. A change that alters a golden diff is a behaviour change, not a refactor.
4. **Leaf crates know nothing about settings or the engine.** Options are passed in as structs named after the
   C# setting keys; `lg-gcode`/`lg-services`/`app` read settings and fill them.
5. **Licensing.** Every `.rs`/`.py`/`.sh` file starts with an SPDX + copyright header (ported files credit Diego
   Settimi and LaserGRBL, third-party-derived files also their authors); new files need one too
   (`python3 -I tools/check_headers.py`, run in CI). Authors and third-party licenses live in `NOTICE.md`,
   Rust dependencies in `THIRD_PARTY_RUST.md` (`tools/gen_third_party.py`). The license dialog embeds `NOTICE.md`.

## 2. Crate layers (dependencies point downwards only)

```
app (laisercraft binary, egui)                       forms, preview renderer, theme/icons/i18n
 ├── lasergrbl-core   (crates/core)                GrblCore engine, com links, emulator, laser life
 ├── lg-services      (crates/services)            image processor, hotkeys, custom buttons, projects, layer presets,
 │                                                 autotrace, avrdude, wifi discovery
 │      both ↓
 ├── lg-gcode         (crates/gcode)               GrblCommand, StateBuilder, GrblFile (+generators), GrblConfST
 │      ↓
 ├── lg-base          (crates/base)                settings, paths, logger, timers/threads, CSV, NRBF, sound
 │      ↓
 └── leaf crates      lg-svg2gcode  lg-potrace(→lg-clipper)  lg-raster  lg-hershey  lg-expr
                      all ↓
                      lg-dotnet                    .NET Framework number/char/TimeSpan semantics
```
`lasergrbl-core` and `lg-services` do not depend on each other; the app wires them (e.g. hotkey effects,
custom-button enable state, image processor → `GrblCore::loaded_file()`).

## 3. Runtime model

### 3.1 Threads
| thread | owner | role |
|---|---|---|
| UI (egui frame loop) | `app::main_form::MainForm::ui` | draws, drains `CoreEvent`s, polls core getters, repaints every 100 ms (C# `UpdateTimer`) |
| TX | `GrblCore` (`ThreadObject` "Serial TX Thread") | continuous jog, `send_line` when the buffer has room, status queries, hang detection, cooling cycles |
| RX | `GrblCore` ("Serial RX Thread") | `read_line_blocking` → `manage_received_line` (replies, realtime status, welcome/vendor messages) |
| link reader | each `ComWrapper` | reads the socket/serial port into a `LineQueue` so RX never blocks writers |
| file loaders | `GrblFile::spawn_load` | G-code/SVG load + `analyze` off the UI thread (C# `SafeLoadFile`); also the raster-image dock regeneration |
| image processor | `lg_services::image_processor` | preview generation and G-code generation for the raster dialog |
| helpers | core | delayed `$$`/`$I` refresh 500 ms after connect, SincroStart UDP listener, laser-life saves |

### 3.2 Engine (`crates/core/src/grbl_core`)
* `GrblCore` (`mod.rs`) is used as `Arc<GrblCore>`. Its public methods map 1:1 to C# `GrblCore` members.
* All mutable state that C# protects with `lock(this)` lives in one `Mutex<CoreState>` (`state.rs`), and the
  protocol logic runs as `CoreState` methods: `thread_tx`, `send_line`, `has_space_in_buffer`,
  `manage_received_line`, `manage_command_response`, `parse_machine_status`, `manage_overrides`,
  `detect_hang`, `on_program_end`, `execute_custom_code`. The mutex is not reentrant (unlike C# `lock`), so
  wait loops (`refresh_config`, `write_config`) release it between polls.
* Queues mirror C#: `queue` (to send), `pending` (sent, awaiting `ok`), `sent` (log rows), `retry`;
  `queue_ptr`/`sent_ptr` are swapped to private lists during `$$`/`$I`/config writes.
* Buffer accounting counts **UTF-16 units** of `SerialData` (C# `string.Length`); serial/telnet transmit
  ASCII-lossy like .NET.
* Firmware variants (`MarlinCore`, `SmoothieCore`, `VigoCore`) are a `Firmware` value fixed at construction;
  each C# `virtual` method is a `match` on it.
* UI callbacks are inverted: blocking dialogs the C# core opened itself are replaced by decision/continuation
  pairs (`run_program() -> RunDecision`, `open_file() -> OpenFileOutcome`, `laser_life::on_connect()`).
* Events (`CoreEvent`) go over a `crossbeam_channel`; the UI drains them each frame.

### 3.3 G-code model (`crates/gcode`)
* `GrblCommand` (`command.rs`): line text + lazily parsed elements (`build_helper`, keeps the C# abort-on-
  first-exception quirk). **Clones share the result holder** (`Arc<Mutex<Option<String>>>`): when the queued
  clone receives `ok`, the line in the loaded file shows as executed, and the preview recolours it through the
  `DisplayListLink` generation counter.
* `StatePositionBuilder` (`state_builder.rs`): modal state + position tracking, time estimation
  (`compute_execution_time`), arc helper (`G2G3Helper`).
* `GrblFile` (`grbl_file.rs` + `l2l.rs`, `vector.rs`, `generators.rs`, `optimize.rs`, `draw.rs`): the loaded
  program; every producer (G-code file, raster L2L, potrace, SVG, centerline, test generators) fills the list and
  then calls `analyze` (ranges + per-line time offsets).
* `config.rs`: `GrblConfST` (`$` settings, derived MaxPWM/table size/…), `GrblVersionInfo`, `Firmware`;
  `configuration()` is the persisted current machine config (setting "Grbl Configuration ST").

### 3.4 Image → G-code pipeline
```
RasterImport dialog (app/forms/raster) ──► lg_services::ImageProcessor
   preview thread: resize → grayscale(ColorMatrix) → whitenize → threshold/dither   (lg-raster)
                   → overlays (L2L lines / potrace fill+border / autotrace SVG)
   generate:       same chain at target resolution → GrblFile::load_image_l2l   (raster, dithering, passthrough)
                                                     GrblFile::load_image_potrace (vectorize → lg-potrace
                                                        trace → lg-clipper fill → biarc G-code)
                                                     GrblFile::load_image_centerline (autotrace → lg-svg2gcode)
Raster image (jpg, jpeg, bmp, png, gif; not `.lps` entries) ──► ImageDock (right dock, auto regen on edit)
   ──► lg_services::raster_dock (gray image = the one brightness definition, histogram, layout, `generate`)
   ──► GrblFile::load_image_l2l_body (L2LConf.power_map: brightness → S, see below)
   The dock replaces the modal dialog for plain line-to-line engraving. Its button "Classic import dialog…" and
   every `.lps` project entry still use the RasterImport dialog above (dithering, vectorize, centerline).
SVG file ──► ParamsDock (right dock, auto regen on edit) ──► GrblFile::load_imported_svg ──► lg-svg2gcode
                                                     (auto regen: per-layer params via GrblFile::load_imported_svg_layered → GCodeFromSvg::convert_from_file_layered,
                                                      not used by the golden paths; returns each layer's command range for the preview highlight/hide;
                                                      optional per-layer hatch fill (hatch.rs) of closed curves, per connected area or row by row, emitted as normal burn lines after each pass's outline)
```

Raster dock brightness map (`PowerMap { start, stop }`, `crates/gcode/src/l2l.rs`, not in the C# code): `start` → S-MAX,
`stop` → S-MIN, linear in between; `start > stop` is allowed (brighter = more power). Brightness outside
`[min(start,stop), max(start,stop)]` gets power 0 (the optimizer turns those runs into G0 skips); without hardware PWM
the pixels inside the range burn on/off. `L2LConf.power_map = None` keeps the classic `GetColor` byte-exact (goldens).

### 3.5 Persistence
`lg_base::paths::data_path()` = `~/.config/L-ai-SERcraft` (Windows `%APPDATA%\L-ai-SERcraft`). First start copies an existing old `LaserGRBL` config folder there once (`paths::migrate_dir`, old folder untouched). File names inside keep the original names (e.g. `LaserGRBL.Settings.json`) on purpose. Temp dir: `<temp>/L-ai-SERcraft`.
| file | content | code |
|---|---|---|
| `LaserGRBL.Settings.json` | all settings, C# key names, debounced save | `lg_base::settings` |
| `CustomButtons.json` | custom buttons (`.zbn` import/export = NRBF) | `lg_services::custom_button`, `lg_base::nrbf` |
| `LaserLifeCounter.json` (+`.old`) | laser module usage | `lasergrbl_core::laser_life` |
| `UsageStats.json` | local counters | `lasergrbl_core::usage_stats` |
| `LayerPresets.json` | named layer presets (JSON, `<config dir>/L-ai-SERcraft/`: `~/.config` Linux, `%APPDATA%` Windows, `~/Library/Application Support` macOS); created with one example preset when missing, a broken file is moved to `.bak` and replaced the same way | `lg_services::layer_presets` |
| `GrblEmulator.v11.json` | emulator `$` config | `lasergrbl_core::emulator` |
| `sessionlog.txt`, com log | logs | `lg_base::logger` |
Windows `LaserGRBL.Settings.bin`, `.zbn`, `.lps` are BinaryFormatter files read by `lg_base::nrbf`.

### 3.6 UI (`app`)
* `main.rs` (C# `Program.cs`): args, logger, settings, theme/fonts, window geometry, panic hook → `MainForm`.
* `main_form.rs` (C# `MainForm.cs`): owns `AppState` (`app_state.rs`: core, event receiver, custom buttons,
  hotkeys), lays out menus/status bar/left column/preview, and runs every **flow** (file open incl. `.lps`
  projects, save, run → resume dialog → safety countdown, connect/disconnect, hotkeys, dialogs).
  The right panel holds one of two docks: `ImageDock` while a raster image is loaded (`image.has_source()`),
  else `ParamsDock` (SVG, or no file). `open_outcome` clears the other dock; `dock_*` helpers dispatch
  needs-apply / apply / poll / job-running to the active one for `request_run` and `pump_project`.
* Dialogs: struct per C# form with `open(..)` + `show(ctx) -> Option<Outcome>`; the main window keeps at most
  one modal `Overlay` (`forms/main/dialogs.rs`) like `ShowDialog`.
* Shared UI foundation (`app/src/ui`, see `ui/README.md`): `theme` (ColorScheme), `icons` (SvgIcons/legacy,
  tinted), `i18n` (`tr("MainForm.MnConnect.Text")` keys generated from the original `.resx`), `dialog`
  (modal, MessageBox, async file dialogs), `fonts`, `widgets/*` (ports of the custom WinForms controls),
  `preview/*` (GL renderer = `GrblPanel3D`, CPU renderer = `GrblPanel`).
* `--gallery` opens a developer gallery that shows every widget, dialog and the preview with sample data.

## 4. Testing and verification
| what | where | oracle |
|---|---|---|
| numeric formatting/parsing | `crates/dotnet/tests` | Mono tables + hand-computed .NET Fx cases |
| Clipper, potrace curves/pipeline, Hershey, expressions, SVG, raster ops | `crates/*/tests/golden*.rs` | original C# under Mono |
| G-code parse/analyze/state/L2L/potrace/generators/config | `crates/gcode/tests/golden_*.rs` | original C# under Mono |
| streaming protocol (61 scenarios) + emulator | `crates/core/tests/golden_protocol.rs`, `engine.rs` | original `GrblCore` under Mono with a scripted board |
| UI forms | unit tests + headless render of every gallery tab | – |
Generators live in `tools/golden/<area>/` (`run.sh`, needs `../.mono-build/env.sh`, see `PLAN.md` §4).
Run `cargo test --workspace --release` and `cargo clippy --workspace --all-targets -- -D warnings`.

**Test isolation.** Tests never touch the real config dir (`~/.config/L-ai-SERcraft`), the repository or loose files in the
system temp dir. Rules:
* Every test that can reach settings, session log, laser life, usage stats, emulator config, presets, custom buttons or the
  app temp dir (`paths::temp_path()`) calls `lg_base::paths::isolate_for_tests(tag)` FIRST, before anything caches the real
  path. It points the data dir (`<root>/data`), the settings file (`<root>/settings.json`) and the app temp dir
  (`<root>/tmp`) into the per-process scratch root `<temp>/lg-tests/<tag>-<pid>-<unix_secs>/`. Shortcuts:
  `test_support::init_env()` in `lasergrbl-core` and `lg-services`, `form_util::isolate_env()` and `form_util::test_guard()`
  in the app (the guard also serialises and resets the settings store), the shared `root()` fn in `crates/base/tests/nrbf.rs`.
* A test that needs its own scratch files calls `lg_base::paths::test_dir(name)`: a fresh, unique sub dir of the same root.
  Never build paths from `std::env::temp_dir()` in test code (integration tests in `crates/*/tests/` included).
* Both live in `crates/base/src/test_support.rs`. The root is removed when the test process exits (`atexit` handler: flushes
  the settings saver and the loggers, then deletes the tree). Roots of killed or aborted processes, and old `lg_*` leftovers
  in `<temp>`, are swept after one hour by the next process that creates a root. Debugging: `LG_KEEP_TEST_DIRS=1` keeps the root.
* `python3 -I tools/check_test_isolation.py` enforces it: it runs EVERY test alone in its own process with scratch
  `HOME`/`XDG_*`/`APPDATA` and its own `TMPDIR`/`TEMP`/`TMP`, reports any file or dir created in the config dirs (binary + test
  name), any "temp leftover" in the temp dir after the process exited, and any change of `git status`, and fails on test
  failures too. CI runs it on Linux after `cargo test`. A new test that touches config or leaves temp files fails that check.

## 5. Making changes efficiently
* Find the C# origin of a behaviour → `PORTING.md` (search the C# member) → Rust location.
* Find a Rust file by topic → [`SOURCE_MAP.md`](SOURCE_MAP.md) task index.
* Change touches G-code bytes or protocol bytes → run the golden tests of `lg-gcode`/`lasergrbl-core`; if the
  change is intentional, regenerate nothing from Rust — goldens must only come from the original.
* New settings key → use the exact C# key string and default; read it where C# reads it (some are read once
  per process).
* New UI text → add an i18n key only if the original resx has one; otherwise follow the Designer text.
* Packaging: `packaging/appimage/build-appimage.sh` (AppImage `L-ai-SERcraft-<v>-x86_64.AppImage`, glibc 2.28 baseline via cargo-zigbuild).
* Branding: app name const `lg_base::APP_NAME`; own version = `CARGO_PKG_VERSION` (display only). Compatibility values stay at 7.14.1 (`settings::CURRENT_VERSION`, `.lps` assembly name, NRBF `ASM_VERSION`). Artwork: `resources/branding/` (SVG sources + PNG), renderer `app/examples/render_artwork.rs`.
