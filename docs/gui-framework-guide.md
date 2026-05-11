# Vellum GUI Framework Guide

Vellum is now organized around a MoonBit + Rust/GPUI GUI framework. MoonBit components describe typed UI trees, Rust/GPUI renders them natively, and WIT keeps the boundary type-safe.

The v1 framework path is intentionally small and typed:

- MoonBit components own app state and produce a declarative UI tree.
- Rust loads the component with Wasmtime Component Model.
- GPUI renders the typed tree natively and routes UI events back to MoonBit.
- The existing Markdown editor remains Rust/GPUI code and is embedded as a `native-view` node.

## Current Architecture

```mermaid
flowchart LR
    MoonBit["MoonBit App Component"] --> WIT["vellum:app/app-world"]
    WIT --> Runtime["Rust App Runtime"]
    Runtime --> Renderer["GPUI Tree Renderer"]
    Renderer --> Native["Native MarkdownEditor Widget"]
```

The canonical WIT package lives at `wit/vellum-app.wit`.

The shared MoonBit bindings and tree helpers live in `moonbit/vellum-app-sdk`. Runnable apps and plugins keep their handwritten logic in `src/`, while `gen/world/appWorld/` stays as the generated export layer.

The first implementation uses whole-tree replacement after each event. Stable node ids are still required because the Rust renderer keeps local GPUI state for controls such as text inputs.

## App Lifecycle

MoonBit apps export:

```wit
init: func(ctx: app-context) -> result<view-tree, app-error>
update: func(event: app-event) -> result<view-tree, app-error>
shutdown: func() -> result<_, app-error>
```

`init` returns the first `view-tree`. `update` receives a typed event and returns the next tree.

## UI Tree

`view-tree` is a flattened typed tree:

```wit
record view-node {
    id: string,
    kind: view-kind,
    children: list<u32>,
}

record view-tree {
    root: u32,
    nodes: list<view-node>,
}
```

The renderer currently supports:

- `column`
- `row`
- `text`
- `button`
- `input`
- `tabs`
- `split-view`
- `scroll-view`
- `native-view`

`native-view` supports `kind = "markdown-editor"` in v1.

## Manifest

Apps and plugins both use `vellum.toml`:

```toml
id = "vellum.demo.markdown"
name = "Vellum Markdown Demo"
version = "0.1.0"
kind = "app"
component = "target/wasm32-wasip2/release/vellum_markdown_demo.wasm"

[capabilities]
native_markdown_editor = true
filesystem = true
```

Plugin manifests look the same, but set `kind = "plugin"` and declare `contributes.panels` or `contributes.commands`.

## Run The Demo

Build the MoonBit component:

```bash
cd moonbit/demos/markdown-editor
./build.sh
```

Launch Vellum with the MoonBit shell enabled:

```bash
cd /Volumes/Data/Code/Note/vellum
VELLUM_APP=moonbit/demos/markdown-editor cargo run -p Vellum
```

Without `VELLUM_APP`, Vellum keeps running as the existing Rust Markdown editor.

## Host Services

The runtime currently exposes a small set of typed host calls:

- `log`
- `show-status-message`
- `request-render`
- `editor-command`
- `get-editor-snapshot`
- `plugin-list`
- `plugin-enable`
- `plugin-disable`
- `plugin-reload`

The Markdown demo already uses editor snapshots and plugin listing to build its sidebar, and plugin panels are mounted through `NativeView(kind = "plugin-panel")`.
