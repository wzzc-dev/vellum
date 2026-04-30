# Vellum Code Wiki

## 1. 项目概述

Vellum 是一个现代化的 Markdown 编辑器，采用 Rust 语言开发，集成了实时预览和源码编辑模式。它使用 tree-sitter 进行 Markdown 语法解析，支持代码高亮、表格对齐、数学公式渲染等功能，并提供基于 WASM 的扩展系统。

### 1.1 核心特性

- **双视图模式**: 支持 Source 模式和 Preview 模式，可无缝切换
- **智能渲染**: 自动处理 Markdown 的隐藏语法（如多余的换行符）
- **语法高亮**: 支持多种编程语言的代码块高亮
- **表格支持**: 完整的 Pipe Table 支持，包括列对齐
- **数学公式**: 支持 KaTeX 渲染数学块
- **扩展系统**: 基于 WASM Component Model 的插件系统

---

## 2. 架构设计

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        Vellum Application                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────────┐  │
│  │   App       │───▶│  Workspace    │───▶│  Editor           │  │
│  │   Layer     │    │  Layer        │    │  Layer            │  │
│  └─────────────┘    └──────────────┘    └───────────────────┘  │
│                                                 │                │
│                      ┌──────────────────────────┼───────────┐    │
│                      ▼                          ▼           ▼    │
│              ┌──────────────┐           ┌───────────┐ ┌───────┐ │
│              │   UI Layer   │           │   Core    │ │Syntax │ │
│              │   (view.rs)  │           │ Controller│ │ Parser│ │
│              └──────────────┘           └───────────┘ └───────┘ │
│                                                 │                │
│                      ┌──────────────────────────┼───────────┐    │
│                      ▼                          ▼           ▼    │
│              ┌──────────────┐           ┌───────────┐ ┌───────┐ │
│              │  Document    │           │  Display  │ │ File  │ │
│              │  Buffer      │           │   Map     │ │ Sync  │ │
│              └──────────────┘           └───────────┘ └───────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Extension System (WASM)                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │  Extension   │───▶│  Extension   │───▶│   WASM Runtime   │  │
│  │  Host        │    │  Registry    │    │   (wasmtime)     │  │
│  └──────────────┘    └──────────────┘    └──────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 分层职责

| 层级 | 模块 | 职责 |
|------|------|------|
| Application | `vellum` | 应用入口、窗口管理、命令面板 |
| Workspace | `workspace` | 工作区状态管理 |
| Editor | `editor` | 核心编辑功能 |
| UI | `ui/` | 用户界面渲染 |
| Core | `core/` | 文档模型、语法解析、显示映射 |
| Extension | `extension` | WASM 扩展运行时 |

---

## 3. Crate 结构

### 3.1 工作空间配置

```toml
# /workspace/Cargo.toml
[workspace]
resolver = "3"
members = [
    "crates/editor",
    "crates/vellum",
    "crates/workspace",
    "crates/extension",
    "crates/extension-sdk",
    "examples-extensions/markdown-lint"
]
default-members = ["crates/vellum"]
```

### 3.2 各 Crate 说明

| Crate | 类型 | 描述 |
|-------|------|------|
| `Vellum` | 应用 | 主应用程序入口 |
| `editor` | 库 | 核心编辑器逻辑 |
| `workspace` | 库 | 工作区管理 |
| `vellum-extension` | 库 | 扩展宿主运行时 |
| `extension-sdk` | 库 | 扩展开发 SDK |
| `markdown-lint` | 示例 | 扩展开发示例 |

---

## 4. 核心模块 (editor/src/core/)

### 4.1 Controller 模块 (`controller.rs`)

**文件路径**: `/workspace/crates/editor/src/core/controller.rs`

**核心数据结构**:

