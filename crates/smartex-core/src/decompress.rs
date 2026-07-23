//! 解压模块 — 支持所有格式 + 加密解压 + 文件名编码修复 + 并行解压 + 炸弹检测
//!
//! 支持格式: zip / 7z / rar / tar / tar.gz / tar.xz / tar.zst / tar.bz2 / tar.lz4
//! 加密支持: ZIP (AES-256/ZipCrypto) / 7z (AES-256) / RAR (加密)
//! 编码修复: ZIP 文件名自动检测 UTF-8/GBK/Shift-JIS
//! 性能优化: 1MB 大缓冲区 + ZIP 多线程并行解压 + 压缩包炸弹检测
//! 部分解压: extract_partial 只解压指定文件/目录
//! 错误恢复: cleanup_on_error 失败时清理半成品

use crate::format::{detect, Container};
use crate::progress::Progress;
use crate::rar;
use anyhow::{Context, Result};
use encoding_rs;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

/// 大缓冲区大小: 1MB
const BUF_SIZE: usize = 1024 * 1024;

/// 压缩包炸弹防护: 最大解压比例 (归档大小的 N 倍)
const MAX_RATIO: u64 = 100;
/// 压缩包炸弹防护: 最大解压绝对大小 (10GB)
const MAX_EXTRACTED: u64 = 10 * 1024 * 1024 * 1024;

// ───────────────────────── 解压选项 ─────────────────────────

/// 解压冲突策略
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// 覆盖已存在文件
    Overwrite,
    /// 跳过已存在文件
    Skip,
    /// 自动重命名 (file.txt → file_1.txt)
    Rename,
}

/// 解压选项: 全局策略 + 单次密码 + 符号链接开关 + 错误恢复
pub struct ExtractOptions {
    pub conflict: ConflictPolicy,
    pub password: Option<String>,
    /// 是否保留符号链接 (仅 tar 系列, Windows 自动跳过)
    pub preserve_symlinks: bool,
    /// 解压失败时是否清理已解压的半成品文件 (默认 true)
    pub cleanup_on_error: bool,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            conflict: ConflictPolicy::Overwrite,
            password: None,
            preserve_symlinks: true,
            cleanup_on_error: true,
        }
    }
}

