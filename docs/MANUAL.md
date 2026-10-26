# Manual: multi-layer SVG from Inkscape

L-ai-SERcraft gives every layer of an SVG its own laser settings: speed, passes, power, laser mode, hatch fill
and colour filter. This page shows the workflow with a small example: a circle that is **cut** out and a
black box in its middle that is **engraved** (filled).

Example file: [`manual/layers-example.svg`](manual/layers-example.svg) (100 × 100 mm). Screenshots: Inkscape
1.4 and L-ai-SERcraft.

Photos and other bitmaps (JPG, PNG, BMP, GIF) are described at the end:
[Raster images in the Parameters dock](#manual-raster-images-in-the-parameters-dock).

## How layers are read

* Every **top-level layer** of the SVG becomes one entry in the *Layer* picker of the *Parameters* dock. The
  entry shows the Inkscape layer name and the number of shapes, e.g. `Cut (1)`.
* Sublayers belong to their top-level layer and share its settings.
* Objects outside any layer are collected in the pseudo-layer `(ungrouped)`, which is converted first.
* Layers are converted **in file order**. In the Inkscape *Layers and Objects* panel the file order runs from
  the **bottom** row to the **top** row. Put the cut layer at the top so that it is cut last, after
  the engraving. Otherwise the part may shift or fall out before it is engraved.
* Hidden or locked Inkscape layers **are still converted**. To skip a layer, untick *Enabled* in L-ai-SERcraft.
* Text is not burned. Convert it first with *Path ▸ Object to Path*. Embedded bitmaps are ignored.
* Save as **Inkscape SVG**. *Plain SVG* removes the layer names, so the layers then show up under their ids
  (`layer1`, `layer2`, …).

## 1. Document

*File ▸ Document Properties* (`Ctrl+Shift+D`): set the units to **mm** and the size to the work area you need,
here 100 × 100 mm. Keep *Scale* at 1.0 so that one user unit is 1 mm.

![Document Properties: format Custom, 100 × 100 mm, scale 1.0](manual/img/inkscape-document-properties.png)

## 2. Layers

1. Open *Layer ▸ Layers and Objects* (`Ctrl+Shift+L`).
2. Rename the default layer to `Cut`: double-click its name in the panel.
3. *Layer ▸ Add Layer* (`Ctrl+Shift+N`): enter the name `Fill`, choose position **Below current** and click *Add*.

![Add Layer dialog: layer name and position](manual/img/inkscape-add-layer.png)

Result: `Cut` is above `Fill` in the panel, so `Fill` is engraved first and `Cut` runs last. To change the
order later, drag a layer in the panel or use *Layer ▸ Raise Layer* / *Lower Layer*.

## 3. Cut layer: circular outline

1. Click the `Cut` layer in the panel to make it the current layer.
2. Ellipse tool (`E`). Hold `Ctrl` and drag to draw a circle. For exact values, set `cx = 50`, `cy = 50`
   and `r = 45` in the XML editor (step 5). The X/Y/W/H toolbar includes the stroke width (90.3 mm here).
3. *Object ▸ Fill and Stroke* (`Ctrl+Shift+F`): set *Fill* to none (✕) and *Stroke paint* to a flat colour,
   here red `ff0000ff`. Use a thin stroke (0.1–0.3 mm). The laser follows the centre line, so the stroke width
   only affects the look in Inkscape.

![Circle selected: status bar shows "Circle in layer Cut"](manual/img/inkscape-layers-cut.png)

![Fill and Stroke: stroke paint flat red](manual/img/inkscape-stroke-paint.png)

## 4. Fill layer: blackened box

1. Click the `Fill` layer in the panel.
2. Rectangle tool (`R`). Draw a square and set X = 30, Y = 30, W = 40, H = 40 mm in the toolbar.
3. Fill: flat black, stroke: none.

The black fill only shows the intention in Inkscape. L-ai-SERcraft reads the **outline** of the shape and fills
it with hatch lines when *Fill closed shapes* is on for this layer (step 6). Hatch fill needs **closed**
shapes: rectangles, circles, ellipses, polygons and closed paths.

![Box selected: status bar shows "Rectangle in layer Fill"](manual/img/inkscape-layers-fill.png)

## 5. Check and save

*Edit ▸ XML Editor* (`Ctrl+Shift+X`). Each layer must be a `<g … inkscape:label="…">` **directly** under
`<svg>`, with the shapes inside. `layer1` (Fill) comes before `layer2` (Cut).

![XML editor: two top-level layer groups](manual/img/inkscape-xml-editor.png)

Save with *File ▸ Save As…*, type **Inkscape SVG (\*.svg)**.

## 6. Configure in L-ai-SERcraft

Open the file with *File ▸ Open File* or the folder button next to *Filename*. SVGs load directly into the
*Parameters* dock on the right. Pick a layer in the *Layer* box. All other settings in the dock apply to
that layer only.

![Layer picker with the layers Fill and Cut](manual/img/app-layer-picker.png)

Example values. Speed and power depend on your laser and material, so test them on scrap first.

| setting | Fill | Cut |
|---|---|---|
| Speed ▸ Laser | 2000 mm/min | 300 mm/min |
| Speed ▸ Passes | 1 | 3 |
| Laser Options ▸ Laser Mode | M3 - Constant Power | M3 - Constant Power |
| Laser Options ▸ S-MAX | 300 (30 %) | 1000 (100 %) |
| Fill ▸ Fill closed shapes | on | off |
| Fill ▸ Line distance / Angle | 0.20 mm / 45° | – |
| Filter ▸ Color | All | All |

The preview draws the selected layer in red.

![Fill layer selected: hatch fill 0.2 mm at 45°, 30 % power](manual/img/app-fill-layer.png)

![Cut layer selected: 3 passes, 300 mm/min, 100 % power](manual/img/app-cut-layer.png)

How the settings behave:

* **The G-code regenerates** after every edit. No apply button is needed. Lines and *Estimated Time* in the
  status bar update with it.
* **Saved per file.** Settings are stored for each SVG file path and come back when the same file is opened
  again.
* **Enabled** off skips the layer. Example: engrave first, check the result, then enable `Cut` only.
* **Sync layer settings** copies the current layer's values, except *Enabled*, to all other layers.
* **Presets:** type a name (e.g. `Cut 3 mm plywood`) in *Preset name* and click the save button. Later, pick the
  preset in the *Preset* box for any layer. The database button opens the preset list.
* **Fill options:**
  * *Line distance* sets the hatch spacing: smaller is darker and slower.
  * *Cross hatch (+90°)* adds a second, perpendicular hatch.
  * *Fill per shape* finishes each shape before moving to the next. Turn it off to hatch row by row
    across the whole layer.
* **Colour filter:** keeps only the shapes of the layer whose stroke colour (or fill colour, when there is no
  stroke) is red, green, blue or black. When the layers already separate the jobs, leave it on *All*.

## Troubleshooting

| symptom | fix |
|---|---|
| only `(ungrouped)` in the Layer picker | the shapes are not in a layer: select them, *Layer ▸ Move Selection to Layer…* |
| layer names show as `layer1`, `layer2` | the file was saved as Plain SVG: save as Inkscape SVG |
| cut runs before the engraving | move the cut layer to the top of the Inkscape panel |
| no hatch lines | *Fill closed shapes* is off for that layer, or the path is not closed (join the end nodes with the Node tool, `N`) |
| text is missing | *Path ▸ Object to Path* before saving |
| a hidden Inkscape layer is burned | hidden layers are converted too: untick *Enabled* in the Parameters dock |

# Manual: raster images in the Parameters dock

A raster image (JPG, JPEG, BMP, PNG, GIF) opens with *File ▸ Open File* straight into the dock on the right, the
same place where SVG settings live, and is engraved **line to line**. The G-code is generated at once and
regenerates after every committed edit: Enter or leaving an input field, or releasing a histogram marker. The
preview follows, also after the target size changed.

## Brightness to power

The histogram shows how many pixels have each brightness (0 = black … 255 = white, bar height on a log scale).
The strip under it is the brightness scale. Two markers set the **map** from brightness to laser power: drag
them, or type the values into *Start* and *Stop*. The orange curve is the resulting power; the scale on the
right ends at the machine's maximum S value.

| rule | effect |
|---|---|
| *Start* gets **S-MAX**, *Stop* gets **S-MIN**, linear in between | normal map, e.g. Start 16, Stop 210: darker pixels burn harder |
| *Start* is larger than *Stop* | **inverted** map, e.g. Start 210, Stop 16: brighter pixels burn harder. Allowed, no warning |
| brightness **outside** the range between both markers | power 0, **not burned**. The laser skips these parts with fast moves, which also shortens the job |
| *Start* = *Stop* | only exactly this brightness burns, with S-MAX |

Notes:

* Brightness is the grey value of the pixel (luma weights 0.299 / 0.587 / 0.114) laid over white, so transparent
  areas count as white.
* The default map is 0 … 254: pure white (255) never burns, even when S-MIN is above 0. The last used map is
  the start value for the next image.
* Without hardware PWM (*Support Hardware PWM* off in the settings) the S-MIN and S-MAX inputs are hidden and
  every pixel inside the range burns fully (on/off).

## Position and size

*Lower left* and *Upper right* are the corners of the engraving in mm. The size is the difference of both and the
lower-left corner is the job offset. The aspect ratio is **free**: the image is stretched to the rectangle. Below
the inputs the dock shows, read-only:

* the target size in mm,
* the aspect ratio of the target and, in brackets, of the image,
* the size of the raster in pixels (width × height).

*Fit height to image aspect* sets the upper-right Y so that the target has the aspect ratio of the image.
The upper-right corner must be to the right of **and** above the lower-left corner. Otherwise a red message
appears, no G-code is generated and *Run* does nothing until the values are valid again.

## Line to line, speed and laser

* **Direction:** Horizontal, Vertical or Diagonal.
* **Quality:** lines per mm, 0.1 to 20 (50 with *Raster Hi-Res* in the settings). Higher quality makes a finer
  raster and a longer job. Very large targets are limited by the memory limit of the converter.
* **Engraving speed** in mm/min, **Laser Mode** (M3 constant, M4 dynamic power) and **S-MIN** / **S-MAX**.

All values are saved for each image file and come back when the same file is opened again. A new file starts with
the last used values.

## Classic import dialog

*Classic import dialog…* at the bottom hands the image to the original raster dialog, which also offers
dithering, vectorize and centerline. If you cancel that dialog the image returns to the dock. Entries of `.lps`
project files always open in the classic dialog.