```rust
pub struct EditorController {
    pub(crate) sync_policy: SyncPolicy,           // 同步策略
    pub(crate) document: DocumentBuffer,           // 文档缓冲区
    pub(crate) sync: FileSyncCoordinator,         // 文件同步协调器
    pub(crate) selection: SelectionState,          // 选择状态
    pub(crate) undo_stack: Vec<EditHistoryEntry>,  // 撤销栈
    pub(crate) redo_stack: Vec<EditHistoryEntry>,  // 重做栈
    pub(crate) view_mode: EditorViewMode,          // 视图模式
}
```

**主要职责**:
- 管理编辑器状态
- 处理所有编辑命令 (`EditCommand`)
- 协调文件同步
- 管理撤销/重做

**关键方法**:

| 方法 | 说明 |
|------|------|
| `new()` | 创建新的编辑器控制器 |
| `dispatch()` | 分发编辑命令 |
| `save()` | 保存文档到文件 |
| `undo()` | 撤销操作 |
| `redo()` | 重做操作 |
| `apply_file_event()` | 应用文件变化事件 |
| `snapshot()` | 获取编辑器快照 |

**EditCommand 枚举** (主要命令):

```rust
pub enum EditCommand {
    Insert { text: String, position: Anchor },
    Delete { range: Range<Anchor> },
    InsertBreak,
    ToggleHeading(HeadingVariant),
    ToggleParagraph,
    ToggleBlockquote,
    ToggleBulletList,
    ToggleOrderedList,
    ToggleCodeFence,
    InsertTable { headless: bool },
    InsertMathBlock,
    InsertHorizontalRule,
    BoldSelection,
    ItalicSelection,
    LinkSelection,
    ToggleInlineCode,
    ToggleStrikethrough,
    PromoteBlock,
    DemoteBlock,
    FocusPrevBlock,
    FocusNextBlock,
    ExitBlockEdit,
    ToggleSourceMode,
    UndoEdit,
    RedoEdit,
}
```

### 4.2 Document 模块 (`document.rs`)

**文件路径**: `/workspace/crates/editor/src/core/document.rs`

**核心数据结构**:

```rust
pub struct DocumentBuffer {
    rope: Rope,                    // 使用 ropey 的 rope 数据结构
    blocks: BlockProjection,       // 块投影
}
```

**BlockKind 枚举** (所有块类型):

```rust
pub enum BlockKind {
    Raw,                           // 原始文本
    Paragraph,                     // 段落
    Heading { depth: u8 },         // 标题 (1-6)
    Blockquote,                    // 引用块
    List,                          // 列表
    Table,                         // 表格
    CodeFence { language: Option<String> },  // 代码块
    MathBlock,                     // 数学块
    ThematicBreak,                  // 分隔线
    Html,                          // HTML块
    YamlFrontMatter,              // YAML 前置元数据
    FootnoteDefinition,            // 脚注定义
    Footnote,                      // 脚注引用
    SourceCode,                    // 源码块
    Unknown,                       // 未知类型
}
```

**BlockProjection 结构**:

```rust
pub struct BlockProjection {
    blocks: Arc<[BlockProjectionEntry]>,
}

pub struct BlockProjectionEntry {
    start_offset: usize,
    end_offset: usize,
    kind: BlockKind,
    indent: u8,
}
```

**SelectionState 结构**:

```rust
pub struct SelectionState {
    anchor: Anchor,
    head: Anchor,
    affinity: SelectionAffinity,
}
```

**Transaction 结构**:

```rust
pub struct Transaction {
    edits: Vec<DocumentEdit>,
    timestamp: DateTime<Utc>,
}
```

### 4.3 DisplayMap 模块 (`display_map.rs`)

**文件路径**: `/workspace/crates/editor/src/core/display_map.rs`

**核心数据结构**:

```rust
pub struct DisplayMap {
    source: DocumentSource,
    blocks: Vec<RenderBlock>,
    hidden_syntax_policy: HiddenSyntaxPolicy,
}
```

**主要职责**:
- 映射源文本和可见文本
- 处理隐藏语法 (如多余的换行)
- 解析 emoji
- 处理表格对齐

**RenderBlock 结构**:

