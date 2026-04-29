pub mod decoration;
pub mod event;
pub mod host;
pub mod manifest;
pub mod plugin;
pub mod ui;

pub mod bindings {
    wit_bindgen::generate!({
        path: "../extension/wit",
        world: "extension-world",
        pub_export_macro: true,
        default_bindings_module: "$crate::bindings",
    });
}

pub use manifest::ExtensionManifest;
pub use plugin::{Extension, ExtensionContext};

#[cfg(test)]
mod tests;
