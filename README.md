# ⚡ smart_ex

**智能压缩 · 加密 · 解压 — 跨平台高速压缩工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)](https://github.com/smartex/smart_ex)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Version-0.5.0-green.svg)](https://github.com/smartex/smart_ex/releases)

smart_ex 是一个用 Rust 编写的跨平台压缩/解压/加密工具，支持 14+ 种压缩格式，内置优美的深色玻璃拟态 GUI，兼容 7-Zip / WinRAR / Bandizip 的加密格式，免费开源。

## 🚀 v0.5.0 全面升级

- **明暗主题切换** — 顶栏一键切换 ☀️ 浅色 / 🌙 深色主题
- **安全删除源文件** — 压缩/加密后可选安全删除源文件（3 次覆写 + 删除）
- **归档内容浏览** — GUI 浮动窗口查看归档内文件列表、大小、压缩比
- **完整性测试** — 一键检测归档是否损坏（支持 zip/7z/rar/tar 系列/单文件流）
- **任务取消** — 工作中可随时取消，协作式取消机制
- **进度详情** — 实时显示已处理/总字节数
- **密码生成器** — 🎲 一键生成强密码（大小写+数字+符号混合）
- **密码可见性切换** — 👁/🙈 显示/隐藏密码
- **Toast 通知** — 操作完成/失败顶部弹出通知
- **最近文件** — 下拉菜单快速选择最近使用过的文件
- **拖放支持** — 直接拖文件到窗口打开
- **键盘快捷键** — Ctrl+Enter 开始 / Esc 取消 / Ctrl+L 清空日志
- **文件排除规则** — 通配符排除（`*.tmp, *.log, .git`）
- **分卷压缩** — 支持 K/M/G/B 后缀（`100M, 1G, 700K, 512B`）
- **🐛 修复 7z 单文件压缩** — 修复 `push_source_path` 对单文件生成空条目名导致解压失败的问题

## 🚀 v0.4.0 性能优化

- **zstd 多线程编码** — 利用全部 CPU 核心并行压缩，大文件速度提升 4-8 倍
- **1MB 大缓冲区** — 所有压缩/解压路径从默认 8KB 提升至 1MB，IO 效率提升 10 倍+
- **ZIP 多线程并行解压** — rayon 分块并行处理 ZIP 条目，多 worker 独立打开 ZipArchive
- **压缩包炸弹检测** — 解压时监控总数据量，超过 100x 比例或 10GB 绝对限制时自动终止

## ✨ 特性

### 🗜️ 全格式支持

| 格式 | 压缩 | 解压 | 加密 | 备注 |
|------|:----:|:----:|:----:|------|
| ZIP | ✅ | ✅ | ✅ AES-256 | Deflate / Zstd / Bzip2 / LZMA |
| 7z | ✅ | ✅ | ✅ AES-256 | LZMA2 |
| RAR | ❌ | ✅ | ✅ | 闭源格式，仅解压 (RAR3/RAR5) |
| TAR | ✅ | ✅ | ❌ | 无压缩 |
| TAR.GZ | ✅ | ✅ | ❌ | Gzip |
| TAR.XZ | ✅ | ✅ | ❌ | LZMA |
| TAR.ZST | ✅ | ✅ | ❌ | Zstandard ⚡ |
| TAR.BZ2 | ✅ | ✅ | ❌ | BZip2 |
| TAR.LZ4 | ✅ | ✅ | ❌ | LZ4 ⚡⚡ |
| GZ | ✅ | ✅ | ❌ | 单文件流 |
| XZ | ✅ | ✅ | ❌ | 单文件流 |
| ZST | ✅ | ✅ | ❌ | 单文件流 |
| BZ2 | ✅ | ✅ | ❌ | 单文件流 |
| LZ4 | ✅ | ✅ | ❌ | 单文件流 |
| .enc | ✅ | ✅ | ✅ AES-256-GCM | smart_ex 原生加密格式 |

### 🔐 加密兼容性

- **ZIP AES-256**：兼容 7-Zip、WinRAR、Bandizip
- **7z AES-256**：兼容 7-Zip、WinRAR
- **RAR 加密**：支持 RAR3/RAR5 加密解压
- **.enc 原生加密**：AES-256-GCM + Argon2id 密钥派生

### 🎨 GUI 特性

- 深色玻璃拟态 / 浅色主题一键切换 (☀️/🌙)
- 中英双语实时切换
- 压缩 / 解压 / 加密 / 解密 四大模式
- 解压到当前文件夹 / 解压另存为 快捷操作
- 归档内容浏览 (浮动窗口, 文件列表 + 压缩比统计)
- 完整性测试 (一键检测归档是否损坏)
- 实时进度条 + 字节详情 + 日志面板
- 任务取消 (协作式取消机制)
- 密码生成器 (🎲 强密码) + 密码可见性切换 (👁/🙈)
- Toast 通知 (操作完成/失败提示)
- 最近文件下拉菜单
- 拖放文件支持
- 键盘快捷键 (Ctrl+Enter / Esc / Ctrl+L)
- 文件排除规则 + 分卷压缩
- 安全删除源文件 (3 次覆写)
- 智能输出路径自动匹配

### 🖱️ 系统集成

- **右键菜单**：解压到当前文件夹 / 解压另存为...
- **文件关联**：双击压缩文件直接用 smart_ex 打开
- **跨平台**：macOS / Linux / Windows 全支持

## 📦 安装

### 方式一：下载安装包（推荐）

从 [Releases](https://github.com/smartex/smart_ex/releases) 下载对应平台的安装包：

| 平台 | 安装包 | 安装方式 |
|------|--------|----------|
| macOS | `smart_ex-0.5.0.pkg` | 双击安装，或 `sudo installer -pkg smart_ex-0.5.0.pkg -target /` |
| Linux | `smart_ex_0.5.0_amd64.deb` | `sudo dpkg -i smart_ex_0.5.0_amd64.deb` |
| Windows | `smart_ex-0.5.0-windows-x64-setup.exe` | 双击运行安装向导 |

#### Windows 安装选项

安装时可选择绑定压缩文件格式（全选或单选）：
- ✅ 勾选「绑定压缩文件格式」→ 自动关联所有格式
- ✅ 右键菜单自动添加「解压到当前文件夹」「解压另存为...」
- ✅ 可自定义安装路径

#### macOS 权限说明

首次访问「下载」「桌面」「文稿」目录时，macOS 会弹出权限授权对话框，请点击「允许」。

如需手动授权：**系统设置 → 隐私与安全性 → 完全磁盘访问权限 → 添加 smart_ex**

### 方式二：从源码编译

```bash
# 克隆仓库
git clone https://github.com/smartex/smart_ex.git
cd smart_ex

# 编译 release 版本
cargo build --release

# 运行
./target/release/smart_ex          # 启动 GUI
./target/release/smart_ex --help   # 查看 CLI 帮助
```

### 方式三：使用安装脚本（macOS/Linux）

```bash
# macOS / Linux 交互式安装
./installer/install.sh

# 全选格式绑定，安装到默认路径
./installer/install.sh --all

# 指定安装路径
./installer/install.sh --path /opt/smart_ex
```

## 🚀 使用方法

### GUI 模式

```bash
# 直接启动 GUI（无参数默认启动）
smart_ex

# 或显式指定
smart_ex gui
```

### 命令行模式

```bash
# 压缩
smart_ex compress -i ./my_folder -o archive.tar.zst -f tar.zst -l 3

# 压缩 + 加密 (ZIP AES-256, 兼容 7-Zip/WinRAR)
smart_ex compress -i ./my_folder -o secret.zip -f zip -l 3 --password MyPass123

# 压缩 + 排除文件 (通配符, 可多次指定)
smart_ex compress -i ./project -o project.zip -f zip --exclude "*.tmp" --exclude "*.log" --exclude ".git"

# 压缩 + 分卷 (支持 K/M/G/B 后缀)
smart_ex compress -i ./large_file -o archive.zip -f zip --split 100M

# 解压
smart_ex decompress -i archive.tar.zst -o ./output

# 解压加密归档
smart_ex decompress -i secret.zip -o ./output --password MyPass123

# 浏览归档内容 (不解压)
smart_ex list -i archive.zip

# 测试归档完整性
smart_ex test -i archive.zip

# 解压到当前文件夹 (右键菜单)
smart_ex extract-here -i archive.zip

# 解压另存为 (右键菜单, 弹出目录选择器)
smart_ex extract-as -i archive.zip

# 解压另存为 (指定目录)
smart_ex extract-as -i archive.zip -o ./output

# 加密任意文件 (AES-256-GCM)
smart_ex encrypt -i secret.pdf -o secret.enc --password MyPass123

# 解密文件
smart_ex decrypt -i secret.enc -o secret.pdf --password MyPass123

# 智能模式 (自动判断压缩/解压/加密/解密)
smart_ex smart -i archive.zip --password MyPass123

# 切换语言
smart_ex --lang en gui    # English
smart_ex --lang zh gui    # 中文
```

### 压缩级别

| 级别 | 速度 | 压缩比 | 适用场景 |
|------|------|--------|----------|
| 0 | 最快 | 无压缩 (Stored) | 仅打包 |
| 1-3 | ⚡ 极快 | 较低 | 日常使用（推荐） |
| 4-6 | 快 | 中等 | 平衡 |
| 7-9 | 慢 | 较高 | 归档存储 |
| 10-12 | 最慢 | 最高 | 长期备份 |

## 🗑️ 卸载

### macOS / Linux

```bash
# 交互式卸载
smart_ex-uninstall

# 或从源码运行
./installer/uninstall.sh

# 自动确认
./installer/uninstall.sh --yes
```

### Windows

- **通过控制面板**：设置 → 应用 → smart_ex → 卸载
- **通过 PowerShell**：`.\installer\uninstall.ps1`

### Linux (apt)

```bash
sudo apt remove smart_ex
```

## 🏗️ 构建安装包

### macOS .pkg

```bash
cargo build --release
./installer/build_pkg.sh
# 生成: dist/smart_ex-0.5.0.pkg
```

### Linux .deb

```bash
cargo build --release
./installer/build_deb.sh amd64    # 或 arm64
# 生成: dist/deb-build/smart_ex_0.5.0_amd64.deb
```

### Windows .exe (Inno Setup)

```bash
cargo build --release
# 需安装 Inno Setup: https://jrsoftware.org/isdl.php
iscc installer\smart_ex.iss
# 生成: installer\output\smart_ex-0.5.0-windows-x64-setup.exe
```

## 📁 项目结构

```
smart_ex/
├── src/
│   ├── main.rs          # 入口 + 命令分发 (compress/decompress/list/test/encrypt/decrypt)
│   ├── cli.rs           # CLI 参数定义 (clap)
│   ├── compress.rs      # 压缩逻辑 (zip/7z/tar.*/单文件 + 排除 + 分卷)
│   ├── decompress.rs    # 解压逻辑 (含编码修复 + 炸弹检测 + 完整性测试)
│   ├── crypto.rs        # AES-256-GCM 加密/解密
│   ├── format.rs        # 格式检测与容器定义
│   ├── gui.rs           # eframe/egui GUI (主题切换 + 安全删除 + 归档浏览)
│   ├── i18n.rs          # 中英双语国际化
│   ├── archive_list.rs  # 归档内容浏览
│   ├── progress.rs      # 进度条 (4 参数回调)
│   └── rar.rs           # RAR 解压
├── installer/
│   ├── smart_ex.iss     # Windows Inno Setup 脚本
│   ├── Info.plist       # macOS 应用配置
│   ├── smart_ex.desktop # Linux 桌面文件
│   ├── install.sh       # macOS/Linux 安装脚本
│   ├── uninstall.sh     # macOS/Linux 卸载脚本
│   ├── uninstall.ps1    # Windows 卸载脚本
│   ├── build_pkg.sh     # macOS .pkg 构建
│   ├── build_deb.sh     # Linux .deb 构建
│   └── build_macos_app.sh # macOS .app bundle 构建
├── test_regression.sh   # 回归测试脚本
├── Cargo.toml
├── LICENSE
└── README.md
```

## 🧪 测试

```bash
# 全格式回归测试 (19 项)
./test_regression.sh
```

测试覆盖：
- 8 种容器格式（zip/7z/tar/tar.gz/tar.xz/tar.zst/tar.bz2/tar.lz4）
- 5 种单文件流（gz/xz/zst/bz2/lz4）
- 加密 ZIP (AES-256) / 加密 7z (AES-256) / .enc (AES-256-GCM)
- tar.zst + .enc 包装
- extract-here / extract-as
- 归档列表 (list) / 完整性测试 (test)
- 文件排除规则 / 分卷压缩

## 🔧 技术栈

| 组件 | 库 | 版本 |
|------|-----|------|
| GUI | eframe + egui | 0.29 |
| CLI | clap | 4.5 |
| ZIP | zip | 2.2 (aes-crypto) |
| 7z | sevenz-rust | 0.6 (aes256) |
| RAR | unrar | 0.5 (内置 unRAR) |
| 加密 | aes-gcm + argon2 | 0.10 / 0.5 |
| 编码 | encoding_rs | 0.8 (GBK/Shift-JIS) |
| 并行 | rayon | 1.10 |
| 文件对话框 | rfd | 0.15 |

## 🌍 跨平台支持

| 平台 | 状态 | 架构 |
|------|------|------|
| macOS | ✅ 完整支持 | arm64 (Apple Silicon) / x86_64 |
| Linux | ✅ 完整支持 | x86_64 / arm64 |
| Windows | ✅ 完整支持 | x86_64 |

## 📝 许可证

[MIT License](LICENSE) — 免费开源，可自由使用、修改、分发。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

## 📊 性能

smart_ex 使用 Zstandard 作为默认压缩算法，在速度和压缩比上远超传统 Deflate：

| 算法 | 压缩速度 | 解压速度 | 压缩比 |
|------|----------|----------|--------|
| Zstd (默认) | ⚡⚡⚡ 极快 | ⚡⚡⚡⚡ 极快 | 高 |
| LZ4 | ⚡⚡⚡⚡ 最快 | ⚡⚡⚡⚡⚡ 最快 | 中 |
| Deflate (ZIP) | ⚡⚡ 快 | ⚡⚡⚡ 快 | 中 |
| LZMA (7z/xz) | ⚡ 慢 | ⚡⚡ 快 | 最高 |
| BZip2 | ⚡ 慢 | ⚡⚡ 快 | 高 |

### v0.4.0 性能基准 (7.4MB 混合数据, 8 核 CPU)

| 格式 | 级别 | 压缩时间 | 压缩大小 | 解压时间 |
|------|:----:|:--------:|:--------:|:--------:|
| tar.zst | 3 | **0.05s** | 3666KB | **0.04s** |
| tar.zst | 9 | 0.12s | 3428KB | 0.04s |
| tar.zst | 12 | 1.76s | 3090KB | 0.04s |
| zip | 3 | 0.16s | 3729KB | 0.05s |
| zip | 9 | 0.31s | 3667KB | 0.05s |
| 7z | 3 | 0.93s | 2942KB | 0.20s |
| 7z | 9 | 2.06s | **2721KB** | 0.21s |
| tar.gz | 9 | 0.32s | 3665KB | 0.05s |
| tar.xz | 9 | 1.71s | 2719KB | 0.15s |
| tar.lz4 | 3 | 0.11s | 4233KB | **0.03s** |
| tar.bz2 | 9 | 0.41s | 3606KB | 0.21s |

> zstd 多线程编码 + 1MB 大缓冲区 + ZIP 并行解压 + 压缩包炸弹检测

## 🔄 版本历史

### v0.5.0
- 🎨 明暗主题切换 (☀️ 浅色 / 🌙 深色)
- 🗑️ 安全删除源文件 (3 次覆写: 0xFF / 0x00 / 随机)
- 📋 归档内容浏览 (GUI 浮动窗口, 文件列表 + 压缩比统计)
- 🧪 完整性测试 (CLI `test` 命令 + GUI 按钮, 支持 zip/7z/rar/tar/单文件流)
- 📋 归档列表 (CLI `list` 命令, 表格输出)
- ✕ 任务取消 (协作式取消, Arc<AtomicBool>)
- 📊 进度详情 (已处理/总字节数实时显示)
- 🎲 密码生成器 (xorshift64 PRNG, 混合字符池)
- 👁 密码可见性切换
- 💬 Toast 通知 (3 秒自动消失, 颜色编码)
- 📂 最近文件下拉菜单
- 🖱️ 拖放文件支持
- ⌨️ 键盘快捷键 (Ctrl+Enter / Esc / Ctrl+L)
- 📝 文件排除规则 (通配符, GUI + CLI)
- ✂️ 分卷压缩 (支持 B/K/M/G 后缀, GUI + CLI)
- 🐛 修复 7z 单文件压缩 (push_source_path 空条目名导致解压失败)
- 🐛 修复 7z 解压需预创建输出目录
- 🔧 i18n 补全所有新功能翻译键

### v0.4.0
- ⚡ zstd 多线程编码 (利用全部 CPU 核心并行压缩)
- ⚡ 1MB 大缓冲区 (所有压缩/解压路径, 默认 8KB → 1MB, IO 提升 10x+)
- ⚡ ZIP 多线程并行解压 (rayon 分块并行, 多 worker 独立打开 ZipArchive)
- ⚡ `copy_large` 大缓冲拷贝函数 (替代 `io::copy` 默认 8KB)
- 🔒 压缩包炸弹检测 (100x 比例限制 + 10GB 绝对限制)
- 🔧 修复 7z COPY/BCJ 兼容性 (sevenz-rust 0.6 不支持, 统一用 LZMA2)

### v0.3.0
- ✨ 新增 RAR 格式解压支持 (RAR3/RAR5)
- ✨ 新增中英双语 GUI (i18n)
- ✨ 新增 extract-here / extract-as 右键菜单集成
- ✨ 新增 ZIP/7z AES-256 加密 (兼容 7-Zip/WinRAR/Bandizip)
- ✨ 新增 .enc 原生加密 (AES-256-GCM + Argon2id)
- ✨ 新增跨平台安装器 (Windows Inno Setup / macOS .pkg / Linux .deb)
- ✨ 新增文件关联与右键菜单注册
- ✨ 新增卸载脚本
- 🐛 修复 ZIP 中文文件名编码 (GBK/Shift-JIS → UTF-8)
- 🐛 修复 macOS TCC 权限提示
- 🐛 修复单文件流解压输出路径

### v0.2.0
- ✨ 新增 eframe/egui 深色玻璃拟态 GUI
- ✨ 支持 14+ 压缩格式
- ⚡ 使用 Zstandard 默认算法，速度远超 7zip

### v0.1.0
- 🎉 初始版本，CLI 压缩/解压/加密/解密
