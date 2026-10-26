// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Job geometry for the GL preview: port of `Objects3D.cs` `Grbl3D.Draw` / `Invalidate`.
//!
//! The C# builds GL display lists of 5000 vertices (`Object3DDisplayList`). Here a [`Layer`] is a list
//! of [`Chunk`]s of line segments (instanced quads on the GPU). Each on-layer chunk owns a
//! [`DisplayListLink`] that the commands share: `set_result` / `clear_result` bump its generation and
//! the panel recolors only that chunk (C# `Object3DDisplayList.Invalidate`).

use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use lg_dotnet::decimal_to_f32;
use lg_gcode::{CommandStatus, DisplayListLink, GrblFile, StatePositionBuilder};

/// Segments per chunk (C# `MAX_VECTOR_IN_DISPLAY_LIST` = 5000 vertices = 2500 segments).
pub const SEGMENTS_PER_CHUNK: usize = 2048;

/// Commands analysed per read-lock hold in the builder.
const BATCH: usize = 4096;

/// One line segment as uploaded to the GPU (20 bytes, instanced).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub rgba: [u8; 4],
}

/// RGB color, 0..255.
pub type Rgb = [u8; 3];

/// Colors used to paint the job (C# `ColorScheme.Preview*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JobColors {
    pub background: Rgb,
    pub laser_power: Rgb,
    pub other_movement: Rgb,
    pub command_ok: Rgb,
    pub command_ko: Rgb,
    pub command_wait: Rgb,
}

/// C# `Grbl3D.Map` blend of a color over the background by intensity (0..255).
///
/// `bg`: background. `c`: foreground. `intensity`: 0..255. Returns the opaque RGBA.
pub fn blend(bg: Rgb, c: Rgb, intensity: u8) -> [u8; 4] {
    let t = intensity as f32 / 255.0;
    let m = |b: u8, f: u8| -> u8 { ((t * (f as f32 - b as f32) + b as f32).round().clamp(0.0, 255.0)) as u8 };
    [m(bg[0], c[0]), m(bg[1], c[1]), m(bg[2], c[2]), 255]
}

/// C# `ColorHelper.Blend`: 1/3 this + 2/3 blend color, per channel (0..1).
///
/// `a`: color RGBA. `b`: blend color RGBA. Returns mixed RGBA.
pub fn color_helper_blend(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [(a[0] + b[0] * 2.0) / 3.0, (a[1] + b[1] * 2.0) / 3.0, (a[2] + b[2] * 2.0) / 3.0, (a[3] + b[3] * 2.0) / 3.0]
}

/// C# `GlColorHelper.FromColor`: byte color to 0..1 floats RGBA.
///
/// `c`: RGBA bytes. Returns floats.
pub fn gl_color_from(c: [u8; 4]) -> [f32; 4] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0]
}

/// A GPU chunk of segments plus the CPU data needed to recolor it.
pub struct Chunk {
    pub segs: Vec<Segment>,
    /// Intensity 0..255 per segment.
    pub intensity: Vec<u8>,
    /// Command index per segment.
    pub cmd: Vec<u32>,
    /// Link shared with the commands of this chunk.
    pub link: DisplayListLink,
    /// Link generation applied to the colors.
    pub seen_gen: u64,
    /// Bumped on every change of `segs`; the GL side re-uploads when it differs.
    pub version: u64,
    /// Bounding box `[xmin, ymin, xmax, ymax]` of the segments (view culling).
    pub bbox: [f32; 4],
}

