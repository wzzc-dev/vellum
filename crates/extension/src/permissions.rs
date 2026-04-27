use crate::manifest::ExtensionManifest;

/// Checks whether an extension is allowed to perform a given action
/// based on its declared capabilities.
pub struct PermissionChecker;

impl PermissionChecker {
    pub fn check_document_read(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.document_read {
            anyhow::bail!(
                "extension '{}' does not have 'document_read' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_document_write(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.document_write {
            anyhow::bail!(
                "extension '{}' does not have 'document_write' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_decorations(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.decorations {
            anyhow::bail!(
                "extension '{}' does not have 'decorations' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_panels(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.panels {
            anyhow::bail!(
                "extension '{}' does not have 'panels' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_commands(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.commands {
            anyhow::bail!(
                "extension '{}' does not have 'commands' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_webview(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.webview {
            anyhow::bail!(
                "extension '{}' does not have 'webview' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_webview_scripts(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.webview_scripts {
            anyhow::bail!(
                "extension '{}' does not have 'webview_scripts' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_webview_devtools(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.webview_devtools {
            anyhow::bail!(
                "extension '{}' does not have 'webview_devtools' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_workspace_read(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.workspace_read {
            anyhow::bail!(
                "extension '{}' does not have 'workspace_read' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_workspace_write(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.workspace_write {
            anyhow::bail!(
                "extension '{}' does not have 'workspace_write' capability",
                manifest.id
            );
        }
        Ok(())
    }

    pub fn check_network(manifest: &ExtensionManifest) -> anyhow::Result<()> {
        if !manifest.capabilities.network {
            anyhow::bail!(
                "extension '{}' does not have 'network' capability",
                manifest.id
            );
        }
        Ok(())
    }
}