```rust
pub struct RenderBlock {
    blocks: Vec<RenderSpan>,
    selection_anchors: Vec<(usize, usize)>,
    view_id: ViewId,
}
```

**RenderSpan 结构**:

```rust
pub struct RenderSpan {
    source_range: Range<usize>,
    kind: RenderSpanKind,
    meta: RenderSpanMeta,
}
```

**RenderSpanKind 枚举**:

```rust
pub enum RenderSpanKind {
    Unchanged,
    Hidden,
    Inserted,
    Modified,
}
```

**BlockBuilder 结构**:

```rust
pub struct BlockBuilder<'a> {
    syntax: &'a SyntaxState,
    source: &'a DocumentSource,
    display_map: &'a mut DisplayMap,
    hidden_syntax_policy: HiddenSyntaxPolicy,
}
```

### 4.4 Syntax 模块 (`syntax.rs`)

**文件路径**: `/workspace/crates/editor/src/core/syntax.rs`

**核心数据结构**:

```rust
pub struct SyntaxState {
    parser: MarkdownParser,
    last_parsed_revision: u64,
}
```

**主要职责**:
- 使用 tree-sitter 解析 Markdown
- 管理语法树
- 提供块解析接口

**MarkdownParser 结构**:

```rust
pub struct MarkdownParser {
    parser: tree_sitter::Parser,
    languages: HashMap<String, tree_sitter::Language>,
}
```

**PreviewBlock 结构** (预览块):

```rust
pub struct PreviewBlock {
    kind: BlockKind,
    source_range: Range<usize>,
    rendered_range: Range<usize>,
    seed: BlockSeed,
}
```

---

## 5. UI 模块 (editor/src/ui/)

### 5.1 View 模块 (`view.rs`)

**文件路径**: `/workspace/crates/editor/src/ui/view.rs`

**核心数据结构**:

```rust
pub struct MarkdownEditor {
    workspace: WeakView<Workspace>,
    controller: View<EditorController>,
    surface: Renderable<EditorSurface>,
    pending_input: Vec<InputEvent>,
    cursor_shape: CursorShape,
    view_mode: EditorViewMode,
}
```

**主要职责**:
- 用户界面渲染
- 输入事件处理
- 光标管理
- 滚动处理

**EditorEvent 枚举** (编辑器事件):

```rust
pub enum EditorEvent {
    Focus,
    Blur,
    Select { selection: SelectionModel },
    Highlight { range: Range<usize> },
    Edit,
    TitleChanged,
    DirtyStateChanged,
    Saved,
    Conflict { conflict: ConflictState },
    WrapChanged,
}
```

### 5.2 其他 UI 组件

| 文件 | 功能 |
|------|------|
| `commands.rs` | 命令绑定 |
| `file_ops.rs` | 文件操作 |
| `input_bridge.rs` | 输入桥接 |
| `layout.rs` | 布局管理 |
| `math_completion_panel.rs` | 数学补全面板 |
| `slash_command.rs` | 斜杠命令 |
| `surface.rs` | 编辑器表面渲染 |
| `theme.rs` | 主题管理 |

---

## 6. 扩展系统

### 6.1 Extension Host (`extension/src/host.rs`)

**文件路径**: `/workspace/crates/extension/src/host.rs`

**核心数据结构**:

```rust
pub struct ExtensionHost {
    engine: Engine,
    linker: Linker<ExtensionRuntimeState>,
    registry: ExtensionRegistry,
    loaded_extensions: HashMap<String, LoadedExtension>,
}
```

**ExtensionRuntimeState 结构**:

```rust
pub struct ExtensionRuntimeState {
    capabilities: Capabilities,
    outputs: ExtensionOutputs,
}
```

**主要职责**:
- 加载和管理 WASM 扩展
- 管理扩展生命周期
- 提供扩展 API

**支持的能力 (Capabilities)**:

| 能力 | 说明 |
|------|------|
| `DocumentRead` | 读取文档内容 |
| `DocumentWrite` | 修改文档内容 |
| `Decorations` | 装饰器 (高亮、下划线等) |
| `Panels` | 侧边面板 |
| `Timers` | 定时器 |
| `Webview` | Web视图 |

