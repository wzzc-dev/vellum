# Vellum

[English](./README.md)

Vellum 是一个基于 Rust 和 `gpui` 的纯 Markdown 所见即所得桌面编辑器。
项目方向是专注的 Typora 式写作体验：本地 Markdown 文件、实时编辑、安静的桌面 UI，
不再包含插件/扩展系统。

## 当前功能

- Markdown 所见即所得编辑
- 打开 Markdown 文件或文件夹
- 单窗口多标签编辑
- Live Preview 与源码模式切换
- 标题、段落、引用、列表、任务列表、代码块、表格、分隔线、链接、图片、数学块
- Markdown 与常见代码块语言的语法高亮
- 大纲侧边栏和工作区文件树
- 查找与替换
- 命令面板
- 自动保存
- 监听文件外部变更、删除、重命名
- 冲突提示与处理
- 启动时恢复上次打开的文件

## 技术栈

- [Rust](https://www.rust-lang.org/)
- [gpui](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [gpui-components](https://github.com/longbridge/gpui-component)
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/)
- [ropey](https://github.com/cessen/ropey)

## 项目结构

- `crates/vellum`：应用入口、窗口布局、菜单、标签页、文件操作
- `crates/editor`：编辑器核心、Markdown 投影、交互、自动保存、冲突处理
- `crates/workspace`：工作区树和文件监听

## 运行

```bash
cargo run
```

## 测试

```bash
cargo check
cargo test -p editor
cargo test -p workspace
cargo test --workspace
```

## 说明

- 侧边栏目前只显示 `.md`、`.markdown`、`.mdown`
- `Enter` 会对段落、列表、引用等块执行语义化换行
- 代码块保持普通多行编辑
- 当前是单窗口、多标签页模型
- Vellum 现在是纯 Markdown 编辑器，插件/扩展支持已经移除
