# L-ai-SERctrl

L-ai-SERctrl (pronounced: `laser control`) is a GRBL laser engraver/cutter controller for Linux, Windows and macOS, written in Rust with
[egui/eframe](https://github.com/emilk/egui) (OpenGL).

It is based on [LaserGRBL](https://github.com/arkypita/LaserGRBL) 7.14.1 by Diego Settimi (arkypita), the
free GRBL laser GUI written in C#/WinForms for Windows. Version 1.0 is a faithful Rust port of it, with
the same G-code generation (raster line-to-line, dithering, vectorize/potrace with all fill patterns,
centerline via autotrace, SVG import, test generators), the same GRBL streaming protocol, and a UI that mirrors
the original forms, menus, settings, hotkeys, custom buttons, color schemes and 20 languages. All credit for
the original design and the years of real-world refinement goes to Diego Settimi and the LaserGRBL
contributors and translators. L-ai-SERctrl adds per-layer SVG settings, named layer presets and hatch fill.

L-ai-SERctrl is an independent project. See [Authors & credits](#authors--credits) and [License](#license).
Release notes: [`CHANGELOG.md`](CHANGELOG.md).

## Authors & credits

* **Andreas Pösch (mechaot)** is the author of L-ai-SERctrl (2026-): the Rust/egui port, per-layer SVG
  settings, named layer presets, hatch fill and the packaging.
* **Diego Settimi (arkypita)** is the original author of
  [LaserGRBL](https://github.com/arkypita/LaserGRBL) (2016-2025), on which L-ai-SERctrl is based.
* [`NOTICE.md`](NOTICE.md) lists the LaserGRBL contributors and translators, and the third-party code and
  assets (GRBL-Plotter, Potrace, Clipper, Hershey fonts, Material Design Icons, ...) with their licenses.
  Rust dependencies: [`THIRD_PARTY_RUST.md`](THIRD_PARTY_RUST.md).
* **AI-assisted development:** the port and its new features were written with AI coding assistants by
  Anthropic, Claude Opus (planning and review) and Claude Sonnet (implementation), under the direction and
  review of Andreas Pösch.

## License

L-ai-SERctrl is free software under the GNU General Public License, version 3 or (at your option) any later
version (`GPL-3.0-or-later`, see [`LICENSE.md`](LICENSE.md)).

* Copyright (C) 2016-2025 Diego Settimi (arkypita) and the LaserGRBL contributors (LaserGRBL)
* Copyright (C) 2026 Andreas Pösch (L-ai-SERctrl)

This program comes with ABSOLUTELY NO WARRANTY. L-ai-SERctrl is an independent project. It is not affiliated
with or endorsed by Diego Settimi or the LaserGRBL project. The name "LaserGRBL" is used only to identify and
credit the original work. Source files carry SPDX headers (`python3 -I tools/check_headers.py` checks them).
Release packages contain `LICENSE.md`, `NOTICE.md`, `CHANGELOG.md` and `THIRD_PARTY_LICENSES.txt`, and the
complete source is published next to them as `L-ai-SERctrl-<version>-source.tar.gz`.

## Build and run

Requirements: Rust 1.95+ (edition 2024). No system development packages are needed: serial access works
without libudev, file dialogs use the XDG desktop portal.

```sh
cargo build --release
./target/release/laiserctrl                # optional: a file to open (.nc .gcode .svg .png .jpg .bmp .lps .zbn)
./target/release/laiserctrl --gallery      # developer widget/dialog gallery
```

Resources (`resources/`) are found next to the executable, in `../share/laiserctrl/resources`, in the workspace
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

Settings and data live in `~/.config/L-ai-SERctrl/` (Linux) or `%APPDATA%\L-ai-SERctrl\` (Windows). An existing `LaserGRBL` config folder is copied there once on first start (the old folder is left alone); file names inside keep the original names. An existing
Windows `LaserGRBL.Settings.bin` is imported on first start; custom-button `.zbn` files and `.lps` projects from
the Windows version can be imported, and `.zbn` exports stay readable by Windows LaserGRBL.

## AppImage

```sh
packaging/appimage/build-appimage.sh --install-deps   # first run: zig + cargo-zigbuild into .cache/ (no root)
packaging/appimage/build-appimage.sh                  # later runs
# options: --native (plain cargo build), --skip-build, --out DIR
```

Output: `dist/L-ai-SERctrl-<version>-x86_64.AppImage`. The binary is built with `cargo-zigbuild` against a glibc 2.28
baseline, so it runs on older distros too (the script prints the max required `GLIBC_*` symbol version). With
`--native`, or when zigbuild is missing, it falls back to a plain build that needs a glibc as new as the build
host's. GL/EGL/X11/Wayland/xkbcommon libraries are not bundled: they are loaded at runtime from the host, so
graphics drivers always come from the host system. `appimagetool` (pinned, sha256-checked) is downloaded to
`.cache/appimage/` and run without FUSE.

## CI

`.gitlab-ci.yml` (GitLab CE, docker executor): `lint:clippy` (warnings are errors) and `test:cargo` (whole
workspace incl. golden tests) run in parallel. After both, three independent cross builds (cargo-zigbuild on
the Linux runner) each keep a package + `.sha256` as job artifact for a month:

| job | artifact | notes |
|---|---|---|
| `build:linux-appimage` | `L-ai-SERctrl-<v>-x86_64.AppImage` | fails if the binary needs glibc > 2.28 |
| `build:windows` | `L-ai-SERctrl-<v>-windows-x86_64.zip` | `laiserctrl.exe` (GUI subsystem) + `resources/` |
| `build:macos` | `L-ai-SERctrl-<v>-macos-universal.tar.gz` | arm64 + x86_64, macOS 11.3 SDK from the cargo-zigbuild image |
| `build:source` | `L-ai-SERctrl-<v>-source.tar.gz` | `git archive` of the tagged tree (corresponding source, GPLv3 section 6) |

`lint:headers` (`tools/check_headers.py`) fails when a source file lacks an SPDX header.

The build jobs are `allow_failure: true`: a broken platform never blocks the others. Windows/macOS builds are
unsigned (SmartScreen warning; on macOS run `xattr -dr com.apple.quarantine L-ai-SERctrl` once). Keep `resources/`
next to the binary. Tool versions (Rust image, cargo-zigbuild, zig) are pinned at the top of the file.

`.github/workflows/ci.yml` (GitHub Actions, GitHub-hosted runners) runs the same checks and packages. Windows
(MSVC, static CRT) and macOS (arm64 + x86_64 merged with `lipo`, ad-hoc signed) build natively on their own
runners; the AppImage uses the same zigbuild script and glibc 2.28 gate. Tests also run on Windows and macOS
for information only. Every run keeps its packages as workflow artifacts for 30 days. A push to `master` replaces
the rolling pre-release `nightly`. A `v<version>` tag that matches the `Cargo.toml` version creates a GitHub
Release with all packages and `SHA256SUMS.txt`. Keep the pinned tool versions in sync with `.gitlab-ci.yml`.

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
| `crates/services` | image processor, autotrace, projects, hotkeys, custom buttons, layer presets, firmware flashing, WiFi discovery |
| `app` | the egui application (all forms, preview renderer, theming, icons, i18n) |
| `resources` | CSV code tables, sounds, icons, firmware images, translations (`resources/README.md`) |
| `tests/golden` | reference outputs produced by the original C# code |
| `tools` | golden-data generators, resx→JSON converter, checklist and source-map generators, license tools (`gen_third_party.py`, `check_headers.py`) |

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
