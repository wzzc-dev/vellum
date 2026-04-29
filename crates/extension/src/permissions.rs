use crate::manifest::ExtensionManifest;

#[derive(Debug, Clone, Copy)]
pub enum Capability {
    DocumentRead,
    DocumentWrite,
    Decorations,
    Panels,
    Commands,
    Webview,
}

impl Capability {
    pub fn name(self) -> &'static str {
        match self {
            Self::DocumentRead => "document_read",
            Self::DocumentWrite => "document_write",
            Self::Decorations => "decorations",
            Self::Panels => "panels",
            Self::Commands => "commands",
            Self::Webview => "webview",
        }
    }
}

pub fn has_capability(manifest: &ExtensionManifest, capability: Capability) -> bool {
    match capability {
        Capability::DocumentRead => manifest.capabilities.document_read,
        Capability::DocumentWrite => manifest.capabilities.document_write,
        Capability::Decorations => manifest.capabilities.decorations,
        Capability::Panels => manifest.capabilities.panels,
        Capability::Commands => manifest.capabilities.commands,
        Capability::Webview => manifest.capabilities.webview,
    }
}

pub fn check_capability(
    manifest: &ExtensionManifest,
    capability: Capability,
) -> anyhow::Result<()> {
    if has_capability(manifest, capability) {
        Ok(())
    } else {
        anyhow::bail!(
            "extension '{}' does not have '{}' capability",
            manifest.id,
            capability.name()
        )
    }
}
