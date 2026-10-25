// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! OpenGL renderer of the job preview (replaces SharpGL display lists of `Objects3D.cs`).
//!
//! Lines are drawn as instanced quads (`gl_VertexID` corners, one 20 byte [`Segment`] per instance) in a
//! vertex shader that expands them to the wanted pixel width, with analytic edge anti-aliasing and an
//! optional dash pattern. GLSL 330 core on desktop GL, 300 es on GLES. Everything is drawn from an egui
//! `PaintCallback` (`egui_glow::CallbackFn`).

use std::sync::{Arc, Mutex};

use eframe::glow::{self, HasContext};

use super::scene::{Scene, SceneShared, Segment};

/// A batch of dynamic segments drawn with one width / dash (grid, axes, bbox, pointer).
#[derive(Debug, Clone, Default)]
pub struct Batch {
    pub segs: Vec<Segment>,
    /// Line width in points.
    pub width: f32,
    /// Dash length in points (0 = solid); on = off = this length.
    pub dash: f32,
}

/// One draw step, in painting order.
#[derive(Clone)]
pub enum Item {
    Batch(Batch),
    /// Laser-off layer of the scene.
    Off,
    /// Laser-on layer of the scene (width in points).
    On(f32),
}

/// Everything the paint callback needs for one frame.
#[derive(Clone)]
pub struct Frame {
    /// World window `(left, right, bottom, top)`.
    pub view: [f64; 4],
    /// Background color.
    pub background: [u8; 3],
    pub items: Arc<Vec<Item>>,
    pub scene: Option<Arc<SceneShared>>,
}

/// Shared between UI thread and the paint callback.
#[derive(Default)]
pub struct GlShared {
    pub state: Option<GlState>,
    /// Init failed: caller should fall back to the CPU renderer.
    pub failed: Option<String>,
    /// Init finished OK.
    pub ready: bool,
    /// Info strings for the log (vendor, renderer, version).
    pub info: String,
    /// Frames painted.
    pub frames: u64,
}

/// Uniform locations.
struct Uniforms {
    origin: Option<glow::UniformLocation>,
    scale: Option<glow::UniformLocation>,
    size: Option<glow::UniformLocation>,
    halfw: Option<glow::UniformLocation>,
    dash: Option<glow::UniformLocation>,
}

struct ChunkBuf {
    buf: glow::Buffer,
    version: u64,
    count: i32,
    /// Copy of the chunk bbox for culling.
    bbox: [f32; 4],
}

/// True when a bbox may touch the view.
///
/// `bbox`: `[xmin, ymin, xmax, ymax]`. `view`: `(left, right, bottom, top)`. `margin`: world units.
fn bbox_visible(bbox: &[f32; 4], view: [f64; 4], margin: f64) -> bool {
    (bbox[0] as f64 - margin) <= view[1] && (bbox[2] as f64 + margin) >= view[0] && (bbox[1] as f64 - margin) <= view[3] && (bbox[3] as f64 + margin) >= view[2]
}

/// GL objects.
pub struct GlState {
    gl: Arc<glow::Context>,
    prog: glow::Program,
    vao: glow::VertexArray,
    dynbuf: glow::Buffer,
    u: Uniforms,
    scene_id: u64,
    on: Vec<ChunkBuf>,
    off: Vec<ChunkBuf>,
}

