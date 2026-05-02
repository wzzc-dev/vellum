# Vellum 项目架构总览

本文档详细介绍 Vellum 项目的整体架构和各模块的职责。

---

## 目录

1. [项目概览](#项目概览)
2. [目录结构](#目录结构)
3. [核心模块详解](#核心模块详解)
4. [工作流程](#工作流程)
5. [技术栈](#技术栈)

---

## 项目概览

Vellum 是一个使用 Rust + GPUI 构建的桌面 Markdown 编辑器。项目采用模块化架构，支持通过 WASM Component Model 加载 MoonBit 扩展。

### 设计理念

- **模块化**：各功能独立封装，降低耦合
- **扩展性**：使用 WASM Component Model 支持第三方扩展
- **声明式 UI**：支持声明式 UI 树，由 Rust 侧渲染为原生控件
- **跨语言**：Rust (宿主) + MoonBit (扩展/UI逻辑)

---

## 目录结构

```
vellum/
├── crates/                           # Rust 工作空间成员
│   ├── vellum/                       # 主应用入口
│   ├── editor/                       # 编辑器核心
│   ├── workspace/                    # 工作区管理
│   ├── extension/                    # 扩展宿主实现
│   ├── extension-sdk/                # 扩展 SDK
│   └── gpui-adapter/                 # GPUI 适配器（MoonBit 集成）
│
├── examples-extensions/              # 示例扩展
│   ├── pomodoro/                     # 番茄钟扩展示例
│   ├── moonbit-gui/                  # MoonBit GUI 扩展示例
│   └── markdown-lint/                # Markdown 校验示例
│
├── moonbit/                          # MoonBit 模块
│   └── vellum-gui-sdk/               # MoonBit GUI SDK
│
├── docs/                             # 文档
│   ├── architecture.md               # 本文档
│   ├── gui-framework-guide.md        # GUI 框架指南
│   └── moonbit-extension-guide.md    # MoonBit 扩展开发指南
│
├── Cargo.toml                        # Rust 工作空间配置
└── README.md
```

---

## 核心模块详解

### 1. `crates/vellum` — 主应用入口

职责：
- 应用程序启动与配置
- 主窗口创建与管理
- 菜单栏实现
- 文件对话框集成
- 加载与初始化其他模块

关键文件：
- `src/main.rs` — 主入口点
- `src/app.rs` — 应用程序状态与事件循环
- `src/window.rs` — 主窗口配置

### 2. `crates/editor` — 编辑器核心

职责：
- Markdown 解析与渲染
- 编辑缓冲区管理
- 语法高亮
- 自动保存机制
- 冲突检测与处理
- 编辑操作历史

关键文件：
- `src/buffer.rs` — 文本缓冲区
- `src/parser.rs` — Markdown 解析
- `src/editor_view.rs` — 编辑器视图
- `src/history.rs` — 操作历史

### 3. `crates/workspace` — 工作区管理

职责：
- 文件树浏览
- 文件监视（检测外部变更）
- 多文件管理
- 最近打开文件记录

关键文件：
- `src/workspace_tree.rs` — 工作区树
- `src/file_watcher.rs` — 文件监视器
- `src/recent_files.rs` — 最近文件记录

### 4. `crates/extension` — 扩展宿主

职责：
- 加载 WASM Component 扩展
- 管理扩展生命周期（激活/停用）
- 分发事件给扩展
- 提供扩展宿主 API (host, editor, ui, timer)
- UI 节点的 GPUI 渲染

关键文件：
- `src/extension_host.rs` — 扩展宿主核心
- `src/extension_store.rs` — 扩展存储
- `wit/vellum-extension.wit` — WIT 接口定义

### 5. `crates/gpui-adapter` — GPUI 适配器

职责：
- 提供 MoonBit ↔ GPUI 的桥接
- Widget 树管理
- 事件分发
- Canvas 绘制 API
- 条件编译支持（可选功能）

此模块是为 MoonBit GUI Framework 设计的，用于将 MoonBit 侧的 UI 声明映射到 GPUI 渲染。

关键文件：
- `src/bridge.rs` — 核心桥接 API
- `src/widget.rs` — Widget 管理
- `src/gpui_render.rs` — GPUI 渲染集成
- `src/wit_host.rs` — WIT Host 实现
- `src/types.rs` — 共享类型定义

---

## MoonBit 扩展架构

MoonBit 扩展通过 WASM Component Model 与宿主交互，整个流程如下：

```
┌─────────────────────────────────────────────────────────────┐
│  MoonBit 扩展 (WASM Component)                              │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 声明式 UI 构建 (JSON 格式 UiNode 树)                   │ │
│  │ 业务逻辑处理                                            │ │
│  └───────────────────────┬───────────────────────────────┘ │
└───────────────────────────┼───────────────────────────────┘
                            │ export: activate/handle-event/...
                            │ import: host/editor/ui/timer
                            │
┌───────────────────────────▼───────────────────────────────┐
│  ExtensionHost (Rust)                                     │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 加载 WASM Component                                    │ │
│  │ 调用扩展导出函数                                        │ │
│  │ 提供扩展导入接口的实现                                  │ │
│  └───────────────────────┬───────────────────────────────┘ │
└───────────────────────────┼───────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────┐
│  GPUI (Native Rendering)                                  │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 将 UiNode 渲染为原生 UI 控件                            │ │
│  └───────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────┘
```

### 主要数据流

1. **激活**：`activate()` → 构建初始 UiNode → `set_panel_view()`
2. **交互**：用户点击按钮 → `handle_ui_event()` → 状态变更 → 重新构建 UiNode
3. **事件**：文档变更/定时器 → `handle_event()` → 状态变更 → UI 刷新

---

## WIT 接口契约

WIT 定义了宿主与扩展之间的接口契约：

### `vellum-extension.wit`

位于 `crates/extension/wit/vellum-extension.wit`，定义了：

- **Import 接口（宿主提供）**
  - `host`：日志、状态提示
  - `editor`：文档操作
  - `ui`：面板 UI
  - `timer`：定时器

- **Export 函数（扩展实现）**
  - `activate`：激活
  - `deactivate`：停用
  - `handle_event`：事件处理
  - `execute_command`：命令执行
  - `handle_ui_event`：UI 事件处理
  - `handle_hover`：悬停处理

---

## MoonBit GUI SDK

`moonbit/vellum-gui-sdk/` 提供了 MoonBit 侧的 UI 框架：

功能：
- 声明式 UI 组件库 (Text, Button, Column, Row, 等)
- 样式系统 (背景、边框、阴影、透明度、光标)
- 布局修饰符 (Padding, Margin, Width, Height, FlexGrow)
- 滚动视图组件
- 应用生命周期管理

---

## 工作流程

### 应用启动流程

```
1. main()
   ↓
2. App::new()
   ↓
3. 创建主窗口
   ↓
4. 初始化 Workspace
   ↓
5. 初始化 ExtensionHost
   ↓
6. 扫描、加载扩展
   ↓
7. 渲染初始 UI
   ↓
8. 进入事件循环
```

### 扩展加载与激活

```
1. 读取 extension.toml
   ↓
2. 加载 WASM Component
   ↓
3. 调用 activate()
   ↓
4. 扩展调用 set_panel_view() 显示 UI
   ↓
5. 用户与 UI 交互 → handle_ui_event()
```

### 文档编辑流程

```
1. 用户键盘/鼠标输入
   ↓
2. EditorView 处理
   ↓
3. Buffer 更新
   ↓
4. 触发 document.changed 事件
   ↓
5. 扩展收到 handle_event()
   ↓
6. 扩展可根据需要处理
```

---

## 技术栈

### Rust 侧

| 技术 | 用途 | 来源 |
|------|------|------|
| `gpui` | UI 框架与渲染引擎 | [zed-industries/gpui](https://github.com/zed-industries/zed/tree/main/crates/gpui) |
| `gpui-component` | UI 组件库 | [longbridge/gpui-component](https://github.com/longbridge/gpui-component) |
| `wasmtime` | WASM Component 运行时 | [bytecodealliance/wasmtime](https://github.com/bytecodealliance/wasmtime) |
| `wit-bindgen` | WIT 绑定生成器 | [bytecodealliance/wit-bindgen](https://github.com/bytecodealliance/wit-bindgen) |
| `serde` + `serde_json` | 序列化与 JSON 处理 | 标准 |
| `thiserror` | 错误处理宏 | 标准 |
| `parking_lot` | 同步原语 | 标准 |

### MoonBit 侧

| 技术 | 用途 | 来源 |
|------|------|------|
| `moon` | MoonBit 编译器 | [moonbitlang.com](https://www.moonbitlang.com) |
| `moonbitlang/core` | 核心库 | 标准 |
| `wit-bindgen-moonbit` | MoonBit 绑定生成 | 标准 |

---

## 模块依赖关系

```
vellum (主应用)
├── editor (编辑器)
│   ├── workspace (工作区)
│   └── extension (扩展宿主)
│       ├── extension-sdk (扩展 SDK)
│       └── gpui-adapter (GPUI 适配器，可选)
```

---

## 功能特性

- [x] Markdown 所见即所得编辑
- [x] 块级编辑与预览
- [x] 文件树浏览
- [x] 自动保存
- [x] 文件变更监视与冲突处理
- [x] WASM 扩展支持
- [x] MoonBit 扩展开发
- [x] 扩展面板 UI
- [x] 声明式 UI 构建
- [x] 扩展命令注册
- [x] 扩展定时器
- [x] 编辑器装饰（如 Markdown 校验警告）

---

## 下一步学习

- 阅读 [gui-framework-guide.md](./gui-framework-guide.md) 了解 MoonBit GUI Framework
- 阅读 [moonbit-extension-guide.md](./moonbit-extension-guide.md) 学习扩展开发
- 查看 [examples-extensions/](../examples-extensions/) 中的示例
- 运行 `cargo run` 启动应用

---

## 许可证

与 Vellum 项目保持一致。
