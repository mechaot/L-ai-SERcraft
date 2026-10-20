//! egui equivalents of the custom WinForms controls (see `ui/README.md`).

pub mod color_slider;
pub mod combo;
pub mod data_grid;
pub mod graph;
pub mod grbl_button;
pub mod group_box;
pub mod image_button;
pub mod input_box;
pub mod label_tb;
pub mod numeric;
pub mod picture_box;
pub mod progress;
pub mod text;
pub mod track_bar;
pub mod usage_class_preview;

pub use color_slider::{ColorSlider, Orientation, SliderColors, SliderSchema};
pub use combo::{enum_combo, flat_combo};
pub use data_grid::{data_grid, GridColumn};
pub use grbl_button::GrblButton;
pub use group_box::grbl_group_box;
pub use image_button::ImageButton;
pub use input_box::{InputBox, InputBoxResult};
pub use label_tb::{LabelTb, OverrideKind};
pub use numeric::{DecimalInputRanged, IntegerInputRanged, NumericUpDown};
pub use picture_box::PictureBox;
pub use progress::{ColorProgressBar, DoubleProgressBar, WaitingProgressBar};
pub use text::{colored_border, GrblTextBox, PlaceholderTextBox, TextInput};
pub use track_bar::{TickStyle, TrackBar};
pub use usage_class_preview::UsageClassPreview;
