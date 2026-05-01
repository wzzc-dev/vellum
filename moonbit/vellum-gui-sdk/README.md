# Vellum GUI Framework

Vellum 是一个受 SwiftUI 和 React 启发的 MoonBit 声明式 UI 框架。

## 特性

- **响应式状态管理
- 声明式 UI 描述
- 组件化设计
- 事件系统

## 快速开始

```moonbit
fn main() {
  let state = State::new("Hello Vellum!")
  
  Column::new([
    Text::new(state.get()),
    Button::new("Click Me")
      .on_click(fn() {
        state.set("Updated!")
      })
  ]).padding(24.0)
}
```

## 组件

### 容器

## 许可

MIT
