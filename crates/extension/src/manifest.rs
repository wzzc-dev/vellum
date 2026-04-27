use serde::{Deserialize, Serialize};

/// Represents a parsed `extension.toml` manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// Relative path from the extension directory to the WASM component.
    pub entry: String,

    #[serde(default)]
    pub activation: ActivationConfig,

    #[serde(default)]
    pub capabilities: Capabilities,

    #[serde(default)]
    pub contributes: Contributions,
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivationConfig {
    /// Events that trigger activation, e.g. ["onDocumentOpened:markdown", "onCommand:lint.run"].
    #[serde(default)]
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub document_read: bool,
    #[serde(default)]
    pub document_write: bool,
    #[serde(default)]
    pub decorations: bool,
    #[serde(default)]
    pub panels: bool,
    #[serde(default)]
    pub commands: bool,
    #[serde(default)]
    pub webview: bool,
    #[serde(default)]
    pub webview_scripts: bool,
    #[serde(default)]
    pub webview_devtools: bool,
    #[serde(default)]
    pub workspace_read: bool,
    #[serde(default)]
    pub workspace_write: bool,
    #[serde(default)]
    pub network: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Contributions {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub panels: Vec<PanelContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelContribution {
    pub id: String,
    pub title: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "default_location")]
    pub location: String,
}

fn default_icon() -> String {
    "file-text".into()
}

fn default_location() -> String {
    "right".into()
}

impl ExtensionManifest {
    /// Parse an `extension.toml` file from raw bytes.
    pub fn from_toml_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let s = std::str::from_utf8(bytes)?;
        let manifest: Self = toml::from_str(s)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parse an `extension.toml` file from a string.
    pub fn from_toml_str(s: &str) -> anyhow::Result<Self> {
        let manifest: Self = toml::from_str(s)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.id.is_empty() {
            anyhow::bail!("extension id must not be empty");
        }
        if self.name.is_empty() {
            anyhow::bail!("extension name must not be empty");
        }
        if self.entry.is_empty() {
            anyhow::bail!("extension entry must not be empty");
        }
        // Validate command contributions
        for cmd in &self.contributes.commands {
            if cmd.id.is_empty() {
                anyhow::bail!("command contribution id must not be empty");
            }
            if cmd.title.is_empty() {
                anyhow::bail!("command contribution title must not be empty");
            }
        }
        // Validate panel contributions
        for panel in &self.contributes.panels {
            if panel.id.is_empty() {
                anyhow::bail!("panel contribution id must not be empty");
            }
            if panel.title.is_empty() {
                anyhow::bail!("panel contribution title must not be empty");
            }
        }
        Ok(())
    }

    /// Returns the fully qualified command ID: `{extension_id}.{command_id}`.
    pub fn qualified_command_id(&self, command_id: &str) -> String {
        format!("{}.{}", self.id, command_id)
    }

    /// Returns the fully qualified panel ID: `{extension_id}.{panel_id}`.
    pub fn qualified_panel_id(&self, panel_id: &str) -> String {
        format!("{}.{}", self.id, panel_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_manifest() {
        let toml = r#"
id = "test.extension"
name = "Test Extension"
version = "0.1.0"
entry = "target/wasm32-wasip2/release/test.wasm"
"#;
        let manifest = ExtensionManifest::from_toml_str(toml).unwrap();
        assert_eq!(manifest.id, "test.extension");
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.schema_version, 1);
    }

    #[test]
    fn test_parse_full_manifest() {
        let toml = r#"
id = "vellum.markdown-lint"
name = "Markdown Lint"
version = "0.1.0"
schema_version = 1
description = "Checks Markdown documents for common issues"
author = "Vellum"
entry = "target/wasm32-wasip2/release/markdown_lint.wasm"

[activation]
events = ["onDocumentOpened:markdown", "onDocumentChanged:markdown"]

[capabilities]
document_read = true
document_write = true
decorations = true
panels = true
commands = true

[[contributes.commands]]
id = "markdown-lint.run"
title = "Run Markdown Lint"
key = "cmd-shift-l"

[[contributes.panels]]
id = "markdown-lint.panel"
title = "Lint"
icon = "triangle-alert"
location = "right"
"#;
        let manifest = ExtensionManifest::from_toml_str(toml).unwrap();
        assert_eq!(manifest.id, "vellum.markdown-lint");
        assert_eq!(manifest.capabilities.document_read, true);
        assert_eq!(manifest.contributes.commands.len(), 1);
        assert_eq!(manifest.contributes.panels.len(), 1);
        assert_eq!(
            manifest.qualified_command_id("markdown-lint.run"),
            "vellum.markdown-lint.markdown-lint.run"
        );
    }

    #[test]
    fn test_validate_empty_id() {
        let toml = r#"
id = ""
name = "Test"
version = "0.1.0"
entry = "test.wasm"
"#;
        let result = ExtensionManifest::from_toml_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_entry() {
        let toml = r#"
id = "test"
name = "Test"
version = "0.1.0"
entry = ""
"#;
        let result = ExtensionManifest::from_toml_str(toml);
        assert!(result.is_err());
    }
}
