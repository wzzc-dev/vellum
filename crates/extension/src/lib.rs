pub mod contributions;
pub mod event;
pub mod host;
pub mod manifest;
pub mod permissions;
pub mod registry;
pub mod ui;

// Re-export for backward compatibility
pub use host::ExtensionHost;

pub use contributions::{
    Decoration, DecorationKind, PendingEdit, RegisteredCommand, RegisteredPanel, UnderlineStyle,
};
pub use host::ExtensionOutputs;

#[cfg(test)]
mod tests;
