# Phase 4 audit P4-2: non-UI crates

Scope: `crates/{dotnet,clipper,potrace,expr,hershey,svg2gcode,raster,base,gcode,services}`.
Not covered here: `crates/core` (P4-1) and `app/` (P4-3/P4-4).

Method: every C# file listed below was read completely and compared with its Rust counterpart, method by
method (control flow, exception paths, numeric types and formatting, setting keys and defaults, file names,
thread semantics). Code covered by the golden tests (`tests/golden/*`, generated from the original under Mono)
was cross-checked for coverage only; code paths the goldens do not reach were read line by line. Every
PORTING.md row of these sections was re-validated (DONE rows point to an existing Rust item, N/A rows were
proven by grep or by PLAN section 6).

Golden coverage in short: dotnet (formatting/parsing tables), clipper (clip/offset/misc, 60k lines), potrace
(trace, curves, vector fills), l2l, generators, gcode_parse (486 lines of corpus), gcode_analyze, state_builder,
config (139 versions), expr (2233 cases) + custom_code, hershey (all glyphs), svg (410 cases) + svg_unit,
raster (216 ops), base (NRBF, settings sample, TimeSpan), services (hotkeys, materials, project files).
NOT covered by goldens: raster-filling branch of `LoadImagePotrace` (smoke only, GDI+ resampling), `SaveGCODE`,
`LoadFile` text decoding, `DrawOnGraphics`, `LoadImageCenterline`, everything in `ImageProcessor`, custom
button logic, wifi discovery, flash/autotrace runners, Settings/Logger/Serializer I/O.

## 1. Verdict per C# file

