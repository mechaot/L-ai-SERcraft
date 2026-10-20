# LaserGRBL (Rust port)

A Rust rewrite of [LaserGRBL](https://github.com/arkypita/LaserGRBL) 7.14.1, the GRBL laser engraver/cutter
controller originally written in C#/WinForms for Windows. It runs on Linux and Windows and uses
[egui/eframe](https://github.com/emilk/egui) (OpenGL) for the UI.

The goal is feature parity: same G-code generation (raster line-to-line, dithering, vectorize/potrace with all
fill patterns, centerline via autotrace, SVG import, test generators), same GRBL streaming protocol, and a UI that
mirrors the original forms, menus, settings, hotkeys, custom buttons, color schemes and 20 languages.

This is an unofficial port and is not affiliated with the original author. Licensed GPLv3 like the original
(see `LICENSE.md`).

## Build and run

Requirements: Rust 1.95+ (edition 2024). No system development packages are needed: serial access works
without libudev, file dialogs use the XDG desktop portal.

```sh
cargo build --release
./target/release/lasergrbl                # optional: a file to open (.nc .gcode .svg .png .jpg .bmp .lps .zbn)
./target/release/lasergrbl --gallery      # developer widget/dialog gallery
```

Resources (`resources/`) are found next to the executable, in `../share/lasergrbl/resources`, in the workspace
(for `cargo run`), or via `LASERGRBL_RESOURCES=<dir>`.

Optional runtime tools (same role as the executables bundled with the Windows version):

| feature | tool | setting to override path |
|---|---|---|
| Centerline vectorization | `autotrace` | `Autotrace Path` |
| Flash GRBL firmware | `avrdude` | `Avrdude Path` |
| Sounds | `pw-play`, `paplay` or `aplay` (Linux), `afplay` (macOS), PowerShell (Windows) | – |
| WiFi discovery | `ip`, `ping`, `getent` (Linux) | – |

On Linux your user must be allowed to open serial ports (usually the `dialout` or `uucp` group). CH340
adapters need no driver on Linux. If the window does not appear under some Wayland setups, run with
`WAYLAND_DISPLAY` unset to use X11.

Settings and data live in `~/.config/LaserGRBL/` (Linux) or `%APPDATA%\LaserGRBL\` (Windows). An existing
Windows `LaserGRBL.Settings.bin` is imported on first start; custom-button `.zbn` files and `.lps` projects from
the Windows version can be imported, and `.zbn` exports stay readable by Windows LaserGRBL.

## AppImage

```sh
packaging/appimage/build-appimage.sh --install-deps   # first run: zig + cargo-zigbuild into .cache/ (no root)
packaging/appimage/build-appimage.sh                  # later runs
# options: --native (plain cargo build), --skip-build, --out DIR
```

Output: `dist/LaserGRBL-<version>-x86_64.AppImage`. The binary is built with `cargo-zigbuild` against a glibc 2.28
baseline, so it runs on older distros too (the script prints the max required `GLIBC_*` symbol version). With
`--native`, or when zigbuild is missing, it falls back to a plain build that needs a glibc as new as the build
host's. GL/EGL/X11/Wayland/xkbcommon libraries are not bundled: they are loaded at runtime from the host, so
graphics drivers always come from the host system. `appimagetool` (pinned, sha256-checked) is downloaded to
`.cache/appimage/` and run without FUSE.

## CI

`.gitlab-ci.yml` (GitLab CE, docker executor): `lint:clippy` (warnings are errors) and `test:cargo` (whole
workspace incl. golden tests) run in parallel; `build:appimage` runs after both and keeps
`dist/LaserGRBL-<version>-x86_64.AppImage` (+ `.sha256`) as job artifact for a month. The job fails if the
binary needs a glibc newer than 2.28. Tool versions (Rust image, cargo-zigbuild, zig) are pinned at the top.

## Repository layout

Before editing, read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) (layers, threads, data flow, rules) and
use [`docs/SOURCE_MAP.md`](docs/SOURCE_MAP.md) (task index + every source file with its purpose) to find the
right place. After adding or renaming source files, run `python3 -I tools/gen_source_map.py`.

| path | content |
|---|---|
| `crates/dotnet` | .NET Framework numeric semantics (number formatting, parsing, `Math.Round`, `decimal`, `TimeSpan`) so generated numbers match byte for byte |
| `crates/clipper` | port of Clipper 6.4.2 (C#) |
| `crates/potrace` | CsPotrace, Bezier→biarc, potrace filling patterns, GDI+ path flattening |
| `crates/expr`, `crates/hershey` | custom-button expression evaluator, Hershey single-line font |
| `crates/svg2gcode` | SVG → G-code converter |
| `crates/raster` | image transforms and dithering |
| `crates/base` | settings, paths, logger, timers/threads, CSV dictionaries, .NET BinaryFormatter (NRBF) reader/writer, sound, telegram |
| `crates/gcode` | G-code command model, parser state, `GrblFile` (load/analyse/raster/vector/SVG/generators), GRBL config |
| `crates/core` | `GrblCore` streaming engine (+ Marlin/Smoothie/Vigo), serial/telnet/websocket/emulator links, built-in GRBL emulator, laser life, multi-instance start |
| `crates/services` | image processor, autotrace, projects, hotkeys, custom buttons, material DB, firmware flashing, WiFi discovery |
| `app` | the egui application (all forms, preview renderer, theming, icons, i18n) |
| `resources` | CSV code tables, sounds, icons, firmware images, translations (`resources/README.md`) |
| `tests/golden` | reference outputs produced by the original C# code |
| `tools` | golden-data generators, resx→JSON converter, checklist generator |

## How fidelity is verified

The original C# application is compiled with Mono (user space, no root) and driven by C# harnesses in
`tools/golden/` that call the real original code and record its outputs. The Rust tests compare against these
files byte for byte:

* number formatting/parsing (~190k cases), Clipper (627), expression evaluator (2232), Hershey (1407)
* G-code parsing (487), analysis/time estimation (88), parser state traces (35), GRBL config (128)
* raster line-to-line (180), vectorize incl. all fill patterns (264 + curve lists), SVG import (409 + 183),
  cutting/power-speed/shake test generators (18), dithering and pixel operations
* the streaming protocol: 61 scripted sessions (connect, buffered/synchronous/repeat-on-error streaming,
  hold/resume/abort/reset, jog, overrides, custom code, resume from position, hang and stuck-buffer
  detection, Marlin/Smoothie/Vigo) comparing the exact byte stream sent to the machine.

`PORTING.md` lists every class/method/field of the original (4449 rows) with its Rust location or the reason it
is not needed. `docs/AUDIT_*.md` hold the review results. To regenerate golden data, see
`tools/golden/*/run.sh` (needs the Mono environment described in `PLAN.md` §4).

```sh
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
```

## Known differences from the Windows version

* Not byte-exact because Windows GDI+ cannot be reproduced: bitmap resampling (`HighQualityBicubic`),
  anti-aliased fills in the "vectorize with raster filling" mode (Horizontal/Vertical/Diagonal fill under
  Vectorize), preview anti-aliasing. All other G-code paths are byte-exact.
* Number formatting always uses the invariant culture (the original sometimes used the Windows locale).
* Not ported on purpose: automatic update check (upstream ships Windows installers only), usage statistics
  upload and other telemetry to the upstream servers, the CH340 Windows driver installer, Windows-only
  power/timer API calls.
* Settings are stored as JSON; the Telegram bot code is stored in plain text (file permissions 0600)
  instead of Windows DPAPI.
* External tools (`autotrace`, `avrdude`) come from the system instead of bundled `.exe` files.
* Layout uses egui widgets and fonts, so sizes differ slightly from WinForms; captions, options, behaviour and
  setting keys follow the original.

Full lists: `docs/AUDIT_NONUI.md` §4, `docs/AUDIT_UI_MAIN.md`, `docs/AUDIT_UI_DIALOGS.md`,
`crates/dotnet/README.md`.
