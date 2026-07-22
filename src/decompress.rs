//! 解压模块 — 支持所有格式 + 加密解压 + 文件名编码修复 + 并行解压 + 炸弹检测
//!
//! 支持格式: zip / 7z / rar / tar / tar.gz / tar.xz / tar.zst / tar.bz2 / tar.lz4
//! 加密支持: ZIP (AES-256/ZipCrypto) / 7z (AES-256) / RAR (加密)
//! 编码修复: ZIP 文件名自动检测 UTF-8/GBK/Shift-JIS
//! 性能优化: 1MB 大缓冲区 + ZIP 多线程并行解压 + 压缩包炸弹检测

use crate::format::{detect, Container};
use crate::progress::Progress;
use crate::rar;
use anyhow::{Context, Result};
use encoding_rs;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 大缓冲区大小: 1MB
const BUF_SIZE: usize = 1024 * 1024;

/// 压缩包炸弹防护: 最大解压比例 (归档大小的 N 倍)
const MAX_RATIO: u64 = 100;
/// 压缩包炸弹防护: 最大解压绝对大小 (10GB)
const MAX_EXTRACTED: u64 = 10 * 1024 * 1024 * 1024;

// ───────────────────────── 编码修复 ─────────────────────────

/// 修复 ZIP 条目文件名编码
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

/// 将文件名转换为安全路径 (防止路径穿越攻击)
fn safe_join(base: &Path, name: &str) -> Option<std::path::PathBuf> {
    let p = Path::new(name);
    let clean: std::path::PathBuf = p.components()
        .filter(|c| {
            matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir)
        })
        .collect();
    if clean.as_os_str().is_empty() {
        return None;
    }
    Some(base.join(clean))
}

/// 大缓冲区拷贝 (比 io::copy 默认 8KB 快 10 倍+)
fn copy_large<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> io::Result<u64> {
    let mut buf = vec![0u8; BUF_SIZE];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        total += n as u64;
    }
    Ok(total)
}

/// 检测压缩包炸弹
fn check_bomb(extracted: u64, archive_size: u64) -> Result<()> {
    if extracted > MAX_EXTRACTED {
        return Err(anyhow::anyhow!(
            "压缩包炸弹检测: 解压数据已达 {} (超过 {} 限制), 终止解压",
            format_bytes(extracted),
            format_bytes(MAX_EXTRACTED)
        ));
    }
    if archive_size > 0 && extracted > archive_size.saturating_mul(MAX_RATIO) {
        return Err(anyhow::anyhow!(
            "压缩包炸弹检测: 解压数据已达归档大小的 {} 倍, 终止解压",
            MAX_RATIO
        ));
    }
    Ok(())
}

fn format_bytes(n: u64) -> String {
    if n >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", n as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if n >= 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{} B", n)
    }
}

// ───────────────────────── Zip (多线程并行解压) ─────────────────────────

