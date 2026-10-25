# Authors, contributors and third-party notices

L-ai-SERctrl is free software under the GNU General Public License, version 3 or (at your option) any
later version (see [LICENSE.md](LICENSE.md)). It is a modified version of LaserGRBL 7.14.1 and contains
work of many people. This file credits them. Where the original LaserGRBL project does not state a
license for a component, this file says "license: see original project" instead of guessing.

## 1. L-ai-SERctrl

- **Andreas Pösch (mechaot)**, 2026-, author of this fork: Rust/egui port of LaserGRBL, per-layer SVG
  settings, named layer presets, hatch fill, new branding, packaging.
- AI-assisted development: the port and its new features were written with the help of AI coding
  assistants by Anthropic, Claude Opus (planning and review) and Claude Sonnet (implementation), under
  the direction and review of Andreas Pösch.

## 2. LaserGRBL (original work)

- **Diego Settimi (arkypita)**, original author of LaserGRBL, 2016-2025.
- https://github.com/arkypita/LaserGRBL
- https://lasergrbl.com

L-ai-SERctrl is an independent project. It is not affiliated with or endorsed by Diego Settimi or the
LaserGRBL project. The name "LaserGRBL" is used only to identify and credit the original work.

## 3. LaserGRBL contributors and translators, as credited in the LaserGRBL README

