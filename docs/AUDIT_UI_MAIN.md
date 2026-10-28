# Audit: main window UI (P4-3)

Scope: `app/src/main.rs`, `main_form.rs`, `app_state.rs`, `forms/main/**`, `forms/preview_form/**`, `ui/**`.
Method: Designer/resx/code of the C# form read against the Rust code, app run under X11 with the built-in emulator
(`LASERGRBL_DEV_CONNECT`, `LASERGRBL_DEV_RUN`, `LASERGRBL_SHOT`, `LASERGRBL_SHOT_SECS`, new `LASERGRBL_DEV_SEQ`).
Screenshots (scratchpad `p4-3/shots/`): disc, telnet, idle, run, run2 (30 s, executed colouring), hold, alarm,
scheme_CADStyle/BlueLaser/Hacker, lang_zh-CN/ja-JP/ru (+ default CADDark everywhere).

## Fixes
| Area | Fix |
|---|---|
| NumericUpDown | spinner arrows were missing-glyph boxes: now painted triangles (`ui/widgets/numeric.rs::spin_button`) |
| Bold | `ui/fonts.rs`: bold family from system Ubuntu-B / DejaVuSans-Bold / NotoSans-Bold / Liberation / Windows fonts, `fonts::bold()` helper; group box falls back to fake bold; dialog/exception captions use it |
| EmulatorUI position | C# `CenterScreen`: first fixed in `main_form.rs`; P4-4's `emulator_ui.rs` now centres itself (pivot centre, fixed size), so my workaround was removed |
| ConnectLogForm | panels are borderless like the Designer (were framed); address/emulator rows rebuilt (text box was squeezed to 40 px), `TxtEmulator` is a disabled text box; Panel1MinSize 210 |
| CommandLog | monospace (Consolas 8.25) font, row height 17; icon 4 = real `IL` image 4, icon 5 = real diagnostic icon extracted from `CommandLog.resx` with the Mono oracle (`resources/icons/diagnostic.png`, `logwarning.png`; README row added) |
| JogForm | Z buttons got the designer tooltips (0.1mm/1mm/10mm); "Jog Speed"/"Jog Step" written only on change (C# `ValueChanged`) |
| Status bar | `TTTLines` default "0" |
| Preview | before the first file load no scene is built (C# `mGrbl3D == null`): usage tips overlay shows, no bounding-box lines |
| Dev | `LASERGRBL_DEV_SEQ="6:hold,9:resume,10:cmd=G0 X1,2:emuhide,door,reset"` timed core actions |

## Verified equal (no change)
* `Grbl3D.Draw/Invalidate`: off layer = every linear move at intensity 1 (width 1), on layer only while burning with S-alpha, arcs as 36-step polylines, status colours OK/KO/Wait only when "show executed", bounding box = dashed infinite lines at DrawingRange. C# GL (`Objects3D`) has no first-movement colouring; `PreviewFirstMovement` (blue, DarkOrange in Nighty) exists only in `GrblFile.DrawJobPreview`, ported 1:1 in `crates/gcode/src/draw.rs` (`firstline`) and used by the CPU preview/raster dialog. Seen in run2.png (pink wait / green OK, cross cursor follows WPos).
* Rulers/labels, bounding-box label suppression and yellow edge labels, MCO/WCO/PTR/F/S overlays, cross cursor, tips overlay.
* Menu items/enable rules (`TimerUpdate`), status bar order, override colours/texts, status colours, buffer bar, laser-life label.
* Colour schemes switch live; zh-CN/ja-JP/ru/el-GR render (CJK fallback font present).

## Remaining differences
* Alarm/hold screenshots: emulator did not raise an alarm for `$20=1; G0 X9999` and the hold did not show in 9 s (emulator pace is real time, emulator belongs to core); status colour code is unit tested (`status_colors`).
* Fonts: egui default sans instead of Microsoft Sans Serif; per-pixel layout is approximate (flow layout, not TableLayoutPanel).
* Other forms still call `RichText::strong()` (`forms/form_util.rs`, `legal_disclaimer.rs`, owned by P4-4): switch to `crate::ui::fonts::bold`.
* `TTLinkToNews` toolbar news is intentionally not ported (PLAN §6). Help button caption is the English "Help" (WinForms uses the OS language).
* `UserControls/ImageTransform.cs` N/A rows now carry a real reason; `Move2DControl` is compiled but never instantiated (N/A valid). All DONE rows in scope point to existing files (scripted check).