impl Chunk {
    /// Empty chunk.
    fn new() -> Self {
        Self {
            segs: Vec::with_capacity(SEGMENTS_PER_CHUNK),
            intensity: Vec::with_capacity(SEGMENTS_PER_CHUNK),
            cmd: Vec::with_capacity(SEGMENTS_PER_CHUNK),
            link: DisplayListLink::new(),
            seen_gen: 0,
            version: 1,
            bbox: [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
        }
    }

    /// Grow the bounding box by a segment.
    ///
    /// `s`: segment.
    fn grow(&mut self, s: &Segment) {
        self.bbox[0] = self.bbox[0].min(s.x0).min(s.x1);
        self.bbox[1] = self.bbox[1].min(s.y0).min(s.y1);
        self.bbox[2] = self.bbox[2].max(s.x0).max(s.x1);
        self.bbox[3] = self.bbox[3].max(s.y0).max(s.y1);
    }
}

/// A list of chunks (C# `Grbl3D` display lists).
#[derive(Default)]
pub struct Layer {
    pub chunks: Vec<Chunk>,
}

impl Layer {
    /// Total segments (vertex counter = 2 per segment).
    pub fn segment_count(&self) -> usize {
        self.chunks.iter().map(|c| c.segs.len()).sum()
    }
}

/// Built job geometry, shared between builder thread, UI thread and GL callback.
#[derive(Default)]
pub struct Scene {
    /// Laser-on movements (C# `mGrbl3D`).
    pub on: Layer,
    /// All linear movements + arcs in the "other movement" color (C# `mGrbl3DOff`).
    pub off: Layer,
    /// Builder finished.
    pub done: bool,
}

/// Scene + build state.
pub struct SceneShared {
    /// Unique id (GL buffers are keyed by it).
    pub id: u64,
    pub scene: Mutex<Scene>,
    /// Progress in 1/100 percent.
    pub progress: AtomicU32,
    /// Ask the builder thread to stop.
    pub cancel: AtomicBool,
}

impl SceneShared {
    /// New empty scene.
    ///
    /// `id`: unique id.
    pub fn new(id: u64) -> Arc<Self> {
        Arc::new(Self { id, scene: Mutex::new(Scene::default()), progress: AtomicU32::new(0), cancel: AtomicBool::new(false) })
    }

    /// Loading percentage 0..100 (C# `LoadingPercentage`).
    pub fn percentage(&self) -> f64 {
        self.progress.load(Ordering::Relaxed) as f64 / 100.0
    }