/// 解析目标路径冲突
///
/// 返回 `Some(实际写入路径)` 表示应继续写入 (可能为原路径或重命名后的路径),
/// 返回 `None` 表示应跳过该条目 (Skip 策略且文件已存在).
fn resolve_conflict(target: &Path, policy: ConflictPolicy) -> Option<PathBuf> {
    match policy {
        ConflictPolicy::Overwrite => Some(target.to_path_buf()),
        ConflictPolicy::Skip => {
            if target.exists() {
                None
            } else {
                Some(target.to_path_buf())
            }
        }
        ConflictPolicy::Rename => {
            if !target.exists() {
                return Some(target.to_path_buf());
            }
            // 寻找 file_1.txt, file_2.txt, ... 直到不存在的名字
            let parent = target.parent().unwrap_or(Path::new("."));
            let file_name = target
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            // 拆分 stem 与扩展名 (含点), 例: "file.tar.gz" → ("file.tar", ".gz")
            let (stem, ext) = match file_name.rfind('.') {
                Some(idx) if idx > 0 => (&file_name[..idx], &file_name[idx..]),
                _ => (file_name.as_str(), ""),
            };
            for i in 1..u32::MAX {
                let new_name = if ext.is_empty() {
                    format!("{}_{}", stem, i)
                } else {
                    format!("{}_{}{}", stem, i, ext)
                };
                let new_path = parent.join(&new_name);
                if !new_path.exists() {
                    return Some(new_path);
                }
            }
            None
        }
    }
}

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
fn safe_join(base: &Path, name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    let clean: PathBuf = p.components()
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

// ───────────────────────── 错误恢复 ─────────────────────────

/// 错误恢复: 包装解压函数, 失败时清理已解压的半成品文件
///
/// `f` 在执行过程中应把已创建的文件路径 push 到 `extracted`.
/// 若 `f` 返回 `Err`, 此函数会清理 `extracted` 中的所有文件,
/// 以及由这些文件派生出的空目录 (从深到浅, 只删 output_dir 之内的空目录).
fn run_extract<F>(output_dir: &Path, f: F) -> Result<()>
where
    F: FnOnce(&mut Vec<PathBuf>) -> Result<()>,
{
    let mut extracted: Vec<PathBuf> = Vec::new();
    match f(&mut extracted) {
        Ok(()) => Ok(()),
        Err(e) => {
            // 清理半成品文件
            for path in &extracted {
                if path.is_file() {
                    let _ = std::fs::remove_file(path);
                }
            }
            // 清理空目录 (从深到浅, 仅删 output_dir 之内)
            let mut dirs: Vec<PathBuf> = extracted
                .iter()
                .filter_map(|p| p.parent().map(|p| p.to_path_buf()))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            dirs.sort_by(|a, b| b.components().count().cmp(&a.components().count()));
            for d in dirs {
                if d == output_dir || !d.starts_with(output_dir) {
                    continue;
                }
                // remove_dir 只删空目录, 非空会失败, 这里忽略错误
                let _ = std::fs::remove_dir(&d);
            }
            Err(e)
        }
    }
}

// ───────────────────────── Zip (多线程并行解压) ─────────────────────────

pub fn zip_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
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

    // 共享状态: 解压字节数 + 密码 + 已解压文件列表 (跨线程收集)
    let extracted_bytes = Arc::new(AtomicU64::new(0));
    let tracked_files: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    let pwd = opts.password.clone();
    let conflict = opts.conflict;

    // 分块并行处理 (每个 worker 独立打开 ZipArchive)
    let indices: Vec<usize> = (0..total).collect();
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let chunk_size = ((total + nthreads - 1) / nthreads).max(1);

    let tracked_ref = tracked_files.clone();
    indices.par_chunks(chunk_size).try_for_each(move |chunk| -> Result<()> {
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
                if let Some(p) = resolve_conflict(&outpath, conflict) {
                    std::fs::create_dir_all(&p)?;
                }
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                // 冲突处理: 跳过 / 重命名 / 覆盖
                let target_path = match resolve_conflict(&outpath, conflict) {
                    Some(p) => p,
                    None => {
                        bar.inc(1);
                        continue;
                    }
                };
                // 大缓冲区写入
                let mut outfile = BufWriter::with_capacity(BUF_SIZE, File::create(&target_path)?);
                // 文件创建成功后立即记录, 便于失败时清理半成品
                if let Ok(mut guard) = tracked_ref.lock() {
                    guard.push(target_path.clone());
                }
                let written = copy_large(&mut entry, &mut outfile)?;
                outfile.flush()?;
                extracted_bytes.fetch_add(written, Ordering::Relaxed);

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = entry.unix_mode() {
                        let _ = std::fs::set_permissions(
                            &target_path,
                            std::fs::Permissions::from_mode(mode),
                        );
                    }
                }
            }

            bar.inc(1);
        }
        Ok(())
    })?;

    // 合并并行收集的文件列表到外部
    if let Ok(mut guard) = tracked_files.lock() {
        extracted_files.append(&mut guard);
    }

    Ok(())
}

// ───────────────────────── 7z ─────────────────────────

pub fn sevenz_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    _extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    // sevenz-rust 要求输出目录预先存在
    std::fs::create_dir_all(output)?;
    // 注: sevenz-rust 库内部直接写文件, 无法应用 conflict 策略; 仅透传密码
    // 也不支持按文件追踪, 故 _extracted_files 不填充
    let result = if let Some(pwd) = opts.password.as_deref() {
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
    opts: &ExtractOptions,
    bar: &Progress,
    _extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    // 注: unrar 库内部直接写文件, 不支持按文件追踪, 故 _extracted_files 不填充
    rar::rar_decompress(input, output, opts.password.as_deref(), bar)
}

// ───────────────────────── Tar 系列 (缓冲区优化 + 炸弹检测) ─────────────────────────

