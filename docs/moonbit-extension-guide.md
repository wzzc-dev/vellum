# Vellum MoonBit 扩展开发指南

## 概述

Vellum 支持使用 [MoonBit](https://www.moonbitlang.com/) 编写 WASM Component 扩展。MoonBit 扩展通过 WASM Component Model 与宿主（Vellum app）交互，可以访问文档、渲染 UI 面板、注册命令、使用定时器等能力。

### 架构概览

```
┌─────────────────────────────────────────────────────┐
│  Vellum App (Host)                                   │
│  ┌─────────────────────────────────────────────────┐ │
│  │ ExtensionHost                                    │ │
│  │  - 加载 WASM Component                          │ │
│  │  - 分发事件 (dispatch_event / dispatch_timer)    │ │
│  │  - 收集输出 (take_outputs)                       │ │
│  └───────────────┬─────────────────────────────────┘ │
│                  │ WIT Interface                       │
│  ┌───────────────▼─────────────────────────────────┐ │
│  │ MoonBit Extension (WASM Component)               │ │
│  │  - 导出: activate / handle-event / handle-ui ... │ │
│  │  - 导入: host / editor / ui / timer              │ │
│  └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

扩展通过 **import**（导入）调用宿主提供的 API，通过 **export**（导出）实现生命周期钩子供宿主调用。接口契约由 [WIT](https://component-model.bytecodealliance.org/design/wit.html) 文件定义。

---

## 前置条件

| 工具 | 用途 | 安装方式 |
|------|------|----------|
| `moon` ≥ 0.1.x | MoonBit 编译器 | [官方安装指南](https://www.moonbitlang.com/download/) |
| `wit-bindgen` ≥ 0.40 | 从 WIT 生成 MoonBit 绑定 | `cargo install wit-bindgen-cli` |
| `wasm-tools` ≥ 1.238 | 构建 WASM Component | `cargo install wasm-tools` |
| `cargo` + `wasm32-wasip2` target | Rust 工具链（仅构建宿主时需要） | Rustup |

验证安装：

```bash
moon version
wit-bindgen --version
wasm-tools --version
```

---

## 快速开始

### 1. 创建项目目录

```bash
mkdir my-extension && cd my-extension
```

### 2. 创建 WIT 符号链接

WIT 文件定义了宿主与扩展之间的接口。将其符号链接到项目中：

```bash
ln -s /path/to/vellum/crates/extension/wit wit
```

### 3. 生成 MoonBit 绑定

```bash
wit-bindgen moonbit wit/vellum-extension.wit \
    --world extension-world \
    --out-dir . \
    --derive-show --derive-eq --derive-error
```

此命令会生成以下目录结构：

```
.
├── interface/vellum/extension/   # WIT 接口对应的 MoonBit 类型
│   ├── types/top.mbt             # ExtensionEvent, UiEvent, LogLevel 等
│   ├── host/top.mbt              # log(), show_status_message()
│   ├── editor/top.mbt            # document_text(), replace_range() 等
│   ├── ui/top.mbt                # set_panel_view()
│   └── timer/top.mbt             # now_ms(), request_tick(), cancel_tick()
├── gen/
│   ├── world/extensionWorld/     # World 绑定（导出函数声明）
│   │   ├── top.mbt               # declare pub fn activate / handle-event / ...
│   │   ├── import.mbt            # import 函数的 FFI 声明
│   │   └── ffi.mbt / ffi_import.mbt
│   └── gen/world/extensionWorld/ # 适配层
└── moon.mod.json                 # 模块配置
```

### 4. 修正包路径

wit-bindgen 生成的 `moon.pkg.json` 中包路径默认为 `vellum/extension/`，需要替换为你的模块名：

```bash
find . -name "moon.pkg.json" -not -path "./_build/*" \
    -exec sed -i '' 's|vellum/extension/interface/vellum/extension/|<你的模块名>/interface/vellum/extension/|g' {} +
```

例如模块名为 `myname/my-extension`，则替换为 `myname/my-extension/interface/vellum/extension/`。

### 5. 配置 moon.mod.json

编辑根目录的 `moon.mod.json`：

```json
{
  "name": "myname/my-extension",
  "preferred-target": "wasm"
}
```

### 6. 编写扩展逻辑

在 `gen/world/extensionWorld/` 目录下创建 `.mbt` 文件，实现 wit-bindgen 声明的导出函数。详见后续章节。

### 7. 编写 extension.toml

在项目根目录创建 `extension.toml`，声明扩展的元信息、能力和贡献点。详见后续章节。

### 8. 构建与运行

```bash
# 构建 core wasm
moon build --target wasm --release

# 查找输出的 wasm 文件
WASM_INPUT="_build/wasm/release/build/gen/gen.wasm"

# 嵌入 WIT 元数据（注意 --encoding utf16）
wasm-tools component embed wit "$WASM_INPUT" \
    --world extension-world \
    --encoding utf16 \
    -o target/component.embed.wasm

# 创建 WASM Component
wasm-tools component new target/component.embed.wasm \
    -o target/wasm32-wasip2/release/my_extension.wasm
```

### 9. 验证组件

```bash
# 查看组件的 WIT 接口
wasm-tools component wit target/wasm32-wasip2/release/my_extension.wasm
```

---

## 项目结构详解

一个完整的 MoonBit 扩展项目结构如下：

```
my-extension/
├── extension.toml                    # 扩展清单（Vellum 读取）
├── moon.mod.json                     # MoonBit 模块配置
├── wit/                              # WIT 文件（符号链接）
│   └── vellum-extension.wit
├── build.sh                          # 构建脚本
├── interface/                        # wit-bindgen 生成的接口类型
│   └── vellum/extension/
│       ├── types/moon.pkg.json + top.mbt
│       ├── host/moon.pkg.json + top.mbt
│       ├── editor/moon.pkg.json + top.mbt
│       ├── ui/moon.pkg.json + top.mbt
│       └── timer/moon.pkg.json + top.mbt
└── gen/                              # wit-bindgen 生成的 world 绑定
    └── world/extensionWorld/
        ├── moon.pkg.json             # 包配置（需编辑添加 import）
        ├── top.mbt                   # 导出函数声明（declare pub fn）
        ├── import.mbt                # import FFI 声明
        ├── ffi.mbt                   # export FFI 声明
        └── <你的实现文件>.mbt         # ← 在这里编写你的代码
```

> **重要**：wit-bindgen 生成的 `declare pub fn` 声明和你的实现必须在同一个 MoonBit 包（同一目录）中。

---

## extension.toml 参考

```toml
# 必填字段
id = "vellum.my-extension"          # 全局唯一 ID，建议使用反向域名格式
name = "My Extension"                # 显示名称
version = "0.1.0"                    # 语义版本
schema_version = 1                   # 当前仅支持 1

# 可选字段
authors = ["Author Name"]
description = "Extension description"
repository = ""

# WASM 组件路径（相对于 extension.toml 所在目录）
[wasm]
component = "../../target/wasm32-wasip2/release/my_extension.wasm"

# 激活事件（空数组 = 所有事件都激活）
[activation]
events = []                          # 或 ["document.opened", "document.changed"]

# 能力声明（默认全部 false，按需开启）
[capabilities]
document_read = false                # 读取文档内容
document_write = false               # 修改文档内容
decorations = false                  # 添加编辑器装饰（波浪线等）
panels = false                       # 注册侧边面板
commands = false                     # 注册命令
webview = false                      # 嵌入 WebView
timers = false                       # 使用定时器

# 注册命令（需要 commands = true）
[[contributes.commands]]
id = "my-extension.run"              # 命令 ID（不含扩展 ID 前缀）
title = "Run My Extension"           # 命令显示标题
key = "cmd-shift-x"                  # 可选，快捷键

# 注册面板（需要 panels = true）
[[contributes.panels]]
id = "my-panel"                      # 面板 ID（不含扩展 ID 前缀）
title = "My Panel"                   # 面板标题
icon = "file-text"                   # 图标名（默认 "file-text"）
location = "right"                   # 面板位置（默认 "right"）
```

### 能力与接口对应关系

| 能力 | WIT 接口 | 允许调用的函数 |
|------|----------|---------------|
| `document_read` | `editor` | `document-text()`, `document-path()` |
| `document_write` | `editor` | `replace-range()`, `insert-text()` |
| `decorations` | `editor` | `set-decorations()`, `clear-decorations()` |
| `panels` | `ui` | `set-panel-view()` |
| `webview` | `ui` | `set-panel-view()`（含 WebView 节点时） |
| `timers` | `timer` | `now-ms()`, `request-tick()`, `cancel-tick()` |
| `commands` | — | 注册 `contributes.commands` |

调用未声明的能力会返回 `HostError`，扩展应妥善处理。

---

## WIT 接口参考

### 导入接口（Host 提供给扩展）

#### `host` — 基础宿主能力

```wit
interface host {
    log: func(level: log-level, message: string);
    show-status-message: func(message: string) -> result<_, host-error>;
}
```

| 函数 | MoonBit 签名 | 说明 |
|------|-------------|------|
| `log` | `@host.log(level, message)` | 输出日志到 stderr |
| `show-status-message` | `@host.show_status_message(msg)` | 在状态栏显示消息 |

`LogLevel` 枚举：`Trace` / `Debug` / `Info` / `Warn` / `Error`

#### `editor` — 文档操作（需要 document_read / document_write）

```wit
interface editor {
    document-text: func() -> result<string, host-error>;
    document-path: func() -> result<option<string>, host-error>;
    replace-range: func(start: u64, end: u64, text: string) -> result<_, host-error>;
    insert-text: func(position: u64, text: string) -> result<_, host-error>;
    set-decorations: func(data: list<u8>) -> result<_, host-error>;
    clear-decorations: func() -> result<_, host-error>;
}
```

#### `ui` — 面板 UI（需要 panels）

```wit
interface ui {
    set-panel-view: func(panel-id: string, data: list<u8>) -> result<_, host-error>;
}
```

`data` 参数为 `VersionedPayload<UiNode>` 的 JSON 编码字节。详见 [UI 构建](#ui-构建) 章节。

#### `timer` — 定时器（需要 timers）

```wit
interface timer {
    now-ms: func() -> u64;
    request-tick: func(interval-ms: u32) -> result<_, host-error>;
    cancel-tick: func() -> result<_, host-error>;
}
```

| 函数 | MoonBit 签名 | 说明 |
|------|-------------|------|
| `now-ms` | `@timer.now_ms()` | 返回当前 Unix 毫秒时间戳 |
| `request-tick` | `@timer.request_tick(interval_ms)` | 请求定时 tick，间隔单位毫秒 |
| `cancel-tick` | `@timer.cancel_tick()` | 取消定时 tick |

> **重要**：`request-tick` 请求后，宿主每秒检查是否有到期扩展，到期时通过 `handle-event` 发送 `"timer.tick"` 事件。tick 间隔由宿主以 1 秒粒度调度，实际精度取决于宿主循环。

### 导出函数（扩展必须实现）

```wit
world extension-world {
    export activate: func(ctx: activation-context) -> result<_, extension-error>;
    export deactivate: func() -> result<_, extension-error>;
    export handle-event: func(event: extension-event) -> result<_, extension-error>;
    export execute-command: func(command-id: string) -> result<_, extension-error>;
    export handle-ui-event: func(event: ui-event) -> result<_, extension-error>;
    export handle-hover: func(hover-data: string) -> result<option<list<u8>>, extension-error>;
}
```

| 函数 | 触发时机 | 说明 |
|------|----------|------|
| `activate` | 扩展首次被激活时 | 初始化状态、渲染初始 UI |
| `deactivate` | 扩展被卸载时 | 清理资源、取消定时器 |
| `handle-event` | 收到文档事件或 timer.tick | 根据事件类型分发处理 |
| `execute-command` | 用户触发命令时 | 命令 ID 为完整限定名（`扩展ID.命令ID`） |
| `handle-ui-event` | 面板 UI 交互时 | 按钮点击、输入变更等 |
| `handle-hover` | 编辑器悬停时 | 返回 Tooltip（可选） |

### 核心类型

```wit
record activation-context {
    extension-id: string,
    extension-path: string,
}

record extension-event {
    event-type: string,           // "document.opened" / "document.changed" / "timer.tick"
    document-text: string,
    document-path: option<string>,
    timestamp-ms: option<u64>,    // 仅 timer.tick 时有值
}

record ui-event {
    panel-id: string,             // 限定面板 ID（如 "vellum.my-ext.my-panel"）
    element-id: string,           // UI 元素 ID
    event-kind: string,           // "button.clicked" / "input.changed" 等
    value: option<string>,
    index: option<u32>,
    checked: option<bool>,
}
```

---

## UI 构建

Vellum 使用声明式 UI 树。扩展通过 `set-panel-view` 传递 JSON 编码的 `UiNode` 树给宿主，宿主递归渲染为原生 UI 元素。

### VersionedPayload 格式

所有跨 WASM 边界传递的 UI 数据必须包裹在 `VersionedPayload` 中：

```json
{
  "version": 1,
  "data": { ... }
}
```

当前版本固定为 `1`。

### UiNode 类型与 JSON 格式

UiNode 是一个 Rust 枚举类型，使用 serde 的 **externally tagged** 格式序列化。每个变体序列化为 `{"VariantName": {fields}}`，unit 变体序列化为 `"VariantName"`。

#### 容器节点

| 节点 | JSON 格式 |
|------|-----------|
| **Column**（垂直布局） | `{"Column":{"children":[...],"gap":8.0,"padding":null,"scrollable":false}}` |
| **Row**（水平布局） | `{"Row":{"children":[...],"gap":8.0,"padding":null}}` |

`padding` 为 null 或 `{"top":1.0,"right":1.0,"bottom":1.0,"left":1.0}`。

#### 内容节点

| 节点 | JSON 格式 |
|------|-----------|
| **Text** | `{"Text":{"content":"Hello","style":{"size":null,"color":null,"bold":null,"italic":null,"monospace":null}}}` |
| **Heading** | `{"Heading":{"content":"Title","level":2}}` |

`style` 中所有字段可为 null。`size` 为字号（f32），`color` 为颜色名字符串（如 `"muted-foreground"`），`bold`/`italic`/`monospace` 为布尔值。

#### 交互节点

| 节点 | JSON 格式 |
|------|-----------|
| **Button** | `{"Button":{"id":"btn1","label":"Click","variant":"Primary","icon":null,"disabled":false}}` |
| **TextInput** | `{"TextInput":{"id":"input1","placeholder":"Enter...","value":"","single_line":true}}` |
| **Checkbox** | `{"Checkbox":{"id":"cb1","label":"Enable","checked":false}}` |
| **Select** | `{"Select":{"id":"sel1","options":["A","B"],"selected":null}}` |
| **Toggle** | `{"Toggle":{"id":"tog1","label":"Dark","active":false}}` |

`ButtonVariant`：`"Primary"` / `"Secondary"` / `"Ghost"` / `"Danger"`

#### 指示节点

| 节点 | JSON 格式 |
|------|-----------|
| **Badge** | `{"Badge":{"label":"Info","severity":null}}` |
| **Progress** | `{"Progress":{"value":0.5,"label":null}}` |

`Severity`：`"Hint"` / `"Info"` / `"Warning"` / `"Error"` 或 null。`value` 为 0.0~1.0。

#### 辅助节点

| 节点 | JSON 格式 |
|------|-----------|
| **Separator** | `"Separator"` |
| **Spacer** | `"Spacer"` |
| **Link** | `{"Link":{"id":"link1","label":"Click here"}}` |
| **List** | `{"List":{"items":[{"id":"i1","label":"Item","description":null,"icon":null,"severity":null,"children":[]}]}}` |
| **Disclosure** | `{"Disclosure":{"label":"Details","open":false,"children":[...]}}` |
| **Conditional** | `{"Conditional":{"condition":true,"when_true":{...},"when_false":null}}` |
| **WebView** | `{"WebView":{"id":"wv1","url":"https://...","allow_scripts":false,"allow_devtools":false}}` |

> **注意**：`Separator` 和 `Spacer` 是 unit 变体，序列化为纯字符串 `"Separator"` / `"Spacer"`，**不是** `{"Separator":null}` 或 `{"Separator":{}}`。

### UI 事件类型

| 交互 | event_kind | 含有效字段 |
|------|-----------|-----------|
| 按钮点击 | `"button.clicked"` | panel_id, element_id |
| 输入变更 | `"input.changed"` | + value |
| 复选框切换 | `"checkbox.toggled"` | + checked |
| 下拉选择 | `"select.changed"` | + index |
| 开关切换 | `"toggle.changed"` | + checked |
| 链接点击 | `"link.clicked"` | panel_id, element_id |
| 列表项点击 | `"list.item.clicked"` | + value (item_id) |
| 折叠切换 | `"disclosure.toggled"` | + checked |

---

## 实现导出函数

### 最小实现模板

在 `gen/world/extensionWorld/` 目录下创建 `.mbt` 文件：

```moonbit
pub fn activate(_ctx : @types.ActivationContext) -> Result[Unit, @types.ExtensionError] {
  Ok(())
}

pub fn deactivate() -> Result[Unit, @types.ExtensionError] {
  Ok(())
}

pub fn handle_event(_event : @types.ExtensionEvent) -> Result[Unit, @types.ExtensionError] {
  Ok(())
}

pub fn execute_command(_command_id : String) -> Result[Unit, @types.ExtensionError] {
  Ok(())
}

pub fn handle_ui_event(_event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}
```

### 编辑 moon.pkg.json

在 `gen/world/extensionWorld/moon.pkg.json` 中添加需要的接口导入：

```json
{
  "warn-list": "-44",
  "import": [
    { "path": "myname/my-extension/interface/vellum/extension/types", "alias": "types" },
    { "path": "myname/my-extension/interface/vellum/extension/host", "alias": "host" },
    { "path": "moonbitlang/core/encoding/utf8", "alias": "encoding/utf8" }
  ]
}
```

根据需要添加 `editor`、`ui`、`timer` 的导入。

### 带面板和按钮的示例

```moonbit
fn str_to_bytes(s : String) -> FixedArray[Byte] {
  let buf = @encoding/utf8.encode(s)
  FixedArray::makei(buf.length(), fn(i) { buf[i] })
}

fn versioned_json(json_str : String) -> FixedArray[Byte] {
  str_to_bytes("{\"version\":1,\"data\":" + json_str + "}")
}

fn ui_column(children : Array[String]) -> String {
  "{\"Column\":{\"children\":[" + children.iter().join(",") + "],\"gap\":8.0,\"padding\":null,\"scrollable\":false}}"
}

fn ui_text(content : String) -> String {
  "{\"Text\":{\"content\":\"" + content + "\",\"style\":{\"size\":null,\"color\":null,\"bold\":null,\"italic\":null,\"monospace\":null}}}"
}

fn ui_button(id : String, label : String, variant : String) -> String {
  "{\"Button\":{\"id\":\"" + id + "\",\"label\":\"" + label + "\",\"variant\":\"" + variant + "\",\"icon\":null,\"disabled\":false}}"
}

pub fn activate(_ctx : @types.ActivationContext) -> Result[Unit, @types.ExtensionError] {
  let ui = ui_column([
    ui_text("Hello from MoonBit!"),
    ui_button("my-btn", "Click Me", "Primary"),
  ])
  ignore(@ui.set_panel_view("my-panel", versioned_json(ui)))
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" && event.element_id == "my-btn" {
    ignore(@host.show_status_message("Button clicked!"))
  }
  Ok(())
}
```

### 使用定时器

```moonbit
pub(all) struct State {
  mut count : Int
}