const VS_BODY: &str = r#"
layout(location = 0) in vec4 a_seg;
layout(location = 1) in vec4 a_col;
uniform vec2 u_origin;   // world left, bottom
uniform vec2 u_scale;    // px per world unit
uniform vec2 u_size;     // viewport px
uniform float u_halfw;   // half line width px
out vec4 v_col;
out float v_across;
out float v_along;
void main() {
    float t = float(gl_VertexID >> 1);
    float s = float((gl_VertexID & 1) * 2 - 1);
    vec2 p0 = (a_seg.xy - u_origin) * u_scale;
    vec2 p1 = (a_seg.zw - u_origin) * u_scale;
    vec2 d = p1 - p0;
    float len = length(d);
    vec2 dir = len > 1e-6 ? d / len : vec2(1.0, 0.0);
    vec2 n = vec2(-dir.y, dir.x);
    float hw = u_halfw + 1.0;
    // square caps of half width, so joined segments leave no gaps
    vec2 p = mix(p0, p1, t) + dir * (t * 2.0 - 1.0) * u_halfw + n * s * hw;
    gl_Position = vec4(p / u_size * 2.0 - 1.0, 0.0, 1.0);
    v_col = a_col;
    v_across = s * hw;
    // dash phase from the world coordinate along the (axis aligned) line, in px
    vec2 w = mix(a_seg.xy, a_seg.zw, t);
    vec2 dw = a_seg.zw - a_seg.xy;
    v_along = (abs(dw.x) >= abs(dw.y) ? w.x * u_scale.x : w.y * u_scale.y);
}
"#;

const FS_BODY: &str = r#"
in vec4 v_col;
in float v_across;
in float v_along;
uniform float u_halfw;
uniform float u_dash;
out vec4 o_color;
void main() {
    float a = clamp(u_halfw + 0.5 - abs(v_across), 0.0, 1.0);
    if (u_dash > 0.0 && mod(v_along, 2.0 * u_dash) > u_dash) discard;
    o_color = vec4(v_col.rgb * (v_col.a * a), v_col.a * a);
}
"#;

/// View a segment slice as bytes.
///
/// `s`: segments (repr(C), 20 bytes each, no padding). Returns the byte view.
fn seg_bytes(s: &[Segment]) -> &[u8] {
    // SAFETY: Segment is repr(C) of 4 f32 + 4 u8 (20 bytes, align 4, no padding); any bytes are valid to read.
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s)) }
}

impl GlState {
    /// Compile shaders and create buffers.
    ///
    /// `gl`: context. Returns the state or an error text (caller falls back to the CPU renderer).
    pub fn new(gl: &Arc<glow::Context>) -> Result<Self, String> {
        let v = gl.version();
        let ok = if v.is_embedded { v.major >= 3 } else { v.major > 3 || (v.major == 3 && v.minor >= 3) };
        if !ok {
            return Err(format!("OpenGL {}.{} too old (need 3.3 / ES 3.0)", v.major, v.minor));
        }
        let header = if v.is_embedded { "#version 300 es\nprecision highp float;\n" } else { "#version 330 core\n" };
        unsafe {
            let prog = gl.create_program()?;
            let mut shaders = Vec::new();
            for (kind, body) in [(glow::VERTEX_SHADER, VS_BODY), (glow::FRAGMENT_SHADER, FS_BODY)] {
                let sh = gl.create_shader(kind)?;
                gl.shader_source(sh, &format!("{header}{body}"));
                gl.compile_shader(sh);
                if !gl.get_shader_compile_status(sh) {
                    let log = gl.get_shader_info_log(sh);
                    gl.delete_shader(sh);
                    gl.delete_program(prog);
                    return Err(format!("shader compile: {log}"));
                }
                gl.attach_shader(prog, sh);
                shaders.push(sh);
            }
            gl.link_program(prog);
            let linked = gl.get_program_link_status(prog);
            for sh in shaders {
                gl.detach_shader(prog, sh);
                gl.delete_shader(sh);
            }
            if !linked {
                let log = gl.get_program_info_log(prog);
                gl.delete_program(prog);
                return Err(format!("program link: {log}"));
            }
            let u = Uniforms {
                origin: gl.get_uniform_location(prog, "u_origin"),
                scale: gl.get_uniform_location(prog, "u_scale"),
                size: gl.get_uniform_location(prog, "u_size"),
                halfw: gl.get_uniform_location(prog, "u_halfw"),
                dash: gl.get_uniform_location(prog, "u_dash"),
            };
            let vao = gl.create_vertex_array()?;
            let dynbuf = gl.create_buffer()?;
            Ok(Self { gl: gl.clone(), prog, vao, dynbuf, u, scene_id: 0, on: Vec::new(), off: Vec::new() })
        }
    }

