use anyhow::Result;
use clap::Parser;
use smartex_core::format::Container;
use smartex_core::i18n::Lang;
use smartex_core::progress::Progress;
use std::path::PathBuf;

mod cli;
use cli::{Cli, Commands};

use smartex_core::{archive_list, compress, crypto, decompress, format, i18n, progress};

fn main() -> Result<()> {
    let args = Cli::parse();

    // 初始化语言 (zh / en)
    i18n::set_lang(Lang::from_code(&args.lang));

    // 无子命令 + --gui 或完全无参数 → GUI 模式 (Tauri 版本尚未就绪)
    if args.gui || args.command.is_none() {
        println!("GUI 模式将在 Tauri 版本中提供, 请使用 CLI 命令 (smart_ex --help)");
        return Ok(());
    }

    let cmd = args.command.unwrap();
    match cmd {
        Commands::Gui => {
            println!("GUI 模式将在 Tauri 版本中提供, 请使用 CLI 命令 (smart_ex --help)");
            Ok(())
        }
        Commands::Compress {
            input,
            output,
            format,
            level,
            password,
            exclude,
            split,
        } => run_compress(input, output, format, level, password, exclude, split),
        Commands::Decompress {
            input,
            output,
            password,
        } => run_decompress(input, output, password),
        Commands::ExtractHere { input, password } => run_extract_here(input, password),
        Commands::ExtractAs { input, output, password } => run_extract_as(input, output, password),
        Commands::Encrypt {
            input,
            output,
            password,
        } => run_encrypt(input, output, password),
        Commands::Decrypt {
            input,
            output,
            password,
        } => run_decrypt(input, output, password),
        Commands::Smart {
            input,
            output,
            password,
            format,
        } => run_smart(input, output, password, format),
        Commands::List { input, password } => run_list(input, password),
        Commands::Test { input, password } => run_test(input, password),
    }
}

fn parse_container(s: &str) -> Result<Container> {
    Ok(match s.to_lowercase().as_str() {
        "zip" => Container::Zip,
        "7z" => Container::SevenZ,
        "tar" => Container::Tar,
        "tar.gz" | "tgz" => Container::TarGz,
        "tar.xz" | "txz" => Container::TarXz,
        "tar.zst" | "tzst" => Container::TarZst,
        "tar.bz2" | "tbz2" => Container::TarBz2,
        "tar.lz4" => Container::TarLz4,
        // 单文件流格式 (仅对单文件输入有效, 算法由输出扩展名决定)
        "gz" | "xz" | "zst" | "bz2" | "lz4" => Container::Single,
        other => return Err(anyhow::anyhow!("不支持的格式: {}", other)),
    })
}

/// 根据输入和期望输出扩展名, 推断最适合的容器
fn infer_container(input: &std::path::Path, format: &Option<String>, output: &Option<PathBuf>) -> Container {
    if let Some(f) = format {
        if let Ok(c) = parse_container(f) {
            // 单文件流仅对单文件输入有效; 目录输入则升级为 tar 系列
            if c == Container::Single && input.is_dir() {
                let ext = f.to_lowercase();
                return match ext.as_str() {
                    "gz" => Container::TarGz,
                    "xz" => Container::TarXz,
                    "zst" => Container::TarZst,
                    "bz2" => Container::TarBz2,
                    "lz4" => Container::TarLz4,
                    _ => Container::Zip,
                };
            }
            return c;
        }
    }
    // 无 -f: 按输出扩展名推断, 否则默认 zip
    if let Some(out) = output {
        if let Some(c) = format::detect(out) {
            // 单文件流仅对单文件输入有效; 目录输入则升级为 tar 系列
            if c == Container::Single && input.is_dir() {
                let name = out
                    .file_name()
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                return if name.ends_with(".gz") {
                    Container::TarGz
                } else if name.ends_with(".xz") {
                    Container::TarXz
                } else if name.ends_with(".zst") {
                    Container::TarZst
                } else if name.ends_with(".bz2") {
                    Container::TarBz2
                } else if name.ends_with(".lz4") {
                    Container::TarLz4
                } else {
                    Container::TarZst
                };
            }
            return c;
        }
    }
    Container::Zip
}