### 6.2 Extension SDK (`extension-sdk/`)

**文件路径**: `/workspace/crates/extension-sdk/src/`

扩展开发者使用的 SDK，提供:

- `Extension`: 扩展主结构
- `ExtensionContext`: 扩展上下文
- 事件处理
- 装饰器 API
- UI 组件

### 6.3 WIT 接口定义

**文件路径**: `/workspace/crates/extension/wit/vellum-extension.wit`

定义扩展与宿主之间的接口契约。

---

## 7. 依赖关系

### 7.1 外部依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `gpui` | 0.2.2 | UI 框架 |
| `gpui-component` | 0.5.1 | UI 组件库 |
| `ropey` | 1.6 | 高效文本存储 |
| `tree-sitter` | 0.25.10 | 语法解析 |
| `tree-sitter-md` | 0.5.3 | Markdown 语法 |
| `wasmtime` | 35 | WASM 运行时 |
| `anyhow` | 1.0 | 错误处理 |
| `serde` | 1.0 | 序列化 |

### 7.2 内部依赖关系

```
vellum (应用)
├── editor
│   └── core/ (controller, document, display_map, syntax)
│   └── ui/ (view, commands, surface)
├── vellum-extension
└── workspace

extension (扩展运行时)
└── wasmtime (with component-model)

extension-sdk (扩展开发)
└── wit_bindgen
```

---

## 8. 运行方式

### 8.1 开发环境

**前置条件**:
- Rust 1.85+ (edition 2024)
- Cargo

### 8.2 构建项目

```bash
# 构建整个工作空间
cargo build

# 或只构建主应用
cargo build -p Vellum
```

### 8.3 运行应用

```bash
# 运行 Vellum 应用
cargo run -p Vellum

# 或者在 examples 目录运行示例
cargo run --example <example_name>
```

### 8.4 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定 crate 的测试
cargo test -p editor
cargo test -p vellum-extension
```

### 8.5 示例扩展开发

```bash
# 查看示例扩展
ls examples-extensions/

# 构建示例扩展
cargo build -p markdown-lint
```

---

## 9. 关键设计模式

### 9.1 MVC 分离

- **Model**: `DocumentBuffer`, `BlockProjection`
- **View**: `MarkdownEditor`, `EditorSurface`
- **Controller**: `EditorController`

### 9.2 快照模式

使用不可变快照进行状态记录，便于撤销/重做和协作编辑:

```rust
pub struct EditorSnapshot {
    document: Arc<DocumentBuffer>,
    selection: SelectionState,
    display_map: Arc<DisplayMap>,
}
```

### 9.3 事务机制

所有编辑操作通过 `Transaction` 封装，保证操作的原子性和可撤销性。

### 9.4 异步文件同步

使用 `FileSyncCoordinator` 管理文件同步，支持:
- 自动保存 (autosave)
- 脏状态跟踪 (dirty tracking)
- 冲突检测 (conflict detection)

---

## 10. 文件索引

| 文件路径 | 主要内容 |
|----------|----------|
| `crates/editor/src/core/controller.rs` | 编辑器控制器 |
| `crates/editor/src/core/document.rs` | 文档缓冲区 |
| `crates/editor/src/core/display_map.rs` | 显示映射 |
| `crates/editor/src/core/syntax.rs` | 语法解析 |
| `crates/editor/src/core/text_ops.rs` | 文本操作 |
| `crates/editor/src/core/table.rs` | 表格处理 |
| `crates/editor/src/core/math_render.rs` | 数学渲染 |
| `crates/editor/src/core/code_highlight.rs` | 代码高亮 |
| `crates/editor/src/ui/view.rs` | 主视图 |
| `crates/editor/src/ui/surface.rs` | 渲染表面 |
| `crates/extension/src/host.rs` | 扩展宿主 |
| `crates/vellum/src/main.rs` | 应用入口 |