    /// Total vertices of the on layer (C# `VertexCounter`).
    pub fn vertex_count(&self) -> u64 {
        let s = self.scene.lock().unwrap_or_else(|e| e.into_inner());
        s.on.segment_count() as u64 * 2
    }
}

/// Incremental chunk filler for one layer.
struct LayerBuilder {
    cur: Chunk,
}

impl LayerBuilder {
    /// Add a segment; returns a finished chunk when the current one is full.
    ///
    /// `seg`: segment. `intensity`: 0..255. `cmd`: command index.
    fn push(&mut self, seg: Segment, intensity: u8, cmd: u32) -> Option<Chunk> {
        self.cur.grow(&seg);
        self.cur.segs.push(seg);
        self.cur.intensity.push(intensity);
        self.cur.cmd.push(cmd);
        if self.cur.segs.len() >= SEGMENTS_PER_CHUNK {
            Some(std::mem::replace(&mut self.cur, Chunk::new()))
        } else {
            None
        }
    }
}

/// Color of a command result (C# `Grbl3D.Invalidate` switch).
///
/// `status`: command status. `colors`: palette. `base`: color without status. Returns the color.
pub fn status_color(status: CommandStatus, colors: &JobColors, base: Rgb) -> Rgb {
    match status {
        CommandStatus::ResponseGood => colors.command_ok,
        CommandStatus::ResponseBad | CommandStatus::InvalidResponse => colors.command_ko,
        CommandStatus::WaitingResponse => colors.command_wait,
        CommandStatus::Queued => base,
    }
}

/// Segment color that draws nothing (premultiplied blend: rgb and alpha are 0).
pub const HIDDEN_RGBA: [u8; 4] = [0, 0, 0, 0];

/// Is command `cmd` inside one of `hidden`. `hidden`: ranges. `cmd`: command index. Returns bool.
fn is_hidden(hidden: &[Range<u32>], cmd: u32) -> bool {
    hidden.iter().any(|r| r.contains(&cmd))
}

/// Recolor one on-layer chunk from the command states (C# `Grbl3D.Invalidate` per display list).
///
/// `chunk`: chunk to update. `file`: program (read). `colors`: palette. `show_executed`: use status colors.
/// `hidden`: command ranges that are not drawn (alpha 0).
pub fn recolor_chunk(chunk: &mut Chunk, file: &GrblFile, colors: &JobColors, show_executed: bool, hidden: &[Range<u32>]) {
    // read generation first so a later invalidation is not lost
    let generation = chunk.link.generation();
    let cmds = file.commands();
    for i in 0..chunk.segs.len() {
        let base = if show_executed {
            match cmds.get(chunk.cmd[i] as usize) {
                Some(c) => status_color(c.status(), colors, colors.laser_power),
                None => colors.laser_power,
            }
        } else {
            colors.laser_power
        };
        chunk.segs[i].rgba = if is_hidden(hidden, chunk.cmd[i]) { HIDDEN_RGBA } else { blend(colors.background, base, chunk.intensity[i]) };
    }
    chunk.seen_gen = generation;
    chunk.version += 1;
}

/// Recolor an off-layer chunk (no status coloring).
///
/// `chunk`: chunk. `colors`: palette. `hidden`: command ranges that are not drawn (alpha 0).
pub fn recolor_off_chunk(chunk: &mut Chunk, colors: &JobColors, hidden: &[Range<u32>]) {
    for i in 0..chunk.segs.len() {
        chunk.segs[i].rgba = if is_hidden(hidden, chunk.cmd[i]) { HIDDEN_RGBA } else { blend(colors.background, colors.other_movement, chunk.intensity[i]) };
    }
    chunk.version += 1;
}

/// Outcome of [`build_scene`].
#[derive(Debug, PartialEq, Eq)]
pub enum BuildResult {
    Done,
    Cancelled,
}

/// Build the geometry of `file` into `shared` (C# `Grbl3D.Draw`, both layers in one pass).
///
/// `file`: program. `colors`: palette for the initial colors. `show_executed`: initial colors follow command
/// status. `shared`: sink (chunks are published as they fill, progress updated). Returns `Done` or `Cancelled`.
pub fn build_scene(file: &Arc<RwLock<GrblFile>>, colors: &JobColors, show_executed: bool, shared: &SceneShared) -> BuildResult {
    let mut spb = StatePositionBuilder::new();
    let mut on = LayerBuilder { cur: Chunk::new() };
    let mut off = LayerBuilder { cur: Chunk::new() };
    let mut idx = 0usize;
    let mut warned = false;
    loop {
        if shared.cancel.load(Ordering::Relaxed) {
            return BuildResult::Cancelled;
        }
        // short read lock per batch so writers (core) are not starved
        let guard = file.read().unwrap_or_else(|e| e.into_inner());
        let total = guard.commands().len();
        if idx >= total {
            break;
        }
        let end = (idx + BATCH).min(total);
        let spindle = guard.range().spindle_range;
        let mut new_on: Vec<Chunk> = Vec::new();
        let mut new_off: Vec<Chunk> = Vec::new();
        for i in idx..end {
            // work on a copy: helper build rewrites the line
            let mut cmd = guard.commands()[i].clone();
            cmd.build_helper();
            if let Err(e) = spb.analyze_command(&mut cmd, false, None) {
                if !warned {
                    log::warn!("preview: command {i} skipped: {e}");
                    warned = true;
                }
                continue;
            }
            if !spb.true_movement() {
                continue;
            }
            if spb.g0g1() && cmd.is_linear_movement() {
                let seg_xy = (
                    decimal_to_f32(spb.x().previous()),
                    decimal_to_f32(spb.y().previous()),
                    decimal_to_f32(spb.x().number()),
                    decimal_to_f32(spb.y().number()),
                );
                let alpha = spb.get_current_alpha(&spindle).unwrap_or(255).clamp(0, 255) as u8;
                let seg = |rgba: [u8; 4]| Segment { x0: seg_xy.0, y0: seg_xy.1, x1: seg_xy.2, y1: seg_xy.3, rgba };
                // off layer: every linear move, full intensity
                if let Some(c) = off.push(seg(blend(colors.background, colors.other_movement, 255)), 255, i as u32) {
                    new_off.push(c);
                }
                // on layer: only while burning, intensity from S
                if spb.laser_burning() && alpha > 0 {
                    let base = if show_executed { status_color(guard.commands()[i].status(), colors, colors.laser_power) } else { colors.laser_power };
                    if let Some(c) = on.push(seg(blend(colors.background, base, alpha)), alpha, i as u32) {
                        new_on.push(c);
                    }
                }
            } else if spb.g2g3() && cmd.is_arc_movement() {
                let Ok(ah) = spb.get_arc_helper(&cmd) else { continue };
                if ah.rect_w > 0.0 && ah.rect_h > 0.0 {
                    let alpha = spb.get_current_alpha(&spindle).unwrap_or(255).clamp(0, 255) as u8;
                    let start = ah.start_angle;
                    let aw = ah.angular_width;
                    let sign = if aw > 0.0 { 1.0 } else if aw < 0.0 { -1.0 } else { 0.0 };
                    let step = (aw / std::f64::consts::PI / 36.0).abs();
                    let mut last: Option<(f32, f32)> = None;
                    let mut angle = start;
                    // polyline as in C#: pairs (last, current)
                    let mut guard_n = 0;
                    while sign * (angle - start) <= sign * aw && guard_n < 4096 {
                        guard_n += 1;
                        let x = (ah.center_x + ah.rect_w / 2.0 * angle.cos()) as f32;
                        let y = (ah.center_y + ah.rect_h / 2.0 * angle.sin()) as f32;
                        if let Some((lx, ly)) = last {
                            let mk = |rgba: [u8; 4]| Segment { x0: lx, y0: ly, x1: x, y1: y, rgba };
                            if let Some(c) = off.push(mk(blend(colors.background, colors.other_movement, alpha)), alpha, i as u32) {
                                new_off.push(c);
                            }
                            if alpha > 0 {
                                let base = if show_executed { status_color(guard.commands()[i].status(), colors, colors.laser_power) } else { colors.laser_power };
                                if let Some(c) = on.push(mk(blend(colors.background, base, alpha)), alpha, i as u32) {
                                    new_on.push(c);
                                }
                            }
                        }
                        last = Some((x, y));
                        if step <= 0.0 {
                            break;
                        }
                        angle += sign * step;
                    }
                }
            }
        }
        drop(guard);
        idx = end;
        shared.progress.store(((idx as f64 / total.max(1) as f64) * 10000.0) as u32, Ordering::Relaxed);
        if !new_on.is_empty() || !new_off.is_empty() {
            let mut s = shared.scene.lock().unwrap_or_else(|e| e.into_inner());
            s.on.chunks.extend(new_on);
            s.off.chunks.extend(new_off);
        }
    }
    // flush partial chunks
    {
        let mut s = shared.scene.lock().unwrap_or_else(|e| e.into_inner());
        if !on.cur.segs.is_empty() {
            s.on.chunks.push(std::mem::replace(&mut on.cur, Chunk::new()));
        }
        if !off.cur.segs.is_empty() {
            s.off.chunks.push(std::mem::replace(&mut off.cur, Chunk::new()));
        }
    }
    // link commands to their chunk, so result changes invalidate it
    link_commands(file, shared);
    shared.scene.lock().unwrap_or_else(|e| e.into_inner()).done = true;
    shared.progress.store(10000, Ordering::Relaxed);
    BuildResult::Done
}

/// Attach each chunk's [`DisplayListLink`] to its commands (C# `command.LinkedDisplayList = list`).
///
/// `file`: program (write lock, briefly). `shared`: built scene.
fn link_commands(file: &Arc<RwLock<GrblFile>>, shared: &SceneShared) {
    let pairs: Vec<(u32, DisplayListLink)> = {
        let s = shared.scene.lock().unwrap_or_else(|e| e.into_inner());
        let mut v = Vec::new();
        for c in &s.on.chunks {
            let mut last = u32::MAX;
            for &ci in &c.cmd {
                if ci != last {
                    v.push((ci, c.link.clone()));
                    last = ci;
                }
            }
        }
        v
    };
    let mut f = file.write().unwrap_or_else(|e| e.into_inner());
    let cmds = f.commands_mut();
    for (ci, link) in pairs {
        if let Some(c) = cmds.get_mut(ci as usize) {
            c.set_linked_display_list(Some(link));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lg_gcode::GrblCommand;
    use std::time::Instant;

    /// Raster-like program with `lines` G1 commands.
    fn raster_file(lines: usize) -> Arc<RwLock<GrblFile>> {
        let mut f = GrblFile::new();
        {
            let l = f.commands_mut();
            l.push(GrblCommand::new("G90"));
            l.push(GrblCommand::new("M4"));
            l.push(GrblCommand::new("G0 X0 Y0"));
            let mut y = 0.0f64;
            let mut x = 0.0f64;
            let mut dir = 1.0;
            for i in 0..lines {
                // short strokes with changing S, row change every 200 moves
                if i % 200 == 199 {
                    y += 0.1;
                    l.push(GrblCommand::new(&format!("G0 X{:.3} Y{:.3}", x, y)));
                    dir = -dir;
                } else {
                    x += dir * 0.1;
                    l.push(GrblCommand::new(&format!("G1 X{:.3} Y{:.3} S{}", x, y, (i * 7) % 1000)));
                }
            }
            l.push(GrblCommand::new("M5"));
        }
        f.analyze().unwrap();
        Arc::new(RwLock::new(f))
    }

    fn colors() -> JobColors {
        JobColors { background: [30, 30, 30], laser_power: [220, 220, 220], other_movement: [60, 80, 85], command_ok: [0, 200, 0], command_ko: [200, 0, 0], command_wait: [250, 150, 150] }
    }

    #[test]
    fn blend_endpoints() {
        assert_eq!(blend([10, 20, 30], [200, 100, 50], 0), [10, 20, 30, 255]);
        assert_eq!(blend([10, 20, 30], [200, 100, 50], 255), [200, 100, 50, 255]);
    }

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn small_build_and_invalidate() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let file = raster_file(5000);
        let shared = SceneShared::new(1);
        assert_eq!(build_scene(&file, &colors(), true, &shared), BuildResult::Done);
        let mut s = shared.scene.lock().unwrap();
        assert!(s.done);
        assert!(s.on.segment_count() > 4000);
        assert!(s.on.chunks.len() >= 2);
        // find a linked command and set its result: generation of its chunk must bump
        let (cmd_idx, g0) = (s.on.chunks[0].cmd[0] as usize, s.on.chunks[0].link.generation());
        file.read().unwrap().commands()[cmd_idx].set_result("OK", false);
        assert!(s.on.chunks[0].link.generation() > g0);
        // recolor shows the OK color on the first segment
        let f = file.read().unwrap();
        let cols = colors();
        let ch = &mut s.on.chunks[0];
        ch.intensity[0] = 255;
        recolor_chunk(ch, &f, &cols, true, &[]);
        let first = ch.segs[0].rgba;
        let cmd0 = ch.cmd[0];
        recolor_chunk(ch, &f, &cols, true, &[cmd0..cmd0 + 1]);
        assert_eq!(ch.segs[0].rgba, HIDDEN_RGBA);
        // only segments of that command vanish
        for i in 0..ch.segs.len() {
            assert_eq!(ch.segs[i].rgba == HIDDEN_RGBA, ch.cmd[i] == cmd0);
        }
        recolor_chunk(ch, &f, &cols, true, &[]);
        assert_eq!(ch.segs[0].rgba, first);
        recolor_chunk(ch, &f, &cols, true, &[]);
        assert_eq!(ch.segs[0].rgba, [0, 200, 0, 255]);
    }

    #[test]
    fn arc_builds_polyline() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let mut f = GrblFile::new();
        for l in ["G90", "M4", "G0 X10 Y0", "G2 X10 Y0 I-10 J0 S500 F1000"] {
            f.commands_mut().push(GrblCommand::new(l));
        }
        f.analyze().unwrap();
        let file = Arc::new(RwLock::new(f));
        let shared = SceneShared::new(2);
        build_scene(&file, &colors(), true, &shared);
        let s = shared.scene.lock().unwrap();
        assert!(s.on.segment_count() > 50, "got {}", s.on.segment_count());
    }

    /// Vertex buffer build for 500k lines: must be fast enough for smooth loading.
    #[test]
    fn build_time_500k_lines() {
        // isolated settings + data dir
        let _g = crate::forms::form_util::test_guard();
        let file = raster_file(500_000);
        let shared = SceneShared::new(3);
        let t = Instant::now();
        assert_eq!(build_scene(&file, &colors(), true, &shared), BuildResult::Done);
        let ms = t.elapsed().as_millis();
        let segs = shared.scene.lock().unwrap().on.segment_count();
        eprintln!("build 500k: {ms} ms, {segs} on-segments");
        assert!(segs > 450_000);
        assert!(ms < 15_000, "build too slow: {ms} ms");
        // per-frame work with no dirty chunk is a generation compare only
        let t = Instant::now();
        let s = shared.scene.lock().unwrap();
        let dirty = s.on.chunks.iter().filter(|c| c.link.generation() != c.seen_gen).count();
        eprintln!("dirty scan {} chunks: {} us ({} dirty)", s.on.chunks.len(), t.elapsed().as_micros(), dirty);
    }
}
