use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub repository: String,
    #[serde(default)]
    pub wasm: WasmConfig,
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
pub struct WasmConfig {
    /// Relative path from the extension directory to the WASM component.
    #[serde(default)]
    pub component: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivationConfig {
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
    pub fn from_toml_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let s = std::str::from_utf8(bytes)?;
        Self::from_toml_str(s)
    }

    pub fn from_toml_str(s: &str) -> anyhow::Result<Self> {
        let manifest: Self = toml::from_str(s)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.id.trim().is_empty() {
            anyhow::bail!("extension id must not be empty");
        }
        if self.name.trim().is_empty() {
            anyhow::bail!("extension name must not be empty");
        }
        if self.version.trim().is_empty() {
            anyhow::bail!("extension version must not be empty");
        }
        if self.schema_version != 1 {
            anyhow::bail!(
                "unsupported extension schema_version: {}",
                self.schema_version
            );
        }
        if self.wasm.component.trim().is_empty() {
            anyhow::bail!("extension wasm.component must not be empty");
        }

        for cmd in &self.contributes.commands {
            if cmd.id.trim().is_empty() {
                anyhow::bail!("command contribution id must not be empty");
            }
            if cmd.title.trim().is_empty() {
                anyhow::bail!("command contribution title must not be empty");
            }
        }

        for panel in &self.contributes.panels {
            if panel.id.trim().is_empty() {
                anyhow::bail!("panel contribution id must not be empty");
            }
            if panel.title.trim().is_empty() {
                anyhow::bail!("panel contribution title must not be empty");
            }
        }

        Ok(())
    }

    pub fn qualified_command_id(&self, command_id: &str) -> String {
        format!("{}.{}", self.id, command_id)
    }

    pub fn qualified_panel_id(&self, panel_id: &str) -> String {
        format!("{}.{}", self.id, panel_id)
    }

    pub fn activates_on(&self, event_type: &str) -> bool {
        self.activation
            .events
            .iter()
            .any(|event| event == event_type)
    }

    pub fn author_line(&self) -> String {
        if self.authors.is_empty() {
            String::new()
        } else {
            self.authors.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_zed_style_manifest() {
        let toml = r#"
id = "vellum.markdown-lint"
name = "Markdown Lint"
version = "0.1.0"
schema_version = 1
authors = ["Vellum"]
description = "Checks Markdown documents"
repository = "https://example.com"

[wasm]
component = "target/wasm32-wasip2/release/vellum_markdown_lint.wasm"

[activation]
events = ["document.opened", "document.changed"]

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
id = "markdown-lint"
title = "Lint"
icon = "triangle-alert"
"#;
        let manifest = ExtensionManifest::from_toml_str(toml).unwrap();
        assert_eq!(manifest.id, "vellum.markdown-lint");
        assert_eq!(manifest.authors, vec!["Vellum"]);
        assert_eq!(
            manifest.wasm.component,
            "target/wasm32-wasip2/release/vellum_markdown_lint.wasm"
        );
        assert!(manifest.activates_on("document.changed"));
        assert_eq!(
            manifest.qualified_panel_id("markdown-lint"),
            "vellum.markdown-lint.markdown-lint"
        );
    }

    #[test]
    fn validate_requires_component() {
        let toml = r#"
id = "test.extension"
name = "Test Extension"
version = "0.1.0"
"#;
        assert!(ExtensionManifest::from_toml_str(toml).is_err());
    }
}
