mod commands;
mod file_ops;
mod input_bridge;
mod layout;
mod math_completion_panel;
mod slash_command;
mod surface;
pub mod theme;
mod typography;
mod view;

pub use commands::bind_keys;
pub use typography::{
    DEFAULT_BODY_FONT_SIZE, MAX_BODY_FONT_SIZE, MIN_BODY_FONT_SIZE, normalize_body_font_size,
    set_body_font_size,
};
pub use view::{EditorEvent, MarkdownEditor};

pub(crate) const EDITOR_CONTEXT: &str = "MarkdownEditor";
pub(crate) const MAX_EDITOR_WIDTH: f32 = 780.;

#[cfg(target_os = "macos")]
pub(crate) const MONOSPACE_FONT_FAMILY: &str = "Menlo";
#[cfg(not(target_os = "macos"))]
pub(crate) const MONOSPACE_FONT_FAMILY: &str = "Consolas";
