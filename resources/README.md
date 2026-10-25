# resources/ inventory

Copied from the original C# project `LaserGRBL/LaserGRBL/` (paths below relative to that dir).
Generated files: `i18n/` (by `tools/resx2json.py`).

| Dir | Origin | Purpose | Used by (C#) |
|---|---|---|---|
| `csv/` | `CSV/*.csv` | GRBL lookup tables, 3 cols (settings: code;name;unit/desc) or 2 cols (alarm/error: code;text). Variants per firmware: `v0.8`, `v0.9`, `v1.1` (default), `ortur.v1.4.x/v1.5.x/v1.7.x`, `ortur.GrblHal`, `longer.nanoduo`. | `GrblCommand.cs` (`CsvDictionary("LaserGRBL.CSV...")`, variant picked per firmware/vendor), `CSV/CsvDictionary.cs` loads embedded stream |
| `sound/` | `Sound/*.wav` | `beep`, `connect`, `disconnect`, `fatal`, `success`, `warning` default event sounds. | `Sound/SoundEvent.cs` (default `Sound\<event>.wav`, user override in setting `Sound.<event>`), `SettingsForm.cs` |
| `svgicons/` | `SvgIcons/*.data,*.header` | Icon packs: `Mdi` (Material Design Icons), `Flags` (country flags, ids `flags-xx`), `Custom` (app-specific). `.header` = index, `.data` = base64 SVG blob. | `SvgIcons/SvgIcons.cs` (embedded stream `LaserGRBL.SvgIcons.<name>.data/.header`), `IconsMgr.PrepareMenuItem(item, "mdi-...")` |
| `grafica/` | `Grafica/*` | Raster UI images and app icon. `LaserGrbl.ico` = application icon (csproj `ApplicationIcon`). `aN/aNE/.../aNW.png` = 8 jog arrow pads, `zup/zdown{01,1,10}.png` = Z jog, `flipH/V, rotCW/CCW, Crop-tool-icon, Actions-transform-*` = raster-converter toolbar, `Gcode{OK,KO,Queue}.png` = job status, `control_*`, `media_*`, `document_*`, `check*`, `add2`, `closeall`, `configure`, `connect_creating`, `fileopen`, `flash`, `info`, `warning`, `error`, `stop`, `text_align_left`, `khelpcenter_blue`, `777494-200` = legacy button/menu images. | Embedded as `Image`/`BackgroundImage` of form controls (set in `*.resx` / `*.Designer.cs`, csproj `<Content Include="Grafica\...">`). Most are superseded by SvgIcons/LegacyIcons; no direct `.cs` reference. `MnWiFiDiscovery.Image.png`, `settings.ico`, `Actions-transform-fill/outliner.png`: not listed in csproj, no code reference found. |
| `icons/` | `Icons/*.png` + `ImageList IL` of `UserControls/CommandLog.resx` | `logqueued/logok/logko/loginfo.png` (copied), `logwarning.png` = `IL` image 4 and `diagnostic.png` = `IL` image 5, both extracted from the resx ImageListStreamer with the Mono oracle (`ResXResourceReader` + `ImageList.Images[i].Save`). `LegacyIcons.data/.header` = legacy icon pack. | `UserControls/CommandLog.cs` (`IL.Images[cmd.ImageIndex]`), `Icons/LegacyIcons.cs` |
| `firmware/` | `Firmware/` | AVR hex images (`01-v1.1h-original`, `02-v1.1h-custom XY Homing`, `03-v1.1f-original`) and `avrdude.conf` for flashing. `avrdude.exe`/`libusb0.dll` NOT copied (Windows only; Linux uses system `avrdude`). | `Firmware/FlashGrbl.cs` (lists `Firmware\*.hex`, runs `Firmware\avrdude.exe` with the conf) |
| `generator/` | `Generator/SVG/LaserGRBL-accuracy-test-file.svg` | Accuracy test pattern, opened via menu. | `MainForm.cs` (`Core.OpenFile("LaserGRBL.Generator.SVG.LaserGRBL-accuracy-test-file.svg")`), `SvgConverter/GCodeFromSVG.cs` + `Resources/ResourceHelper.cs` (embedded-resource path) |
| `misc/` | `StandardButtons.zbn`, `Resources/*.png` | `StandardButtons.zbn`: zip of default custom buttons (loaded next to exe). `book_Thesaurus_32.png`, `book_addressHS.png`, `flag_greece.png`: small menu images (csproj `<None Include="Resources\...">`). | `CustomButton.cs` (`StandardFile`), `MainForm.cs` (`.zbn` import), csproj |
| `i18n/` | generated from every `*.resx` | `<culture>.json` flat key->string maps; `en.json` = invariant. `cultures.json` = language menu (order, display name, flag icon id; handlers `MainForm.cs` `*_Click` -> `SetLanguage`). | `Strings*.resx` (`LaserGRBL.Strings`), every Form `.resx` (WinForms `ApplyResources`) |

## i18n key scheme

* `Strings.<Name>` from `Strings*.resx`.
* `<FormClass>.<Control>.<Property>` for forms in the project root, e.g. `MainForm.MnConnect.Text`, `MainForm.$this.Text`.
* `<Folder>.<Form>.<Control>.<Property>` for forms in sub-folders, e.g. `RasterConverter.ConvertSizeAndOptionForm.BtnOk.Text`, `Generator.CuttingTest.$this.Text`, `UserControls.CommandLog...`.
* Only plain string entries; typed/binary data and `>>` designer metadata dropped. `\r\n` -> `\n`.