    /// Free GL objects. Call from the paint callback only (context current).
    pub fn release(&mut self) {
        unsafe {
            self.free_chunks();
            self.gl.delete_buffer(self.dynbuf);
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_program(self.prog);
        }
    }

    /// Delete the chunk buffers.
    unsafe fn free_chunks(&mut self) {
        for c in self.on.drain(..).chain(self.off.drain(..)) {
            unsafe { self.gl.delete_buffer(c.buf) };
        }
    }

    /// Bind segment attributes of a buffer to the VAO.
    unsafe fn bind_segments(&self, buf: glow::Buffer) {
        unsafe {
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(buf));
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_f32(0, 4, glow::FLOAT, false, 20, 0);
            self.gl.vertex_attrib_divisor(0, 1);
            self.gl.enable_vertex_attrib_array(1);
            self.gl.vertex_attrib_pointer_f32(1, 4, glow::UNSIGNED_BYTE, true, 20, 16);
            self.gl.vertex_attrib_divisor(1, 1);
        }
    }

    /// Sync GPU buffers of a layer with the CPU chunks (upload changed ones).
    unsafe fn sync_layer(gl: &glow::Context, bufs: &mut Vec<ChunkBuf>, chunks: &[super::scene::Chunk]) {
        unsafe {
            while bufs.len() < chunks.len() {
                match gl.create_buffer() {
                    Ok(buf) => bufs.push(ChunkBuf { buf, version: 0, count: 0, bbox: [0.0; 4] }),
                    Err(_) => return,
                }
            }
            for (b, c) in bufs.iter_mut().zip(chunks) {
                if b.version != c.version {
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(b.buf));
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, seg_bytes(&c.segs), glow::DYNAMIC_DRAW);
                    b.version = c.version;
                    b.count = c.segs.len() as i32;
                    b.bbox = c.bbox;
                }
            }
        }
    }

    /// Set per-draw uniforms and draw `count` instances from `buf`.
    unsafe fn draw(&self, buf: glow::Buffer, count: i32, halfw: f32, dash: f32) {
        if count <= 0 {
            return;
        }
        unsafe {
            self.gl.uniform_1_f32(self.u.halfw.as_ref(), halfw);
            self.gl.uniform_1_f32(self.u.dash.as_ref(), dash);
            self.bind_segments(buf);
            self.gl.draw_arrays_instanced(glow::TRIANGLE_STRIP, 0, 4, count);
        }
    }

    /// Paint one frame into the current viewport.
    ///
    /// `frame`: parameters. `vp`: viewport `(x, y, w, h)` in physical px (y from bottom). `ppp`: pixels per point.
    pub fn paint(&mut self, frame: &Frame, vp: (i32, i32, i32, i32), ppp: f32) {
        let gl = self.gl.clone();
        let [l, r, b, t] = frame.view;
        let (w, h) = (vp.2 as f32, vp.3 as f32);
        unsafe {
            gl.viewport(vp.0, vp.1, vp.2, vp.3);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(Some(self.prog));
            gl.bind_vertex_array(Some(self.vao));
            gl.uniform_2_f32(self.u.origin.as_ref(), l as f32, b as f32);
            gl.uniform_2_f32(self.u.scale.as_ref(), (w as f64 / (r - l)) as f32, (h as f64 / (t - b)) as f32);
            gl.uniform_2_f32(self.u.size.as_ref(), w, h);

            // background: clear obeys egui's scissor (clip rect of the widget)
            gl.clear_color(
                frame.background[0] as f32 / 255.0,
                frame.background[1] as f32 / 255.0,
                frame.background[2] as f32 / 255.0,
                1.0,
            );
            gl.clear(glow::COLOR_BUFFER_BIT);

            // scene buffers
            if let Some(sc) = &frame.scene {
                if sc.id != self.scene_id {
                    self.free_chunks();
                    self.scene_id = sc.id;
                }
                let s: std::sync::MutexGuard<'_, Scene> = sc.scene.lock().unwrap_or_else(|e| e.into_inner());
                Self::sync_layer(&gl, &mut self.on, &s.on.chunks);
                Self::sync_layer(&gl, &mut self.off, &s.off.chunks);
            } else if self.scene_id != 0 {
                self.free_chunks();
                self.scene_id = 0;
            }

            // one point in world units (culling margin)
            let px_world = (r - l) / (w as f64 / ppp as f64).max(1.0);
            for item in frame.items.iter() {
                match item {
                    Item::Batch(batch) => {
                        if batch.segs.is_empty() {
                            continue;
                        }
                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.dynbuf));
                        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, seg_bytes(&batch.segs), glow::STREAM_DRAW);
                        self.draw(self.dynbuf, batch.segs.len() as i32, batch.width * ppp / 2.0, batch.dash * ppp);
                    }
                    Item::Off => {
                        for c in self.off.iter().filter(|c| bbox_visible(&c.bbox, frame.view, px_world * 2.0)) {
                            self.draw(c.buf, c.count, 0.5 * ppp, 0.0);
                        }
                    }
                    Item::On(width) => {
                        for c in self.on.iter().filter(|c| bbox_visible(&c.bbox, frame.view, px_world * (*width as f64 + 1.0))) {
                            self.draw(c.buf, c.count, width * ppp / 2.0, 0.0);
                        }
                    }
                }
            }

            // leave state clean for egui
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }
}