let state : State = { count: 0 }

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" && event.element_id == "start-btn" {
    state.count = 0
    ignore(@timer.request_tick(1000U))
    refresh_ui()
  }
  Ok(())
}

pub fn handle_event(event : @types.ExtensionEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_type == "timer.tick" {
    state.count = state.count + 1
    refresh_ui()
  }
  Ok(())
}
```

> **注意**：MoonBit 不支持顶层 `let mut`。使用 `struct` + `mut` 字段 + 顶层 `let` 绑定来实现全局可变状态。

---

## 构建脚本

推荐的 `build.sh`：

```bash
#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

WIT_DIR="$SCRIPT_DIR/wit"
MODULE_NAME="myname/my-extension"

# 1. 生成 MoonBit 绑定（--ignore-stub 保留手写代码）
wit-bindgen moonbit "$WIT_DIR/vellum-extension.wit" \
    --world extension-world \
    --out-dir "$SCRIPT_DIR" \
    --derive-show --derive-eq --derive-error \
    --ignore-stub

# 2. 修正包路径
find . -name "moon.pkg.json" -not -path "./_build/*" \
    -exec sed -i '' "s|vellum/extension/interface/vellum/extension/|${MODULE_NAME}/interface/vellum/extension/|g" {} +

# 3. 构建 core wasm
moon build --target wasm --release

