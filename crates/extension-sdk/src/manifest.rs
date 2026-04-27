use serde::{Deserialize, Serialize};

/// Plugin manifest metadata.
/// This is the SDK-side representation; the host uses `ExtensionManifest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}
