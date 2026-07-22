use crate::format::{default_archive_name, Container};
use crate::progress::Progress;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 递归收集目录下所有文件 (跳过根目录自身)
fn collect_files(input: &Path) -> Result<Vec<PathBuf>> {
    if input.is_file() {
        return Ok(vec![input.to_path_buf()]);
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(input).min_depth(1) {
        let entry = entry?;
        if entry.path().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(files)
}

/// 收集所有条目 (含目录), 用于 tar 系列
fn collect_entries(input: &Path) -> Result<Vec<PathBuf>> {
    if input.is_file() {
        return Ok(vec![input.to_path_buf()]);
    }
    let mut entries = Vec::new();
    for entry in WalkDir::new(input).min_depth(1) {
        let entry = entry?;
        entries.push(entry.path().to_path_buf());
    }
    Ok(entries)
}

fn base_dir(input: &Path) -> &Path {
    if input.is_dir() {
        input
    } else {
        input.parent().unwrap_or(Path::new("."))
    }
}

// ───────────────────────── Zip ─────────────────────────

pub fn zip_compress(
    input: &Path,
    output: &Path,
    level: i32,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    let file = File::create(output).with_context(|| format!("创建文件失败: {}", output.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    // 使用 zstd 作为默认算法: 速度远超 deflate, 压缩比更优
    let method = if level <= 0 {
        zip::CompressionMethod::Stored
    } else {
        zip::CompressionMethod::Zstd
    };
    let mut options = zip::write::SimpleFileOptions::default()
        .compression_method(method)
        .compression_level(Some(level as i64));

    // 如果有密码, 启用 AES-256 加密 (兼容 7-Zip/WinRAR/Bandizip)
    if let Some(pwd) = password {
        options = options.with_aes_encryption(zip::AesMode::Aes256, pwd);
    }

    let entries = collect_entries(input)?;
    bar.set_total(entries.len() as u64);
    let base = base_dir(input);

    for entry in entries {
        let relative = entry.strip_prefix(base).unwrap_or(&entry);
        let name = relative.to_string_lossy().replace('\\', "/");
        if entry.is_dir() {
            zip.add_directory(format!("{}/", name), options)?;
        } else {
            zip.start_file(&name, options)?;
            let mut f = File::open(&entry)?;
            io::copy(&mut f, &mut zip)?;
        }
        bar.inc(1);
    }
    zip.finish()?;
    Ok(())
}

// ───────────────────────── 7z ─────────────────────────

pub fn sevenz_compress(
    input: &Path,
    output: &Path,
    _level: i32,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    let entries = collect_files(input)?;
    bar.set_total(entries.len() as u64);

    let result = if let Some(pwd) = password {
        sevenz_rust::compress_to_path_encrypted(input, output, pwd.into())
    } else {
        sevenz_rust::compress_to_path(input, output)
    };
    result.map_err(|e| anyhow::anyhow!("7z 压缩失败: {}", e))?;

    // sevenz-rust 一次性压缩, 进度直接置满
    for _ in 0..entries.len() {
        bar.inc(1);
    }
    Ok(())
}

// ───────────────────────── Tar 系列 ─────────────────────────

fn tar_with_encoder<W: Write>(input: &Path, encoder: W, bar: &Progress) -> Result<()>
where
    W: Write + Send,
{
    let mut tar = tar::Builder::new(encoder);
    let entries = collect_entries(input)?;
    bar.set_total(entries.len() as u64);
    let base = base_dir(input);

    for entry in entries {
        let relative = entry.strip_prefix(base).unwrap_or(&entry);
        if entry.is_dir() {
            tar.append_dir(relative, &entry)?;
        } else {
            tar.append_file(relative, &mut File::open(&entry)?)?;
        }
        bar.inc(1);
    }
    tar.finish()?;
    Ok(())
}

pub fn tar_compress(input: &Path, output: &Path, _level: i32, _pwd: Option<&str>, bar: &Progress) -> Result<()> {
    let f = File::create(output)?;
    tar_with_encoder(input, f, bar)
}

pub fn targz_compress(input: &Path, output: &Path, level: i32, _pwd: Option<&str>, bar: &Progress) -> Result<()> {
    let f = File::create(output)?;
    let enc = flate2::write::GzEncoder::new(
        f,
        flate2::Compression::new(level.clamp(1, 9) as u32),
    );
    tar_with_encoder(input, enc, bar)
}

pub fn tarxz_compress(input: &Path, output: &Path, level: i32, _pwd: Option<&str>, bar: &Progress) -> Result<()> {
    let f = File::create(output)?;
    let enc = xz2::write::XzEncoder::new(f, level.clamp(0, 9) as u32);
    tar_with_encoder(input, enc, bar)
}

pub fn tarzst_compress(input: &Path, output: &Path, level: i32, _pwd: Option<&str>, bar: &Progress) -> Result<()> {
    let f = File::create(output)?;
    // zstd level 范围 1-22, 我们将 0-9 映射到 1-19
    let zst_level = (level.max(1) * 2).min(19) as i32;
    let enc = zstd::stream::Encoder::new(f, zst_level)
        .map_err(|e| anyhow::anyhow!("zstd 编码器初始化失败: {}", e))?
        .auto_finish();
    tar_with_encoder(input, enc, bar)
}

pub fn tarbz2_compress(input: &Path, output: &Path, level: i32, _pwd: Option<&str>, bar: &Progress) -> Result<()> {
    let f = File::create(output)?;
    let enc = bzip2::write::BzEncoder::new(f, bzip2::Compression::new(level.clamp(1, 9) as u32));
    tar_with_encoder(input, enc, bar)
}

pub fn tarlz4_compress(input: &Path, output: &Path, level: i32, _pwd: Option<&str>, bar: &Progress) -> Result<()> {
    let f = File::create(output)?;
    let mut enc = lz4::EncoderBuilder::new()
        .level(level.clamp(1, 12) as u32)
        .build(f)
        .map_err(|e| anyhow::anyhow!("lz4 编码器初始化失败: {}", e))?;
    // lz4 Encoder 实现了 Write 但不能复用 tar_with_encoder 因为需要 auto_finish
    {
        let mut tar = tar::Builder::new(&mut enc);
        let entries = collect_entries(input)?;
        bar.set_total(entries.len() as u64);
        let base = base_dir(input);
        for entry in entries {
            let relative = entry.strip_prefix(base).unwrap_or(&entry);
            if entry.is_dir() {
                tar.append_dir(relative, &entry)?;
            } else {
                tar.append_file(relative, &mut File::open(&entry)?)?;
            }
            bar.inc(1);
        }
        tar.finish()?;
    }
    let (_w, result) = enc.finish();
    result.map_err(|e| anyhow::anyhow!("lz4 完成失败: {}", e))?;
    Ok(())
}

// ───────────────────────── 单文件流压缩 ─────────────────────────

pub fn single_compress(
    input: &Path,
    output: &Path,
    container: Container,
    level: i32,
    _pwd: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    bar.set_total(1);
    let reader = BufReader::new(File::open(input)?);
    let writer = BufWriter::new(File::create(output)?);

    match container {
        Container::Single => {
            // 根据输出扩展名判断算法
            let out_name = output
                .file_name()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if out_name.ends_with(".gz") {
                let mut enc = flate2::write::GzEncoder::new(
                    writer,
                    flate2::Compression::new(level.clamp(1, 9) as u32),
                );
                let mut r = reader;
                io::copy(&mut r, &mut enc)?;
                enc.finish()?;
            } else if out_name.ends_with(".xz") {
                let mut enc = xz2::write::XzEncoder::new(writer, level.clamp(0, 9) as u32);
                let mut r = reader;
                io::copy(&mut r, &mut enc)?;
                enc.finish()?;
            } else if out_name.ends_with(".zst") {
                let zst_level = (level.max(1) * 2).min(19) as i32;
                let enc = zstd::stream::Encoder::new(writer, zst_level)?
                    .auto_finish();
                let mut r = reader;
                let mut enc = enc;
                io::copy(&mut r, &mut enc)?;
            } else if out_name.ends_with(".bz2") {
                let mut enc =
                    bzip2::write::BzEncoder::new(writer, bzip2::Compression::new(level.clamp(1, 9) as u32));
                let mut r = reader;
                io::copy(&mut r, &mut enc)?;
                enc.finish()?;
            } else if out_name.ends_with(".lz4") {
                let mut enc = lz4::EncoderBuilder::new()
                    .level(level.clamp(1, 12) as u32)
                    .build(writer)?;
                let mut r = reader;
                io::copy(&mut r, &mut enc)?;
                let (_w, res) = enc.finish();
                res?;
            } else {
                return Err(anyhow::anyhow!("无法识别的单文件输出扩展名: {}", out_name));
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "single_compress 仅接受 Container::Single"
            ));
        }
    }
    bar.inc(1);
    Ok(())
}

// ───────────────────────── 统一分发 ─────────────────────────

pub fn compress(
    input: &Path,
    output: &Path,
    container: Container,
    level: i32,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    match container {
        Container::Zip => zip_compress(input, output, level, password, bar),
        Container::SevenZ => sevenz_compress(input, output, level, password, bar),
        Container::Tar => tar_compress(input, output, level, password, bar),
        Container::TarGz => targz_compress(input, output, level, password, bar),
        Container::TarXz => tarxz_compress(input, output, level, password, bar),
        Container::TarZst => tarzst_compress(input, output, level, password, bar),
        Container::TarBz2 => tarbz2_compress(input, output, level, password, bar),
        Container::TarLz4 => tarlz4_compress(input, output, level, password, bar),
        Container::Single => single_compress(input, output, container, level, password, bar),
        // RAR 为闭源格式, 仅支持解压, 不支持压缩
        Container::Rar => Err(anyhow::anyhow!("RAR 为闭源格式, 仅支持解压, 暂不支持压缩")),
    }
}

/// 默认输出路径
pub fn default_output(input: &Path, container: Container) -> PathBuf {
    default_archive_name(input, container)
}

// 保留 Read trait import 以供未来扩展使用
#[allow(dead_code)]
fn _ensure_read_import() -> Option<Box<dyn Read>> {
    None
}
