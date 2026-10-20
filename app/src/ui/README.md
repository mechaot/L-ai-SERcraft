# `ui/` foundation (WP-U0)

Run the gallery: `cargo run -p lasergrbl -- --gallery` (or `LASERGRBL_GALLERY=1`). Tabs: Theme (all 7 schemes,
every color), Icons (filterable), Widgets, Dialogs, i18n. `cargo test -p lasergrbl` renders every tab headless.

## Conventions
* Immediate mode: widgets bind to `&mut` values. Changes show up as `response.changed()`.
* Colors always from `Theme::current()` (re-read each frame, scheme switch is instant).
* Never hard-code strings: `tr("MainForm.MnConnect.Text")`, menus use `tr_plain` (strips `&`).
* Framework entry points (all in `main.rs::App`): `fonts::install`, `Theme::load_from_settings` + `apply`.
  Extension point: `app/src/main_form.rs::MainForm::ui(&mut self, ui)`.

## `theme.rs`
* `Scheme` (CADStyle, CADDark, BlueLaser, RedLaser, Dark, Hacker, Nighty), setting key `"Color Schema"` (default CADDark).
* `Theme::of(scheme)` / `Theme::current()` -> `Theme` with every `IColorScheme` property as `snake_case` `Color32`
  (`form_back_color`, `log_back_color`, `preview_*`, `log_left_*`, `log_right_*`, `controls_border`, ...), plus
  `form_buttons_color()`, `menu_highlight_color()`, `menu_separator_color()`, `log_color(LogRowColor)`.
* `Theme::switch(ctx, scheme)` persists + applies; `Theme::of(s).apply(ctx)` only restyles.
* `LogRowColor { Command, Startup, Alarm, Config, Feedback, Position, Others, RightGood, RightBad, RightOthers }`
  -> `theme.log_color(row)`; core's enum gets mapped to this.
* `change_color_brightness`, `control_paint_light/dark`, `named::*` (.NET named colors).

## `icons.rs`
* `icons()` global (`LegacyIcons` setting read once). Keys: `mdi-<name>`, `custom-<name>`, `flags-<name>`; legacy keys
  as in `LegacyIcons.header`. Unknown key -> `mdi-square-rounded`.
* `icon(ctx, "mdi-power-plug", 16.0) -> egui::Image` (themed tint, C# `colorize=true`);
  `icons().image_plain(..)` (no tint, flags/menu `colorize=false`); `image_tinted(.., Tint::Color(c))`.
* `icons().texture(ctx, key, px, tint)` for raw handles; `Icons::color_for(key, dark)`; `keys(prefix)`; `contains`.
* Legacy loader ignores tint like C#.

## `i18n.rs`
* `tr(key) -> &str` (culture -> `en` -> key), `tr_plain(key)`, `tr_fmt(key, &[&dyn Display])` for `{0}`/`{1}`,
  `format_cs`. `cultures()` = language menu (name + flag icon key). `set_language(Some("de"))` stores setting
  `"User Language"` (C# key, culture-name string); restart needed like C#. Default: env locale (`LANG`...) else `en`.
* Key scheme: `Strings.X`, `Form.Control.Property` (`.Text`, `.ToolTip`), sub-folder forms `Generator.CuttingTest...`.

## `dialog.rs`
* `show_modal(ctx, id, title, |ui| -> Option<T>)` generic modal form (result when closure returns Some).
* `MessageBox::new(text, title).buttons(..).icon(..).default_button(i)`; `.show(ctx) -> Option<DialogResult>`.
  `MessageBoxHost<Tag>`: `push(tag, mb)`; `show(ctx) -> Option<(Tag, DialogResult)>` (use for continuations).
* `PendingFile::open/save/folder(ctx, FileDialogOptions{filter: Some(filters::OPEN_ANY.into()), ..})`,
  `poll() -> Option<Option<PathBuf>>` each frame. `filters::*` = exact C# filter strings.

## `fonts.rs`
`fonts::install(ctx)`: default fonts + first system CJK font found (Noto CJK, WQY, Droid fallback, `msyh.ttc`).

## `widgets/` (all themed)
| widget | C# | use |
|---|---|---|
| `ColorSlider` | ColorSlider | `ui.add(ColorSlider::new(&mut i32, 0..=100).schema(..).orientation(..))` |
| `TrackBar` | WinForms TrackBar (Jog speed, LabelTB) | `ui.add(TrackBar::new(&mut v, 10..=200).small_change(5).tick_frequency(10))` |
| `LabelTb` | LabelTB | state struct, `show(ui, len) -> {changed, commit}` |
| `ImageButton` | ImageButton | `ui.add(ImageButton::new("mdi-play").size(v).rounded_borders(true).alt(k, on).caption(t))` |
| `GrblButton` | GrblButton | `ui.add(GrblButton::new(text).icon("mdi-.."))` |
| `grbl_group_box` | GrblGroupBox | `grbl_group_box(ui, title, \|ui\| ..)` |
| `GrblTextBox` | GrblTextBox | state struct, `show(ui, watermark, w) -> (Response, Option<String>)` |
| `PlaceholderTextBox`, `TextInput`, `colored_border` | PlaceholderTextbox, TextInput, ColoredBorderControl | `ui.add(..)` |
| `DecimalInputRanged`, `IntegerInputRanged`, `NumericUpDown` | NumericInput/* | `ui.add(..)`; `.show(ui)` gives `NumericOutput{committed,on_the_fly,..}` |
| `ColorProgressBar`, `DoubleProgressBar`, `WaitingProgressBar` | same | state structs, `show(ui)` |
| `flat_combo`, `enum_combo` | FlatComboBox, EnumComboBox | fns |
| `UsageClassPreview` | UsageClassPreview | `ui.add(UsageClassPreview::new(&hours))` |
| `PictureBox` | MyPictureBox | state struct (`fence`), `show(ui, tex, size)` |
| `data_grid` | MyDatagridView | egui_extras table helper |
| `InputBox` | Tools/InputBox | state struct, `show(ctx) -> Option<InputBoxResult>` |
| `graph::*` | Tools/Graph | `rounded_rect_points`, gradients, `wheel_notches` |

Numeric inputs and `ColorSlider`/`TrackBar` need distinct ids when several share one call site: use `.id(Id::unique(..))`
on numeric inputs.

## Deviations
* Vertical `ColorSlider`/`TrackBar` supported; no `ThumbCustomShape`.
* Legacy-icon hover/disabled brightness is approximated with tint/overlay.
* `Id::new` is deprecated in egui 0.37: use `egui::Id::unique`.
