use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VellumManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_kind")]
    pub kind: ComponentKind,
    pub component: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub capabilities: AppCapabilities,
    #[serde(default)]
    pub contributes: AppContributions,
}

fn default_schema_version() -> u32 {
    1
}

fn default_kind() -> ComponentKind {
    ComponentKind::App
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentKind {
    App,
    Plugin,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCapabilities {
    #[serde(default)]
    pub native_markdown_editor: bool,
    #[serde(default)]
    pub filesystem: bool,
    #[serde(default)]
    pub timers: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppContributions {
    #[serde(default)]
    pub commands: Vec<AppCommandContribution>,
    #[serde(default)]
    pub panels: Vec<AppPanelContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCommandContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppPanelContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
}

impl VellumManifest {
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
            anyhow::bail!("manifest id must not be empty");
        }
        if self.name.trim().is_empty() {
            anyhow::bail!("manifest name must not be empty");
        }
        if self.version.trim().is_empty() {
            anyhow::bail!("manifest version must not be empty");
        }
        if self.schema_version != 1 {
            anyhow::bail!(
                "unsupported manifest schema_version: {}",
                self.schema_version
            );
        }
        if self.component.trim().is_empty() {
            anyhow::bail!("manifest component must not be empty");
        }
        for command in &self.contributes.commands {
            if command.id.trim().is_empty() {
                anyhow::bail!("command contribution id must not be empty");
            }
            if command.title.trim().is_empty() {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_app_manifest() {
        let manifest = VellumManifest::from_toml_str(
            r#"
id = "vellum.demo"
name = "Vellum Demo"
version = "0.1.0"
component = "target/wasm32-wasip2/release/demo.wasm"
kind = "app"

[capabilities]
native_markdown_editor = true
"#,
        )
        .unwrap();

        assert_eq!(manifest.kind, ComponentKind::App);
        assert!(manifest.capabilities.native_markdown_editor);
    }
}
