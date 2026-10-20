# preview (WP-U2a)
`PreviewPanel::new(Backend::Auto).show(ui, &PreviewInput) -> Vec<PreviewEvent>` each frame.
* `PreviewInput`: file `Arc<RwLock<GrblFile>>` + `file_revision` (bump after each load), positions, F/S, flags, `PreviewOptions` (settings keys), hotkey texts, optional `machine_area`.
* Events: `JogToPosition{x,y,fast}` (double click, right = fast; gated by `click_n_jog` and `jog_enabled`), `FellBackToCpu`, `ViewChanged`.
* Methods: `auto_size_drawing`, `zoom_in/out`, `on_color_change`, `PreviewPanel::on_program_ended(file)`.
* GL: instanced quads (GLSL 330 / ES 300), chunks of 2048 segments, per-chunk recolor via `DisplayListLink` generation. CPU: tiny-skia texture.
* Dev: `LASERGRBL_GALLERY_TAB=preview`, `LASERGRBL_PREVIEW_SAMPLE`, `LASERGRBL_PREVIEW_SHOT=out.png`. Use X11 (`env -u WAYLAND_DISPLAY`) in the sandbox.
