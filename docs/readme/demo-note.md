# Vellum Demo Note

Vellum is a desktop Markdown editor focused on block editing, live structure, and a lightweight workspace flow.

## Editing Flow

- Open a folder and jump between Markdown files in the sidebar
- Edit blocks directly without dropping into raw Markdown all the time
- Keep auto save on while the editor watches for external file changes

## What To Look At

> The screenshots in the README show the app with this note open.

### Structured Content

1. Outline headings should appear in the sidebar
2. Lists, quotes, and code blocks stay readable while editing
3. Inline assets can be resolved from the current document directory

### Quick Table

| Capability | Status |
| --- | --- |
| WYSIWYG editing | Ready |
| Workspace sidebar | Ready |
| Auto save | Ready |
| Conflict handling | Ready |

### Embedded Asset

![Vellum illustration](./vellum-preview.svg)

### Code Sample

```rust
fn open_note(path: &str) {
    println!("Opening {path}");
}
```

## Search Targets

Search for the word `Vellum` to show the find panel, or search for `Ready` to exercise multiple matches.
