mod commands;
pub(crate) mod component_ui;
pub(crate) mod layout;
mod session;
mod view;

pub use commands::bind_keys;
pub use view::{EditorEvent, MarkdownEditor};

pub(crate) const EDITOR_CONTEXT: &str = "MarkdownEditor";
pub(crate) const INPUT_CONTEXT: &str = "MarkdownEditorInput";
pub(crate) const MAX_EDITOR_WIDTH: f32 = 780.;
pub(crate) const BODY_FONT_SIZE: f32 = 17.;
pub(crate) const BODY_LINE_HEIGHT: f32 = 28.;
pub(crate) const CODE_FONT_SIZE: f32 = 15.;
pub(crate) const CODE_LINE_HEIGHT: f32 = 24.;
