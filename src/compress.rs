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
    // 使用 Deflate 算法 (兼容性最好: 7-Zip/WinRAR/Bandizip/系统自带)
    // level 0 = Stored (仅打包), 1-9 = Deflate 压缩级别
    let method = if level <= 0 {
        zip::CompressionMethod::Stored
    } else {
        zip::CompressionMethod::Deflated
    };
    let mut options = zip::write::SimpleFileOptions::default()
        .compression_method(method)
        .compression_level(Some(level.clamp(0, 9) as i64));

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
    level: i32,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    let entries = collect_files(input)?;
    bar.set_total(entries.len() as u64);

    // 将 smart_ex 的 level (0-12) 映射到 LZMA2 preset (0-9)
    // level 0 = COPY (仅打包), 1-9 = LZMA2 preset 1-9
    let preset = level.clamp(0, 9) as u32;

    // 使用 SevenZWriter Builder API 自定义压缩级别
    // (compress_to_path 不支持自定义 level, 硬编码 preset 6)
    let mut sz = sevenz_rust::SevenZWriter::create(output)
        .map_err(|e| anyhow::anyhow!("7z 创建 writer 失败: {}", e))?;

    if let Some(pwd) = password {
        // 加密模式: AES 在前, LZMA2 在后 (顺序很重要!)
        sz.set_content_methods(vec![
            sevenz_rust::AesEncoderOptions::new(pwd.into()).into(),
            sevenz_rust::lzma::LZMA2Options::with_preset(preset).into(),
        ]);
        sz.set_encrypt_header(true);
    } else {
        if preset == 0 {
            // level 0: 不压缩, 仅打包 (COPY)
            sz.set_content_methods(vec![sevenz_rust::SevenZMethod::COPY.into()]);
        } else {
            sz.set_content_methods(vec![
                sevenz_rust::lzma::LZMA2Options::with_preset(preset).into(),
            ]);
        }
    }

    // solid 模式: 多文件一次性压缩, 压缩比更高
    sz.push_source_path(input, |_| true)
        .map_err(|e| anyhow::anyhow!("7z 压缩失败: {}", e))?;

    sz.finish()
        .map_err(|e| anyhow::anyhow!("7z 完成失败: {}", e))?;

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
    // zstd level 范围 1-22, smart_ex level 0-12 映射到 zstd 1-19
    // level 0 → zstd 1 (最快), level 3 → zstd 3, level 9 → zstd 19 (最高)
    let zst_level = if level <= 0 {
        1
    } else {
        // 线性映射: 1-9 → 1-19, 10-12 → 20-22
        if level <= 9 {
            level
        } else {
            19 + (level - 9)
        }
    };
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
                let zst_level = if level <= 0 {
                    1
                } else if level <= 9 {
                    level
                } else {
                    19 + (level - 9)
                };
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
