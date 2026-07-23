# SmartEx

一个用 Rust 写的跨平台压缩工具。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blueviolet.svg)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-ff3e00.svg)](https://svelte.dev/)
[![Version](https://img.shields.io/badge/Version-0.6.0-green.svg)](https://github.com/1764712542/smart_ex/releases)

## 这是什么

SmartEx 是一个压缩/解压/加密工具，支持 ZIP、7z、RAR、TAR 系列等 14 种格式。它有两个不常见的设计：

1. **上下文感知**——压缩前问几个问题（发给谁、怎么传、对方用什么系统），自动选合适的格式和参数。不是所有人都知道发邮件该用 zip 而不是 7z。
2. **全层自定义**——主题、布局、功能模块、工作流都能调。不需要的功能可以关掉，常用的操作可以编排成工作流一键执行。

## 下载

从 [Releases](https://github.com/1764712542/smart_ex/releases) 下载：

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | `SmartEx_0.6.0_aarch64.dmg` |
| Linux | `smartex_0.6.0_amd64.deb` |
| Windows | `SmartEx_0.6.0_x64-setup.exe` |

macOS 安装后，双击 `.zip` `.7z` `.rar` 等文件会自动用 SmartEx 打开。

## 从源码构建

需要 Rust 1.75+、Node 18+、Xcode CLT（macOS）。

```bash
git clone https://github.com/1764712542/smart_ex.git
cd smart_ex

# 构建 GUI
cargo tauri build

# 开发模式
cargo tauri dev

# 只构建 CLI
cargo build --release
```

## 命令行用法

```bash
# 压缩
smart_ex compress -i ./folder -o archive.tar.zst -f tar.zst -l 3

# 加密压缩（ZIP AES-256，兼容 7-Zip/WinRAR）
smart_ex compress -i ./folder -o secret.zip -f zip --password MyPass123

# 排除文件 + 分卷
smart_ex compress -i ./project -o project.zip -f zip \
  --exclude "*.tmp" --exclude ".git" --split 100M

# 解压
smart_ex decompress -i archive.tar.zst -o ./output

# 加密（流式 AES-256-GCM，恒定 8MB 内存）
smart_ex encrypt -i large.iso -o large.enc --password MyPass123

# 解密
smart_ex decrypt -i large.enc -o large.iso --password MyPass123

# 浏览归档内容
smart_ex list -i archive.zip

# 测试归档完整性
smart_ex test -i archive.zip
```

## 格式支持

| 格式 | 压缩 | 解压 | 加密 |
|------|:----:|:----:|:----:|
| ZIP | ✅ | ✅ | ✅ AES-256 |
| 7z | ✅ | ✅ | ✅ AES-256 |
| RAR | ❌ | ✅ | ✅ |
| TAR / TAR.GZ / TAR.XZ / TAR.ZST / TAR.BZ2 / TAR.LZ4 | ✅ | ✅ | ❌ |
| GZ / XZ / ZST / BZ2 / LZ4（单文件流） | ✅ | ✅ | ❌ |
| .enc（原生加密） | ✅ | ✅ | ✅ AES-256-GCM |

加密格式兼容 7-Zip、WinRAR、Bandizip。.enc 是 SmartEx 自己的流式加密格式，用 Argon2id 派生密钥。

## 三个主要功能

### 1. 上下文感知压缩

点"智能推荐"，选四个选项：
- **收件人**：自己 / 同事 / 外部客户 / 公开下载
- **传输方式**：邮件 / 即时通讯 / 网盘 / U盘 / 本地
- **目标系统**：Windows / macOS / Linux / 手机 / 未知
- **优先级**：最小体积 / 最快速度 / 最大兼容 / 最高安全

系统给出推荐格式、级别、是否分卷、是否需要 UTF-8 文件名，并说明理由。比如选"邮件 + 外部客户"会推荐 zip + 25MB 分卷，因为邮件附件有大小限制且对方可能没有装 7-Zip。

### 2. 会话钥匙串

输过一次密码后，会话内自动复用。macOS 上可以存到系统 Keychain，跨会话也能用。默认 30 分钟过期。按模式分组存储——压缩用的密码不会和加密用的混在一起。

### 3. 流式加密

大文件加密不 OOM。4MB 一块，每块独立 AES-256-GCM 认证，内存恒定 8MB 左右。中断了可以从断点续传。

## 自定义

**外观**：6 个预设主题色 + 自定义色、深色/浅色/跟随系统、字体和字号、面板布局（左右/右左/上下）、快捷键自定义。配置可以导出成 JSON 文件，换台机器导入就行。

**功能**：压缩/解压/加密/解密四个模式可以单独开关，不需要的从顶部 tab 消失。智能推荐、钥匙串、分卷、排除等功能也可以单独关。

**工作流**：拖拽节点编排多步操作。比如"压缩 → 加密 → 删源文件"可以串成一条链，保存后一键执行，前一步的输出自动传给下一步。

## 项目结构

```
smart_ex/
├── crates/
│   ├── smartex-core/           # 核心库（压缩/解压/加密，不含 GUI）
│   └── smartex-tauri/          # Tauri 后端（IPC 命令）
├── src/                         # CLI
├── ui/                          # Svelte 5 前端
└── installer/                   # 安装脚本
```

核心库 `smartex-core` 是纯 Rust，不依赖任何 GUI 框架，可以单独引用。

## 技术栈

- **核心**：Rust 2021，zip / tar / zstd / xz2 / sevenz-rust / unrar / aes-gcm / argon2
- **GUI 后端**：Tauri 2
- **GUI 前端**：Svelte 5 + Vite + Tailwind CSS
- **CLI**：clap 4

## 测试

```bash
# 格式回归测试（19 项）
./test_regression.sh

# 核心库单元测试（15 项）
cargo test -p smartex-core --lib
```

## 跨平台

| 平台 | 状态 | 备注 |
|------|------|------|
| macOS | 完整支持 | Keychain 集成、原生红绿灯、.app / .dmg |
| Linux | 完整支持 | .deb / .AppImage |
| Windows | 完整支持 | .msi / .exe，文件关联 |

## 版本记录

**v0.6.0** — Tauri 2 + Svelte 5 重写 GUI，新增上下文感知压缩、会话钥匙串、流式加密、全层自定义系统

**v0.5.0** — 主题切换、安全删除、归档浏览、完整性测试、任务取消

**v0.4.0** — zstd 多线程、大缓冲区、ZIP 并行解压、炸弹检测

**v0.3.0** — RAR 解压、中英双语、AES-256 加密、跨平台安装器

**v0.2.0** — egui GUI、14 种格式

**v0.1.0** — 初始 CLI

## 许可证

MIT