fn tar_extract_with<R: Read>(
    reader: R,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let mut archive = tar::Archive::new(BufReader::with_capacity(BUF_SIZE, reader));
    let mut extracted_bytes: u64 = 0;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        // Bug 1 修复: 路径穿越防护 (tar slip), 不再直接 output.join(&path)
        let outpath = match safe_join(output, &path.to_string_lossy()) {
            Some(p) => p,
            None => {
                eprintln!("警告: 检测到路径穿越, 跳过: {}", path.display());
                continue;
            }
        };
        let entry_type = entry.header().entry_type();

        if entry_type.is_symlink() {
            // 符号链接条目
            if opts.preserve_symlinks {
                #[cfg(unix)]
                {
                    if let Some(parent) = outpath.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    match entry.link_name()? {
                        Some(target) => {
                            let target_path = target.as_ref();
                            // Bug 2 修复: symlink 目标消毒
                            // 拒绝绝对路径目标, 防止 symlink 指向敏感系统文件
                            let target_str = target_path.to_string_lossy();
                            if target_str.starts_with('/') || target_str.contains("..") {
                                eprintln!(
                                    "警告: 符号链接目标不安全, 跳过: {} -> {}",
                                    outpath.display(),
                                    target_str
                                );
                                continue;
                            }
                            if let Err(e) = std::os::unix::fs::symlink(target_path, &outpath) {
                                eprintln!(
                                    "警告: 创建符号链接失败 {}: {}",
                                    outpath.display(),
                                    e
                                );
                            } else {
                                // Bug 18 修复: symlink 也记录到 extracted_files, 便于失败时清理
                                extracted_files.push(outpath.clone());
                            }
                        }
                        None => {
                            eprintln!(
                                "警告: 符号链接条目缺少目标, 跳过: {}",
                                outpath.display()
                            );
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = outpath;
                    eprintln!(
                        "警告: Windows 不支持符号链接, 跳过: {}",
                        entry.path()?.display()
                    );
                }
            }
            // preserve_symlinks == false: 静默跳过
        } else if entry_type.is_dir() {
            // 目录条目
            if let Some(p) = resolve_conflict(&outpath, opts.conflict) {
                std::fs::create_dir_all(&p)?;
            }
        } else {
            // 普通文件条目
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let target_path = match resolve_conflict(&outpath, opts.conflict) {
                Some(p) => p,
                None => {
                    bar.inc(1);
                    continue;
                }
            };
            // 大缓冲区写入, 减少 syscall
            let mut outfile =
                BufWriter::with_capacity(BUF_SIZE, File::create(&target_path)?);
            // 文件创建成功后立即记录, 便于失败时清理半成品
            extracted_files.push(target_path.clone());
            let written = copy_large(&mut entry, &mut outfile)?;
            outfile.flush()?;
            extracted_bytes += written;

            // 炸弹检测
            check_bomb(extracted_bytes, 0)?;

            // 保留文件权限
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(mode) = entry.header().mode() {
                    let _ = std::fs::set_permissions(
                        &target_path,
                        std::fs::Permissions::from_mode(mode),
                    );
                }
            }

            // 保留 mtime (从 header 读取, 用 filetime 写回)
            if let Ok(mtime) = entry.header().mtime() {
                let _ = filetime::set_file_mtime(
                    &target_path,
                    filetime::FileTime::from_unix_time(mtime as i64, 0),
                );
            }
        }
        // Bug 10 修复: 不再把 total 设成当前 count (会导致进度条恒 100%)
        // tar 是流式格式无法预知总数, 仅 inc 不 set_total (前端显示 indeterminate)
        bar.inc(1);
    }
    Ok(())
}

pub fn tar_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    tar_extract_with(File::open(input)?, output, opts, bar, extracted_files)
}

pub fn targz_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let gz = flate2::read::GzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(input)?));
    tar_extract_with(gz, output, opts, bar, extracted_files)
}

pub fn tarxz_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let xz = xz2::read::XzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(input)?));
    tar_extract_with(xz, output, opts, bar, extracted_files)
}

pub fn tarzst_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let zst = zstd::Decoder::new(File::open(input)?)
        .map_err(|e| anyhow::anyhow!("zstd 解码器初始化失败: {}", e))?;
    tar_extract_with(zst, output, opts, bar, extracted_files)
}

pub fn tarbz2_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let bz2 = bzip2::read::BzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(input)?));
    tar_extract_with(bz2, output, opts, bar, extracted_files)
}

