//! Job preview panels (WP-U2a): ports of `GrblPanel3D.cs` (OpenGL), `GrblPanel.cs` (GDI fallback),
//! `IGrblPanel.cs` and `Obj3D/*`. See `README.md` in this folder.
//!
//! The panels do not know the core. The host fills a [`PreviewInput`] every frame and handles the
//! [`PreviewEvent`]s returned by [`PreviewPanel::show`].

pub mod camera;
pub mod demo;
pub mod gl;
pub mod overlay;
pub mod panel_cpu;
pub mod panel_gl;
pub mod scene;

use std::sync::{Arc, RwLock};

use eframe::egui::{self, Ui};
use lg_base::settings::{self, GraphicMode};
use lg_gcode::{GPoint, GrblFile};

pub use panel_cpu::CpuPanel;
pub use panel_gl::GlPanel;

/// Machine state, as far as the preview cares (C# `GrblCore.MacStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MachineStatus {
    #[default]
    Disconnected,
    Connecting,
    Idle,
    Run,
    Hold,
    Door,
    Home,
    Alarm,
    Check,
    Jog,
    Queue,
    Cooling,
    AutoHold,
}

/// View options (C# `RetainedSetting`s of `GrblCore`, same keys).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewOptions {
    pub show_laser_off_movements: bool,
    pub show_executed_commands: bool,
    pub show_performance_diagnostic: bool,
    pub show_bounding_box: bool,
    pub cross_cursor: bool,
    pub preview_line_size: f32,
    pub auto_size_on_drawing: bool,
}

impl Default for PreviewOptions {
    /// C# defaults.
    fn default() -> Self {
        Self {
            show_laser_off_movements: true,
            show_executed_commands: true,
            show_performance_diagnostic: false,
            show_bounding_box: true,
            cross_cursor: true,
            preview_line_size: 1.0,
            auto_size_on_drawing: true,
        }
    }
}

impl PreviewOptions {
    /// Read the retained settings.
    ///
    /// Returns options from the settings store (keys `ShowLaserOffMovements`, `ShowExecutedCommands`,
    /// `ShowPerformanceDiagnostic`, `ShowBoundingBox`, `CrossCursor`, `PreviewLineSize`, `AutoSizeOnDrawing`).
    pub fn from_settings() -> Self {
        let d = Self::default();
        Self {
            show_laser_off_movements: settings::get("ShowLaserOffMovements", d.show_laser_off_movements),
            show_executed_commands: settings::get("ShowExecutedCommands", d.show_executed_commands),
            show_performance_diagnostic: settings::get("ShowPerformanceDiagnostic", d.show_performance_diagnostic),
            show_bounding_box: settings::get("ShowBoundingBox", d.show_bounding_box),
            cross_cursor: settings::get("CrossCursor", d.cross_cursor),
            preview_line_size: settings::get("PreviewLineSize", d.preview_line_size),
            auto_size_on_drawing: settings::get("AutoSizeOnDrawing", d.auto_size_on_drawing),
        }
    }
}

/// Shortcut texts for the usage tips, already formatted like `GetShortcut` (C#), empty = none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hotkeys {
    pub auto_size: String,
    pub zoom_in: String,
    pub zoom_out: String,
}

/// Everything the panels read each frame.
#[derive(Clone)]
pub struct PreviewInput {
    /// Loaded program (`Core.LoadedFile`).
    pub file: Option<Arc<RwLock<GrblFile>>>,
    /// Bump on every completed file load (C# `OnFileLoaded`): rebuilds the preview and auto sizes.
    pub file_revision: u64,
    /// `Core.MachinePosition`.
    pub machine_position: GPoint,
    /// `Core.WorkPosition`.
    pub work_position: GPoint,
    /// `Core.WorkingOffset`.
    pub working_offset: GPoint,
    /// `Core.CurrentF` / `CurrentS`.
    pub current_f: f32,
    pub current_s: f32,
    pub machine_status: MachineStatus,
    pub connected: bool,
    /// `Core.JogEnabled`: double click jog is dropped when false.
    pub jog_enabled: bool,
    /// `Core.InProgram`.
    pub in_program: bool,
    pub options: PreviewOptions,
    /// Setting "Click N Jog" (default true).
    pub click_n_jog: bool,
    /// Setting "Enale Z Jog Control" (sic, C# key): always show Z.
    pub enable_z_jog: bool,
    /// Machine table `(width, height)` in mm: drawn as dashed rectangle from the origin when set
    /// (use [`PreviewInput::table_size_from_config`]).
    pub machine_area: Option<(f32, f32)>,
    pub hotkeys: Hotkeys,
}

impl Default for PreviewInput {
    /// Empty input with C# defaults.
    fn default() -> Self {
        Self {
            file: None,
            file_revision: 0,
            machine_position: GPoint::ZERO,
            work_position: GPoint::ZERO,
            working_offset: GPoint::ZERO,
            current_f: 0.0,
            current_s: 0.0,
            machine_status: MachineStatus::Disconnected,
            connected: false,
            jog_enabled: false,
            in_program: false,
            options: PreviewOptions::default(),
            click_n_jog: true,
            enable_z_jog: false,
            machine_area: None,
            hotkeys: Hotkeys::default(),
        }
    }
}

