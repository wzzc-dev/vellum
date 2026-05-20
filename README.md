# Vellum

[简体中文](./README.zh-CN.md)

Vellum is a pure desktop Markdown WYSIWYG editor built with Rust and `gpui`.
Its direction is a focused Typora-style writing experience: local Markdown
files, live editing, and a quiet desktop UI without a plugin system.

![Vellum screenshot](./docs/vellum-screenshot.png)

## Current Features

- WYSIWYG Markdown editing
- Open a Markdown file or folder
- Multi-tab editing in a single window
- Switch between live preview and source mode
- Headings, paragraphs, blockquotes, lists, task lists, code fences, tables,
  horizontal rules, links, images, and math blocks
- Syntax highlighting for Markdown and common code fence languages
- Outline sidebar and workspace file tree
- Find and replace
- Command palette
- Auto save
- Watch for external file changes, deletions, and renames
- Conflict detection and handling
- Restore the last opened file on startup

## Tech Stack

- [Rust](https://www.rust-lang.org/)
- [gpui](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [gpui-components](https://github.com/longbridge/gpui-component)
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/)
- [ropey](https://github.com/cessen/ropey)

## Project Structure

- `crates/vellum`: app entry point, window layout, menus, tabs, and file operations
- `crates/editor`: editor core, Markdown projection, interactions, auto save, and conflict handling
- `crates/workspace`: workspace tree and file watching

## Run

```bash
cargo run
```

## Test

```bash
cargo check
cargo test -p editor
cargo test -p workspace
cargo test --workspace
```

## Notes

- The sidebar currently shows only `.md`, `.markdown`, and `.mdown`
- `Enter` performs semantic line breaks for paragraphs, lists, blockquotes, and similar blocks
- Code blocks keep normal multi-line editing behavior
- The current app model is single-window with multiple editor tabs
- Vellum is intentionally a pure Markdown editor; plugin and extension support has been removed
