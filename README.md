# Vellum

[简体中文](./README.zh-CN.md)

Vellum is a desktop Markdown editor built with Rust and `gpui`.

![Vellum screenshot](./docs/vellum-screenshot.png)

## Current Features

- WYSIWYG
- Open a Markdown file or folder
- Switch between block-level preview and editing
- Auto save
- Workspace sidebar
- Watch for external file changes, deletions, and renames
- Conflict detection and handling
- Restore the last opened file on startup

## Tech Stack

- [Rust](https://www.rust-lang.org/)
- [gpui](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [gpui-components](https://github.com/longbridge/gpui-component)

## Project Structure

- `crates/vellum`: app entry point, window layout, menus, and file operations
- `crates/editor`: editor core, block parsing, interactions, auto save, and conflict handling
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
```

## Notes

- The sidebar currently shows only `.md`, `.markdown`, and `.mdown`
- `Enter` performs semantic line breaks for paragraphs, lists, blockquotes, and similar blocks
- Code blocks keep normal multi-line editing behavior
- The current model is single-window and single-document
