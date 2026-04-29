use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::manifest::ExtensionManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionState {
    Discovered,
    Active,
    Disabled,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct ExtensionEntry {
    pub manifest: ExtensionManifest,
    pub directory: PathBuf,
    pub state: ExtensionState,
    pub is_dev: bool,
}

#[derive(Default)]
pub struct ExtensionRegistry {
    entries: HashMap<String, ExtensionEntry>,
    disabled_ids: Vec<String>,
    dev_extension_dirs: Vec<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DevExtensionsFile {
    #[serde(default)]
    extensions: Vec<PathBuf>,
    #[serde(default)]
    disabled: Vec<String>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_disabled_ids(&mut self, ids: Vec<String>) {
        self.disabled_ids = ids;
    }

    pub fn disabled_ids(&self) -> &[String] {
        &self.disabled_ids
    }

    pub fn dev_extension_dirs(&self) -> &[PathBuf] {
        &self.dev_extension_dirs
    }

    pub fn load_dev_extensions_file(&mut self, path: &Path) -> anyhow::Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let text = std::fs::read_to_string(path)?;
        let file: DevExtensionsFile = toml::from_str(&text)?;
        self.dev_extension_dirs = file.extensions;
        self.disabled_ids = file.disabled;
        Ok(())
    }

    pub fn save_dev_extensions_file(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = DevExtensionsFile {
            extensions: self.dev_extension_dirs.clone(),
            disabled: self.disabled_ids.clone(),
        };
        std::fs::write(path, toml::to_string_pretty(&file)?)?;
        Ok(())
    }

    pub fn add_dev_extension_dir(&mut self, dir: PathBuf) {
        if !self.dev_extension_dirs.iter().any(|path| path == &dir) {
            self.dev_extension_dirs.push(dir);
        }
    }

    pub fn discover_dev_extensions(&mut self) -> anyhow::Result<Vec<String>> {
        let mut discovered = Vec::new();
        for dir in self.dev_extension_dirs.clone() {
            discovered.push(self.discover_extension_dir(&dir, true)?);
        }
        Ok(discovered)
    }

    pub fn discover_in_dir(&mut self, dir: &Path) -> anyhow::Result<Vec<String>> {
        let mut discovered = Vec::new();
        if !dir.exists() || !dir.is_dir() {
            return Ok(discovered);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("extension.toml").exists() {
                match self.discover_extension_dir(&path, false) {
                    Ok(id) => discovered.push(id),
                    Err(err) => {
                        eprintln!("failed to discover extension in {}: {err}", path.display())
                    }
                }
            }
        }
        Ok(discovered)
    }

    pub fn discover_extension_dir(&mut self, dir: &Path, is_dev: bool) -> anyhow::Result<String> {
        let manifest_path = dir.join("extension.toml");
        let manifest = ExtensionManifest::from_toml_bytes(&std::fs::read(&manifest_path)?)?;
        let ext_id = manifest.id.clone();

        if self.entries.contains_key(&ext_id) {
            anyhow::bail!("duplicate extension id: {}", ext_id);
        }

        let state = if self.disabled_ids.iter().any(|id| id == &ext_id) {
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
                is_dev,
            },
        );
        Ok(ext_id)
    }

    pub fn mark_active(&mut self, extension_id: &str) {
        if let Some(entry) = self.entries.get_mut(extension_id) {
            entry.state = ExtensionState::Active;
        }
    }

    pub fn mark_failed(&mut self, extension_id: &str, error: String) {
        if let Some(entry) = self.entries.get_mut(extension_id) {
            entry.state = ExtensionState::Failed(error);
        }
    }

    pub fn disable(&mut self, extension_id: &str) {
        if let Some(entry) = self.entries.get_mut(extension_id) {
            entry.state = ExtensionState::Disabled;
        }
        if !self.disabled_ids.iter().any(|id| id == extension_id) {
            self.disabled_ids.push(extension_id.to_string());
        }
    }

    pub fn enable(&mut self, extension_id: &str) {
        self.disabled_ids.retain(|id| id != extension_id);
        if let Some(entry) = self.entries.get_mut(extension_id) {
            if !matches!(entry.state, ExtensionState::Active) {
                entry.state = ExtensionState::Discovered;
            }
        }
    }

    pub fn is_disabled(&self, extension_id: &str) -> bool {
        self.disabled_ids.iter().any(|id| id == extension_id)
    }

    pub fn get(&self, extension_id: &str) -> Option<&ExtensionEntry> {
        self.entries.get(extension_id)
    }

    pub fn get_mut(&mut self, extension_id: &str) -> Option<&mut ExtensionEntry> {
        self.entries.get_mut(extension_id)
    }

    pub fn all_entries(&self) -> Vec<&ExtensionEntry> {
        self.entries.values().collect()
    }

    pub fn all_manifests(&self) -> Vec<&ExtensionManifest> {
        self.entries.values().map(|entry| &entry.manifest).collect()
    }

    pub fn active_extensions(&self) -> Vec<&ExtensionEntry> {
        self.entries
            .values()
            .filter(|entry| entry.state == ExtensionState::Active)
            .collect()
    }

    pub fn available_extensions(&self) -> Vec<&ExtensionEntry> {
        self.entries
            .values()
            .filter(|entry| !matches!(entry.state, ExtensionState::Disabled))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_registry_round_trips() {
        let dir = std::env::temp_dir().join("vellum_dev_registry_test");
        let file = dir.join("dev-extensions.toml");
        let ext = dir.join("my-extension");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&ext).unwrap();

        let mut registry = ExtensionRegistry::new();
        registry.add_dev_extension_dir(ext.clone());
        registry.disable("test.extension");
        registry.save_dev_extensions_file(&file).unwrap();

        let mut loaded = ExtensionRegistry::new();
        loaded.load_dev_extensions_file(&file).unwrap();
        assert_eq!(loaded.dev_extension_dirs(), &[ext]);
        assert!(loaded.is_disabled("test.extension"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