# 4. 查找输出
WASM_INPUT="_build/wasm/release/build/gen/gen.wasm"

# 5. 创建 component
mkdir -p target/wasm32-wasip2/release

wasm-tools component embed "$WIT_DIR" "$WASM_INPUT" \
    --world extension-world \
    --encoding utf16 \
    -o target/component.embed.wasm

wasm-tools component new target/component.embed.wasm \
    -o target/wasm32-wasip2/release/my_extension.wasm

echo "Built: target/wasm32-wasip2/release/my_extension.wasm"
```

### 关键注意事项

1. **`--encoding utf16`**：MoonBit 生成的 WASM 使用 UTF-16 编码字符串。省略此参数会导致字符串包含 `\0` 字节，宿主无法匹配 panel ID 等字符串。
2. **`--ignore-stub`**：后续重新生成绑定时保留手写的 `.mbt` 实现文件。
3. **首次生成去掉 `--ignore-stub`**：第一次运行时不要加此参数，让 wit-bindgen 生成完整的 stub 文件。

---

## 事件处理模式

### 文档事件

```moonbit
pub fn handle_event(event : @types.ExtensionEvent) -> Result[Unit, @types.ExtensionError] {
  match event.event_type {
    "document.opened" => { /* 文档打开 */ }
    "document.changed" => { /* 文档变更 */ }
    "timer.tick" => {
      let ts = match event.timestamp_ms {
        Some(t) => t
        None => @timer.now_ms()
      }
      /* 处理定时器 tick */
    }
    _ => ()
  }
  Ok(())
}
```

### 命令执行

命令 ID 为完整限定格式：`<扩展ID>.<命令ID>`。例如扩展 ID 为 `vellum.my-ext`，命令 ID 为 `run`，则完整命令 ID 为 `vellum.my-ext.run`。

```moonbit
pub fn execute_command(command_id : String) -> Result[Unit, @types.ExtensionError] {
  match command_id {
    "vellum.my-ext.run" => { /* 执行 run 命令 */ }
    _ => ()
  }
  Ok(())
}
```

### UI 事件

UI 事件中 `panel_id` 是限定面板 ID（如 `vellum.my-ext.my-panel`），`element_id` 是你在 UiNode 中设置的 `id` 字段。

```moonbit
pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" {
    match event.element_id {
      "my-button" => { /* 处理按钮点击 */ }
      _ => ()
    }
  }
  Ok(())
}
```

---

## 常见陷阱

### 1. Separator / Spacer 的 JSON 格式

**错误**：`{"Separator":{}}` 或 `{"Separator":null}`
**正确**：`"Separator"`

serde 的 unit variant 使用 externally tagged 格式序列化为纯字符串。

### 2. UTF-16 编码

构建 WASM Component 时**必须**使用 `--encoding utf16`，否则字符串在宿主侧会包含 `\0` 字节，导致 panel ID 匹配失败等诡异问题。

### 3. 包路径修正

wit-bindgen 生成的代码默认使用 `vellum/extension` 作为模块路径前缀。如果你的模块名不同（如 `myname/my-extension`），必须修正所有 `moon.pkg.json` 中的 `import` 路径，将 `vellum/extension/interface/` 替换为 `myname/my-extension/interface/`。

### 4. 全局可变状态

MoonBit 不支持顶层 `let mut`。使用 struct + mut 字段：

```moonbit
pub(all) struct State {
  mut counter : Int
  mut name : String
}

