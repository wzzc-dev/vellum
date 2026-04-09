# Vellum

Vellum 是一个基于 Rust 和 `gpui` 的桌面 Markdown 编辑器。
## 当前功能

- 打开 Markdown 文件或文件夹
- 块级预览与编辑切换
- 自动保存
- 工作区侧边栏
- 监听文件外部变更、删除、重命名
- 冲突提示与处理
- 启动时恢复上次打开的文件

## 技术栈

- [Rust](https://www.rust-lang.org/)
- [gpui](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [gpui-components](https://github.com/longbridge/gpui-component)


## 项目结构

- `crates/vellum`：应用入口、窗口布局、菜单、文件操作
- `crates/editor`：编辑器核心、块解析、交互、自动保存、冲突处理
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
```

## 说明

- 侧边栏目前只显示 `.md`、`.markdown`、`.mdown`
- `Enter` 会对段落、列表、引用等块执行语义化换行
- 代码块保持普通多行编辑
- 当前是单窗口、单文档模型
