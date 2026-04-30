# Vellum GUI Framework 使用指南

## 目录

1. [概述](#概述)
2. [架构设计](#架构设计)
3. [环境配置](#环境配置)
4. [MoonBit SDK 快速入门](#moonbit-sdk-快速入门)
5. [Rust 适配器 API](#rust-适配器-api)
6. [WIT 接口参考](#wit-接口参考)
7. [组件开发](#组件开发)
8. [事件系统](#事件系统)
9. [绘制系统](#绘制系统)

---

## 概述

Vellum GUI Framework 是一个完整的跨平台 GUI 框架，结合了：

- **MoonBit**：用于应用逻辑和 UI 描述 DSL
- **Rust + GPUI**：负责原生渲染、窗口管理、事件处理
- **WASM Component Model (WIT)**：提供类型安全的跨语言通信

这种架构类似于 Flutter（Dart + C++ Skia），但使用 MoonBit 作为前端语言，具有更好的性能和类型安全。

---

## 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────────────┐
│  MoonBit Application (WASM Component)                    │
│  ┌───────────────────────────────────────────────────┐  │
│  │ Vellum GUI SDK                                    │  │
│  │  - 声明式 UI DSL                                 │  │
│  │  - 组件系统 (Button, Text, Input, ...)          │  │
│  │  - 布局系统 (Flex, Grid, Stack)                 │  │
│  │  - 事件处理 API                                  │  │
│  └──────────────────────┬────────────────────────────┘  │
└──────────────────────────┼───────────────────────────────┘
                           │ WIT Interface
                           │
┌──────────────────────────▼───────────────────────────────┐
│  GPUI Adapter (Rust)                                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │ GpuiBridge                                        │  │
│  │  - Window Manager (创建/管理窗口)                │  │
│  │  - Widget Manager (管理组件树)                   │  │
│  │  - Event Dispatcher (事件分发)                  │  │
│  │  - Paint State (绘制状态)                       │  │
│  └──────────────────────┬────────────────────────────┘  │
└──────────────────────────┼───────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────┐
│  GPUI (原生渲染)                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │ 窗口系统、布局引擎、渲染管线                      │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 核心模块

1. **WIT 接口定义**：`vellum-gui.wit`
2. **GPUI 适配器**：`gpui-adapter`
3. **MoonBit GUI SDK**：`moonbit/vellum-gui-sdk`
4. **扩展系统**：`vellum-extension`

---

## 环境配置

### 系统依赖

#### Linux

```bash
apt-get update && apt-get install -y \
    libfontconfig1-dev \
    libx11-dev \
    libxkbcommon-dev \
    libxext-dev \
    libxcb1-dev \
    libx11-xcb-dev \
    libglib2.0-dev \
    libwayland-dev \
    libxkbcommon-x11-dev \
    libdbus-1-dev \
    pkg-config
```

#### macOS

```bash
brew install fontconfig pkg-config
```

#### Windows

Windows 用户通常不需要特别的系统依赖，通过 MSVC 即可。

### 开发工具

| 工具 | 最低版本 | 用途 | 安装方式 |
|------|---------|------|---------|
| Rust | 1.75+ | 编译 GPUI 适配器 | [rustup.rs](https://rustup.rs/) |
| MoonBit | 0.1.x | 编写应用逻辑 | [MoonBit 官方](https://www.moonbitlang.com/download/) |
| wit-bindgen | 0.40 | 生成绑定代码 | `cargo install wit-bindgen-cli` |
| wasm-tools | 1.238 | 构建 WASM Component | `cargo install wasm-tools` |

验证安装：

```bash
rustc --version
moon version
wit-bindgen --version
wasm-tools --version
```

---

## MoonBit SDK 快速入门

### 创建第一个应用

1. 设置项目结构

```bash
mkdir my-first-gui-app && cd my-first-gui-app
```

2. 创建 `moon.mod.json`

```json
{
  "name": "myname/my-gui-app",
  "preferred-target": "wasm"
}
```

3. 链接 WIT 文件

```bash
ln -s /path/to/vellum/crates/extension/wit wit
```

4. 生成绑定代码

```bash
wit-bindgen moonbit wit/vellum-gui.wit \
    --world vellum-gui \
    --out-dir . \
    --derive-show --derive-eq
```

### 基础应用示例

创建 `main.mbt`：

```moonbit
// 导入 SDK
pub mod @gui/import
pub mod @gui/export

// 主函数
pub fn main() -> Unit {
  // 初始化应用
  let app_options = @gui/types.AppOptions {
    name: "My First App",
    version: "0.1.0",
    window_options: default_window_options()
  }

  let app = @gui/app.create(app_options)

  // 创建主窗口
  let window_id = @gui/window.create_window(default_window_options())

  // 创建 UI
  let root = @gui/container.column([
    @gui/text.text("Hello from Vellum GUI Framework!"),
    @gui/button.button("Click Me"),
  ])

  // 挂载组件
  @gui/widget.mount_widget(root.id, "root")

  // 运行应用
  @gui/app.run(app)
}

fn default_window_options() -> @gui/types.WindowOptions {
  @gui/types.WindowOptions {
    title: "My App",
    width: 800U,
    height: 600U,
    resizable: true,
    ...
  }
}
```

---

## Rust 适配器 API

### GpuiBridge 初始化

```rust
use gpui_adapter::GpuiBridge;
use gpui_adapter::types::AppTheme;

// 使用默认主题（系统）
let bridge = GpuiBridge::new()?;

// 使用构建器自定义
let bridge = GpuiBridgeBuilder::new()
    .with_theme(AppTheme::Dark)
    .build()?;
```

### 窗口管理

```rust
use gpui_adapter::window::WindowOptions;
use gpui_adapter::types::{Size, Point};

// 创建窗口
let window_id = bridge.create_window(WindowOptions {
    title: "Main Window".into(),
    width: 800,
    height: 600,
    resizable: true,
    ..Default::default()
});

// 关闭窗口
bridge.close_window(window_id)?;

// 设置标题
bridge.set_window_title(window_id, "New Title".into())?;
```

### 组件管理

```rust
// 创建组件
let widget_id = bridge.create_widget("column");

// 挂载组件
bridge.mount_widget(&widget_id, "root".into())?;

// 设置属性
bridge.set_widget_size(&widget_id, 200.0, 100.0)?;
bridge.set_widget_background(&widget_id, Color::new(1.0, 0.0, 0.0, 1.0))?;
```

---

## WIT 接口参考

### 核心类型

```wit
// 颜色 (RGBA, 0.0~1.0)
record color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

// 点坐标
record point {
    x: f32,
    y: f32,
}

// 尺寸
record size {
    width: f32,
    height: f32,
}

// 矩形区域
record rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
```

### 窗口接口

```wit
interface window {
    create-window: func(options: window-options) -> option<u32>;
    close-window: func(id: u32);
    set-title: func(id: u32, title: string);
    set-size: func(id: u32, width: u32, height: u32);
    minimize: func(id: u32);
    maximize: func(id: u32);
}
```

### 组件接口

```wit
interface widget {
    create-widget: func(type: string) -> widget;
    destroy-widget: func(id: string);
    mount-widget: func(id: string, parent-id: string);
    unmount-widget: func(id: string);
    
    // 布局属性
    set-widget-layout: func(id: string, layout: widget-layout);
    set-widget-size: func(id: string, width: f32, height: f32);
    set-widget-position: func(id: string, x: f32, y: f32);
    
    // 视觉属性
    set-widget-background: func(id: string, color: color);
    set-widget-opacity: func(id: string, opacity: f32);
    set-widget-visibility: func(id: string, visibility: visibility);
}
```

### 事件接口

```wit
interface event {
    subscribe: func(widget-id: string, event-types: list<string>);
    unsubscribe: func(widget-id: string);
    
    record mouse-event {
        kind: mouse-event-kind,
        button: mouse-button,
        position: point,
        global-position: point,
        delta: point,
        modifiers: key-modifiers,
    }
    
    record key-event {
        kind: key-event-kind,
        code: key-code,
        key: string,
        modifiers: key-modifiers,
    }
}
```

### 绘制接口

```wit
interface paint {
    create-canvas: func(width: f32, height: f32) -> canvas;
    
    draw-rect: func(canvas: canvas, rect: rect, paint: paint-style);
    draw-text: func(canvas: canvas, text: string, position: point, style: text-style);
    draw-image: func(canvas: canvas, image: image, dest: rect, source: option<rect>);
    
    save: func(canvas: canvas);
    restore: func(canvas: canvas);
    translate: func(canvas: canvas, x: f32, y: f32);
    scale: func(canvas: canvas, x: f32, y: f32);
    rotate: func(canvas: canvas, angle: f32);
}
```

---

## 组件开发

### 内置组件

#### 容器组件

```moonbit
// 垂直布局 (Column)
let column = @gui/container.column([
    widget1, widget2, widget3
])

// 水平布局 (Row)
let row = @gui/container.row([
    widgetA, widgetB, widgetC
])

// 堆叠布局 (Stack)
let stack = @gui/container.stack([
    background, foreground
])

// 弹性布局容器
let expanded = @gui/container.expanded(child_widget)
```

#### 文本组件

```moonbit
// 普通文本
let text = @gui/text.text("Hello World!")

// 标题 (level: 1~6)
let heading = @gui/text.heading("Title", 2)

// 段落
let paragraph = @gui/text.paragraph("Long text content")

// 富文本 (带样式)
let rich_text = @gui/text.rich_text(spans)
```

#### 按钮组件

```moonbit
// 普通按钮
let button = @gui/button.button("Click Me")
let icon_btn = @gui/button.icon_button("save")
let text_btn = @gui/button.text_button("Okay")
```

#### 输入组件

```moonbit
// 文本输入
let input = @gui/input.text_input("name", "Enter your name...")

// 复选框
let checkbox = @gui/input.checkbox("agree", "I agree", false)

// 开关
let toggle = @gui/input.toggle("enable", "Dark Mode", false)

// 下拉选择
let select = @gui/input.select("color", ["Red", "Green", "Blue"])

// 滑块
let slider = @gui/input.slider("volume", 0.5, 0.0, 1.0)

// 进度条
let progress = @gui/input.progress_bar(0.75)
```

### 自定义组件

创建自定义组件（MoonBit）：

```moonbit
pub struct MyButtonProps {
    label: String,
    on_click: () -> Unit
}

pub fn my_button(props: MyButtonProps) -> @gui/widget.Widget {
    let btn = @gui/button.button(props.label)
    
    // 设置点击事件
    btn.on_click(props.on_click)
    
    btn
}

// 使用自定义组件
let btn = my_button(MyButtonProps {
    label: "Custom Button",
    on_click: fn() {
        @gui/dialog.alert("Clicked!", "")
    }
})
```

### 布局系统

#### Flexbox 布局

```moonbit
let container = @gui/container.column([
    child1, child2, child3
])
.with_layout(@gui/types.WidgetLayout {
    flex_direction: @gui/types.FlexDirection::Column,
    justify_content: @gui/types.Alignment::Center,
    align_items: @gui/types.CrossAlignment::Center,
    gap: 16.0,
    padding: @gui/types.EdgeInsets::all(24.0),
    ...
})
```

#### 布局修饰符

```moonbit
widget
.with_layout(layout)
.with_padding(@gui/layout.padding(16.0))
.with_margin(@gui/layout.margin(8.0))
.with_width(200.0)
.with_height(100.0)
.with_flex_grow(1.0)
```

### 样式系统

```moonbit
widget
.with_background(@gui/types.Color::from_hex("#ffffff"))
.with_foreground(@gui/types.Color::black())
.with_border(1.0, @gui/types.Color::gray())
.with_border_radius(8.0)
.with_shadow(0.0, 2.0, 10.0, @gui/types.Color::from_hex("#00000030"))
.with_opacity(0.8)
.with_cursor(@gui/types.CursorShape::Pointer)
```

---

## 事件系统

### 事件订阅

```moonbit
use @gui/event.*

// 订阅点击事件
widget
.on_click(fn(event: MouseEvent) {
    // 处理点击
})

// 订阅输入变化
input
.on_input(fn(value: String) {
    // 处理输入
})

// 订阅键盘事件
widget
.on_key_down(fn(event: KeyEvent) {
    if event.code == KeyCode::Enter {
        // Enter 键按下
    }
})
```

### 事件类型

| 事件 | 说明 |
|------|------|
| `on_click` | 鼠标点击 |
| `on_mouse_down` | 鼠标按下 |
| `on_mouse_up` | 鼠标释放 |
| `on_mouse_move` | 鼠标移动 |
| `on_mouse_enter` | 鼠标进入 |
| `on_mouse_leave` | 鼠标离开 |
| `on_key_down` | 键盘按下 |
| `on_key_up` | 键盘释放 |
| `on_input` | 输入变更 |
| `on_scroll` | 滚动事件 |

### 键盘码示例

```moonbit
fn on_key_event(event: KeyEvent) -> Unit {
    match event.code {
        KeyCode::Enter => { /* 回车键 */ }
        KeyCode::Escape => { /* 退出键 */ }
        KeyCode::Space => { /* 空格键 */ }
        KeyCode::Digit0..=KeyCode::Digit9 => { /* 数字键 */ }
        KeyCode::KeyA..=KeyCode::KeyZ => { /* 字母键 */ }
        KeyCode::F1..=KeyCode::F12 => { /* 功能键 */ }
        _ => ()
    }
}
```

---

## 绘制系统

### Canvas 绘制

```moonbit
// 创建 Canvas
let canvas = @gui/paint.create_canvas(800.0, 600.0)

// 保存状态
@gui/paint.save(canvas)

// 变换
@gui/paint.translate(canvas, 100.0, 100.0)
@gui/paint.scale(canvas, 2.0, 2.0)

// 绘制矩形
let rect = @gui/types.Rect::new(0.0, 0.0, 100.0, 50.0)
let paint = @gui/types.PaintStyle::fill(@gui/types.Color::red())
@gui/paint.draw_rect(canvas, rect, paint)

// 绘制文本
let text_style = @gui/types.TextStyle::default()
@gui/paint.draw_text(canvas, "Hello", @gui/types.Point::new(0.0, 0.0), text_style)

// 恢复状态
@gui/paint.restore(canvas)
```

### Path 绘制

```moonbit
// 创建 Path
let path = @gui/paint.create_path()

// 构建路径
@gui/paint.path_move_to(path, 0.0, 0.0)
@gui/paint.path_line_to(path, 100.0, 100.0)
@gui/paint.path_arc_to(path, @gui/types.Point::new(150.0, 50.0), 50.0, 0.0, 3.14)
@gui/paint.path_close(path)

// 绘制路径
@gui/paint.draw_path(canvas, path, paint_style)
```

### 图像操作

```moonbit
// 加载图像
let image = @gui/paint.load_image_from_bytes(bytes)

// 绘制图像
let dest = @gui/types.Rect::new(0.0, 0.0, 200.0, 150.0)
@gui/paint.draw_image(canvas, image, dest, None)

// 缩放绘制
@gui/paint.draw_image_resized(canvas, image, 0.0, 0.0, 200.0, 150.0)
```

---

## 完整示例

### 待办事项应用

```moonbit
// todo-app.mbt

pub struct TodoItem {
    id: String,
    text: String,
    completed: Bool
}

pub struct AppState {
    mut todos: List[TodoItem],
    mut new_todo_text: String
}

let state: AppState = {
    todos: List::empty(),
    new_todo_text: ""
}

pub fn main() -> Unit {
    let app = @gui/app.create(@gui/types.AppOptions {
        name: "Todo App",
        version: "1.0.0",
        window_options: @gui/types.WindowOptions {
            title: "Todo List",
            width: 400U,
            height: 600U,
            resizable: true,
            ...
        }
    })

    let window_id = @gui/window.create_window(app.window_options)

    let ui = render_ui()
    @gui/widget.mount_widget(ui.id, "root")

    @gui/app.run(app)
}

fn render_ui() -> @gui/widget.Widget {
    @gui/container.column([
        @gui/text.heading("My Todos", 1),

        // 输入区域
        @gui/container.row([
            @gui/input.text_input("new-todo", "What needs to be done?")
                .with_value(state.new_todo_text)
                .on_input(fn(val) {
                    state.new_todo_text = val
                }),
            @gui/button.button("Add")
                .on_click(fn() {
                    add_todo()
                })
        ]).with_gap(8.0).with_padding(@gui/layout.padding(16.0)),

        // 分隔线
        @gui/ui.separator(),

        // 列表
        render_todo_list()
    ])
    .with_padding(@gui/layout.padding(24.0))
    .with_gap(16.0)
}

fn render_todo_list() -> @gui/widget.Widget {
    if state.todos.is_empty() {
        @gui/text.text("No todos yet. Add one above!")
    } else {
        @gui/scroll.scroll_view(
            @gui/container.column(
                state.todos.map(fn(todo) {
                    render_todo_item(todo)
                })
            ).with_gap(8.0)
        )
    }
}

fn render_todo_item(todo: TodoItem) -> @gui/widget.Widget {
    @gui/container.row([
        @gui/input.checkbox(todo.id, todo.text, todo.completed)
            .on_checked(fn(checked) {
                toggle_todo(todo.id, checked)
            }),
        @gui/container.expanded(@gui/ui.spacer()),
        @gui/button.icon_button("delete")
            .on_click(fn() {
                remove_todo(todo.id)
            })
    ]).with_gap(8.0)
}

fn add_todo() -> Unit {
    if state.new_todo_text.trim().is_empty() {
        return
    }

    let todo = TodoItem {
        id: @uuid.generate(),
        text: state.new_todo_text,
        completed: false
    }

    state.todos = state.todos.append(todo)
    state.new_todo_text = ""
    refresh_ui()
}

fn toggle_todo(id: String, completed: Bool) -> Unit {
    state.todos = state.todos.map(fn(todo) {
        if todo.id == id {
            todo.with_completed(completed)
        } else {
            todo
        }
    })
    refresh_ui()
}

fn remove_todo(id: String) -> Unit {
    state.todos = state.todos.filter(fn(todo) {
        todo.id != id
    })
    refresh_ui()
}

fn refresh_ui() -> Unit {
    // 更新 UI
    let ui = render_ui()
    @gui/widget.unmount_widget("root")
    @gui/widget.mount_widget(ui.id, "root")
}
```

---

## 调试技巧

### 日志输出

```moonbit
use @gui/debug.*

// 打印日志
log("Debug message")

// 打印组件树
print_widget_tree(root_widget)

// 检查组件
let widget_info = inspect_widget(widget_id)

// 性能测量
let duration = measure_performance(fn() {
    // 执行耗时操作
})
```

### 验证 WASM Component

```bash
# 检查组件接口
wasm-tools component wit target/my-component.wasm

# 验证组件
wasm-tools validate target/my-component.wasm
```

---

## 参考资料

- [GPUI 官方文档](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [MoonBit 语言](https://www.moonbitlang.com/)
- [WASM Component Model](https://component-model.bytecodealliance.org/)
- [WIT 语言参考](https://component-model.bytecodealliance.org/design/wit.html)

---

## 许可证

Vellum GUI Framework 遵循与 Vellum 项目相同的许可证。