pub fn zip_decompress(
    input: &Path,
    output: &Path,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    let file = File::open(input).with_context(|| format!("打开文件失败: {}", input.display()))?;
    let archive_size = file.metadata()?.len();
    let archive = zip::ZipArchive::new(file)?;
    let total = archive.len();
    drop(archive); // 释放主线程的 archive, 各 worker 各自重新打开

    bar.set_total(total as u64);

    if total == 0 {
        return Ok(());
    }

    // 共享状态: 解压字节数 + 密码
    let extracted_bytes = Arc::new(AtomicU64::new(0));
    let pwd = password.map(|s| s.to_string());

    // 分块并行处理 (每个 worker 独立打开 ZipArchive)
    let indices: Vec<usize> = (0..total).collect();
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let chunk_size = ((total + nthreads - 1) / nthreads).max(1);

    indices.par_chunks(chunk_size).try_for_each(|chunk| -> Result<()> {
        // 炸弹检测
        let extracted = extracted_bytes.load(Ordering::Relaxed);
        check_bomb(extracted, archive_size)?;

        // 每个 worker 独立打开 archive
        let f = File::open(input)?;
        let mut archive = zip::ZipArchive::new(f)?;
        let pwd_bytes = pwd.as_deref().map(|s| s.as_bytes());

        for &i in chunk {
            // 再次检查炸弹
            let extracted = extracted_bytes.load(Ordering::Relaxed);
            if let Err(e) = check_bomb(extracted, archive_size) {
                return Err(e);
            }

            let mut entry = if let Some(pwd) = pwd_bytes {
                archive.by_index_decrypt(i, pwd)
            } else {
                archive.by_index(i)
            }?;

            let raw_name = entry.name_raw().to_vec();
            let fixed_name = fix_zip_name(&raw_name, entry.name());
            let outpath = match safe_join(output, &fixed_name) {
                Some(p) => p,
                None => {
                    bar.inc(1);
                    continue;
                }
            };

            if entry.is_dir() {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                // 大缓冲区写入
                let mut outfile = BufWriter::with_capacity(BUF_SIZE, File::create(&outpath)?);
                let written = copy_large(&mut entry, &mut outfile)?;
                outfile.flush()?;
                extracted_bytes.fetch_add(written, Ordering::Relaxed);
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ = std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode));
                }
            }

            bar.inc(1);
        }
        Ok(())
    })?;

    Ok(())
}

// ───────────────────────── 7z ─────────────────────────

pub fn sevenz_decompress(
    input: &Path,
    output: &Path,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    let result = if let Some(pwd) = password {
        sevenz_rust::decompress_file_with_password(input, output, pwd.into())
    } else {
        sevenz_rust::decompress_file(input, output)
    };
    result.map_err(|e| anyhow::anyhow!("7z 解压失败: {}", e))?;
    bar.set_total(1);
    bar.inc(1);
    Ok(())
}

// ───────────────────────── RAR ─────────────────────────

pub fn rar_decompress(
    input: &Path,
    output: &Path,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    rar::rar_decompress(input, output, password, bar)
}

// ───────────────────────── Tar 系列 (缓冲区优化 + 炸弹检测) ─────────────────────────

fn tar_extract_with<R: Read>(reader: R, output: &Path, bar: &Progress) -> Result<()> {
    let mut archive = tar::Archive::new(BufReader::with_capacity(BUF_SIZE, reader));
    let mut count = 0u64;
    let mut extracted_bytes: u64 = 0;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let outpath = output.join(&path);

        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // 大缓冲区写入, 减少 syscall
            let mut outfile = BufWriter::with_capacity(BUF_SIZE, File::create(&outpath)?);
            let written = copy_large(&mut entry, &mut outfile)?;
            outfile.flush()?;
            extracted_bytes += written;

            // 炸弹检测
            check_bomb(extracted_bytes, 0)?;
        }
        count += 1;
        bar.set_total(count);
        bar.inc(1);
    }
    Ok(())
}

pub fn tar_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    tar_extract_with(File::open(input)?, output, bar)
}

pub fn targz_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let gz = flate2::read::GzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(input)?));
    tar_extract_with(gz, output, bar)
}

pub fn tarxz_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let xz = xz2::read::XzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(input)?));
    tar_extract_with(xz, output, bar)
}

pub fn tarzst_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let zst = zstd::Decoder::new(File::open(input)?)
        .map_err(|e| anyhow::anyhow!("zstd 解码器初始化失败: {}", e))?;
    tar_extract_with(zst, output, bar)
}

pub fn tarbz2_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let bz2 = bzip2::read::BzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(input)?));
    tar_extract_with(bz2, output, bar)
}

pub fn tarlz4_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let lz4 = lz4::Decoder::new(File::open(input)?)
        .map_err(|e| anyhow::anyhow!("lz4 解码器初始化失败: {}", e))?;
    tar_extract_with(lz4, output, bar)
}

// ───────────────────────── 单文件流解压 (1MB 缓冲区) ─────────────────────────

