mod commands;
mod file_ops;
mod input_bridge;
mod layout;
mod surface;
mod view;

pub use commands::bind_keys;
pub use view::{EditorEvent, MarkdownEditor};

pub(crate) const EDITOR_CONTEXT: &str = "MarkdownEditor";
pub(crate) const MAX_EDITOR_WIDTH: f32 = 780.;
pub(crate) const BODY_FONT_SIZE: f32 = 17.;
pub(crate) const BODY_LINE_HEIGHT: f32 = 28.;
pub(crate) const CODE_FONT_SIZE: f32 = 15.;
pub(crate) const CODE_LINE_HEIGHT: f32 = 24.;

#[cfg(target_os = "macos")]
pub(crate) const MONOSPACE_FONT_FAMILY: &str = "Menlo";
#[cfg(not(target_os = "macos"))]
pub(crate) const MONOSPACE_FONT_FAMILY: &str = "Consolas";
