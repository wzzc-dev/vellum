# 构建与运行指南

本指南详细介绍如何构建和运行 Vellum 项目。

---

## 目录

1. [系统要求](#系统要求)
2. [安装依赖](#安装依赖)
3. [获取代码](#获取代码)
4. [构建项目](#构建项目)
5. [运行项目](#运行项目)
6. [构建示例扩展](#构建示例扩展)
7. [常见问题](#常见问题)

---

## 系统要求

### 操作系统

| OS | 支持 | 注意 |
|----|------|------|
| Linux | ✅ | 需要 GTK+ 3 开发库 |
| macOS | ✅ | 需要 Xcode 命令行工具 |
| Windows | ✅ | 需要 MSVC 工具链 |

### 硬件要求

- RAM：推荐 4GB 或以上
- 磁盘：至少 500MB 可用空间

---

## 安装依赖

### 1. 安装 Rust

访问 https://www.rust-lang.org/tools/install 获取最新安装说明。

或者使用以下命令（适用于类 Unix 系统）：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安装完成后，配置环境变量：

```bash
source $HOME/.cargo/env
```

验证安装：

```bash
rustc --version
cargo --version
```

### 2. 安装系统依赖（Linux）

在 Linux 上，需要安装 GTK+ 3 开发库：

#### Debian / Ubuntu / Linux Mint

```bash
sudo apt-get update
sudo apt-get install -y \
    libgtk-3-dev \
    libglib2.0-dev \
    libgdk-pixbuf2.0-dev \
    libpango1.0-dev \
    libcairo2-dev \
    libatk1.0-dev \
    libfontconfig1-dev \
    libx11-dev \
    libxkbcommon-dev \
    pkg-config
```

#### Fedora / CentOS / RHEL

```bash
sudo dnf install -y \
    gtk3-devel \
    glib2-devel \
    gdk-pixbuf2-devel \
    pango-devel \
    cairo-devel \
    atk-devel \
    fontconfig-devel \
    libX11-devel \
    libxkbcommon-devel \
    pkg-config
```

#### Arch Linux

```bash
sudo pacman -S \
    gtk3 \
    glib2 \
    gdk-pixbuf2 \
    pango \
    cairo \
    atk \
    fontconfig \
    libx11 \
    libxkbcommon \
    pkgconf
```

### 3. 安装系统依赖（macOS）

macOS 上需要安装 Xcode 命令行工具：

```bash
xcode-select --install
```

如果使用 Homebrew，可以安装一些额外工具：

```bash
brew install pkg-config fontconfig
```

### 4. 安装系统依赖（Windows）

在 Windows 上，需要安装：

- Visual Studio 2022 (或 Build Tools)
- "Desktop development with C++" 工作负载

下载链接：https://visualstudio.microsoft.com/downloads/

### 5. 安装 MoonBit（可选，仅扩展开发需要）

如果要开发 MoonBit 扩展，需要安装 MoonBit 工具链：

访问 https://www.moonbitlang.com/download 获取最新安装说明。

验证安装：

```bash
moon version
```

### 6. 安装辅助工具（可选）

如果要开发扩展，还需要安装以下工具：

```bash
cargo install wit-bindgen-cli
cargo install wasm-tools
```

---

## 获取代码

克隆 Vellum 仓库：

```bash
git clone https://github.com/your-org/vellum.git
cd vellum
```

或者下载 ZIP 压缩包并解压。

---

## 构建项目

### 开发构建（快速，未优化）

```bash
cargo build
```

### 生产构建（优化，较慢）

```bash
cargo build --release
```

### 检查代码（不进行完整构建）

```bash
cargo check
```

### 运行测试

```bash
cargo test
cargo test -p editor
cargo test -p workspace
```

---

## 运行项目

### 开发模式

```bash
cargo run
```

### 生产模式

```bash
cargo run --release
```

### 指定二进制

```bash
cargo run -p vellum
```

### 传递参数

```bash
cargo run -- --help
```

---

## 构建示例扩展

### 1. 构建 Pomodoro 扩展

```bash
cd examples-extensions/pomodoro
./build.sh
```

### 2. 构建 MoonBit GUI 扩展

```bash
cd examples-extensions/moonbit-gui
./build.sh
```

### 3. 构建所有扩展

目前没有统一的脚本，你可以逐个构建或者自己写个脚本。

### 注意

如果构建扩展时出现错误，请检查：

1. MoonBit 工具链是否安装正确
2. `wit-bindgen-cli` 和 `wasm-tools` 是否安装
3. 运行 `cargo install wit-bindgen-cli wasm-tools`
4. 查看错误提示

---

## 工作流程建议

### 日常开发

```bash
# 1. 修改代码
vim ...

# 2. 检查
cargo check

# 3. 运行测试
cargo test

# 4. 运行应用
cargo run
```

### 扩展开发

```bash
# 1. 修改扩展代码
cd examples-extensions/moonbit-gui
vim gen/world/extensionWorld/...

# 2. 构建扩展
./build.sh

# 3. 在另一个终端运行应用（根目录）
cd ../../
cargo run
```

---

## 常见问题

### Q: 构建时提示找不到 `glib-2.0` 等库

**A**：需要安装系统依赖。请参考 [安装系统依赖（Linux）](#2-安装系统依赖linux) 一节。

### Q: 可以在没有图形环境的机器上构建吗？

**A**：可以！只需要安装头文件和库即可。如果不需要运行 UI，可以只构建核心库：

```bash
cargo check -p extension
cargo check -p gpui-adapter
```

### Q: 如何知道是否有图形环境？

**A**：在 Linux 上，检查 `DISPLAY` 环境变量。如果为空，表示没有图形环境。

### Q: 构建扩展时提示 `moon: command not found`

**A**：需要安装 MoonBit 工具链。请参考 [安装 MoonBit](#5-安装-moonbit可选仅扩展开发需要) 一节。

### Q: `cargo run` 启动非常慢

**A**：首次构建比较慢（需要下载和编译所有依赖）。之后运行会快很多。如果要更快的启动，可以尝试：

```bash
# 只运行应用程序而不重新编译（如果代码没变更）
cargo run

# 或者直接运行编译好的二进制
./target/debug/vellum  # 开发构建
./target/release/vellum  # 生产构建
```

### Q: 如何清理构建产物？

**A**：

```bash
cargo clean
```

这将删除整个 `target/` 目录。

### Q: 如何查看详细构建日志？

**A**：

```bash
cargo build -v  # 详细输出
cargo build -vv # 非常详细
```

### Q: 我的扩展没有显示在应用中？

**A**：可能的原因：
1. `extension.toml` 中的 `component` 路径不对
2. 扩展没有正确构建
3. 扩展缺少 `capabilities` 配置

检查：
1. `target/wasm32-wasip2/release/` 目录下是否有 WASM 文件
2. `extension.toml` 中的路径是否正确
3. `capabilities.panels` 和 `contributes.panels` 是否正确配置

---

## 下一步学习

- 阅读 [architecture.md](./architecture.md) 了解项目架构
- 阅读 [gui-framework-guide.md](./gui-framework-guide.md) 学习 GUI 框架
- 阅读 [moonbit-extension-guide.md](./moonbit-extension-guide.md) 学习扩展开发
- 查看 [examples-extensions/](../examples-extensions/) 中的示例

---

## 获取帮助

如果你在构建或运行过程中遇到问题：

1. 查看本文档的 [常见问题](#常见问题)
2. 查看项目的 GitHub Issues
3. 创建新的 Issue 并提供详细信息

---

## 许可证

与 Vellum 项目保持一致。
