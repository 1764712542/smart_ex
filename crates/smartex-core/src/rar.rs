//! RAR 解压支持 (基于 unrar crate, 内嵌 unRAR C 库)
//!
//! RAR 是专有格式, 只支持解压不支持压缩.
//! 支持加密 RAR (需提供密码) 和多卷 RAR (自动从第一卷开始).

use crate::progress::Progress;
use anyhow::Result;
use std::path::Path;
use unrar::Archive;

/// 解压 RAR 归档
///
/// - `password`: 可选密码 (用于加密 RAR)
/// - 自动处理多卷 RAR (从第一卷开始)
pub fn rar_decompress(
    input: &Path,
    output: &Path,
    password: Option<&str>,
    bar: &Progress,
) -> Result<()> {
    // 确保从第一卷开始
    let first = Archive::new(input).as_first_part();

    let mut open = if let Some(pwd) = password {
        Archive::with_password(first.filename(), pwd).open_for_processing()
    } else {
        first.open_for_processing()
    }
    .map_err(|e| anyhow::anyhow!("打开 RAR 失败: {}", e))?;

    std::fs::create_dir_all(output)?;

    let mut count = 0u64;
    loop {
        match open
            .read_header()
            .map_err(|e| anyhow::anyhow!("读取 RAR 头部失败: {}", e))?
        {
            Some(entry) => {
                let header = entry.entry();
                if header.is_directory() {
                    // Bug 7 修复: 目录路径消毒, 防止路径穿越
                    let name = header.filename.to_string_lossy().to_string();
                    let clean: std::path::PathBuf = std::path::Path::new(&name)
                        .components()
                        .filter(|c| matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir))
                        .collect();
                    if !clean.as_os_str().is_empty() {
                        let dir = output.join(&clean);
                        std::fs::create_dir_all(&dir)?;
                    }
                    open = entry
                        .skip()
                        .map_err(|e| anyhow::anyhow!("跳过目录失败: {}", e))?;
                } else {
                    // 文件条目: 提取到输出目录 (unrar C 库内部消毒路径)
                    open = entry
                        .extract_with_base(output)
                        .map_err(|e| anyhow::anyhow!("提取 RAR 文件失败: {}", e))?;
                }
                count += 1;
                bar.inc(1);
            }
            None => break,
        }
    }

    Ok(())
}