Copied verbatim from the "Thanks to" section of the LaserGRBL 7.14.1 README
(https://github.com/arkypita/LaserGRBL/blob/master/README.md), with one change: the first entry there
reads "Myself, for italian and english language". "Myself" is Diego Settimi, so it is written out
here as "Diego Settimi, for Italian and English language".

Thanks to:
- Diego Settimi, for Italian and English language
- Fernando Luna, sqall123, for spanish translation
- Olivier Salvador, guillaume-rico [#848](https://github.com/arkypita/LaserGRBL/pull/848) for french translation
- Gerd Vogel, for german translation
- Anders Lassen, for danish translation
- Gerson Koppe, for brasilian translation
- Alexey Golovin, Newcomere, AlexeyBond, for russian translation
- Yang Haiqiang, for chinese translation
- 00alkskodi00, for slovak translation [#670](https://github.com/arkypita/LaserGRBL/issues/670)
- ddogman, for hungarian translation [#735](https://github.com/arkypita/LaserGRBL/issues/735)
- Petr Bitnar, for czech translation
- Ozzybanan, for polish translation
- onmaker, for traditional chinese translation [#1066](https://github.com/arkypita/LaserGRBL/pull/1066)
- Nikolaos Ntekas, for Greek translation [#1234](https://github.com/arkypita/LaserGRBL/pull/1234)
- Mrjavaci, for Turkish translation [#1293](https://github.com/arkypita/LaserGRBL/pull/1293)
- Filippo Rivato for code contribution [#305](https://github.com/arkypita/LaserGRBL/pull/305) and again [#1251](https://github.com/arkypita/LaserGRBL/pull/1251)
- Fabio Ferretti for code contribution [#592](https://github.com/arkypita/LaserGRBL/pull/592)
- guillaume-rico for code contribution on Smoothie support
- Tobias Falkner, for code contribution [#937](https://github.com/arkypita/LaserGRBL/pull/937)
- gmmanonymus111, for code contribution [#1032](https://github.com/arkypita/LaserGRBL/pull/1032)
- [GabeMx5](https://github.com/GabeMx5), for OpenGL preview and svg icons implementation
- [jcurl](https://github.com/jcurl), for [RJCP.SerialPort](https://github.com/jcurl/RJCP.DLL.SerialPortStream) implementation

## 4. Contributors recorded in the LaserGRBL git history (up to v7.14.1)

Distinct author names of the commits up to tag v7.14.1 (2016-12-04 to 2025-03-01), listed as recorded
in the repository (some people appear under several names). Command used:

```
git -C LaserGRBL log v7.14.1 --format=%an | sort -u
```

- A.Bond
- Administrator
- Alex
- arkypita
- AVL01\POCKG
- chris_dibbern
- Chris Dibbern
- Dario
- Darren R. Starr
- Diego Settimi
- Dominik S
- Effer
- Fabio Ferretti
- Flavio
- GabeMx5
- gerdvogel
- gmmanonymus111
- guillaume-rico
- HomineLudens
- Humberto
- iobu
- Jan Klass
- Jean-Jacques Carron
- jvkaam
- Marcel
- mccj
- MikelSV
- Mrjavaci
- NgocLe97
- nyarurato
- Oleg
- onmaker
- Ozzybanan
- pebcz
- peter9811
- Ranthalion
- Richard Strauss
- ristraus
- rOzzy1987
- Sean Miller
- soga172
- SqAll
- sqall123
- TECHITA\capellid
- Thomas Peuß
- Tobias F
- Vito
- Александр
- 毛才君

## 5. Third-party code and assets included in L-ai-SERctrl

LaserGRBL incorporated the components below. L-ai-SERctrl contains Rust ports or copies of them. Each
component keeps its own authors and license. The listed locations are in this repository.

| Component | Authors / copyright | License | Upstream | Location here |
|---|---|---|---|---|
| GCodeFromSVG / gcodeRelated (SVG to G-code), taken from GRBL-Plotter and modified for LaserGRBL | Copyright (C) 2015-2018 Sven Hasemann; modified for LaserGRBL by Diego Settimi | GPL-3.0-or-later (per the file headers) | https://github.com/svenhb/GRBL-Plotter | `crates/svg2gcode/src/{convert,gcode,lib}.rs` |
| BezierTools (adaptive Bezier flattening) | Written by Darren Starr (https://github.com/darrenstarr); parts with roots in Graphics Gems vol. 1 (https://github.com/erich666/GraphicsGems) | license: see original project (the file header only points to the Graphics Gems license) | https://github.com/darrenstarr | `crates/svg2gcode/src/bezier.rs` |
| Potrace / CsPotrace (bitmap tracer) | Copyright (C) 2001-2016 Peter Selinger; C# port Copyright (C) 2009-2016 Wolfgang Nagl | GPL-2.0-or-later (per the header) | https://drawing3d.de/Downloads.aspx | `crates/potrace/src/trace.rs` |
| CsPotrace export code | Copyright (C) Dileep.M | GPL-2.0-or-later (per the header) | - | `crates/potrace/src/export.rs` |
| Bezier2Biarc | László Domoszlai (upstream README: "Copyright Laszlo") | license: see original project | https://github.com/domoszlai/bezier2biarc | `crates/potrace/src/biarc/*.rs` |
| Clipper 6.4.2 | Copyright Angus Johnson 2010-2017 (the LaserGRBL README says 2010-2014; the file header says 2010-2017 and is used here) | Boost Software License 1.0 (full text below) | http://www.angusj.com/delphi/clipper.php | `crates/clipper/` |
| Expression Evaluator | Jeremy Roberts (2004-2005), Will Gray (2006) | license: see original project | https://github.com/vubiostat/expression.cs | `crates/expr/` |
| ColorSlider | Copyright Michal Brylka | license: see original project | https://www.codeproject.com/articles/17395/owner-drawn-trackbar-slider | `app/src/ui/widgets/color_slider.rs` |
| Dithering classes (Floyd-Steinberg, Atkinson, Burkes, Jarvis-Judice-Ninke, Sierra, Stucki, ...) | Copyright (c) 2015 Cyotek Ltd. | MIT License (per the file headers; text below) | https://www.cyotek.com/blog/dithering-an-image-using-the-floyd-steinberg-algorithm-in-csharp | `crates/raster/src/dithering/` |
| Hershey single-stroke fonts | Dr. A. V. Hershey, U. S. National Bureau of Standards (see below) | license: see original project | - | `crates/hershey/` |
| Material Design Icons | Pictogrammers | see https://pictogrammers.com/docs/general/license/ (the LaserGRBL README names no license) | https://pictogrammers.com/library/mdi/ | `resources/svgicons/Mdi.data`, `Mdi.header` |
| Flag and custom SVG icons | As shipped with LaserGRBL; the SVG icon implementation was contributed by GabeMx5 (LaserGRBL README) | license: see original project | https://github.com/GabeMx5 | `resources/svgicons/Flags.*`, `Custom.*` |
| Grbl firmware images | The Grbl project (https://github.com/gnea/grbl) and its contributors. `02-v1.1h-custom, XY Homing` is a variant, its author is not recorded in LaserGRBL | GPL-3.0 per the Grbl project; LaserGRBL ships no notice with the images | https://github.com/gnea/grbl | `resources/firmware/*.hex` |
| avrdude.conf | The AVRDUDE project (file begins "$Id: avrdude.conf.in 1371 2016-02-15 20:15:07Z joerg_wunsch $") | the file carries no license text; license: see original project | https://github.com/avrdudes/avrdude | `resources/firmware/avrdude.conf` |
| Mono libgdiplus (algorithms of `GraphicsPath` flattening and ColorMatrix semantics, used as the reference for the Rust emulation) | The Mono project and libgdiplus contributors | license: see original project | https://github.com/mono/libgdiplus | `crates/potrace/src/gdi_path.rs`, `crates/raster/src/color_matrix.rs` |
| Images, sounds, translations and CSV data | LaserGRBL and its contributors (section 3). The Grbl message tables follow the Grbl project; some image names suggest third-party icon sets, their provenance is not documented in LaserGRBL | same license as L-ai-SERctrl unless noted; where unknown: license: see original project | https://github.com/arkypita/LaserGRBL | `resources/grafica/`, `resources/icons/`, `resources/sound/`, `resources/csv/`, `resources/i18n/`, `resources/generator/`, `resources/misc/` (see `resources/README.md`) |
| Noto Sans (font used to render the text in `resources/branding/splash.png`) | The Noto Project Authors | SIL Open Font License 1.1 | https://fonts.google.com/noto | `resources/branding/splash.png` (rendered glyphs only; at runtime the app uses system fonts) |

`resources/branding/` is original artwork by Andreas Pösch (GPL-3.0-or-later), not LaserGRBL material.

### Hershey fonts

"The Hershey Fonts were originally created by Dr. A. V. Hershey while working at the U. S. National
Bureau of Standards." The LaserGRBL source (`Hershey/Hershey.cs`) carries no acknowledgement or license
text for the glyph data, so none is quoted from it here.

### Not carried over

LaserGRBL used these components. L-ai-SERctrl replaces them with Rust libraries, they are credited for
their role (the LaserGRBL README names them):

- [websocket-sharp](https://github.com/sta/websocket-sharp), Copyright sta.blockhead (WebSocket client)
- [RJCP.SerialPortStream](https://github.com/jcurl/RJCP.DLL.SerialPortStream), by Jason Curl (serial port)
- [SharpGL](https://github.com/dwmkerr/sharpgl), Dave Kerr (OpenGL preview)
- [MS SVG Library](https://archive.codeplex.com/?p=svg), Microsoft Public License (SVG rendering)

### Boost Software License 1.0 (Clipper)

The Clipper header says: "Use, modification & distribution is subject to Boost Software License Ver 1."
(http://www.boost.org/LICENSE_1_0.txt). The license requires this text with source copies:

```
Boost Software License - Version 1.0 - August 17th, 2003

Permission is hereby granted, free of charge, to any person or organization
obtaining a copy of the software and accompanying documentation covered by
this license (the "Software") to use, reproduce, display, distribute,
execute, and transmit the Software, and to prepare derivative works of the
Software, and to permit third-parties to whom the Software is furnished to
do so, all subject to the following:

The copyright notices in the Software and this entire statement, including
the above license grant, this restriction and the following disclaimer,
must be included in all copies of the Software, in whole or in part, and
all derivative works of the Software, unless such copies or derivative
works are solely in the form of machine-executable object code generated by
a source language processor.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE, TITLE AND NON-INFRINGEMENT. IN NO EVENT
SHALL THE COPYRIGHT HOLDERS OR ANYONE DISTRIBUTING THE SOFTWARE BE LIABLE
FOR ANY DAMAGES OR OTHER LIABILITY, WHETHER IN CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

### MIT License (Cyotek dithering)

The Cyotek file headers say "Copyright (c) 2015 Cyotek Ltd. Licensed under the MIT License. See
LICENSE.txt for the full text." The LICENSE.txt is not part of LaserGRBL, so the standard MIT text is
given here:

```
MIT License

Copyright (c) 2015 Cyotek Ltd.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## 6. Rust dependencies

The Rust libraries L-ai-SERctrl is built with are listed in [THIRD_PARTY_RUST.md](THIRD_PARTY_RUST.md)
(generated by `tools/gen_third_party.py`). Their license texts are bundled in release packages as
`THIRD_PARTY_LICENSES.txt`.
