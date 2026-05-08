# Vellum

[简体中文](./README.zh-CN.md)

Vellum is being refactored into a MoonBit + Rust/GPUI GUI framework. MoonBit components describe app logic and typed UI trees, Rust/GPUI renders native UI and manages windows/events, and WIT connects both sides through the WASM Component Model.

The Markdown WYSIWYG editor remains the primary demo. Its outer shell is a MoonBit app component, while the editor widget itself is a Rust/GPUI native view embedded through `NativeView(kind = "markdown-editor")`.

![Vellum screenshot](./docs/vellum-screenshot.png)

---

## Current Features

- ✅ WYSIWYG Markdown editing
- ✅ Open Markdown files or entire folders
- ✅ Block-level preview and editing switching
- ✅ Auto save
- ✅ Workspace sidebar
- ✅ Watch for external file changes, deletions, and renames
- ✅ Conflict detection and handling
- ✅ Restore last opened file on startup
- ✅ Extension support with WASM Component Model
- ✅ MoonBit extension development
- ✅ Extension panels with declarative UI
- ✅ Extension commands
- ✅ Extension timers
- 🚧 Experimental MoonBit App Component runtime with typed WIT UI trees
- 🚧 Native Markdown editor embedded as a framework `native-view`

---

## Tech Stack

### Core Application

- [Rust](https://www.rust-lang.org/)
- [gpui](https://github.com/zed-industries/zed/tree/main/crates/gpui) - UI framework
- [gpui-components](https://github.com/longbridge/gpui-component) - UI component library

### Framework Runtime

- [Wasmtime](https://wasmtime.dev/) - WASM runtime
- [WIT](https://component-model.bytecodealliance.org/) - Interface types
- [MoonBit](https://www.moonbitlang.com/) - App and plugin language

---

## Quick Start

### 1. Prerequisites

- [Rust toolchain](https://rust-lang.org/tools/install)
- System dependencies (see [Building Guide](./docs/building.md))

### 2. Build and Run

```bash
# Clone the repo
git clone https://github.com/your-org/vellum.git
cd vellum

# Build
cargo build

# Run
cargo run
```

---

## Project Structure

```
vellum/
├── crates/                          # Rust crates
│   ├── vellum/                      # Desktop host: windows, menus, app startup
│   ├── vellum-runtime/              # Wasmtime runtime and vellum.toml manifest
│   ├── vellum-renderer-gpui/        # Typed ViewTree -> GPUI renderer
│   ├── vellum-editor/               # Markdown editor native widget facade
│   ├── vellum-workspace/            # Workspace/file services facade
│   ├── vellum-extension-compat/     # Legacy extension-world compatibility
│   ├── editor/                      # Existing editor implementation
│   ├── workspace/                   # Existing workspace implementation
│   └── extension/                   # Existing legacy extension implementation
├── wit/                             # Canonical WIT packages
│   ├── vellum-app.wit               # App/plugin typed UI protocol
│   ├── vellum-extension.wit         # Legacy extension-world compat protocol
│   └── legacy/                      # Old experimental WIT files
├── moonbit/                         # MoonBit modules
│   ├── vellum-app-sdk/              # New app DSL staging package
│   ├── vellum-plugin-sdk/           # New plugin DSL staging package
│   ├── demos/markdown-editor/       # Main MoonBit app demo
│   └── legacy-extensions/           # Old MoonBit extension examples
├── examples/
│   ├── plugins/                     # New typed plugin examples
│   └── legacy-extensions/           # Compatibility examples
├── docs/                            # Documentation
│   ├── architecture.md              # Architecture overview
│   ├── building.md                  # Building & running guide
│   ├── gui-framework-guide.md       # GUI framework guide
│   └── legacy-extension.md          # Legacy extension compatibility guide
├── Cargo.toml
└── README.md
```

---

## Documentation

| Document | Purpose |
|----------|---------|
| [Building Guide](./docs/building.md) | How to build and run the project |
| [Architecture Overview](./docs/architecture.md) | Project architecture, modules, and design |
| [GUI Framework Guide](./docs/gui-framework-guide.md) | How to use the MoonBit GUI framework |
| [Legacy Extension Compatibility](./docs/legacy-extension.md) | How the old extension-world path is preserved |
| [MoonBit Extension Guide](./docs/moonbit-extension-guide.md) | Legacy MoonBit extension reference |
| [Markdown Demo](./moonbit/demos/markdown-editor/) | Main MoonBit app shell with native editor |

---

## Run

```bash
cargo run
```

Or in release mode:

```bash
cargo run --release
```

To run the experimental MoonBit framework shell:

```bash
cd moonbit/demos/markdown-editor
./build.sh
cd ../../..
VELLUM_APP=moonbit/demos/markdown-editor cargo run -p Vellum
```

For more details, see [Building Guide](./docs/building.md).

---

## Test

```bash
cargo check
cargo test -p editor
cargo test -p workspace
```

---

## Build Legacy Example Extensions

### Pomodoro Timer Extension

```bash
cd examples/legacy-extensions/pomodoro
./build.sh
cd ../../..
cargo run
```

### MoonBit GUI Extension

```bash
cd examples/legacy-extensions/moonbit-gui
./build.sh
cd ../../..
cargo run
```

For detailed instructions, see [Building Guide](./docs/building.md).

---

## Notes

- The sidebar currently shows only `.md`, `.markdown`, and `.mdown`
- `Enter` performs semantic line breaks for paragraphs, lists, blockquotes, and similar blocks
- Code blocks keep normal multi-line editing behavior
- The current model is single-window and single-document

---

## Contributing

Contributions are welcome! Please feel free to:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

---

## License

This project is distributed under the same license as Vellum.

---

## Acknowledgments

Thanks to the following projects and communities:

- [gpui](https://github.com/zed-industries/zed)
- [gpui-components](https://github.com/longbridge/gpui-component)
- [Wasmtime](https://wasmtime.dev/)
- [MoonBit](https://www.moonbitlang.com/)
