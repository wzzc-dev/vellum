# Vellum

[简体中文](./README.zh-CN.md)

Vellum is a desktop Markdown editor built with Rust and `gpui`. It also supports extensions written in MoonBit!

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

---

## Tech Stack

### Core Application

- [Rust](https://www.rust-lang.org/)
- [gpui](https://github.com/zed-industries/zed/tree/main/crates/gpui) - UI framework
- [gpui-components](https://github.com/longbridge/gpui-component) - UI component library

### Extension System

- [Wasmtime](https://wasmtime.dev/) - WASM runtime
- [WIT](https://component-model.bytecodealliance.org/) - Interface types
- [MoonBit](https://www.moonbitlang.com/) - Extension language

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
│   ├── vellum/                      # App entry point
│   ├── editor/                      # Editor core
│   ├── workspace/                   # Workspace management
│   ├── extension/                   # Extension host
│   ├── extension-sdk/               # Extension SDK
│   └── gpui-adapter/                # GPUI adapter for MoonBit GUI
├── examples-extensions/             # Example extensions
│   ├── pomodoro/                    # Pomodoro timer extension
│   └── moonbit-gui/                 # MoonBit GUI extension
├── moonbit/                         # MoonBit modules
│   └── vellum-gui-sdk/              # MoonBit GUI SDK
├── docs/                            # Documentation
│   ├── architecture.md              # Architecture overview
│   ├── building.md                  # Building & running guide
│   ├── gui-framework-guide.md       # GUI framework guide
│   └── moonbit-extension-guide.md   # MoonBit extension guide
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
| [MoonBit Extension Guide](./docs/moonbit-extension-guide.md) | How to write extensions in MoonBit |
| [Pomodoro Example](./examples-extensions/pomodoro/) | Full-featured extension example |
| [MoonBit GUI Example](./examples-extensions/moonbit-gui/) | Simple GUI extension example |

---

## Run

```bash
cargo run
```

Or in release mode:

```bash
cargo run --release
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

## Build Example Extensions

### Pomodoro Timer Extension

```bash
cd examples-extensions/pomodoro
./build.sh
cd ../../
cargo run
```

### MoonBit GUI Extension

```bash
cd examples-extensions/moonbit-gui
./build.sh
cd ../../
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