impl PreviewInput {
    /// Table size of the stored Grbl configuration.
    ///
    /// Returns `(width, height)` in mm.
    pub fn table_size_from_config() -> (f32, f32) {
        let c = lg_gcode::config::configuration::configuration();
        (lg_dotnet::decimal_to_f32(c.table_width()), lg_dotnet::decimal_to_f32(c.table_height()))
    }

    /// Settings-backed fields (options, click-n-jog, Z jog).
    ///
    /// Returns an input with those read from the settings store.
    pub fn from_settings() -> Self {
        Self {
            options: PreviewOptions::from_settings(),
            click_n_jog: settings::get("Click N Jog", true),
            enable_z_jog: settings::get("Enale Z Jog Control", false),
            ..Default::default()
        }
    }
}

/// What the panel asks the host to do.
#[derive(Debug, Clone, PartialEq)]
pub enum PreviewEvent {
    /// Double click: `Core.JogToPosition(target, fast)`; `fast` = right button.
    JogToPosition { x: f32, y: f32, fast: bool },
    /// GL could not start; the panel switched to the CPU renderer (`current_graphic_mode` = Gdi).
    FellBackToCpu { reason: String },
    /// The view (GL camera) changed: world window `(left, right, bottom, top)`.
    ViewChanged { left: f64, right: f64, bottom: f64, top: f64 },
}

/// Renderer choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// By `requested_graphic_mode()`: `Gdi` = CPU, anything else GL (CPU when GL fails).
    Auto,
    /// Force GL (gallery).
    Gl,
    /// Force CPU (gallery).
    Cpu,
}

/// GL or CPU preview, chosen like C# `Settings.RequestedGraphicMode`.
pub struct PreviewPanel {
    gl: GlPanel,
    cpu: CpuPanel,
    backend: Backend,
    use_cpu: bool,
    announced: bool,
}

impl Default for PreviewPanel {
    /// Same as `new(Backend::Auto)`.
    fn default() -> Self {
        Self::new(Backend::Auto)
    }
}

impl PreviewPanel {
    /// New panel.
    ///
    /// `backend`: renderer choice.
    pub fn new(backend: Backend) -> Self {
        let use_cpu = match backend {
            Backend::Cpu => true,
            Backend::Gl => false,
            Backend::Auto => settings::requested_graphic_mode() == GraphicMode::Gdi,
        };
        Self { gl: GlPanel::new(), cpu: CpuPanel::new(), backend, use_cpu, announced: false }
    }

    /// True when the CPU renderer is active.
    pub fn is_cpu(&self) -> bool {
        self.use_cpu
    }

    /// Draw the preview into the remaining space of `ui` (C# `TimerUpdate` + paint in one).
    ///
    /// `ui`: target ui. `input`: current state. Returns events for the host (jog, fallback, view change).
    pub fn show(&mut self, ui: &mut Ui, input: &PreviewInput) -> Vec<PreviewEvent> {
        let mut events = Vec::new();
        // GL failed in the paint callback: switch to CPU for good
        if !self.use_cpu {
            if let Some(reason) = self.gl.failure() {
                log::warn!("OpenGL unavailable ({reason}), using CPU preview");
                self.use_cpu = true;
                self.announced = false;
                events.push(PreviewEvent::FellBackToCpu { reason });
            }
        }
        if self.use_cpu {
            if !self.announced {
                self.announced = true;
                if self.backend == Backend::Auto {
                    settings::set_current_graphic_mode(GraphicMode::Gdi);
                }
                log::info!("OpenGL {:?}->{:?} LaserGRBL, Legacy Preview", settings::requested_graphic_mode(), GraphicMode::Gdi);
            }
            self.cpu.show(ui, input, &mut events);
        } else {
            self.gl.show(ui, input, &mut events);
            if !self.announced && self.gl.is_ready() {
                self.announced = true;
                if self.backend == Backend::Auto {
                    let cur = if settings::requested_graphic_mode() == GraphicMode::Dib { GraphicMode::Dib } else { GraphicMode::Fbo };
                    settings::set_current_graphic_mode(cur);
                }
                log::info!("OpenGL {:?}->{:?} {}", settings::requested_graphic_mode(), settings::current_graphic_mode(), self.gl.info());
            }
        }
        events
    }

    /// C# `OnColorChange`: repaint with new scheme colors (colors are re-read every frame anyway).
    pub fn on_color_change(&mut self) {
        self.cpu.invalidate();
    }

    /// C# `Core_OnAutoSizeDrawing` (GL only).
    pub fn auto_size_drawing(&mut self) {
        self.gl.request_auto_size();
    }

    /// C# `ZoomIn` (GL only).
    pub fn zoom_in(&mut self) {
        self.gl.zoom(1.0 / 1.1);
    }

    /// C# `ZoomOut` (GL only).
    pub fn zoom_out(&mut self) {
        self.gl.zoom(1.1);
    }

    /// C# `OnProgramEnded`: clear all command results (preview colors reset).
    ///
    /// `file`: loaded program.
    pub fn on_program_ended(file: &Arc<RwLock<GrblFile>>) {
        let mut f = file.write().unwrap_or_else(|e| e.into_inner());
        for c in f.commands_mut().iter_mut() {
            c.clear_result();
        }
    }
}

/// Convert an egui color to RGB bytes.
///
/// `c`: color. Returns `[r, g, b]`.
pub(crate) fn rgb(c: egui::Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}