pub fn tarlz4_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let lz4 = lz4::Decoder::new(File::open(input)?)
        .map_err(|e| anyhow::anyhow!("lz4 解码器初始化失败: {}", e))?;
    tar_extract_with(lz4, output, opts, bar, extracted_files)
}

// ───────────────────────── 单文件流解压 (1MB 缓冲区) ─────────────────────────

pub fn single_decompress(
    input: &Path,
    output: &Path,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
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

    // 冲突处理: 跳过 / 重命名 / 覆盖
    let target_path = match resolve_conflict(&out_file, opts.conflict) {
        Some(p) => p,
        None => {
            bar.inc(1);
            return Ok(());
        }
    };

    // 1MB 大缓冲区
    let reader = BufReader::with_capacity(BUF_SIZE, File::open(input)?);
    let mut writer = BufWriter::with_capacity(BUF_SIZE, File::create(&target_path)?);
    // 文件创建成功后立即记录, 便于失败时清理半成品
    extracted_files.push(target_path.clone());

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
    decompress_with(input, output, container, &ExtractOptions::default(), bar)
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
    let opts = ExtractOptions {
        password: password.map(|s| s.to_string()),
        ..Default::default()
    };
    decompress_with(input, output, container, &opts, bar)
}

/// 内部分发: 把 container 路由到具体解压函数, 并把已解压文件收集到 `extracted_files`
fn decompress_inner(
    input: &Path,
    output: &Path,
    container: Container,
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    match container {
        Container::Zip => zip_decompress(input, output, opts, bar, extracted_files),
        Container::SevenZ => sevenz_decompress(input, output, opts, bar, extracted_files),
        Container::Rar => rar_decompress(input, output, opts, bar, extracted_files),
        Container::Tar => tar_decompress(input, output, opts, bar, extracted_files),
        Container::TarGz => targz_decompress(input, output, opts, bar, extracted_files),
        Container::TarXz => tarxz_decompress(input, output, opts, bar, extracted_files),
        Container::TarZst => tarzst_decompress(input, output, opts, bar, extracted_files),
        Container::TarBz2 => tarbz2_decompress(input, output, opts, bar, extracted_files),
        Container::TarLz4 => tarlz4_decompress(input, output, opts, bar, extracted_files),
        Container::Single => single_decompress(input, output, opts, bar, extracted_files),
    }
}

pub fn decompress_with(
    input: &Path,
    output: &Path,
    container: Container,
    opts: &ExtractOptions,
    bar: &Progress,
) -> Result<()> {
    if opts.cleanup_on_error {
        // 启用错误恢复: 失败时清理半成品
        run_extract(output, |extracted| {
            decompress_inner(input, output, container, opts, bar, extracted)
        })
    } else {
        // 不启用错误恢复: 直接解压
        let mut dummy: Vec<PathBuf> = Vec::new();
        decompress_inner(input, output, container, opts, bar, &mut dummy)
    }
}

// ───────────────────────── 部分解压 ─────────────────────────

/// 部分解压: 只解压归档中指定的文件/目录
///
/// - ZIP: 遍历所有条目, 匹配 `files_to_extract` 中的路径或以其为前缀 (目录) 时解压
/// - Tar 系列: 同上, 遍历 entries 匹配
/// - 7z / RAR: 底层库不支持按条目解压, 返回错误
/// - Single: 单文件压缩无归档结构, 返回错误
///
/// 注: 此函数不使用并行解压 (适合小批量), 也不推进 GUI 进度条.
pub fn extract_partial(
    archive_path: &Path,
    output_dir: &Path,
    files_to_extract: &[String],
    opts: &ExtractOptions,
) -> Result<()> {
    if files_to_extract.is_empty() {
        return Err(anyhow::anyhow!("未指定要解压的文件列表"));
    }

    let container = detect(archive_path).ok_or_else(|| {
        anyhow::anyhow!(
            "无法识别归档格式: {} (支持 .zip / .tar / .tar.gz / .tar.xz / .tar.zst / .tar.bz2 / .tar.lz4)",
            archive_path.display()
        )
    })?;

    let bar = Progress::new("extract_partial");

    if opts.cleanup_on_error {
        run_extract(output_dir, |extracted| {
            extract_partial_inner(archive_path, output_dir, files_to_extract, opts, container, &bar, extracted)
        })
    } else {
        let mut dummy: Vec<PathBuf> = Vec::new();
        extract_partial_inner(archive_path, output_dir, files_to_extract, opts, container, &bar, &mut dummy)
    }
}