/// 确保输出路径带有可识别的归档扩展名; 若无, 按 container 补全
/// (单文件流容器则按 format 参数补全 .gz/.xz/.zst/.bz2/.lz4)
fn ensure_archive_extension(
    output: &std::path::Path,
    container: Container,
    format: Option<&str>,
) -> PathBuf {
    if format::detect(output).is_some() {
        return output.to_path_buf();
    }
    let parent = output.parent().unwrap_or(std::path::Path::new("."));
    let stem = output
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "archive".to_string());
    let ext = if container == Container::Single {
        format
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "zst".to_string())
    } else {
        container.extension().to_string()
    };
    parent.join(format!("{}.{}", stem, ext))
}

/// 当容器是 tar 系列但输出扩展名是单文件流 (如 .zst), 纠正为 .tar.xxx
/// 避免解压时格式识别错配
fn normalize_output_ext(output: &std::path::Path, container: Container) -> PathBuf {
    let name = output
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = name.to_lowercase();

    let needs_fix = match container {
        Container::TarGz => lower.ends_with(".gz") && !lower.ends_with(".tar.gz"),
        Container::TarXz => lower.ends_with(".xz") && !lower.ends_with(".tar.xz"),
        Container::TarZst => lower.ends_with(".zst") && !lower.ends_with(".tar.zst"),
        Container::TarBz2 => lower.ends_with(".bz2") && !lower.ends_with(".tar.bz2"),
        Container::TarLz4 => lower.ends_with(".lz4") && !lower.ends_with(".tar.lz4"),
        _ => false,
    };

    if needs_fix {
        // 在最后扩展名前插入 .tar
        let parent = output.parent().unwrap_or(std::path::Path::new("."));
        let stem = output
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = output
            .extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let new_name = format!("{}.tar.{}", stem, ext);
        parent.join(new_name)
    } else {
        output.to_path_buf()
    }
}

fn run_compress(
    input: PathBuf,
    output: Option<PathBuf>,
    format: Option<String>,
    level: i32,
    password: Option<String>,
    exclude: Vec<String>,
    split: Option<String>,
) -> Result<()> {
    let container = infer_container(&input, &format, &output);
    // 单文件流: 若用户未指定输出, 根据格式名生成扩展名
    let out = match &output {
        Some(o) => normalize_output_ext(o, container),
        None => {
            if container == Container::Single {
                let ext = format.as_deref().map(|s| s.to_lowercase()).unwrap_or_else(|| "zst".to_string());
                let mut name = input
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "archive".to_string());
                name.push('.');
                name.push_str(&ext);
                input.parent().unwrap_or(std::path::Path::new(".")).join(name)
            } else {
                compress::default_output(&input, container)
            }
        }
    };

    println!(
        "📦 压缩: {} -> {} (格式: {}, 级别: {})",
        input.display(),
        out.display(),
        container.display_name(),
        level
    );

    let bar = Progress::new("压缩中");
    // 若容器本身支持加密且提供了密码, 直接用归档内嵌加密 (兼容 7-Zip/WinRAR/Bandizip)
    // 否则按旧逻辑: 先压缩再 .enc 包装
    let archive_pwd = if container.supports_encryption() {
        password.as_deref()
    } else {
        None
    };

    // 有排除规则 → 使用 compress_with_exclude, 否则用普通 compress
    if exclude.is_empty() {
        compress::compress(&input, &out, container, level, archive_pwd, &bar)?;
    } else {
        println!("🚫 排除规则: {:?}", exclude);
        compress::compress_with_exclude(&input, &out, container, level, archive_pwd, &bar, &exclude)?;
    }
    bar.finish("✅ 压缩完成");

    // 分卷切割 (仅对已生成的归档文件生效)
    let final_path = out.clone();
    if let Some(split_str) = &split {
        let split_size = compress::parse_split_size(split_str)?;
        println!("✂️ 分卷大小: {}", progress::format_bytes(split_size));
        let parts = compress::split_file(&out, split_size)?;
        if parts.len() > 1 {
            println!("📁 分卷归档 ({} 个):", parts.len());
            for (i, p) in parts.iter().enumerate() {
                println!("  {}. {:<3} {}", i + 1, format!(".{:03}", i + 1), p.display());
            }
            // 分卷后原文件已被删除, 不再走加密包装流程
            return Ok(());
        } else {
            println!("ℹ️ 文件小于分卷大小, 无需分卷");
        }
    }

    // 对不支持内嵌加密的容器 (tar 系列), 退回到 .enc 包装
    if let Some(pwd) = &password {
        if !container.supports_encryption() {
            println!("🔐 启用加密...");
            let enc_out = PathBuf::from(format!("{}.enc", final_path.display()));
            let bar = Progress::new("加密中");
            crypto::encrypt_file(&final_path, &enc_out, pwd)?;
            bar.finish("✅ 加密完成");
            let _ = std::fs::remove_file(&final_path);
            println!("📁 加密归档: {}", enc_out.display());
            return Ok(());
        }
    }
    println!("📁 归档: {}", final_path.display());
    Ok(())
}

