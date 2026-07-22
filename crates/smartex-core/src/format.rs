//! 统一管理所有支持的压缩格式与算法
//!
//! 支持矩阵:
//!   zip   — deflate / deflate64 / zstd / bzip2 / lzma / stored (+ AES-256 加密)
//!   7z    — LZMA2 / ZSTD / COPY (+ AES-256 加密)
//!   rar   — RAR3 / RAR5 (仅解压, 支持加密)
//!   tar.* — gz / xz / zst / bz2 / lz4
//!   单文件 — gz / xz / zst / bz2 / lz4

use std::path::Path;

/// 支持的归档容器格式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    Zip,
    SevenZ,
    Rar,        // RAR (仅解压)
    Tar,        // 裸 tar
    TarGz,
    TarXz,
    TarZst,
    TarBz2,
    TarLz4,
    Single,     // 单文件流压缩 (.gz/.xz/.zst/.bz2/.lz4)
}

impl Container {
    pub fn extension(&self) -> &'static str {
        match self {
            Container::Zip => "zip",
            Container::SevenZ => "7z",
            Container::Rar => "rar",
            Container::Tar => "tar",
            Container::TarGz => "tar.gz",
            Container::TarXz => "tar.xz",
            Container::TarZst => "tar.zst",
            Container::TarBz2 => "tar.bz2",
            Container::TarLz4 => "tar.lz4",
            Container::Single => "bin",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Container::Zip => "ZIP (Deflate/Zstd/AES-256)",
            Container::SevenZ => "7z (LZMA2/AES-256)",
            Container::Rar => "RAR (仅解压)",
            Container::Tar => "TAR (无压缩)",
            Container::TarGz => "TAR.GZ (Gzip)",
            Container::TarXz => "TAR.XZ (LZMA)",
            Container::TarZst => "TAR.ZST (Zstandard) ⚡",
            Container::TarBz2 => "TAR.BZ2 (BZip2)",
            Container::TarLz4 => "TAR.LZ4 (LZ4) ⚡⚡",
            Container::Single => "单文件流",
        }
    }

    /// 所有可用于压缩的容器 (RAR 仅解压, Single 不在此列)
    pub fn all() -> &'static [Container] {
        &[
            Container::Zip,
            Container::SevenZ,
            Container::TarGz,
            Container::TarXz,
            Container::TarZst,
            Container::TarBz2,
            Container::TarLz4,
            Container::Tar,
        ]
    }

    /// 所有可解压的容器 (含 RAR, 用于解压格式检测)
    pub fn all_extractable() -> &'static [Container] {
        &[
            Container::Zip,
            Container::SevenZ,
            Container::Rar,
            Container::TarGz,
            Container::TarXz,
            Container::TarZst,
            Container::TarBz2,
            Container::TarLz4,
            Container::Tar,
        ]
    }

    /// 是否支持密码加密压缩
    pub fn supports_encryption(&self) -> bool {
        matches!(self, Container::Zip | Container::SevenZ)
    }
}

/// 根据文件名推断容器格式 (用于解压自动识别)
pub fn detect(path: &Path) -> Option<Container> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    if name.ends_with(".zip") {
        Some(Container::Zip)
    } else if name.ends_with(".7z") {
        Some(Container::SevenZ)
    } else if name.ends_with(".rar") || name.ends_with(".r00") {
        Some(Container::Rar)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Some(Container::TarGz)
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        Some(Container::TarXz)
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        Some(Container::TarZst)
    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
        Some(Container::TarBz2)
    } else if name.ends_with(".tar.lz4") {
        Some(Container::TarLz4)
    } else if name.ends_with(".tar.lzma") || name.ends_with(".tlz") {
        // .tar.lzma 视为 .tar.xz (LZMA 算法族)
        Some(Container::TarXz)
    } else if name.ends_with(".tar") {
        Some(Container::Tar)
    } else if name.ends_with(".gz") {
        Some(Container::Single)
    } else if name.ends_with(".xz") {
        Some(Container::Single)
    } else if name.ends_with(".zst") {
        Some(Container::Single)
    } else if name.ends_with(".bz2") {
        Some(Container::Single)
    } else if name.ends_with(".lz4") {
        Some(Container::Single)
    } else {
        None
    }
}

/// 默认输出路径
pub fn default_archive_name(input: &Path, container: Container) -> std::path::PathBuf {
    let mut name = input
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "archive".to_string());
    name.push('.');
    name.push_str(container.extension());
    input.parent().unwrap_or(Path::new(".")).join(name)
}