fn extract_partial_inner(
    archive_path: &Path,
    output_dir: &Path,
    files_to_extract: &[String],
    opts: &ExtractOptions,
    container: Container,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    match container {
        Container::Zip => extract_partial_zip(archive_path, output_dir, files_to_extract, opts, bar, extracted_files),
        Container::Tar => extract_partial_tar(File::open(archive_path)?, output_dir, files_to_extract, opts, bar, extracted_files),
        Container::TarGz => extract_partial_tar(
            flate2::read::GzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(archive_path)?)),
            output_dir, files_to_extract, opts, bar, extracted_files,
        ),
        Container::TarXz => extract_partial_tar(
            xz2::read::XzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(archive_path)?)),
            output_dir, files_to_extract, opts, bar, extracted_files,
        ),
        Container::TarZst => {
            let dec = zstd::Decoder::new(File::open(archive_path)?)
                .map_err(|e| anyhow::anyhow!("zstd 解码器初始化失败: {}", e))?;
            extract_partial_tar(dec, output_dir, files_to_extract, opts, bar, extracted_files)
        }
        Container::TarBz2 => extract_partial_tar(
            bzip2::read::BzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(archive_path)?)),
            output_dir, files_to_extract, opts, bar, extracted_files,
        ),
        Container::TarLz4 => {
            let dec = lz4::Decoder::new(File::open(archive_path)?)
                .map_err(|e| anyhow::anyhow!("lz4 解码器初始化失败: {}", e))?;
            extract_partial_tar(dec, output_dir, files_to_extract, opts, bar, extracted_files)
        }
        Container::SevenZ => Err(anyhow::anyhow!("7z 不支持部分解压 (底层库限制)")),
        Container::Rar => Err(anyhow::anyhow!("RAR 不支持部分解压 (底层库限制)")),
        Container::Single => Err(anyhow::anyhow!("单文件压缩格式不支持部分解压 (无归档结构)")),
    }
}

/// 判断归档内某条目是否应被解压:
/// - 精确匹配 files_to_extract 中的某项, 或
/// - 以 files_to_extract 中某项 + "/" 为前缀 (目录匹配, 递归包含子项)
fn should_extract(name: &str, files_to_extract: &[String]) -> bool {
    for want in files_to_extract {
        // 精确匹配
        if name == want.as_str() {
            return true;
        }
        // 目录前缀匹配: want 是 name 的父目录
        // 归一化: 确保 want 以 / 结尾再比较, 避免误匹配 (如 want="doc" 不应匹配 "docs/x")
        let want_dir = if want.ends_with('/') {
            want.clone()
        } else {
            format!("{}/", want)
        };
        if name.starts_with(&want_dir) {
            return true;
        }
    }
    false
}