fn run_decompress(
    input: PathBuf,
    output: PathBuf,
    password: Option<String>,
) -> Result<()> {
    let archive_path = if let Some(pwd) = &password {
        if is_encrypted(&input)? {
            println!("🔐 检测到 .enc 加密文件, 先解密...");
            let tmp = PathBuf::from(format!("{}.tmp", input.display()));
            let bar = Progress::new("解密中");
            crypto::decrypt_file(&input, &tmp, pwd)?;
            bar.finish("✅ 解密完成");
            tmp
        } else {
            input.clone()
        }
    } else {
        input.clone()
    };

    println!("📂 解压: {} -> {}", archive_path.display(), output.display());
    let bar = Progress::new("解压中");
    // 统一走带密码入口: 若归档本身加密 (zip/7z/rar), 密码会传递到对应解压器
    decompress::decompress_with_password(&archive_path, &output, password.as_deref(), &bar)?;
    bar.finish("✅ 解压完成");

    if archive_path != input {
        let _ = std::fs::remove_file(&archive_path);
    }
    Ok(())
}

/// 解压到当前文件夹 (右键菜单 "解压到当前文件夹")
/// 输入归档所在目录即为输出目录
fn run_extract_here(input: PathBuf, password: Option<String>) -> Result<()> {
    let parent = input
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    println!("📂 解压到当前文件夹: {} -> {}", input.display(), parent.display());
    let bar = Progress::new("解压中");
    let archive_path = if let Some(pwd) = &password {
        if is_encrypted(&input)? {
            let tmp = PathBuf::from(format!("{}.tmp", input.display()));
            let dec_bar = Progress::new("解密中");
            crypto::decrypt_file(&input, &tmp, pwd)?;
            dec_bar.finish("✅ 解密完成");
            tmp
        } else {
            input.clone()
        }
    } else {
        input.clone()
    };
    decompress::decompress_with_password(&archive_path, &parent, password.as_deref(), &bar)?;
    bar.finish("✅ 解压完成");
    if archive_path != input {
        let _ = std::fs::remove_file(&archive_path);
    }
    Ok(())
}

/// 解压另存为 (右键菜单 "解压到...") — 输出到用户指定目录
/// 若 output 为 None, 弹出原生目录选择器 (rfd) 让用户选择
fn run_extract_as(input: PathBuf, output: Option<PathBuf>, password: Option<String>) -> Result<()> {
    let out = match output {
        Some(o) => o,
        None => {
            println!("📂 请选择解压目标目录...");
            let mut dlg = rfd::FileDialog::new();
            if let Some(parent) = input.parent() {
                dlg = dlg.set_directory(parent);
            }
            dlg.pick_folder()
                .ok_or_else(|| anyhow::anyhow!("未选择解压目录"))?
        }
    };
    println!("📂 解压另存为: {} -> {}", input.display(), out.display());
    std::fs::create_dir_all(&out)?;
    let bar = Progress::new("解压中");
    let archive_path = if let Some(pwd) = &password {
        if is_encrypted(&input)? {
            let tmp = PathBuf::from(format!("{}.tmp", input.display()));
            let dec_bar = Progress::new("解密中");
            crypto::decrypt_file(&input, &tmp, pwd)?;
            dec_bar.finish("✅ 解密完成");
            tmp
        } else {
            input.clone()
        }
    } else {
        input.clone()
    };
    decompress::decompress_with_password(&archive_path, &out, password.as_deref(), &bar)?;
    bar.finish("✅ 解压完成");
    if archive_path != input {
        let _ = std::fs::remove_file(&archive_path);
    }
    Ok(())
}

fn run_encrypt(
    input: PathBuf,
    output: Option<PathBuf>,
    password: String,
) -> Result<()> {
    let out = output.unwrap_or_else(|| crypto::default_encrypt_output(&input));
    println!("🔐 加密: {} -> {}", input.display(), out.display());
    let bar = Progress::new("加密中");
    crypto::encrypt_file(&input, &out, &password)?;
    bar.finish("✅ 加密完成");
    Ok(())
}

