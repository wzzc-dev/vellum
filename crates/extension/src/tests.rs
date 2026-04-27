#[cfg(test)]
mod tests {
    use crate::manifest::ExtensionManifest;

    #[test]
    fn test_manifest_parse_minimal() {
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
    }

    #[test]
    fn test_manifest_parse_full() {
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
    }

    #[test]
    fn test_extension_host_new() {
        let host = crate::host::ExtensionHost::new();
        assert!(host.is_ok());
    }

    #[test]
    fn test_extension_host_update_document() {
        let mut host = crate::host::ExtensionHost::new().unwrap();
        host.update_document("hello world".into(), Some("/test.md".into()));
        // No loaded extensions, so this is a no-op but shouldn't panic
    }

    #[test]
    fn test_registry_new() {
        let registry = crate::registry::ExtensionRegistry::new();
        assert!(registry.all_entries().is_empty());
    }
}