/// ZIP 部分解压
fn extract_partial_zip(
    archive_path: &Path,
    output: &Path,
    files_to_extract: &[String],
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("打开文件失败: {}", archive_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let total = archive.len();
    bar.set_total(total as u64);

    let pwd_bytes = opts.password.as_deref().map(|s| s.as_bytes());
    let conflict = opts.conflict;

    for i in 0..archive.len() {
        let mut entry = if let Some(pwd) = pwd_bytes {
            archive.by_index_decrypt(i, pwd)
                .map_err(|e| anyhow::anyhow!("ZIP 条目解密失败 (index {}): {}", i, e))?
        } else {
            archive.by_index(i)
                .map_err(|e| anyhow::anyhow!("ZIP 条目读取失败 (index {}): {}", i, e))?
        };

        let name = entry.name().to_string();

        // Bug 12 修复: 使用统一的 should_extract 函数 (正确处理尾部斜杠)
        if !should_extract(&name, files_to_extract) {
            bar.inc(1);
            continue;
        }

        let outpath = match safe_join(output, &name) {
            Some(p) => p,
            None => {
                bar.inc(1);
                continue;
            }
        };

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)?;
            bar.inc(1);
            continue;
        }

        if let Some(parent) = outpath.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let target_path = match resolve_conflict(&outpath, conflict) {
            Some(p) => p,
            None => {
                bar.inc(1);
                continue;
            }
        };

        let mut writer = BufWriter::with_capacity(BUF_SIZE, File::create(&target_path)?);
        extracted_files.push(target_path.clone());
        copy_large(&mut entry, &mut writer)?;
        writer.flush()?;

        // 保留权限和 mtime
        #[cfg(unix)]
        {
            if let Some(mode) = entry.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(mode));
            }
        }
        if let Some(dt) = entry.last_modified() {
            if let Ok(mtime) = dt.to_time() {
                let _ = filetime::set_file_mtime(
                    &target_path,
                    filetime::FileTime::from_unix_time(mtime.unix_timestamp(), 0),
                );
            }
        }

        bar.inc(1);
    }

    bar.finish("done");
    Ok(())
}

/// Tar 系列部分解压 (泛型: 接受任何 Read)
fn extract_partial_tar<R: Read>(
    reader: R,
    output: &Path,
    files_to_extract: &[String],
    opts: &ExtractOptions,
    bar: &Progress,
    extracted_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let mut archive = tar::Archive::new(reader);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let name = entry.path()?.to_string_lossy().to_string();

        // Bug 12 修复: 使用统一的 should_extract 函数 (正确处理尾部斜杠)
        if !should_extract(&name, files_to_extract) {
            continue;
        }

        let outpath = match safe_join(output, &name) {
            Some(p) => p,
            None => continue,
        };

        let header = entry.header();

        // 符号链接
        if header.entry_type().is_symlink() && opts.preserve_symlinks {
            #[cfg(unix)]
            {
                if let Some(link_name) = header.link_name()? {
                    let _ = std::os::unix::fs::symlink(link_name, &outpath);
                }
            }
            continue;
        }

        if header.entry_type().is_dir() {
            let target = match resolve_conflict(&outpath, opts.conflict) {
                Some(p) => p,
                None => continue,
            };
            std::fs::create_dir_all(&target)?;
            continue;
        }

        // 普通文件
        if let Some(parent) = outpath.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let target_path = match resolve_conflict(&outpath, opts.conflict) {
            Some(p) => p,
            None => continue,
        };

        // 先读取 mode/mtime (避免与 copy_large 的可变借用冲突)
        #[cfg(unix)]
        let saved_mode = header.mode().ok();
        let saved_mtime = header.mtime().ok();

        let mut writer = BufWriter::with_capacity(BUF_SIZE, File::create(&target_path)?);
        extracted_files.push(target_path.clone());
        copy_large(&mut entry, &mut writer)?;
        writer.flush()?;

        // 保留权限和 mtime
        #[cfg(unix)]
        {
            if let Some(mode) = saved_mode {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(mode));
            }
        }
        if let Some(mtime) = saved_mtime {
            let _ = filetime::set_file_mtime(
                &target_path,
                filetime::FileTime::from_unix_time(mtime as i64, 0),
            );
        }
    }

    bar.finish("done");
    Ok(())
}

// ───────────────────────── 归档完整性测试 ─────────────────────────