fn run_decrypt(
    input: PathBuf,
    output: Option<PathBuf>,
    password: String,
) -> Result<()> {
    let out = output.unwrap_or_else(|| crypto::default_decrypt_output(&input));
    println!("🔓 解密: {} -> {}", input.display(), out.display());
    let bar = Progress::new("解密中");
    crypto::decrypt_file(&input, &out, &password)?;
    bar.finish("✅ 解密完成");

    // 若解密后是归档, 自动继续解压
    if format::detect(&out).is_some() {
        let extract_dir = out.with_extension("");
        if extract_dir.exists() && extract_dir.is_file() {
            std::fs::remove_file(&extract_dir)?;
        }
        std::fs::create_dir_all(&extract_dir)?;
        println!("📂 检测到归档, 继续解压: {} -> {}", out.display(), extract_dir.display());
        let bar = Progress::new("解压中");
        decompress::decompress_with_password(&out, &extract_dir, None, &bar)?;
        bar.finish("✅ 解压完成");
        let _ = std::fs::remove_file(&out);
        println!("📁 解压目录: {}", extract_dir.display());
    }
    Ok(())
}

fn run_smart(
    input: PathBuf,
    output: Option<PathBuf>,
    password: Option<String>,
    format: Option<String>,
) -> Result<()> {
    let name = input
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if name.ends_with(".enc") {
        let pwd = password.ok_or_else(|| {
            anyhow::anyhow!("解密需要密码, 请通过 --password 提供")
        })?;
        let dec_out = crypto::default_decrypt_output(&input);
        println!("🔓 [智能] 解密: {} -> {}", input.display(), dec_out.display());
        let bar = Progress::new("解密中");
        crypto::decrypt_file(&input, &dec_out, &pwd)?;
        bar.finish("✅ 解密完成");

        if format::detect(&dec_out).is_some() {
            let extract_dir = output.unwrap_or_else(|| {
                let stem = dec_out
                    .file_stem()
                    .map(|s| s.to_os_string())
                    .unwrap_or_default();
                PathBuf::from(stem)
            });
            let extract_dir = if extract_dir.extension().is_none() || extract_dir.is_dir() {
                extract_dir
            } else {
                extract_dir.with_extension("")
            };
            if extract_dir.exists() && extract_dir.is_file() {
                std::fs::remove_file(&extract_dir)?;
            }
            std::fs::create_dir_all(&extract_dir)?;
            println!("📂 [智能] 继续解压: {} -> {}", dec_out.display(), extract_dir.display());
            let bar = Progress::new("解压中");
            decompress::decompress_with_password(&dec_out, &extract_dir, None, &bar)?;
            bar.finish("✅ 解压完成");
            let _ = std::fs::remove_file(&dec_out);
            println!("📁 解压目录: {}", extract_dir.display());
        } else {
            if let Some(out) = output {
                std::fs::rename(&dec_out, &out)?;
                println!("📁 解密文件: {}", out.display());
            } else {
                println!("📁 解密文件: {}", dec_out.display());
            }
        }
    } else if format::detect(&input).is_some() {
        let out_dir = output.unwrap_or_else(|| PathBuf::from("."));
        println!("📂 [智能] 解压: {} -> {}", input.display(), out_dir.display());
        let bar = Progress::new("解压中");
        decompress::decompress_with_password(&input, &out_dir, password.as_deref(), &bar)?;
        bar.finish("✅ 解压完成");
    } else if input.is_dir() || input.is_file() {
        // 统一用 infer_container 推断 (含单文件流升级 tar 逻辑)
        let mut container = infer_container(&input, &format, &output);

        // 目录输入 + 无格式 + 输出无可识别归档扩展名 → 默认 tar.zst (高速高比)
        if format.is_none() && input.is_dir() {
            let out_has_archive_ext = output
                .as_ref()
                .and_then(|o| format::detect(o))
                .is_some();
            if !out_has_archive_ext {
                container = Container::TarZst;
            }
        }

        // 输出路径: 若指定但无可识别扩展名, 补全 container 扩展名
        let out = match &output {
            Some(o) => {
                let normalized = normalize_output_ext(o, container);
                ensure_archive_extension(&normalized, container, format.as_deref())
            }
            None => compress::default_output(&input, container),
        };
        println!(
            "📦 [智能] 压缩: {} -> {} (格式: {})",
            input.display(),
            out.display(),
            container.display_name()
        );
        let bar = Progress::new("压缩中");
        // 智能模式: 若容器支持内嵌加密且提供密码, 直接用归档加密
        let archive_pwd = if container.supports_encryption() {
            password.as_deref()
        } else {
            None
        };
        compress::compress(&input, &out, container, 3, archive_pwd, &bar)?;
        bar.finish("✅ 压缩完成");

        // 对不支持内嵌加密的容器 (tar 系列), 退回到 .enc 包装
        if let Some(pwd) = &password {
            if !container.supports_encryption() {
                let enc_out = PathBuf::from(format!("{}.enc", out.display()));
                println!("🔐 [智能] 启用加密...");
                let bar = Progress::new("加密中");
                crypto::encrypt_file(&out, &enc_out, pwd)?;
                bar.finish("✅ 加密完成");
                let _ = std::fs::remove_file(&out);
                println!("📁 加密归档: {}", enc_out.display());
            }
        }
    } else {
        return Err(anyhow::anyhow!("输入路径不存在: {}", input.display()));
    }
    Ok(())
}

