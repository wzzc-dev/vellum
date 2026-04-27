pub mod contributions;
pub mod event;
pub mod host;
pub mod manifest;
pub mod permissions;
pub mod registry;
pub mod ui;

// Re-export for backward compatibility
pub use host::ExtensionHost;

/// Backward-compatible alias for ExtensionHost.
pub type PluginManager = ExtensionHost;

/// Backward-compatible alias for ExtensionManifest.
pub type PluginManifest = manifest::ExtensionManifest;

#[cfg(test)]
mod tests;
