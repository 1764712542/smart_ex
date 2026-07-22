use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// smart_ex — 智能压缩/解压 + 加密/解密工具 (CLI + GUI)
///
/// 支持 14+ 格式: zip / 7z / rar / tar.gz / tar.xz / tar.zst / tar.bz2 / tar.lz4 / tar / gz / xz / zst / bz2 / lz4
/// 加密兼容: AES-256 (ZIP/7z) / ZipCrypto / RAR 加密 / smart_ex .enc
#[derive(Parser, Debug)]
#[command(name = "smart_ex", version, about)]
pub struct Cli {
    /// 启动 GUI 界面
    #[arg(long, global = true)]
    pub gui: bool,

    /// 语言 / Language: zh | en
    #[arg(long, global = true, default_value = "zh")]
    pub lang: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 启动 GUI 界面 / Launch GUI
    Gui,
    /// 压缩文件或目录 / Compress files or directories
    Compress {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 格式: zip | 7z | tar.gz | tar.xz | tar.zst | tar.bz2 | tar.lz4 | tar | gz | xz | zst | bz2 | lz4
        #[arg(short, long)]
        format: Option<String>,

        /// 压缩级别 0-12
        #[arg(short, long, default_value = "3")]
        level: i32,

        /// 用密码加密归档 (ZIP/7z 使用 AES-256, 兼容 7-Zip/WinRAR/Bandizip)
        #[arg(long)]
        password: Option<String>,

        /// 排除文件/目录 (通配符, 可多次指定)
        /// 例: --exclude "*.tmp" --exclude ".git" --exclude "*.log"
        #[arg(long)]
        exclude: Vec<String>,

        /// 分卷大小 (例: 100M, 700M, 1G), 仅 ZIP 支持
        #[arg(long)]
        split: Option<String>,
    },
    /// 解压归档 (自动识别格式) / Extract archive (auto-detect format)
    Decompress {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// 若归档已加密, 提供密码
        #[arg(long)]
        password: Option<String>,
    },
    /// 解压到当前文件夹 (右键菜单集成) / Extract to current folder
    #[command(name = "extract-here")]
    ExtractHere {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// 解压另存为 (右键菜单集成, 若不指定 -o 则弹出目录选择器) / Extract to specified folder
    #[command(name = "extract-as")]
    ExtractAs {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        password: Option<String>,
    },
    /// 加密任意文件 (AES-256-GCM) / Encrypt any file
    Encrypt {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        password: String,
    },
    /// 解密文件 / Decrypt file
    Decrypt {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        password: String,
    },
    /// 智能模式: 自动根据输入判断 / Smart mode: auto-detect
    Smart {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        password: Option<String>,
        #[arg(short, long)]
        format: Option<String>,
    },
    /// 列出归档内容 (不解压) / List archive contents
    List {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// 测试归档完整性 / Test archive integrity
    Test {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
}
