//! 上下文感知压缩引擎
//!
//! 根据用户意图 (收件人/传输方式/目标系统/优先级)
//! 自动推荐最优压缩格式/级别/分卷/编码

use serde::{Deserialize, Serialize};

/// 收件人类型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Recipient {
    /// 自己备份
    Self_,
    /// 同事 (内部, 可装任何工具)
    Colleague,
    /// 外部客户 (可能只能打开 zip)
    External,
    /// 公开下载 (最大兼容性)
    Public,
}

/// 传输方式
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Transport {
    /// ≤25MB 限制
    Email,
    /// 即时通讯, 通常 ≤100MB
    Im,
    /// 网盘, 无大小限制
    Cloud,
    /// U盘
    Usb,
    /// 本地, 无限制
    Local,
}

/// 目标系统
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TargetOs {
    Windows,
    Macos,
    Linux,
    Mobile,
    Unknown,
}

/// 优先级
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Priority {
    /// 最小体积
    Size,
    /// 最快速度
    Speed,
    /// 最大兼容
    Compatibility,
    /// 最高安全
    Security,
}

/// 压缩意图
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressionIntent {
    pub recipient: Recipient,
    pub transport: Transport,
    pub target_os: TargetOs,
    pub priority: Priority,
    /// 预估总大小 (字节), 用于判断是否需要分卷
    pub estimated_size: Option<u64>,
}

/// 格式推荐结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormatSuggestion {
    /// 推荐格式: "zip" | "7z" | "tar.zst" | "tar.gz" 等
    pub format: String,
    /// 压缩级别 0-12
    pub level: i32,
    /// 分卷大小 (如 "25M"), None 表示不分卷
    pub split_size: Option<String>,
    /// 是否强制 UTF-8 文件名编码
    pub use_utf8: bool,
    /// 是否建议加密
    pub encrypt: bool,
    /// 推荐理由 (人类可读)
    pub reason: String,
}

/// 是否为跨平台场景 (收件人系统与常见 Windows/Mac 不同)
fn is_cross_platform(target_os: &TargetOs) -> bool {
    !matches!(target_os, TargetOs::Unknown)
}

/// 根据传输方式计算分卷大小
fn split_for_transport(transport: &Transport) -> Option<String> {
    match transport {
        Transport::Email => Some("25M".to_string()),
        Transport::Im => Some("100M".to_string()),
        Transport::Cloud | Transport::Usb | Transport::Local => None,
    }
}