/// Lazily create the GL state inside the paint callback.
///
/// `shared`: callback state. `gl`: context from the egui painter. Returns true when drawing is possible.
pub fn ensure_init(shared: &mut GlShared, gl: &Arc<glow::Context>) -> bool {
    if shared.state.is_some() {
        return true;
    }
    if shared.failed.is_some() {
        return false;
    }
    match GlState::new(gl) {
        Ok(s) => {
            shared.state = Some(s);
            shared.ready = true;
            // vendor / renderer / version like the C# log line
            let (vendor, renderer, version) = unsafe {
                (gl.get_parameter_string(glow::VENDOR), gl.get_parameter_string(glow::RENDERER), gl.get_parameter_string(glow::VERSION))
            };
            shared.info = format!("OpenGL {version}, {vendor}, {renderer}");
            true
        }
        Err(e) => {
            shared.failed = Some(e);
            false
        }
    }
}

/// Build the paint callback.
///
/// `shared`: GL state. `frame`: this frame's parameters. Returns the egui callback.
pub fn callback(shared: Arc<Mutex<GlShared>>, frame: Frame) -> eframe::egui_glow::CallbackFn {
    eframe::egui_glow::CallbackFn::new(move |info, painter| {
        let mut sh = shared.lock().unwrap_or_else(|e| e.into_inner());
        if !ensure_init(&mut sh, painter.gl()) {
            return;
        }
        let vp = info.viewport_in_pixels();
        if vp.width_px <= 0 || vp.height_px <= 0 {
            return;
        }
        let ppp = info.pixels_per_point;
        if let Some(st) = sh.state.as_mut() {
            st.paint(&frame, (vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px), ppp);
        }
        sh.frames += 1;
    })
}

/// Free the GL objects in the next callback is not possible once painting stops; so provide a callback
/// that releases them (call once when the panel is dropped or switched to the CPU renderer).
///
/// `shared`: GL state. Returns the egui callback.
pub fn release_callback(shared: Arc<Mutex<GlShared>>) -> eframe::egui_glow::CallbackFn {
    eframe::egui_glow::CallbackFn::new(move |_info, _painter| {
        let mut sh = shared.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mut st) = sh.state.take() {
            st.release();
        }
    })
}
