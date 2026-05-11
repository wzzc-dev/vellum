# Vellum MoonBit Apps

This directory contains MoonBit components that target the new `vellum:app/app-world` framework protocol.

## Markdown Demo

```bash
cd moonbit/demos/markdown-editor
./build.sh
```

Run it inside Vellum:

```bash
VELLUM_APP=moonbit/demos/markdown-editor cargo run -p Vellum
```

The demo shell is described by MoonBit and rendered by Rust/GPUI. The central editor is still the existing Rust `MarkdownEditor`, embedded with a `native-view` node whose kind is `markdown-editor`.

The handwritten MoonBit app logic now lives in `src/`, while `gen/world/appWorld/` only contains the generated export layer.
