#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::manifest::ExtensionManifest;
    use crate::permissions::{Capability, check_capability};

    #[test]
    fn test_manifest_parse_minimal() {
        let toml = r#"
id = "test.extension"
name = "Test Extension"
version = "0.1.0"

[wasm]
component = "target/wasm32-wasip2/release/test.wasm"
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
authors = ["Vellum"]
description = "Checks Markdown documents for common issues"

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

    #[test]
    fn test_registry_rejects_duplicate_ids() {
        let dir = unique_temp_dir("vellum_duplicate_extension_test");
        let first = dir.join("first");
        let second = dir.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        write_manifest(&first, "test.duplicate", true, true);
        write_manifest(&second, "test.duplicate", true, true);

        let mut registry = crate::registry::ExtensionRegistry::new();
        registry.discover_extension_dir(&first, true).unwrap();
        let err = registry.discover_extension_dir(&second, true).unwrap_err();
        assert!(err.to_string().contains("duplicate extension id"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_static_contributions_require_capabilities() {
        let dir = unique_temp_dir("vellum_capability_extension_test");
        let ext = dir.join("extension");
        fs::create_dir_all(&ext).unwrap();
        write_manifest(&ext, "test.capabilities", false, false);

        let mut host = crate::host::ExtensionHost::new().unwrap();
        host.discover_in_dir(&dir).unwrap();
        assert!(host.commands().is_empty());
        assert!(host.sidebar_panels().is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_permissions_reject_missing_capability() {
        let toml = r#"
id = "test.permissions"
name = "Permissions"
version = "0.1.0"

[wasm]
component = "target/wasm32-wasip2/release/test.wasm"

[capabilities]
document_read = true
"#;
        let manifest = ExtensionManifest::from_toml_str(toml).unwrap();
        assert!(check_capability(&manifest, Capability::DocumentRead).is_ok());
        assert!(check_capability(&manifest, Capability::DocumentWrite).is_err());
    }

    #[test]
    #[ignore = "requires `cargo build -p vellum-markdown-lint --release --target wasm32-wasip2` first"]
    fn test_markdown_lint_component_integration() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .unwrap()
            .to_path_buf();
        let extension_root = workspace.join("examples-extensions");
        let component = workspace.join("target/wasm32-wasip2/release/vellum_markdown_lint.wasm");
        assert!(
            component.exists(),
            "fixture component missing at {}",
            component.display()
        );

        let mut host = crate::host::ExtensionHost::new().unwrap();
        host.discover_in_dir(&extension_root).unwrap();
        assert!(
            host.commands()
                .iter()
                .any(|command| command.qualified_id == "vellum.markdown-lint.markdown-lint.run")
        );
        assert!(
            host.sidebar_panels()
                .iter()
                .any(|panel| panel.qualified_id == "vellum.markdown-lint.markdown-lint")
        );

        host.dispatch_event(
            "document.changed",
            "doc-1",
            "Title\n# Next\n",
            Some("/tmp/test.md"),
        );
        let outputs = host.take_outputs();

        assert!(
            outputs
                .status_message
                .as_deref()
                .unwrap_or_default()
                .starts_with("Lint:")
        );
        assert!(
            outputs
                .decorations
                .as_ref()
                .map(|decorations| !decorations.is_empty())
                .unwrap_or(false)
        );
        assert!(
            outputs
                .panel_uis
                .contains_key("vellum.markdown-lint.markdown-lint")
        );
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nanos))
    }

    fn write_manifest(dir: &std::path::Path, id: &str, commands: bool, panels: bool) {
        let toml = format!(
            r#"
id = "{id}"
name = "Test"
version = "0.1.0"

[wasm]
component = "target/wasm32-wasip2/release/test.wasm"

[capabilities]
commands = {commands}
panels = {panels}

[[contributes.commands]]
id = "run"
title = "Run"

[[contributes.panels]]
id = "panel"
title = "Panel"
"#
        );
        fs::write(dir.join("extension.toml"), toml).unwrap();
    }
}
