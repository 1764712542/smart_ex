//! 解压模块 — 支持所有格式 + 加密解压 + 文件名编码修复
//!
//! 支持格式: zip / 7z / rar / tar / tar.gz / tar.xz / tar.zst / tar.bz2 / tar.lz4
//! 加密支持: ZIP (AES-256/ZipCrypto) / 7z (AES-256) / RAR (加密)
//! 编码修复: ZIP 文件名自动检测 UTF-8/GBK/Shift-JIS

use crate::format::{detect, Container};
use crate::progress::Progress;
use crate::rar;
use anyhow::{Context, Result};
use encoding_rs;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

// ───────────────────────── 编码修复 ─────────────────────────

/// 修复 ZIP 条目文件名编码
///
/// ZIP 文件名编码问题: Windows 下 WinRAR/好压/7-Zip 等工具创建的中文文件名 ZIP
/// 通常使用 GBK 编码而非 UTF-8. zip crate 会按 CP437 解码产生乱码.
///
/// 修复策略:
/// 1. 获取 name_raw() 原始字节
/// 2. 如果是有效 UTF-8, 直接使用
/// 3. 如果不是, 尝试 GBK 解码 (中文 Windows 最常见)
/// 4. 如果 GBK 也失败, 尝试 Shift-JIS (日文)
/// 5. 全部失败则回退到 zip crate 的 name()
fn fix_zip_name(raw_bytes: &[u8], fallback_name: &str) -> String {
    // 1. 检查是否为有效 UTF-8
    if let Ok(s) = std::str::from_utf8(raw_bytes) {
        return s.to_string();
    }
    // 2. 尝试 GBK 解码 (中文 Windows 最常见)
    let (decoded, _, gbk_errors) = encoding_rs::GBK.decode(raw_bytes);
    if !gbk_errors {
        return decoded.into_owned();
    }
    // 3. 尝试 Shift-JIS (日文)
    let (decoded, _, sjis_errors) = encoding_rs::SHIFT_JIS.decode(raw_bytes);
    if !sjis_errors {
        return decoded.into_owned();
    }
    // 4. 回退到 zip crate 的解码结果
    fallback_name.to_string()
}

/// 将文件名转换为安全路径 (防止路径穿越攻击)
fn safe_join(base: &Path, name: &str) -> Option<std::path::PathBuf> {
    let p = Path::new(name);
    // 去除前导 / 和 ..
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

// ───────────────────────── Zip ─────────────────────────

pub fn zip_decompress(
    input: &Path,
    output: &Path,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    let file = File::open(input).with_context(|| format!("打开文件失败: {}", input.display()))?;
    let mut archive = zip::ZipArchive::new(file)?;

    bar.set_total(archive.len() as u64);

    for i in 0..archive.len() {
        // 若提供密码, 使用 by_index_decrypt (对非加密条目也能正常工作)
        // 若无密码, 使用 by_index
        let mut entry = if let Some(pwd) = password {
            archive.by_index_decrypt(i, pwd.as_bytes())
        } else {
            archive.by_index(i)
        }?;

        // 修复文件名编码
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
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut entry, &mut outfile)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
            }
        }
        bar.inc(1);
    }
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

// ───────────────────────── Tar 系列 ─────────────────────────

fn tar_extract_with<R: Read>(reader: R, output: &Path, bar: &Progress) -> Result<()> {
    let mut archive = tar::Archive::new(reader);

    let mut count = 0u64;
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
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut entry, &mut outfile)?;
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
    let gz = flate2::read::GzDecoder::new(File::open(input)?);
    tar_extract_with(gz, output, bar)
}

pub fn tarxz_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let xz = xz2::read::XzDecoder::new(File::open(input)?);
    tar_extract_with(xz, output, bar)
}

pub fn tarzst_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let zst = zstd::Decoder::new(File::open(input)?)
        .map_err(|e| anyhow::anyhow!("zstd 解码器初始化失败: {}", e))?;
    tar_extract_with(zst, output, bar)
}

pub fn tarbz2_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let bz2 = bzip2::read::BzDecoder::new(File::open(input)?);
    tar_extract_with(bz2, output, bar)
}

pub fn tarlz4_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let lz4 = lz4::Decoder::new(File::open(input)?)
        .map_err(|e| anyhow::anyhow!("lz4 解码器初始化失败: {}", e))?;
    tar_extract_with(lz4, output, bar)
}

// ───────────────────────── 单文件流解压 ─────────────────────────

pub fn single_decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    bar.set_total(1);
    let name = input
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 确定实际输出文件路径:
    // - 若 output 是已存在的目录, 在其中创建文件 (文件名 = 去掉压缩扩展名)
    // - 若 output 不存在但父目录是目录, 同上
    // - 否则 output 视为文件路径
    let out_file = if output.is_dir() {
        let stem = input
            .file_stem()
            .map(|s| s.to_os_string())
            .unwrap_or_else(|| std::ffi::OsString::from("output"));
        output.join(stem)
    } else if !output.exists() {
        // output 不存在: 检查父目录
        if let Some(parent) = output.parent() {
            if parent.is_dir() {
                // 如果 output 路径看起来像目录名 (无扩展名), 当作目录处理
                // 否则当作文件路径
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
        // output 已存在且是文件
        output.to_path_buf()
    };

    // 确保父目录存在
    if let Some(parent) = out_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let reader = BufReader::new(File::open(input)?);
    let mut writer = BufWriter::new(File::create(&out_file)?);

    if name.ends_with(".gz") {
        let mut dec = flate2::read::GzDecoder::new(reader);
        io::copy(&mut dec, &mut writer)?;
    } else if name.ends_with(".xz") {
        let mut dec = xz2::read::XzDecoder::new(reader);
        io::copy(&mut dec, &mut writer)?;
    } else if name.ends_with(".zst") {
        let mut dec = zstd::Decoder::new(reader)?;
        io::copy(&mut dec, &mut writer)?;
    } else if name.ends_with(".bz2") {
        let mut dec = bzip2::read::BzDecoder::new(reader);
        io::copy(&mut dec, &mut writer)?;
    } else if name.ends_with(".lz4") {
        let mut dec = lz4::Decoder::new(reader)?;
        io::copy(&mut dec, &mut writer)?;
    } else {
        return Err(anyhow::anyhow!("无法识别的单文件压缩格式: {}", name));
    }
    writer.flush()?;
    bar.inc(1);
    Ok(())
}

// ───────────────────────── 统一分发 ─────────────────────────

/// 解压入口: 自动识别格式并分发 (无密码)
pub fn decompress(input: &Path, output: &Path, bar: &Progress) -> Result<()> {
    let container = detect(input).ok_or_else(|| {
        anyhow::anyhow!(
            "无法识别归档格式: {} (支持 .zip / .7z / .rar / .tar.gz / .tar.xz / .tar.zst / .tar.bz2 / .tar.lz4 / .tar / .gz / .xz / .zst / .bz2 / .lz4)",
            input.display()
        )
    })?;

    decompress_with(input, output, container, None, bar)
}

/// 解压入口: 自动识别格式并分发 (带可选密码)
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

/// 指定容器格式解压 (带可选密码)
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

// 保留 Read/Write trait 导入
#[allow(dead_code)]
fn _ensure_traits() -> (Option<Box<dyn Read>>, Option<Box<dyn Write>>) {
    (None, None)
}
