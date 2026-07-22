//! 压缩包内容浏览模块 — 列出归档内文件列表 (不解压)
//!
//! 支持格式: zip / tar / tar.gz / tar.xz / tar.zst / tar.bz2 / tar.lz4
//! 不支持: 7z (sevenz_rust 无 list API) / rar (unrar crate API 复杂)
//! 编码修复: ZIP 文件名自动检测 UTF-8/GBK/Shift-JIS

use crate::format::{detect, Container};
use anyhow::{Context, Result};
use encoding_rs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// 大缓冲区大小: 1MB
const BUF_SIZE: usize = 1024 * 1024;

/// 归档条目信息
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// 文件名 (含相对路径)
    pub name: String,
    /// 未压缩大小 (字节)
    pub size: u64,
    /// 是否为目录
    pub is_dir: bool,
    /// 压缩后大小 (字节)
    pub compressed_size: u64,
}

/// 列出压缩包内的文件列表
///
/// 支持格式: zip / tar / tar.gz / tar.xz / tar.zst / tar.bz2 / tar.lz4
/// 7z 暂不支持列表 (sevenz_rust 无 list API)
/// RAR 暂不支持列表 (unrar crate API 复杂)
pub fn list_archive(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
    let container = detect(path).ok_or_else(|| {
        anyhow::anyhow!(
            "无法识别归档格式: {} (支持 .zip / .tar / .tar.gz / .tar.xz / .tar.zst / .tar.bz2 / .tar.lz4)",
            path.display()
        )
    })?;

    match container {
        Container::Zip => list_zip(path, password),
        Container::SevenZ => Err(anyhow::anyhow!("7z 列表暂不支持")),
        Container::Rar => Ok(Vec::new()),
        Container::Tar => list_tar(File::open(path)?),
        Container::TarGz => list_tar(flate2::read::GzDecoder::new(
            BufReader::with_capacity(BUF_SIZE, File::open(path)?),
        )),
        Container::TarXz => list_tar(xz2::read::XzDecoder::new(
            BufReader::with_capacity(BUF_SIZE, File::open(path)?),
        )),
        Container::TarZst => {
            let dec = zstd::Decoder::new(File::open(path)?)
                .map_err(|e| anyhow::anyhow!("zstd 解码器初始化失败: {}", e))?;
            list_tar(dec)
        }
        Container::TarBz2 => list_tar(bzip2::read::BzDecoder::new(
            BufReader::with_capacity(BUF_SIZE, File::open(path)?),
        )),
        Container::TarLz4 => {
            let dec = lz4::Decoder::new(File::open(path)?)
                .map_err(|e| anyhow::anyhow!("lz4 解码器初始化失败: {}", e))?;
            list_tar(dec)
        }
        Container::Single => Err(anyhow::anyhow!("单文件压缩格式不支持列表 (无归档结构)")),
    }
}

// ───────────────────────── 编码修复 ─────────────────────────

/// 修复 ZIP 条目文件名编码 (参考 decompress.rs 的 fix_zip_name 逻辑)
///
/// 优先级: UTF-8 → GBK → Shift-JIS → 原始 fallback
fn fix_zip_name(raw_bytes: &[u8], fallback_name: &str) -> String {
    if let Ok(s) = std::str::from_utf8(raw_bytes) {
        return s.to_string();
    }
    let (decoded, _, gbk_errors) = encoding_rs::GBK.decode(raw_bytes);
    if !gbk_errors {
        return decoded.into_owned();
    }
    let (decoded, _, sjis_errors) = encoding_rs::SHIFT_JIS.decode(raw_bytes);
    if !sjis_errors {
        return decoded.into_owned();
    }
    fallback_name.to_string()
}

// ───────────────────────── Zip ─────────────────────────

/// 列出 ZIP 归档内容
///
/// ZIP 中央目录通常不加密, 即使条目数据加密也能列出文件名
fn list_zip(path: &Path, _password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(path).with_context(|| format!("打开文件失败: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut entries = Vec::with_capacity(archive.len() as usize);
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let raw_name = entry.name_raw().to_vec();
        let name = fix_zip_name(&raw_name, entry.name());
        entries.push(ArchiveEntry {
            name,
            size: entry.size(),
            is_dir: entry.is_dir(),
            compressed_size: entry.compressed_size(),
        });
    }
    Ok(entries)
}

// ───────────────────────── Tar 系列 ─────────────────────────

/// 列出 Tar 归档内容 (支持各种压缩编码包装)
fn list_tar<R: Read>(reader: R) -> Result<Vec<ArchiveEntry>> {
    let mut archive = tar::Archive::new(BufReader::with_capacity(BUF_SIZE, reader));
    let mut entries = Vec::new();

    for entry_result in archive.entries()? {
        let entry = entry_result?;
        let name = entry.path()?.to_string_lossy().to_string();
        let is_dir = entry.header().entry_type().is_dir();
        let size = entry.header().size().unwrap_or(0);
        // tar 流式压缩无独立条目压缩大小, 使用未压缩大小作为近似值
        let compressed_size = size;
        entries.push(ArchiveEntry {
            name,
            size,
            is_dir,
            compressed_size,
        });
    }
    Ok(entries)
}
