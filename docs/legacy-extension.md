# Legacy Extension Compatibility

The old `vellum:extension/extension-world` API is retained as a compatibility layer while the project moves to the typed app/plugin protocol.

Use this path only for existing extensions and regression tests:

- Rust host compatibility crate: `crates/vellum-extension-compat`
- Legacy implementation crate: `crates/extension`
- Legacy Rust SDK package: `crates/extension-sdk`
- Canonical compat WIT: `wit/vellum-extension.wit`
- MoonBit legacy examples: `moonbit/legacy-extensions`
- Older standalone examples: `examples/legacy-extensions`

New work should use `vellum.toml`, `kind = "app"` or `kind = "plugin"`, and `wit/vellum-app.wit`.
