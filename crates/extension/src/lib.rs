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
pub use vellum_runtime::{LoadedAppComponent, VellumAppRuntime};
pub mod app_manifest {
    pub use vellum_runtime::manifest::*;
}
pub mod app_runtime {
    pub use vellum_runtime::runtime::*;
}
pub mod app_ui {
    pub use vellum_runtime::ui::*;
}

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