/// 列出归档内容 (不解压)
fn run_list(input: PathBuf, password: Option<String>) -> Result<()> {
    println!("📋 列出归档内容: {}", input.display());

    // 若是 .enc 加密文件, 先解密到临时文件
    let archive_path = if let Some(pwd) = &password {
        if is_encrypted(&input)? {
            println!("🔐 检测到 .enc 加密文件, 先解密...");
            let tmp = PathBuf::from(format!("{}.tmp", input.display()));
            let bar = Progress::new("解密中");
            crypto::decrypt_file(&input, &tmp, pwd)?;
            bar.finish("✅ 解密完成");
            tmp
        } else {
            input.clone()
        }
    } else {
        input.clone()
    };

    let entries = archive_list::list_archive(&archive_path, password.as_deref())?;

    if archive_path != input {
        let _ = std::fs::remove_file(&archive_path);
    }

    if entries.is_empty() {
        println!("  (空归档或格式不支持列表)");
        return Ok(());
    }

    // 表格式输出
    println!(
        "{:<50} {:>12} {:>12}  {}",
        "名称", "大小", "压缩后", "类型"
    );
    println!("{}", "-".repeat(80));
    let mut total_size = 0u64;
    let mut total_compressed = 0u64;
    for e in &entries {
        let kind = if e.is_dir { "DIR " } else { "FILE" };
        println!(
            "{:<50} {:>12} {:>12}  {}",
            truncate_str(&e.name, 50),
            progress::format_bytes(e.size),
            progress::format_bytes(e.compressed_size),
            kind
        );
        total_size += e.size;
        total_compressed += e.compressed_size;
    }
    println!("{}", "-".repeat(80));
    println!(
        "共 {} 个条目 · 总大小 {} · 压缩后 {}",
        entries.len(),
        progress::format_bytes(total_size),
        progress::format_bytes(total_compressed)
    );
    Ok(())
}

/// 测试归档完整性
fn run_test(input: PathBuf, password: Option<String>) -> Result<()> {
    println!("🧪 测试归档完整性: {}", input.display());

    let archive_path = if let Some(pwd) = &password {
        if is_encrypted(&input)? {
            println!("🔐 检测到 .enc 加密文件, 先解密...");
            let tmp = PathBuf::from(format!("{}.tmp", input.display()));
            let bar = Progress::new("解密中");
            crypto::decrypt_file(&input, &tmp, pwd)?;
            bar.finish("✅ 解密完成");
            tmp
        } else {
            input.clone()
        }
    } else {
        input.clone()
    };

    let bar = Progress::new("测试中");
    let result = decompress::test_archive(&archive_path, password.as_deref(), &bar);
    bar.finish("✅ 测试完成");

    if archive_path != input {
        let _ = std::fs::remove_file(&archive_path);
    }

    match result {
        Ok((entries, bytes)) => {
            println!(
                "✅ 归档完整: {} 个条目, {}",
                entries,
                progress::format_bytes(bytes)
            );
            Ok(())
        }
        Err(e) => {
            println!("❌ 归档损坏: {}", e);
            Err(e)
        }
    }
}

/// 截断字符串到指定长度 (尾部加 ...)
fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 3).collect();
        format!("{}...", truncated)
    }
}

fn is_encrypted(path: &std::path::Path) -> Result<bool> {
    use std::io::Read;
    if !path.exists() {
        return Ok(false);
    }
    let mut f = std::fs::File::open(path)?;
    let mut head = [0u8; 5];
    match f.read_exact(&mut head) {
        Ok(_) => Ok(&head == b"SMEX1"),
        Err(_) => Ok(false),
    }
}
