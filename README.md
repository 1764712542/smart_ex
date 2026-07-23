# ⚡ SmartEx

**理解你为什么压缩的下一代压缩工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)](https://github.com/1764712542/smart_ex)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Version-0.6.0-green.svg)](https://github.com/1764712542/smart_ex/releases)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blueviolet.svg)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-ff3e00.svg)](https://svelte.dev/)

SmartEx 是一个用 Rust 编写的跨平台压缩/解压/加密工具。它不只是另一个 7-Zip 复制品——**它理解你为什么压缩**：发给谁、怎么传、对方能不能打开，它替你想好。

## 🚀 v0.6.0 — 全面重塑

### 三大创新（市面无解）

#### ① 上下文感知压缩
不再纠结选 zip 还是 7z、级别 3 还是 9。告诉 SmartEx 你的意图：
- **收件人**：自己备份 / 同事 / 外部客户 / 公开下载
- **传输方式**：邮件 / 即时通讯 / 网盘 / U盘 / 本地
- **目标系统**：Windows / macOS / Linux / 手机 / 未知
- **优先级**：最小体积 / 最快速度 / 最大兼容 / 最高安全

系统自动推荐最优格式 + 级别 + 分卷 + 编码，并解释理由。一键应用。

#### ② 会话级密码钥匙串
Carnegie Mellon HCII 研究表明，每次重输密码造成 +2.8s 延迟和 43s 注意力残留。SmartEx 解决方案：
- 第一次输入密码后，会话内自动复用
- macOS Keychain 集成（跨会话持久化，可选）
- TTL 自动过期（默认 30 分钟）
- 按文件/模式分组存储

#### ③ 流式分块加密
不再 OOM。AES-256-GCM 分块加密（4MB/chunk），恒定 8MB 内存：
- 支持任意大小文件
- 每块独立认证（GCM tag）
- 中断可续传（从指定 chunk 恢复）
- 流式 I/O，不 `read_to_end`

### 全层自定义系统

**让用户决定程序变成什么样**，而不是让用户适应程序。

#### L1 外观 + 布局
- 主题色：6 预设色 + 自定义 color picker，实时生效
- 明暗主题：深色 / 浅色 / 跟随系统
- 字体：字族选择（系统/Inter/SF Pro/JetBrains Mono）+ 字号三档
- 面板布局：左右 / 右左 / 上下 三种
- 快捷键：全局监听 + 点击录制 + 重置
- Profile：JSON 导入导出

#### L2 功能模块化
- 模式启停：压缩/解压/加密/解密 四个开关
- 功能启停：智能推荐/钥匙串/分卷/排除/归档浏览/安全删除

#### L3 工作流编排器
拖拽组装多步骤工作流：
- 7 种节点：compress / decompress / encrypt / decrypt / delete-source / copy-to / move-to
- HTML5 drag & drop 拖拽编排
- 前一步输出自动作为下一步输入
- 保存为 JSON，多个命名工作流一键执行

### UI 重塑 — Mac 级美丽

- **Tauri 2 + Svelte 5 + Tailwind CSS**，替代 egui
- 玻璃拟态面板（backdrop-blur + 半透明）
- Mac 风格圆角（10px 面板 / 6px 按钮）
- 微动画（hover/active/过渡 150ms）
- 原生红绿灯（macOS `titleBarStyle: overlay`）
- 精致滚动条、滑块、下拉框
- 拖放文件 + 遮罩反馈
- Toast 通知（fade-in/out 动画）
- 实时进度 + 字节详情

## ✨ 特性

### 全格式支持（14+）

| 格式 | 压缩 | 解压 | 加密 | 备注 |
|------|:----:|:----:|:----:|------|
| ZIP | ✅ | ✅ | ✅ AES-256 | Deflate / Zstd / Bzip2 / LZMA |
| 7z | ✅ | ✅ | ✅ AES-256 | LZMA2 |
| RAR | ❌ | ✅ | ✅ | RAR3/RAR5（仅解压） |
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
| .enc | ✅ | ✅ | ✅ AES-256-GCM | 流式分块加密（v0.6 新增） |

### 加密兼容性

- **ZIP AES-256**：兼容 7-Zip、WinRAR、Bandizip
- **7z AES-256**：兼容 7-Zip、WinRAR
- **RAR 加密**：RAR3/RAR5 加密解压
- **.enc 原生加密**：AES-256-GCM 分块流式（v0.6 新增）+ Argon2id 密钥派生

### 性能

- zstd 多线程编码（v0.4）
- 1MB 大缓冲区（v0.4）
- ZIP 多线程并行解压（v0.4）
- 压缩包炸弹检测（100x 比例 + 10GB 绝对限制）
- 流式加密恒定 8MB 内存（v0.6 新增）

## 📦 安装

### 方式一：下载安装包

从 [Releases](https://github.com/1764712542/smart_ex/releases) 下载：

| 平台 | 安装包 | 安装方式 |
|------|--------|----------|
| macOS | `SmartEx_0.6.0_aarch64.dmg` | 双击挂载，拖入 Applications |
| Linux | `smartex_0.6.0_amd64.deb` | `sudo dpkg -i smartex_0.6.0_amd64.deb` |
| Windows | `SmartEx_0.6.0_x64-setup.exe` | 双击运行安装向导 |

### 方式二：从源码编译

```bash
git clone https://github.com/1764712542/smart_ex.git
cd smart_ex

# 编译 CLI
cargo build --release

# 编译 GUI (Tauri)
cargo tauri build

# 运行
./target/release/smart_ex          # CLI 启动 GUI
cargo tauri dev                    # 开发模式
```

## 🚀 使用方法

### GUI 模式

```bash
smart_ex                            # 无参数启动 GUI
smart_ex gui                        # 显式启动 GUI
open SmartEx.app                    # 直接打开 .app
```

### 命令行模式

```bash
# 压缩
smart_ex compress -i ./folder -o archive.tar.zst -f tar.zst -l 3

# 压缩 + 加密 (ZIP AES-256)
smart_ex compress -i ./folder -o secret.zip -f zip --password MyPass123

# 压缩 + 排除 + 分卷
smart_ex compress -i ./project -o project.zip -f zip \
  --exclude "*.tmp" --exclude ".git" --split 100M

# 解压
smart_ex decompress -i archive.tar.zst -o ./output

# 流式加密 (AES-256-GCM, 恒定 8MB 内存)
smart_ex encrypt -i large.iso -o large.enc --password MyPass123

# 解密
smart_ex decrypt -i large.enc -o large.iso --password MyPass123

# 浏览归档内容
smart_ex list -i archive.zip

# 测试归档完整性
smart_ex test -i archive.zip

# 智能模式 (自动判断)
smart_ex smart -i archive.zip
```

### 压缩级别

| 级别 | 速度 | 压缩比 | 适用场景 |
|------|------|--------|----------|
| 0 | 最快 | 无压缩 | 仅打包 |
| 1-3 | ⚡ 极快 | 较低 | 日常使用（推荐） |
| 4-6 | 快 | 中等 | 平衡 |
| 7-9 | 慢 | 较高 | 归档存储 |
| 10-12 | 最慢 | 最高 | 长期备份 |

## 🏗️ 项目结构

```
smart_ex/
├── Cargo.toml                  # workspace 根
├── crates/
│   ├── smartex-core/           # 核心库 (业务逻辑, 无 GUI 依赖)
│   │   └── src/
│   │       ├── compress.rs     # 压缩 (多格式 + 排除 + 分卷)
│   │       ├── decompress.rs   # 解压 (炸弹检测 + 完整性测试)
│   │       ├── crypto.rs       # .enc 传统加密 (Argon2id + AES-GCM)
│   │       ├── stream_crypto.rs# ⭐ 流式分块加密 (v0.6 新增)
│   │       ├── context.rs      # ⭐ 上下文感知压缩引擎 (v0.6 新增)
│   │       ├── keychain.rs     # ⭐ 会话钥匙串 (v0.6 新增)
│   │       ├── format.rs       # 格式检测与容器定义
│   │       ├── archive_list.rs # 归档内容浏览
│   │       ├── rar.rs          # RAR 解压
│   │       ├── progress.rs     # 进度条
│   │       └── i18n.rs         # 中英双语
│   └── smartex-tauri/          # Tauri 2 后端 (IPC 命令层)
│       ├── src/
│       │   ├── main.rs         # 入口
│       │   └── lib.rs          # 13 个 IPC 命令
│       ├── tauri.conf.json     # Tauri 配置
│       ├── capabilities/       # 权限配置
│       └── icons/              # 全平台图标
├── src/                         # CLI 二进制
│   ├── main.rs                 # 命令分发
│   └── cli.rs                  # clap 参数定义
├── ui/                          # Svelte 5 前端
│   ├── src/
│   │   ├── App.svelte          # 主布局 (四模式工作流)
│   │   ├── app.css             # 设计 tokens + Mac 级样式
│   │   ├── lib/
│   │   │   ├── tauri.ts        # IPC 封装
│   │   │   ├── components/     # 12 个 Mac 级组件
│   │   │   └── stores/         # 状态管理 (app/settings/workflows)
│   │   └── main.ts
│   ├── tailwind.config.js      # 设计系统
│   └── vite.config.ts
├── installer/                   # 跨平台安装脚本
└── test_regression.sh          # 回归测试 (19 项)
```

## 🧪 测试

```bash
# 全格式回归测试 (19 项)
./test_regression.sh

# 核心库单元测试 (15 项)
cargo test -p smartex-core --lib
```

测试覆盖：
- 8 种容器格式（zip/7z/tar/tar.gz/tar.xz/tar.zst/tar.bz2/tar.lz4）
- 5 种单文件流（gz/xz/zst/bz2/lz4）
- 加密 ZIP / 加密 7z / .enc 流式加密
- tar.zst + .enc 包装
- extract-here / extract-as
- 归档列表 / 完整性测试
- 文件排除 / 分卷压缩
- 上下文感知决策（5 场景）
- 钥匙串 CRUD + TTL 过期
- 流式加密往返 / 错误密码 / 续传

## 🔧 技术栈

| 层 | 技术 | 版本 |
|------|-----|------|
| GUI 前端 | Svelte 5 (runes) + Vite 8 + Tailwind 3 | 5.56 / 8.1 / 3.4 |
| GUI 后端 | Tauri 2 + tauri-plugin-dialog/fs | 2.x |
| 核心库 | Rust 2021 edition | 1.96 |
| CLI | clap | 4.5 |
| 压缩 | zip / tar / zstd / xz2 / bzip2 / lz4 / sevenz-rust / unrar | 最新 |
| 加密 | aes-gcm + argon2 + zeroize | 0.10 / 0.5 / 1.8 |
| 并行 | rayon | 1.10 |

## 🌍 跨平台支持

| 平台 | 状态 | 架构 | 原生集成 |
|------|------|------|----------|
| macOS | ✅ 完整支持 | arm64 / x86_64 | Keychain / 红绿灯 / .app / .dmg |
| Linux | ✅ 完整支持 | x86_64 / arm64 | .deb / .AppImage |
| Windows | ✅ 完整支持 | x86_64 | .msi / .exe |

## 🔄 版本历史

### v0.6.0 — 全面重塑
- 🎯 上下文感知压缩（收件人/传输/目标/优先级 → 自动推荐格式）
- 🔑 会话钥匙串（macOS Keychain 集成 + TTL 过期 + 按模式分组）
- 🌊 流式分块加密（AES-256-GCM 4MB chunk, 恒定 8MB 内存, 中断续传）
- 🎨 Tauri 2 + Svelte 5 + Tailwind CSS 全新 UI（替代 egui）
- 🎨 全层自定义系统（L1 外观布局 + L2 功能启停 + L3 工作流编排器）
- 🏗️ 架构重构为 workspace（smartex-core + smartex-tauri + CLI）
- 🎨 原生 macOS 红绿灯（titleBarStyle: overlay）

### v0.5.0
- 明暗主题切换 / 安全删除 / 归档浏览 / 完整性测试
- 任务取消 / 密码生成器 / Toast 通知 / 拖放支持

### v0.4.0
- zstd 多线程编码 / 1MB 大缓冲区 / ZIP 并行解压 / 炸弹检测

### v0.3.0
- RAR 解压 / 中英双语 / 右键菜单 / AES-256 加密 / 跨平台安装器

### v0.2.0
- eframe/egui GUI / 14+ 压缩格式

### v0.1.0
- 初始 CLI 版本

## 📝 许可证

[MIT License](LICENSE) — 免费开源

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request