let state : State = { counter: 0, name: "" }
```

### 5. 未使用的接口会被优化掉

如果你的扩展不使用 `editor` 接口的任何函数，`wasm-tools component new` 会自动移除该 import。这是正常的，不影响功能。

### 6. activation.events 为空 = 总是激活

`activation.events = []` 意味着扩展在任何事件触发时都会被激活。如果只想在特定事件时激活，显式列出：`events = ["document.opened"]`。

### 7. UiNode JSON 中所有字段都必须存在

即使是可选字段（如 `style`、`padding`、`icon`），也必须在 JSON 中包含（值为 `null`）。省略字段会导致反序列化失败。

---

## 调试技巧

### 使用 host.log 输出日志

```moonbit
@host.log(@types.LogLevel::INFO, "debug: value=" + value.to_string())
@host.log(@types.LogLevel::ERROR, "error occurred!")
```

日志输出到 Vellum 的 stderr，格式为 `[extension:<id>:<level>] <message>`。

### 验证 WASM Component 接口

```bash
wasm-tools component wit target/wasm32-wasip2/release/my_extension.wasm
```

确认所有 import 和 export 都正确。

### 检查 JSON payload

如果 `set-panel-view` 返回错误，可以用 `@host.log` 打印 JSON 内容，或用在线 JSON 工具验证格式。

---

## 完整示例：Pomodoro 番茄钟

参考项目中的 `examples-extensions/pomodoro/` 目录，它演示了：

- 定时器能力（`request_tick` / `cancel_tick` / `handle timer.tick`）
- 面板 UI 构建（Column、Row、Text、Heading、Button、Badge、Progress、Separator）
- UI 事件处理（按钮点击 → 状态变更 → UI 刷新）
- 命令注册与执行
- 全局可变状态管理

构建与运行：

```bash
cd examples-extensions/pomodoro
./build.sh
cargo run -p vellum
```