pub fn single_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    bar.set_total(1);
    let name = input
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 确定实际输出文件路径
    let out_file = if output.is_dir() {
        let stem = input
            .file_stem()
            .map(|s| s.to_os_string())
            .unwrap_or_else(|| std::ffi::OsString::from("output"));
        output.join(stem)
    } else if !output.exists() {
        if let Some(parent) = output.parent() {
            if parent.is_dir() {
                let has_ext = output
                    .extension()
                    .map(|e| !e.is_empty())
                    .unwrap_or(false);
                if !has_ext {
                    std::fs::create_dir_all(output)?;
                    let stem = input
                        .file_stem()
                        .map(|s| s.to_os_string())
                        .unwrap_or_else(|| std::ffi::OsString::from("output"));
                    output.join(stem)
                } else {
                    output.to_path_buf()
                }
            } else {
                output.to_path_buf()
            }
        } else {
            output.to_path_buf()
        }
    } else {
        output.to_path_buf()
    };

    if let Some(parent) = out_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 1MB 大缓冲区
    let reader = BufReader::with_capacity(BUF_SIZE, File::open(input)?);
    let mut writer = BufWriter::with_capacity(BUF_SIZE, File::create(&out_file)?);

    if name.ends_with(".gz") {
        let mut dec = flate2::read::GzDecoder::new(reader);
        copy_large(&mut dec, &mut writer)?;
    } else if name.ends_with(".xz") {
        let mut dec = xz2::read::XzDecoder::new(reader);
        copy_large(&mut dec, &mut writer)?;
    } else if name.ends_with(".zst") {
        let mut dec = zstd::Decoder::new(reader)?;
        copy_large(&mut dec, &mut writer)?;
    } else if name.ends_with(".bz2") {
        let mut dec = bzip2::read::BzDecoder::new(reader);
        copy_large(&mut dec, &mut writer)?;
    } else if name.ends_with(".lz4") {
        let mut dec = lz4::Decoder::new(reader)?;
        copy_large(&mut dec, &mut writer)?;
    } else {
        return Err(anyhow::anyhow!("无法识别的单文件压缩格式: {}", name));
    }
    writer.flush()?;
    bar.inc(1);
    Ok(())
}

// ───────────────────────── 统一分发 ─────────────────────────

pub fn decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let container = detect(input).ok_or_else(|| {
        anyhow::anyhow!(
            "无法识别归档格式: {} (支持 .zip / .7z / .rar / .tar.gz / .tar.xz / .tar.zst / .tar.bz2 / .tar.lz4 / .tar / .gz / .xz / .zst / .bz2 / .lz4)",
            input.display()
        )
    })?;
    decompress_with(input, output, container, None, bar)
}

pub fn decompress_with_password(
    input: &Path,
    output: &Path,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    let container = detect(input).ok_or_else(|| {
        anyhow::anyhow!(
            "无法识别归档格式: {} (支持 .zip / .7z / .rar / .tar.gz / .tar.xz / .tar.zst / .tar.bz2 / .tar.lz4 / .tar / .gz / .xz / .zst / .bz2 / .lz4)",
            input.display()
        )
    })?;
    decompress_with(input, output, container, password, bar)
}

pub fn decompress_with(
    input: &Path,
    output: &Path,
    container: Container,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    match container {
        Container::Zip => zip_decompress(input, output, password, bar),
        Container::SevenZ => sevenz_decompress(input, output, password, bar),
        Container::Rar => rar_decompress(input, output, password, bar),
        Container::Tar => tar_decompress(input, output, bar),
        Container::TarGz => targz_decompress(input, output, bar),
        Container::TarXz => tarxz_decompress(input, output, bar),
        Container::TarZst => tarzst_decompress(input, output, bar),
        Container::TarBz2 => tarbz2_decompress(input, output, bar),
        Container::TarLz4 => tarlz4_decompress(input, output, bar),
        Container::Single => single_decompress(input, output, bar),
    }
}
