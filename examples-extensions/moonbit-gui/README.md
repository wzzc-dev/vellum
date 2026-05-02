# MoonBit GUI 扩展示例

这个示例扩展演示了如何使用 MoonBit 为 Vellum 编写一个带有 GUI 的扩展。

---

## 功能特性

- ✅ 简单的计数器功能
- ✅ 声明式 UI 构建
- ✅ 按钮交互处理
- ✅ 状态管理
- ✅ 状态栏消息

---

## 目录结构

```
moonbit-gui/
├── extension.toml                # 扩展配置
├── moon.mod.json                 # MoonBit 模块配置
├── build.sh                      # 构建脚本
├── wit/                          # WIT 目录（符号链接）
│   └── vellum-extension.wit
├── interface/                    # wit-bindgen 生成的接口
│   └── vellum/extension/
│       ├── types/
│       ├── host/
│       ├── editor/
│       ├── ui/
│       └── timer/
├── world/                        # World 绑定
│   └── extensionWorld/
└── gen/                          # 主要实现
    └── world/extensionWorld/
        ├── types.mbt            # 类型与状态
        ├── ui.mbt               # UI 构建函数
        ├── moonbit-gui.mbt      # 主要业务逻辑
        └── top.mbt              # 导出函数声明
```

---

## 核心实现详解

### 1. 类型与状态 (`types.mbt`)

```moonbit
pub(all) struct State {
  mut counter : Int
}

let state : State = { counter: 0 }
```

这里定义了一个简单的状态结构体，包含一个可变的计数器。

### 2. UI 构建 (`ui.mbt`)

提供了多个辅助函数：

- `ui_column()` — 垂直布局容器
- `ui_row()` — 水平布局容器
- `ui_text()` — 文本显示
- `ui_large_text()` — 大号文本
- `ui_heading()` — 标题
- `ui_button()` — 按钮
- `ui_separator()` — 分隔线
- `refresh_ui()` — 刷新 UI
- `build_panel_ui()` — 构建完整 UI 树

### 3. 业务逻辑 (`moonbit-gui.mbt`)

实现了以下功能：

| 函数 | 说明 |
|------|------|
| `handle_increment()` | 增加计数器 |
| `handle_decrement()` | 减少计数器 |
| `handle_reset()` | 重置计数器 |
| `activate()` | 扩展激活入口 |
| `deactivate()` | 扩展停用清理 |
| `handle_event()` | 事件处理 |
| `execute_command()` | 命令执行 |
| `handle_ui_event()` | UI 事件处理（关键） |
| `handle_hover()` | 悬停处理 |

---

## 交互流程

### 用户点击按钮

```
用户点击 "+" 按钮
   ↓
ExtensionHost 识别到 button.clicked 事件
   ↓
调用扩展的 handle_ui_event()，传递 element_id: "moonbit-gui-increment"
   ↓
handle_ui_event() 匹配到事件
   ↓
调用 handle_increment()
   ↓
state.counter 增加
   ↓
调用 @host.show_status_message() 显示提示
   ↓
调用 refresh_ui() 重新构建 UI
   ↓
调用 @ui.set_panel_view() 更新面板
   ↓
用户看到新的计数器值
```

---

## 配置文件详解

### `extension.toml`

```toml
id = "vellum.moonbit-gui"          # 全局唯一扩展 ID
name = "MoonBit GUI"                # 显示名称
version = "0.1.0"                  # 版本
schema_version = 1                 # 扩展 schema 版本
authors = ["Vellum"]               # 作者
description = "A MoonBit GUI extension for creating declarative UIs"

[wasm]
# 构建输出的 WASM Component 路径
component = "../../target/wasm32-wasip2/release/vellum_moonbit_gui.wasm"

[activation]
events = []                         # 激活事件：空 = 总是激活

[capabilities]
panels = true                      # 支持面板 UI
commands = true                    # 支持命令注册

# 注册命令
[[contributes.commands]]
id = "moonbit-gui.demo"
title = "MoonBit: Demo"

# 注册面板
[[contributes.panels]]
id = "moonbit-gui"
title = "MoonBit GUI"
icon = "layout"                    # 图标：可用的图标由宿主定义
location = "right"                 # 位置："left" / "right"
```

### `moon.mod.json`

```json
{ "name": "vellum/moonbit-gui", "preferred-target": "wasm" }
```

指定 MoonBit 模块的名称和首选目标。

---

## 构建

### 前置要求

1. 安装 MoonBit 编译器（可在 https://www.moonbitlang.com 下载）
2. 安装 `wit-bindgen-cli`
   ```bash
   cargo install wit-bindgen-cli
   ```
3. 安装 `wasm-tools`
   ```bash
   cargo install wasm-tools
   ```

### 构建步骤

```bash
cd examples-extensions/moonbit-gui
./build.sh
```

构建脚本执行以下操作：

1. （可选）重新生成 MoonBit 绑定（通过 `wit-bindgen`）
2. 修正包路径
3. 使用 `moon` 编译 MoonBit 代码为 WASM
4. 嵌入 WIT 到 WASM（注意使用 `--encoding utf16`）
5. 创建 WASM Component

---

## 运行

```bash
# 在项目根目录
cargo run -p vellum
```

启动应用后，你将：

1. 看到左侧/右侧边栏
2. 找到 "MoonBit GUI" 面板
3. 看到计数器界面
4. 点击 +/-/Reset 按钮测试功能

---

## 从 pomodoro 扩展学习

这个扩展是在 pomodoro 扩展示例的基础上简化而来的。主要变更：

1. 去掉了复杂的定时器逻辑
2. 去掉了番茄钟状态管理
3. 只保留了简单的计数器
4. 保留了完整的 UI 构建模式

如果你想学习更复杂的例子，请查看：
- [pomodoro/](../pomodoro/) — 完整的番茄钟扩展示例

---

## 下一步

现在你已经理解了这个示例，接下来可以：

1. 修改 UI，添加更多组件（如 TextInput, Checkbox, Select）
2. 实现更复杂的状态管理
3. 集成更多的宿主 API（如文档操作）
4. 添加图标、样式
5. 使用定时器功能

完整的开发指南请参考：
- [docs/moonbit-extension-guide.md](../../docs/moonbit-extension-guide.md)
- [docs/gui-framework-guide.md](../../docs/gui-framework-guide.md)

---

## 常见问题

### Q: 如何添加新的 UI 元素？

A: 查看 `ui.mbt` 中的现有模式，创建新的辅助函数，然后在 `build_panel_ui()` 中使用。

### Q: 如何调试扩展？

A: 使用 `@host.log()` 输出日志，或在 Rust 侧设置断点。

### Q: WIT 文件在哪里？

A: 在 `crates/extension/wit/vellum-extension.wit`，pomodoro 和 moonbit-gui 都是符号链接到这个文件。

---

## 许可证

与 Vellum 项目保持一致。
