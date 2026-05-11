# Vellum App SDK

This package contains the shared MoonBit DSL for `vellum:app/app-world`.

What lives here:

- The canonical WIT lives at `../../wit/vellum-app.wit`.
- Canonical generated bindings live under `src/interface/vellum/app/{types,host}`.
- `src/lib.mbt` provides reusable helpers for building typed `ViewTree` values.
- `../demos/markdown-editor` imports this package instead of keeping its own local DSL.

The current public API covers:

- tree construction helpers: `add_node`, `tree`
- layout helpers: `column`, `row`, `split_view`, `scroll_view`
- content helpers: `text`, `styled_text`, `muted_text`, `button`, `input`
- native mounts: `markdown_editor`, `plugin_panel`
- shared types/host imports for app and plugin components
