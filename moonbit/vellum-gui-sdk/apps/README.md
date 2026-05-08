# Vellum MoonBit Apps

This directory contains MoonBit components that target the new `vellum:app/app-world` framework protocol.

## Markdown Demo

```bash
cd moonbit/vellum-gui-sdk/apps/markdown-demo
./build.sh
```

Run it inside Vellum:

```bash
VELLUM_APP=moonbit/vellum-gui-sdk/apps/markdown-demo cargo run -p Vellum
```

The demo shell is described by MoonBit and rendered by Rust/GPUI. The central editor is still the existing Rust `MarkdownEditor`, embedded with a `native-view` node whose kind is `markdown-editor`.