| C# file | Rust | Verdict |
|---|---|---|
| `Settings.cs` | base/settings.rs | OK (JSON store, see residual R1) |
| `Logger/Logger.cs`, `AsyncLogFile.cs`, `ComLogger.cs` | base/logger/* | OK |
| `CSV/CsvDictionary.cs`, `FormattedMessage.cs` (+ resource CSVs) | base/csv/* | OK |
| `Tools/HiResTimer.cs` (+ `Utils`) | base/tools/hires_timer.rs, utils.rs | OK (R2) |
| `Tools/ThreadClass.cs` | base/tools/thread_class.rs | OK (WaitAny priority kept) |
| `Tools/Cronometro.cs` | base/tools/cronometro.rs | FIXED: `PeriodicEventTimer.Period` setter reset the period counter |
| `Tools/MathHelper.cs` (+ `RulerStepCalculator`) | base/tools/math_helper.rs | OK |
| `Tools/MobileAverageCalculator.cs` | base/tools/mobile_average.rs | OK |
| `Tools/WordWrap.cs`, `RetainedSetting.cs` | base/tools/* | OK |
| `Tools/Serializer.cs` | base/nrbf/serializer.rs | FIXED: global lock; docs/rows wrong about the password (R3) |
| `Sound/SoundEvent.cs` | base/sound.rs | OK (R4) |
| `Telegram.cs`, `UrlManager.cs` | removed | intentional: no telemetry or phone-home of any kind (no HTTP client linked) |
| `GrblCommand.cs` | gcode/command.rs, csvd.rs | OK |
| `StateBuilder.cs` | gcode/state_builder.rs | OK |
| `GrblFile.cs` (+ `ProgramRange`) | gcode/grbl_file.rs, l2l.rs, vector.rs, generators.rs, optimize.rs, draw.rs, program_range.rs | FIXED: `LoadFile` text decoding (R5 residual) |
| `Core/GrblCore.cs` parts: `GrblConfST`, `GrblVersionInfo`, `Firmware`, `GPoint` | gcode/config.rs | OK |
| `CsPotrace/*` (+ `BezierToBiarc/*`, `PotraceClipper.cs`, `CsPotraceExport*.cs`) | potrace/* | OK (R6) |
| `Clipper/Clipper.cs` | clipper/* | OK (`use_lines` only; `use_xyz`/`use_int32` undefined upstream) |
| `Hershey/Hershey.cs` | hershey/lib.rs | OK |
| `Tools/ExpressionEvaluator.cs` | expr/* | OK |
| `SvgConverter/GCodeFromSVG.cs`, `gcodeRelated.cs`, `BezierTools.cs` | svg2gcode/* | OK (R7) |
| `RasterConverter/ImageTransform.cs`, `Dithering/*` | raster/* | OK (R8) |
| `RasterConverter/ImageProcessor.cs` | services/image_processor/* | FIXED: `filesize` float product (R9) |
| `HotKeysManager.cs` | services/hotkeys.rs | OK |
| `CustomButton.cs` (model) | services/custom_button.rs | OK |
| `PSHelper/MaterialDB.cs` | services/layer_presets.rs | REPLACED (R10): named layer presets, intentional deviation |
| `Tools/Project.cs` | services/project.rs | OK (R11) |
| `Tools/Json.cs` | services/json.rs | FIXED: `\uXXXX` hex parsing accepted a sign |
| `AutoUpdate/GitHub.cs`, `Tools/Json.cs` | removed | intentional: no update check (Json reader was only used by it) |
| `Autotrace/Autotrace.cs` | services/autotrace.rs | OK (R12) |
| `WiFiDiscovery/IPAddressHelper.cs` (+ non-UI of `DiscoveryForm.cs`) | services/wifi_discovery.rs | OK (R13) |
| `Firmware/FlashGrbl.cs` (logic) | services/flash_grbl.rs | OK (R14) |

## 2. Bugs found and fixed (each with a unit test)

1. `ImageProcessor.DoTrueWork`: `filesize = TargetSize.Width * TargetSize.Height` is a `float * float`
   product in C# (rounded to f32 before widening). Rust multiplied in f64, which changes `maxRes` and the adaptive
   vector quality `GetVectorQuality(filesize, adaptive)` in the last bits. Fix: `image_processor/job.rs`
   `target_area()`; test `area_is_f32_product`.
2. `Serializer.ObjToFile/ObjFromFile`: C# serialises all calls with `lock (ThreadLock)`. Rust had no lock and used
   one temp file name per process (`tmp_<pid>`), so two threads saving different files (custom buttons, laser
   life, emulator config) could clobber each other's temp file. Fix: `THREAD_LOCK` in `nrbf/serializer.rs`; test
   `concurrent_writers_do_not_clash`.
3. `PeriodicEventTimer.Period` setter: C# `ReInit` replaces the stopwatch but keeps `m_periodcount`; Rust reset it
   (visible through `NumPeriod`). Fix + adjusted test (`base/tools/cronometro.rs`).
4. `JSONParser.ParseValue(string)`: `UInt32.TryParse(..., AllowHexSpecifier)` accepts hex digits only; Rust
   `from_str_radix` also accepted a leading `+`, turning `\u+041` into a character instead of keeping it literal.
   Fix: explicit hex-digit check; test in `services/json.rs`.
5. `GrblFile.LoadFile`: `StreamReader` detects UTF-8/UTF-16/UTF-32 byte order marks; Rust only skipped a UTF-8
   BOM, so a UTF-16 `.nc` file produced garbage lines. Fix: `gcode/text.rs::decode_stream_reader` used by
   `load_file`; test `stream_reader_boms`.

## 3. PORTING.md rows corrected

* `GrblFile.cs` (126 rows), `StateBuilder.cs` (74), `GrblCommand.cs` (58): status `ported` (not a legal status)
  replaced by `DONE` with the specific Rust file per row (was one generic multi-file string for every row of
  GrblFile.cs).
* `GrblFile.DirectionChange`: now `N/A` with proof (only referenced from the commented-out `TravelCost`;
  `OptimizePaths` ignores `changecost`).
* `Tools/Serializer.cs` password rows: the old reason "no caller passes a password" was false: `UsageStats.cs`
  passes `"lasergrbl"` for the news-message file. Reason corrected (that file is remote telemetry, PLAN section 6).
* `Tools/Serializer.cs` `ThreadLock`: N/A -> DONE (lock added, see 2.2).
* `Autotrace.BitmapToSvgDocument`: location now names `bitmap_to_svg_string` + `parse_svg` (there is no
  `SvgDocument` type in Rust).
* All other N/A rows of the in-scope sections were checked: "unused" claims (CsPotrace `xprod`, `sq`, `cu`,
  `floordiv`, `abs`, `min`, `max`, `COS179`; `Vector2` / `Complex` members; `HotKeysManager.IsModifier`;
  `Autotrace.ToHexString`; `Complex.LOG_10_INV`) hold by grep; `copy()` is replaced by `Clone`/`Copy`; the
  `HiResTimer` QPC rows are justified by the monotonic `Instant`; Windows-only rows are covered by PLAN section 6.
  The mechanical existence check found no DONE row pointing to a missing file.

## 4. Residual known deviations (all intentional or unavoidable)

* R1 `Settings`: values are JSON, not BinaryFormatter (PLAN 2.2). `GetObject<T>` is lenient (an `int` stored
  for a `float` key reads back; C# requires the exact runtime type and returns the default). `GetAndDelete`
  likewise. Windows `LaserGRBL.Settings.bin` is imported once when no JSON exists.
* R2 `HiResTimer`: no QPC frequency correction (monotonic `Instant`); `TotalNano` starts at 0 instead of the
  tick count (only differences are used). Sleep/hibernation guard kept including the upstream log text bug.
  `Logger.Start` writes one extra `OS ...` line.
* R3 `Serializer`: no password/encryption mode (only the telemetry news file used it); no `.bak` +
  `File.Replace` dance (tmp + rename); no `*.dam` copy of an unreadable file (error is logged); a header-less
  v0.0 file is read but not re-saved in the v1.0 container; XML mode does not exist upstream either.
  Custom buttons use `CustomButtons.json`; `.zbn` import/export (gzip/deflate + NRBF) stays compatible.
* R4 Sound/Telegram: external players (`pw-play`/`paplay`/`aplay`/`afplay`/PowerShell) replace
  `SoundPlayer`; default wav lives in `resources/sound/`; Telegram code is stored in plain text (no DPAPI);
  all `UrlManager` fields are `None` as in the public C# source, so nothing is sent unless configured.
* R5 `LoadFile`/`SaveGCODE`: custom header/footer text is split on `\n` with a trailing `\r` removed (C# splits
  on `Environment.NewLine` only); `AddText` splits generator output on `\r`/`\n` (C# on `Environment.NewLine`
  chars); `SaveGCODE` returns the I/O error instead of swallowing it (callers may ignore it); culture-sensitive `ToUpper/ToLower/IndexOf` of .NET are replaced by invariant behaviour (PLAN 3).
* R6 Potrace raster filling (`Export2GDIPlus` + `ResizeImage`): tiny-skia anti-aliasing and the Rust
  resampler differ from GDI+/libgdiplus in edge pixels, so `Horizontal/Vertical/Diagonal` fills under
  `LoadImagePotrace` match the oracle structurally (same header lines, same border trace, same run structure
  except for AA-dependent S values and a few split/merge runs) but not byte-for-byte. Tested as smoke only
  (`potrace_rasterfill_smoke`); the oracle itself is Mono/libgdiplus, not Windows GDI+. Vector fills, borders,
  curves and biarcs are byte-exact.
* R7 SVG: Svg.NET is replaced by roxmltree + the original's own hand parser; XML error text differs from .NET
  (`SvgError` carries the .NET exception type name). `gcodeRelated.firmwareType` is read once per process like
  the C# static initialiser.
* R8 Raster: `ResizeImage` is a separable Catmull-Rom/triangle resampler, not bit-exact GDI+ (tolerance tested);
  `Threshold` uses the libgdiplus rule `channel > round(threshold*255)` (verified against the libgdiplus 6.1
  source; Windows GDI+ may differ by one level at the boundary); `GrayScale` rounds to nearest like Windows
  (the Mono oracle truncates; tolerance 1 level tested); `RandomDithering` is bit-exact only with an explicit seed.
* R9 `ImageProcessor`: GDI+ drawing of previews is done with direct pixel writes / tiny-skia; the error text of the
  centerline preview uses system fonts through resvg; `Thread.Abort` is replaced by a cooperative exit flag;
  `FileDPI` is parsed from PNG/JPEG/BMP headers (GDI+ semantics, e.g. 11811 ppm -> 299 dpi).
* R10 `MaterialDB`: replaced by named layer presets (`layer_presets.rs`, `LayerPresets.json`); intentional deviation, the C# material data (906 rows) is dropped.
* R11 `Project`: `ConvertImageToBase64` stores the file bytes instead of GDI+ re-encoding the image.
* R12 Autotrace: system `autotrace` binary (setting `Autotrace Path`), temp PNG under `<temp>/L-ai-SERcraft/Autotrace`.
* R13 Wi-Fi discovery: `ip`/`/proc/net/arp`/`getent`/system `ping` replace `NetworkInterface`, `SendARP`,
  `Dns` and `Ping`; a worker pool replaces the 16-per-100 ms `SendAsync` pacing.
* R14 Flash: system `avrdude` (setting `Avrdude Path`); firmware list sorted by file name; date shown as
  `MM/dd/yyyy` (invariant) instead of the user's short date.
* Everywhere: `.NET` culture-dependent formatting is replaced by invariant culture (PLAN section 3). Under a
  Turkish culture the original would upper-case `i` to `İ`; the port does not.
* Documented upstream quirks kept on purpose (all marked `NOTE(upstream)` in the source): see
  `grep -rn "NOTE(upstream)" crates/`. Examples: `F{p_end}` typo in the cutting-test caption, `Vigotec`
  compared against a lower-cased string, `IsLuckyWiFi` precedence, swapped green/blue errors in error-diffusion
  dithering, `Math.Min(-1, ping)` in `SetPing`, `Max 99 cycles` check `>= 100`.
* `dotnet` crate: see `crates/dotnet/README.md` ("Known deviations / uncertainties").

## 5. Checks run

`cargo test --release` and `cargo clippy --release --no-deps --all-targets -- -D warnings` on all ten crates
are clean after the changes above.