/// 测试归档完整性 (不解压到磁盘, 仅校验数据流)
///
/// 返回 (条目数, 总字节数)
pub fn test_archive(path: &Path, password: Option<&str>, bar: &Progress) -> Result<(usize, u64)> {
    let container = detect(path).ok_or_else(|| {
        anyhow::anyhow!("无法识别归档格式: {}", path.display())
    })?;

    match container {
        Container::Zip => test_zip(path, password, bar),
        Container::Tar => test_tar(File::open(path)?, bar),
        Container::TarGz => test_tar(
            flate2::read::GzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(path)?)),
            bar,
        ),
        Container::TarXz => test_tar(
            xz2::read::XzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(path)?)),
            bar,
        ),
        Container::TarZst => {
            let dec = zstd::Decoder::new(File::open(path)?)
                .map_err(|e| anyhow::anyhow!("zstd 解码器初始化失败: {}", e))?;
            test_tar(dec, bar)
        }
        Container::TarBz2 => test_tar(
            bzip2::read::BzDecoder::new(BufReader::with_capacity(BUF_SIZE, File::open(path)?)),
            bar,
        ),
        Container::TarLz4 => {
            let dec = lz4::Decoder::new(File::open(path)?)
                .map_err(|e| anyhow::anyhow!("lz4 解码器初始化失败: {}", e))?;
            test_tar(dec, bar)
        }
        Container::SevenZ => test_7z(path, password, bar),
        Container::Rar => test_rar(path, password, bar),
        Container::Single => {
            // Bug 4 修复: 用唯一临时目录避免泄漏, 测试后清理
            let tmp_dir = std::env::temp_dir().join(format!(
                "smartex_test_single_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&tmp_dir)?;
            let mut dummy: Vec<PathBuf> = Vec::new();
            let opts = ExtractOptions {
                password: password.map(|s| s.to_string()),
                ..Default::default()
            };
            let result = single_decompress(path, &tmp_dir, &opts, bar, &mut dummy);
            // 无论成功失败都清理临时目录
            let _ = std::fs::remove_dir_all(&tmp_dir);
            result?;
            Ok((1, 0))
        }
    }
}

fn test_zip(path: &Path, password: Option<&str>, bar: &Progress) -> Result<(usize, u64)> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let total = archive.len();
    bar.set_total(total as u64);

    let pwd_bytes = password.map(|s| s.as_bytes());
    let mut total_bytes = 0u64;

    for i in 0..archive.len() {
        let mut entry = if let Some(pwd) = pwd_bytes {
            archive.by_index_decrypt(i, pwd)
                .map_err(|e| anyhow::anyhow!("ZIP 条目解密失败 (index {}): {}", i, e))?
        } else {
            archive.by_index(i)
                .map_err(|e| anyhow::anyhow!("ZIP 条目读取失败 (index {}): {}", i, e))?
        };

        let mut buf = [0u8; BUF_SIZE];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 { break; }
            total_bytes += n as u64;
        }
        bar.inc(1);
    }

    bar.finish("done");
    Ok((total, total_bytes))
}

fn test_tar<R: Read>(reader: R, bar: &Progress) -> Result<(usize, u64)> {
    let mut archive = tar::Archive::new(reader);
    let mut count = 0usize;
    let mut total_bytes = 0u64;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let mut buf = [0u8; BUF_SIZE];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 { break; }
            total_bytes += n as u64;
        }
        count += 1;
    }

    bar.set_total(count as u64);
    bar.finish("done");
    Ok((count, total_bytes))
}

fn test_7z(path: &Path, password: Option<&str>, bar: &Progress) -> Result<(usize, u64)> {
    // Bug 5 修复: 用唯一临时目录避免并发冲突
    let tmp = std::env::temp_dir().join(format!(
        "smartex_test_7z_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let result = if let Some(pwd) = password {
        sevenz_rust::decompress_file_with_password(path, &tmp, pwd.into())
    } else {
        sevenz_rust::decompress_file(path, &tmp)
    };

    match result {
        Ok(()) => {
            let count = count_files_recursive(&tmp);
            let bytes = dir_size(&tmp);
            let _ = std::fs::remove_dir_all(&tmp);
            bar.set_total(count as u64);
            bar.finish("done");
            Ok((count, bytes))
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            Err(anyhow::anyhow!("7z 测试失败: {}", e))
        }
    }
}

fn test_rar(path: &Path, password: Option<&str>, bar: &Progress) -> Result<(usize, u64)> {
    // Bug 5 修复: 用唯一临时目录避免并发冲突
    let tmp = std::env::temp_dir().join(format!(
        "smartex_test_rar_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    crate::rar::rar_decompress(path, &tmp, password, bar)?;
    let count = count_files_recursive(&tmp);
    let bytes = dir_size(&tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    Ok((count, bytes))
}

/// 递归计算目录下文件数
fn count_files_recursive(dir: &Path) -> usize {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

/// 递归计算目录大小
fn dir_size(dir: &Path) -> u64 {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}