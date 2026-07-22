//! 中英双语国际化 (i18n)
//!
//! 使用简单的键值对实现,零外部依赖,运行时切换语言.

use std::sync::RwLock;

/// 支持的语言
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "en" | "english" => Lang::En,
            _ => Lang::Zh,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Lang::Zh => Lang::En,
            Lang::En => Lang::Zh,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Lang::Zh => "中文",
            Lang::En => "English",
        }
    }
}

static CURRENT_LANG: RwLock<Lang> = RwLock::new(Lang::Zh);

/// 设置全局语言
pub fn set_lang(lang: Lang) {
    if let Ok(mut g) = CURRENT_LANG.write() {
        *g = lang;
    }
}

/// 获取当前语言
pub fn current_lang() -> Lang {
    match CURRENT_LANG.read() {
        Ok(g) => *g,
        Err(e) => *e.into_inner(),
    }
}

/// 翻译键值
pub fn t(key: &str) -> &'static str {
    let lang = current_lang();
    match lang {
        Lang::Zh => t_zh(key),
        Lang::En => t_en(key),
    }
}

fn t_zh(key: &str) -> &'static str {
    match key {
        // ─── 通用 ───
        "app_name" => "⚡ smart_ex",
        "app_tagline" => "智能压缩 · 加密 · 解压",
        "app_version" => "v0.3 · 14+ 格式",
        "ready" => "就绪",
        "processing" => "处理中...",
        "done" => "完成",
        "failed" => "失败",
        "ok" => "确定",
        "cancel" => "取消",
        "clear" => "清空",
        "browse" => "浏览",

        // ─── 模式 ───
        "mode_compress" => "📦 压缩",
        "mode_decompress" => "📂 解压",
        "mode_encrypt" => "🔐 加密",
        "mode_decrypt" => "🔓 解密",

        // ─── 输入输出 ───
        "input" => "输入",
        "output" => "输出",
        "input_hint" => "选择或拖入路径...",
        "output_hint" => "输出路径 (可留空自动生成)",
        "pick_file" => "文件",
        "pick_dir" => "目录",
        "pick_file_btn" => "选择文件",
        "select_file" => "选择文件",

        // ─── 压缩 ───
        "format" => "压缩格式",
        "compress_level" => "压缩级别",
        "encrypt_after" => "压缩后加密归档",
        "password" => "密码",
        "password_hint" => "AES-256 密钥",

        // ─── 解压快捷操作 ───
        "extract_here" => "📂 解压到当前文件夹",
        "extract_to" => "💾 解压另存为...",
        "extract_current_dir" => "解压到当前目录",

        // ─── 进度/日志 ───
        "progress" => "进度",
        "log" => "日志",

        // ─── 消息 ───
        "select_input" => "请选择输入文件/目录",
        "input_not_exist" => "输入路径不存在",
        "start_prefix" => "▶ 开始",
        "need_password" => "需要密码",
        "encrypt_need_password" => "加密需要密码",
        "decrypt_need_password" => "解密需要密码",
        "decrypting" => "  解密中...",
        "encrypting" => "  启用加密...",
        "continue_extract" => "  继续解压...",
        "compress_done" => "压缩完成",
        "decompress_done" => "解压完成",
        "encrypt_done" => "加密完成",
        "decrypt_done" => "解密完成",
        "error_prefix" => "❌ 错误: ",
        "fail_prefix" => "❌ 失败: ",
        "perm_denied" => "权限不足 (macOS 隐私保护). 请在 系统设置 → 隐私与安全性 → 完全磁盘访问权限 中添加 smart_ex, 或将文件移动到非受保护目录 (如主目录). macOS 受保护目录: 下载/桌面/文稿",
        "perm_denied_short" => "权限不足, 请授予完全磁盘访问权限",

        // ─── 文件关联/安装 ───
        "file_assoc" => "文件关联",
        "assoc_title" => "选择要关联的压缩格式",
        "assoc_desc" => "全选后将自动绑定所有格式, 右键可直接用 smart_ex 解压",
        "select_all" => "全选",
        "deselect_all" => "取消全选",
        "install_path" => "安装路径",

        // ─── 编码 ───
        "encoding_auto" => "自动检测编码",
        "encoding_utf8" => "UTF-8",
        "encoding_gbk" => "GBK (中文)",
        "encoding_shiftjis" => "Shift-JIS (日文)",

        _ => "",
    }
}

fn t_en(key: &str) -> &'static str {
    match key {
        // ─── General ───
        "app_name" => "⚡ smart_ex",
        "app_tagline" => "Smart · Compress · Encrypt · Extract",
        "app_version" => "v0.3 · 14+ formats",
        "ready" => "Ready",
        "processing" => "Processing...",
        "done" => "Done",
        "failed" => "Failed",
        "ok" => "OK",
        "cancel" => "Cancel",
        "clear" => "Clear",
        "browse" => "Browse",

        // ─── Modes ───
        "mode_compress" => "📦 Compress",
        "mode_decompress" => "📂 Extract",
        "mode_encrypt" => "🔐 Encrypt",
        "mode_decrypt" => "🔓 Decrypt",

        // ─── Input/Output ───
        "input" => "Input",
        "output" => "Output",
        "input_hint" => "Select or drop a path...",
        "output_hint" => "Output path (auto-generated if empty)",
        "pick_file" => "File",
        "pick_dir" => "Dir",
        "pick_file_btn" => "Choose File",
        "select_file" => "Select File",

        // ─── Compress ───
        "format" => "Format",
        "compress_level" => "Compression Level",
        "encrypt_after" => "Encrypt archive after compression",
        "password" => "Password",
        "password_hint" => "AES-256 key",

        // ─── Extract quick actions ───
        "extract_here" => "📂 Extract Here",
        "extract_to" => "💾 Extract As...",
        "extract_current_dir" => "Extract to current folder",

        // ─── Progress/Log ───
        "progress" => "Progress",
        "log" => "Log",

        // ─── Messages ───
        "select_input" => "Please select an input file/directory",
        "input_not_exist" => "Input path does not exist",
        "start_prefix" => "▶ Start ",
        "need_password" => "Password required",
        "encrypt_need_password" => "Encryption requires a password",
        "decrypt_need_password" => "Decryption requires a password",
        "decrypting" => "  Decrypting...",
        "encrypting" => "  Encrypting...",
        "continue_extract" => "  Continue extracting...",
        "compress_done" => "Compression done",
        "decompress_done" => "Extraction done",
        "encrypt_done" => "Encryption done",
        "decrypt_done" => "Decryption done",
        "error_prefix" => "❌ Error: ",
        "fail_prefix" => "❌ Failed: ",
        "perm_denied" => "Permission denied (macOS TCC). Please add smart_ex to System Settings → Privacy & Security → Full Disk Access, or move files to a non-protected directory (e.g. home). Protected: Downloads/Desktop/Documents",
        "perm_denied_short" => "Permission denied, please grant Full Disk Access",

        // ─── File association / install ───
        "file_assoc" => "File Associations",
        "assoc_title" => "Select formats to associate",
        "assoc_desc" => "Select all to bind every format — right-click to extract with smart_ex",
        "select_all" => "Select All",
        "deselect_all" => "Deselect All",
        "install_path" => "Install Path",

        // ─── Encoding ───
        "encoding_auto" => "Auto-detect encoding",
        "encoding_utf8" => "UTF-8",
        "encoding_gbk" => "GBK (Chinese)",
        "encoding_shiftjis" => "Shift-JIS (Japanese)",

        _ => "",
    }
}
