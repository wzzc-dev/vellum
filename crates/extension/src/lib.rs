pub mod contributions;
pub mod event;
pub mod host;
pub mod manifest;
pub mod permissions;
pub mod registry;
pub mod ui;

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

pub use host::ExtensionHost;

pub use contributions::{
    Decoration, DecorationKind, PendingEdit, RegisteredCommand, RegisteredPanel, UnderlineStyle,
};
pub use host::ExtensionOutputs;

#[cfg(feature = "gpui-adapter")]
pub mod gui_adapter {
    pub use gpui_adapter::*;
}

#[cfg(test)]
mod tests;
