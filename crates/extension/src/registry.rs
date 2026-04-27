use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::manifest::ExtensionManifest;

/// The state of an extension in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionState {
    /// Discovered but not yet loaded.
    Discovered,
    /// Successfully loaded and active.
    Active,
    /// Disabled by the user.
    Disabled,
    /// Failed to load with an error message.
    Failed(String),
}

/// An entry in the extension registry.
#[derive(Debug, Clone)]
pub struct ExtensionEntry {
    pub manifest: ExtensionManifest,
    pub directory: PathBuf,
    pub state: ExtensionState,
}

/// The extension registry manages discovery, loading state, and
/// enable/disable tracking for all extensions.
pub struct ExtensionRegistry {
    entries: HashMap<String, ExtensionEntry>,
    /// User-disabled extension IDs (persisted across restarts).
    disabled_ids: Vec<String>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            disabled_ids: Vec::new(),
        }
    }

    /// Set the list of disabled extension IDs (e.g., loaded from persistent storage).
    pub fn set_disabled_ids(&mut self, ids: Vec<String>) {
        self.disabled_ids = ids;
    }

    /// Get the list of disabled extension IDs.
    pub fn disabled_ids(&self) -> &[String] {
        &self.disabled_ids
    }

    /// Discover extensions in a directory.
    /// Each subdirectory containing an `extension.toml` is treated as an extension.
    pub fn discover_in_dir(&mut self, dir: &Path) -> anyhow::Result<Vec<String>> {
        let mut discovered = Vec::new();

        if !dir.exists() || !dir.is_dir() {
            return Ok(discovered);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("extension.toml");
            if !manifest_path.exists() {
                continue;
            }

            match self.discover_extension(&path) {
                Ok(ext_id) => discovered.push(ext_id),
                Err(e) => {
                    eprintln!(
                        "failed to discover extension in {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        Ok(discovered)
    }

    /// Discover a single extension from its directory.
    fn discover_extension(&mut self, dir: &Path) -> anyhow::Result<String> {
        let manifest_path = dir.join("extension.toml");
        let manifest_bytes =
            std::fs::read(&manifest_path).map_err(|e| {
                anyhow::anyhow!("failed to read {}: {}", manifest_path.display(), e)
            })?;

        let manifest = ExtensionManifest::from_toml_bytes(&manifest_bytes)?;

        let ext_id = manifest.id.clone();

        if self.entries.contains_key(&ext_id) {
            anyhow::bail!("duplicate extension id: {}", ext_id);
        }

        let state = if self.disabled_ids.contains(&ext_id) {
            ExtensionState::Disabled
        } else {
            ExtensionState::Discovered
        };

        self.entries.insert(
            ext_id.clone(),
            ExtensionEntry {
                manifest,
                directory: dir.to_path_buf(),
                state,
            },
        );

        Ok(ext_id)
    }

    /// Mark an extension as active (successfully loaded).
    pub fn mark_active(&mut self, extension_id: &str) {
        if let Some(entry) = self.entries.get_mut(extension_id) {
            entry.state = ExtensionState::Active;
        }
    }

    /// Mark an extension as failed with an error message.
    pub fn mark_failed(&mut self, extension_id: &str, error: String) {
        if let Some(entry) = self.entries.get_mut(extension_id) {
            entry.state = ExtensionState::Failed(error);
        }
    }

    /// Disable an extension.
    pub fn disable(&mut self, extension_id: &str) {
        if let Some(entry) = self.entries.get_mut(extension_id) {
            entry.state = ExtensionState::Disabled;
        }
        if !self.disabled_ids.contains(&extension_id.to_string()) {
            self.disabled_ids.push(extension_id.to_string());
        }
    }

    /// Enable an extension.
    pub fn enable(&mut self, extension_id: &str) {
        self.disabled_ids.retain(|id| id != extension_id);
        if let Some(entry) = self.entries.get_mut(extension_id) {
            entry.state = ExtensionState::Discovered;
        }
    }

    /// Get an extension entry by ID.
    pub fn get(&self, extension_id: &str) -> Option<&ExtensionEntry> {
        self.entries.get(extension_id)
    }

    /// Get a mutable extension entry by ID.
    pub fn get_mut(&mut self, extension_id: &str) -> Option<&mut ExtensionEntry> {
        self.entries.get_mut(extension_id)
    }

    /// Get all discovered extensions (not disabled).
    pub fn discovered_extensions(&self) -> Vec<&ExtensionEntry> {
        self.entries
            .values()
            .filter(|e| e.state == ExtensionState::Discovered)
            .collect()
    }

    /// Get all active extensions.
    pub fn active_extensions(&self) -> Vec<&ExtensionEntry> {
        self.entries
            .values()
            .filter(|e| e.state == ExtensionState::Active)
            .collect()
    }

    /// Get all extension entries.
    pub fn all_entries(&self) -> Vec<&ExtensionEntry> {
        self.entries.values().collect()
    }

    /// Get all extension manifests.
    pub fn all_manifests(&self) -> Vec<&ExtensionManifest> {
        self.entries.values().map(|e| &e.manifest).collect()
    }

    /// Check if an extension is disabled.
    pub fn is_disabled(&self, extension_id: &str) -> bool {
        self.disabled_ids.contains(&extension_id.to_string())
    }

    /// Remove an extension from the registry.
    pub fn remove(&mut self, extension_id: &str) -> Option<ExtensionEntry> {
        self.disabled_ids.retain(|id| id != extension_id);
        self.entries.remove(extension_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_discover_extension() {
        let dir = std::env::temp_dir().join("vellum_test_registry");
        let ext_dir = dir.join("test-extension");
        std::fs::create_dir_all(&ext_dir).unwrap();

        let manifest_content = r#"
id = "test.extension"
name = "Test Extension"
version = "0.1.0"
entry = "test.wasm"
"#;
        std::fs::write(ext_dir.join("extension.toml"), manifest_content).unwrap();

        let mut registry = ExtensionRegistry::new();
        let discovered = registry.discover_in_dir(&dir).unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0], "test.extension");

        let entry = registry.get("test.extension").unwrap();
        assert_eq!(entry.manifest.name, "Test Extension");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = ExtensionRegistry::new();

        // Manually insert an entry
        let manifest = crate::manifest::ExtensionManifest::from_toml_str(
            r#"
id = "test.ext"
name = "Test"
version = "0.1.0"
entry = "test.wasm"
"#,
        )
        .unwrap();

        registry.entries.insert(
            "test.ext".to_string(),
            ExtensionEntry {
                manifest,
                directory: PathBuf::from("/tmp/test"),
                state: ExtensionState::Active,
            },
        );

        registry.disable("test.ext");
        assert!(registry.is_disabled("test.ext"));

        registry.enable("test.ext");
        assert!(!registry.is_disabled("test.ext"));
    }
}