/// 核心决策引擎
///
/// 决策树覆盖所有 (Recipient × Transport × TargetOs × Priority) 组合,
/// 优先级顺序: Security > Compatibility > Size > Speed (高优先级诉求覆盖其他规则)。
pub fn suggest_format(intent: &CompressionIntent) -> FormatSuggestion {
    let split_size = split_for_transport(&intent.transport);

    // ───────── 1. 安全优先 ─────────
    // 任何场景只要 Priority::Security 都加密。收件人外部用 zip (兼容性最好),
    // 内部 Windows 用 7z (LZMA2 + AES-256, 压缩比更好), 内部 Linux/Mac 用 zip。
    if matches!(intent.priority, Priority::Security) {
        match (&intent.recipient, &intent.target_os) {
            (Recipient::External | Recipient::Public, _) => FormatSuggestion {
                format: "zip".to_string(),
                level: 5,
                split_size,
                use_utf8: true,
                encrypt: true,
                reason: "安全优先 + 外部收件人: zip + AES-256 (7-Zip/WinRAR/系统自带均可解)".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, TargetOs::Windows) => FormatSuggestion {
                format: "7z".to_string(),
                level: 9,
                split_size,
                use_utf8: true,
                encrypt: true,
                reason: "安全优先 + 内部 Windows: 7z + AES-256 + LZMA2 (压缩比更优, 加密文件头)".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, _) => FormatSuggestion {
                format: "zip".to_string(),
                level: 6,
                split_size,
                use_utf8: true,
                encrypt: true,
                reason: "安全优先 + 内部非 Windows: zip + AES-256 (兼容性 + 跨平台)".to_string(),
            },
        }
    }
    // ───────── 2. 兼容性优先 ─────────
    // 外部/公开场景必须 zip; Windows/Mac 系统能直接打开 zip。
    // level 3 平衡速度与体积; Email/IM 已自动分卷。
    else if matches!(intent.priority, Priority::Compatibility) {
        match (&intent.recipient, &intent.target_os) {
            (Recipient::External | Recipient::Public, TargetOs::Windows | TargetOs::Macos) => {
                FormatSuggestion {
                    format: "zip".to_string(),
                    level: 3,
                    split_size,
                    use_utf8: true,
                    encrypt: false,
                    reason: "兼容性优先 + 外部 + Windows/macOS: zip 系统原生支持, level 3 平衡速度".to_string(),
                }
            }
            (Recipient::External | Recipient::Public, _) => FormatSuggestion {
                format: "zip".to_string(),
                level: 3,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "兼容性优先 + 外部: zip 通用格式, 强制 UTF-8 文件名".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, TargetOs::Windows) => FormatSuggestion {
                format: "zip".to_string(),
                level: 5,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "兼容性优先 + Windows: zip 系统原生支持, UTF-8 避免中文乱码".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, TargetOs::Macos) => FormatSuggestion {
                format: "zip".to_string(),
                level: 5,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "兼容性优先 + macOS: zip 系统原生支持 (Archive Utility)".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, TargetOs::Linux) => FormatSuggestion {
                format: "tar.gz".to_string(),
                level: 5,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "兼容性优先 + Linux: tar.gz 系统自带工具链, UTF-8 文件名默认".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, _) => FormatSuggestion {
                format: "zip".to_string(),
                level: 5,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "兼容性优先: zip 通用格式, UTF-8 文件名".to_string(),
            },
        }
    }
    // ───────── 3. 体积优先 ─────────
    // 内部 + Linux: tar.zst level 12 (Zstd 最强压缩比 + 极快解压)
    // 内部 + Windows: 7z level 9 (LZMA2 最强压缩比)
    // 内部 + Mac: tar.xz level 9 (XZ 最强压缩比)
    // 外部仍用 zip (兼容性兜底), 但 level 9 最大化压缩
    else if matches!(intent.priority, Priority::Size) {
        match (&intent.recipient, &intent.target_os) {
            (Recipient::Self_ | Recipient::Colleague, TargetOs::Linux) => FormatSuggestion {
                format: "tar.zst".to_string(),
                level: 12,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "体积优先 + Linux: tar.zst level 12 (Zstd 最强压缩比, 多线程)".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, TargetOs::Windows) => FormatSuggestion {
                format: "7z".to_string(),
                level: 9,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "体积优先 + Windows: 7z LZMA2 level 9 (最强压缩比)".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, TargetOs::Macos) => FormatSuggestion {
                format: "tar.xz".to_string(),
                level: 9,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "体积优先 + macOS: tar.xz level 9 (XZ 最强压缩比, macOS 自带 xz)".to_string(),
            },
            (Recipient::Self_ | Recipient::Colleague, _) => FormatSuggestion {
                format: "tar.zst".to_string(),
                level: 12,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "体积优先 + 未知系统: tar.zst level 12 (最强压缩比)".to_string(),
            },
            (Recipient::External | Recipient::Public, _) => FormatSuggestion {
                format: "zip".to_string(),
                level: 9,
                split_size,
                use_utf8: true,
                encrypt: false,
                reason: "体积优先 + 外部收件人: zip level 9 (兼容性兜底, 最大压缩)".to_string(),
            },
        }
    }
    // ───────── 4. 速度优先 ─────────
    // Linux/未知系统: tar.zst level 1 (Zstd 解压极快 + 编码极快)
    // Windows: zip level 1 (Deflate 速度优先 + 兼容性好)
    // Mac: zip level 1 (系统自带, 启动快)
    else {
        // Priority::Speed 是最后一个分支
        let (format, level, reason) = match &intent.target_os {
            TargetOs::Linux | TargetOs::Unknown => (
                "tar.zst".to_string(),
                1,
                "速度优先: tar.zst level 1 (Zstd 编解码极快, 多线程加速)".to_string(),
            ),
            TargetOs::Windows => (
                "zip".to_string(),
                1,
                "速度优先 + Windows: zip level 1 (Deflate 最快, 系统自带)".to_string(),
            ),
            TargetOs::Macos => (
                "zip".to_string(),
                1,
                "速度优先 + macOS: zip level 1 (Deflate 最快, Archive Utility)".to_string(),
            ),
            TargetOs::Mobile => (
                "zip".to_string(),
                1,
                "速度优先 + 移动端: zip level 1 (兼容性最佳, 解压快)".to_string(),
            ),
        };
        FormatSuggestion {
            format,
            level,
            split_size,
            use_utf8: is_cross_platform(&intent.target_os),
            encrypt: false,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_external_zip() {
        let intent = CompressionIntent {
            recipient: Recipient::External,
            transport: Transport::Email,
            target_os: TargetOs::Windows,
            priority: Priority::Security,
            estimated_size: Some(10 * 1024 * 1024),
        };
        let s = suggest_format(&intent);
        assert_eq!(s.format, "zip");
        assert!(s.encrypt);
        assert_eq!(s.split_size, Some("25M".to_string()));
    }

    #[test]
    fn test_size_linux_zst() {
        let intent = CompressionIntent {
            recipient: Recipient::Colleague,
            transport: Transport::Cloud,
            target_os: TargetOs::Linux,
            priority: Priority::Size,
            estimated_size: None,
        };
        let s = suggest_format(&intent);
        assert_eq!(s.format, "tar.zst");
        assert_eq!(s.level, 12);
        assert!(!s.encrypt);
    }

    #[test]
    fn test_compatibility_external_zip() {
        let intent = CompressionIntent {
            recipient: Recipient::Public,
            transport: Transport::Cloud,
            target_os: TargetOs::Windows,
            priority: Priority::Compatibility,
            estimated_size: None,
        };
        let s = suggest_format(&intent);
        assert_eq!(s.format, "zip");
        assert!(s.use_utf8);
        assert!(!s.encrypt);
    }

    #[test]
    fn test_speed_any() {
        let intent = CompressionIntent {
            recipient: Recipient::Self_,
            transport: Transport::Local,
            target_os: TargetOs::Linux,
            priority: Priority::Speed,
            estimated_size: None,
        };
        let s = suggest_format(&intent);
        assert_eq!(s.level, 1);
        assert_eq!(s.format, "tar.zst");
    }

    #[test]
    fn test_split_im_100m() {
        let intent = CompressionIntent {
            recipient: Recipient::Colleague,
            transport: Transport::Im,
            target_os: TargetOs::Windows,
            priority: Priority::Compatibility,
            estimated_size: None,
        };
        let s = suggest_format(&intent);
        assert_eq!(s.split_size, Some("100M".to_string()));
    }
}
