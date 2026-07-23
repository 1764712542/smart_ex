//! SmartEx Tauri 后端 — IPC 命令层
//!
//! 把 smartex-core 的功能通过 Tauri 命令暴露给前端 Svelte 应用。
//!
//! 命令清单:
//!   compress / decompress / encrypt / decrypt / list_archive / test_archive
//!   suggest_format / keychain_get / keychain_set / keychain_delete
//!   pick_file / pick_folder / save_file

use smartex_core::format::Container;
use smartex_core::progress::ProgressCallback;
use smartex_core::{
    archive_list, compress, context, decompress, format, keychain, progress, stream_crypto,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

// ── 进度事件 payload ──

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub progress: f32,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub message: String,
}

// ── 全局状态: 会话级钥匙串 + 任务取消标志 ──

pub struct AppState {
    pub keychain: keychain::SessionKeychain,
    /// 任务取消标志: compress/decompress 启动前清零, 前端 cancel 命令置为 true
    pub cancel: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            keychain: keychain::SessionKeychain::new(),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

// ── 前端参数 DTO (与 ui/src/lib/tauri.ts 对齐) ──

#[derive(Deserialize)]
pub struct CompressParams {
    pub input: String,
    pub output: Option<String>,
    pub format: Option<String>,
    pub level: Option<i32>,
    pub password: Option<String>,
    pub exclude: Option<Vec<String>>,
    pub split: Option<String>,
}

#[derive(Deserialize)]
pub struct DecompressParams {
    pub input: String,
    pub output: String,
    pub password: Option<String>,
    /// 冲突策略: overwrite | skip | rename (默认 overwrite)
    #[serde(rename = "conflictPolicy", default)]
    pub conflict_policy: Option<String>,
    /// 是否保留符号链接 (默认 true)
    #[serde(rename = "preserveSymlinks", default)]
    pub preserve_symlinks: Option<bool>,
    /// 解压失败时是否清理半成品 (默认 true)
    #[serde(rename = "cleanupOnError", default)]
    pub cleanup_on_error: Option<bool>,
}

/// 从 DecompressParams 构造 ExtractOptions
fn build_extract_opts(params: &DecompressParams) -> decompress::ExtractOptions {
    let conflict = match params.conflict_policy.as_deref() {
        Some("skip") => decompress::ConflictPolicy::Skip,
        Some("rename") => decompress::ConflictPolicy::Rename,
        _ => decompress::ConflictPolicy::Overwrite,
    };
    decompress::ExtractOptions {
        conflict,
        password: params.password.clone(),
        preserve_symlinks: params.preserve_symlinks.unwrap_or(true),
        cleanup_on_error: params.cleanup_on_error.unwrap_or(true),
    }
}

// ── 返回值 DTO ──

/// 归档条目 (core 字段名 name → 前端期望 path)
#[derive(Serialize)]
pub struct ArchiveEntryDto {
    #[serde(rename = "path")]
    pub path: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_dir: bool,
}

#[derive(Serialize)]
pub struct TestResult {
    pub summary: String,
    pub passed: bool,
}

// ── 上下文感知压缩: 前端小写枚举 → core PascalCase 枚举 ──

#[derive(Deserialize)]
pub enum RecipientDto {
    #[serde(rename = "self", alias = "self_")]
    Self_,
    #[serde(rename = "colleague")]
    Colleague,
    #[serde(rename = "external")]
    External,
    #[serde(rename = "public")]
    Public,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportDto {
    Email,
    Im,
    Cloud,
    Usb,
    Local,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetOsDto {
    Windows,
    Macos,
    Linux,
    Mobile,
    Unknown,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PriorityDto {
    Size,
    Speed,
    Compatibility,
    Security,
}

#[derive(Deserialize)]
pub struct IntentDto {
    pub recipient: RecipientDto,
    pub transport: TransportDto,
    pub target_os: TargetOsDto,
    pub priority: PriorityDto,
    #[serde(default)]
    pub estimated_size: Option<u64>,
}

impl From<RecipientDto> for context::Recipient {
    fn from(r: RecipientDto) -> Self {
        match r {
            RecipientDto::Self_ => context::Recipient::Self_,
            RecipientDto::Colleague => context::Recipient::Colleague,
            RecipientDto::External => context::Recipient::External,
            RecipientDto::Public => context::Recipient::Public,
        }
    }
}

impl From<TransportDto> for context::Transport {
    fn from(t: TransportDto) -> Self {
        match t {
            TransportDto::Email => context::Transport::Email,
            TransportDto::Im => context::Transport::Im,
            TransportDto::Cloud => context::Transport::Cloud,
            TransportDto::Usb => context::Transport::Usb,
            TransportDto::Local => context::Transport::Local,
        }
    }
}

impl From<TargetOsDto> for context::TargetOs {
    fn from(t: TargetOsDto) -> Self {
        match t {
            TargetOsDto::Windows => context::TargetOs::Windows,
            TargetOsDto::Macos => context::TargetOs::Macos,
            TargetOsDto::Linux => context::TargetOs::Linux,
            TargetOsDto::Mobile => context::TargetOs::Mobile,
            TargetOsDto::Unknown => context::TargetOs::Unknown,
        }
    }
}

impl From<PriorityDto> for context::Priority {
    fn from(p: PriorityDto) -> Self {
        match p {
            PriorityDto::Size => context::Priority::Size,
            PriorityDto::Speed => context::Priority::Speed,
            PriorityDto::Compatibility => context::Priority::Compatibility,
            PriorityDto::Security => context::Priority::Security,
        }
    }
}

// ── 辅助函数 ──

/// 将格式字符串 ("zip" / "7z" / "tar.gz" ...) 解析为 Container
fn parse_container(format_str: &str) -> Result<Container, String> {
    match format_str.to_lowercase().as_str() {
        "zip" => Ok(Container::Zip),
        "7z" => Ok(Container::SevenZ),
        "tar" => Ok(Container::Tar),
        "tar.gz" | "tgz" => Ok(Container::TarGz),
        "tar.xz" | "txz" => Ok(Container::TarXz),
        "tar.zst" | "tzst" => Ok(Container::TarZst),
        "tar.bz2" | "tbz2" => Ok(Container::TarBz2),
        "tar.lz4" => Ok(Container::TarLz4),
        "gz" | "xz" | "zst" | "bz2" | "lz4" => Ok(Container::Single),
        other => Err(format!("不支持的压缩格式: {}", other)),
    }
}

/// 创建进度回调, 通过 Tauri 事件 "progress" 向前端推送实时进度
fn make_progress_callback(app: &AppHandle, message: &str) -> ProgressCallback {
    let app = app.clone();
    let msg = message.to_string();
    Arc::new(move |current, total, bytes_done, bytes_total| {
        let progress = if total > 0 {
            current as f32 / total as f32
        } else {
            0.0
        };
        let _ = app.emit(
            "progress",
            ProgressPayload {
                progress,
                bytes_done,
                bytes_total,
                message: msg.clone(),
            },
        );
    })
}

// ───────────────────────── IPC 命令 ─────────────────────────

/// 压缩文件/目录
///
/// 支持密码加密 (zip/7z)、排除规则、分卷切割。
/// 通过 "progress" 事件推送实时进度。
#[tauri::command]
async fn compress(params: CompressParams, app: AppHandle) -> Result<String, String> {
    let input = PathBuf::from(&params.input);

    // 解析目标格式: 显式指定 > 从输出路径推断 > 默认 zip
    let container = if let Some(ref fmt) = params.format {
        parse_container(fmt)?
    } else if let Some(ref out) = params.output {
        format::detect(std::path::Path::new(out)).unwrap_or(Container::Zip)
    } else {
        Container::Zip
    };

    let level = params.level.unwrap_or(6);
    let output = PathBuf::from(params.output.unwrap_or_else(|| {
        compress::default_output(&input, container)
            .to_string_lossy()
            .to_string()
    }));
    let password = params.password;
    let excludes = params.exclude.unwrap_or_default();
    let split = params.split;
    let callback = make_progress_callback(&app, "压缩中");
    // Bug 3 修复: 任务启动前清零 cancel 标志
    let cancel = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().map_err(|e| format!("状态锁失败: {}", e))?;
        state.cancel.clone()
    };
    cancel.store(false, Ordering::SeqCst);
    let app_handle = app.clone();

    tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let bar = progress::Progress::new_with_callback("compress", callback);
        let pwd = password.as_deref();
        if excludes.is_empty() {
            compress::compress(&input, &output, container, level, pwd, &bar)?;
        } else {
            compress::compress_with_exclude(
                &input, &output, container, level, pwd, &bar, &excludes,
            )?;
        }
        // Bug 3: 在分卷前检查取消
        if cancel.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(&output);
            anyhow::bail!("任务已取消");
        }
        // 分卷切割 (可选)
        if let Some(ref split_str) = split {
            // Bug 15 修复: 分卷前通知前端
            let _ = app_handle.emit("progress", ProgressPayload {
                progress: 0.99,
                bytes_done: 0,
                bytes_total: 0,
                message: "分卷切割中".to_string(),
            });
            let split_size = compress::parse_split_size(split_str)?;
            let parts = compress::split_file(&output, split_size)?;
            bar.finish("done");
            // Bug 6 修复: split_file 删除原文件, 返回第一个分卷路径
            if let Some(first) = parts.first() {
                return Ok(first.to_string_lossy().to_string());
            }
        }
        bar.finish("done");
        Ok(output.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| e.to_string())
}

/// 解压归档
///
/// 自动检测归档格式, 支持密码解压 (zip/7z/rar)。
#[tauri::command]
async fn decompress(params: DecompressParams, app: AppHandle) -> Result<String, String> {
    let input = PathBuf::from(&params.input);
    let output = PathBuf::from(&params.output);
    let opts = build_extract_opts(&params);
    let callback = make_progress_callback(&app, "解压中");
    // Bug 3 修复: 任务启动前清零 cancel 标志
    let cancel = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().map_err(|e| format!("状态锁失败: {}", e))?;
        state.cancel.clone()
    };
    cancel.store(false, Ordering::SeqCst);

    tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let bar = progress::Progress::new_with_callback("decompress", callback);
        let container = format::detect(&input)
            .ok_or_else(|| anyhow::anyhow!("无法识别归档格式"))?;
        decompress::decompress_with(&input, &output, container, &opts, &bar)?;
        if cancel.load(Ordering::SeqCst) {
            anyhow::bail!("任务已取消");
        }
        bar.finish("done");
        Ok(output.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| e.to_string())
}

/// 取消正在执行的任务
///
/// 设置全局 cancel 标志, 后端在下一个检查点停止并返回错误。
#[tauri::command]
async fn cancel_task(app: AppHandle) -> Result<(), String> {
    let cancel = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().map_err(|e| format!("状态锁失败: {}", e))?;
        state.cancel.clone()
    };
    cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// 流式加密文件 (AES-256-GCM 分块, 恒定内存 ~8MB)
///
/// 使用 stream_crypto::encrypt_stream, 不 OOM, 支持任意大小文件。
#[tauri::command]
async fn encrypt(input: String, output: String, password: String, app: AppHandle) -> Result<String, String> {
    let input = PathBuf::from(&input);
    let output = PathBuf::from(&output);
    let app_handle = app.clone();

    tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let progress_cb = |bytes_done: u64, bytes_total: u64| {
            let progress = if bytes_total > 0 {
                bytes_done as f32 / bytes_total as f32
            } else {
                0.0
            };
            let _ = app_handle.emit(
                "progress",
                ProgressPayload {
                    progress,
                    bytes_done,
                    bytes_total,
                    message: "加密中".to_string(),
                },
            );
        };
        stream_crypto::encrypt_stream(&input, &output, &password, Some(&progress_cb))?;
        Ok(output.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| e.to_string())
}

/// 流式解密文件
#[tauri::command]
async fn decrypt(input: String, output: String, password: String, app: AppHandle) -> Result<String, String> {
    let input = PathBuf::from(&input);
    let output = PathBuf::from(&output);
    let app_handle = app.clone();

    tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let progress_cb = |bytes_done: u64, bytes_total: u64| {
            let progress = if bytes_total > 0 {
                bytes_done as f32 / bytes_total as f32
            } else {
                0.0
            };
            let _ = app_handle.emit(
                "progress",
                ProgressPayload {
                    progress,
                    bytes_done,
                    bytes_total,
                    message: "解密中".to_string(),
                },
            );
        };
        stream_crypto::decrypt_stream(&input, &output, &password, Some(&progress_cb))?;
        Ok(output.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| e.to_string())
}

/// 列出归档内文件列表 (不解压)
///
/// 支持 zip / tar / tar.gz / tar.xz / tar.zst / tar.bz2 / tar.lz4
/// 7z 和 rar 暂不支持列表。
#[tauri::command]
async fn list_archive(
    input: String,
    password: Option<String>,
) -> Result<Vec<ArchiveEntryDto>, String> {
    let path = PathBuf::from(&input);
    let pwd = password;

    tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<ArchiveEntryDto>> {
        let entries = archive_list::list_archive(&path, pwd.as_deref())?;
        Ok(entries
            .into_iter()
            .map(|e| ArchiveEntryDto {
                path: e.name,
                size: e.size,
                compressed_size: e.compressed_size,
                is_dir: e.is_dir,
            })
            .collect())
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| e.to_string())
}

/// 部分解压: 只解压归档中指定的文件
#[tauri::command]
async fn extract_partial(
    input: String,
    output: String,
    files: Vec<String>,
    password: Option<String>,
) -> Result<String, String> {
    let archive_path = PathBuf::from(&input);
    let output_dir = PathBuf::from(&output);
    let opts = decompress::ExtractOptions {
        password,
        ..Default::default()
    };

    tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        decompress::extract_partial(&archive_path, &output_dir, &files, &opts)?;
        Ok(output_dir.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| e.to_string())
}

/// 测试归档完整性 (不解压到磁盘, 仅校验数据流)
#[tauri::command]
async fn test_archive(
    input: String,
    password: Option<String>,
    app: AppHandle,
) -> Result<TestResult, String> {
    let path = PathBuf::from(&input);
    let pwd = password;
    let callback = make_progress_callback(&app, "测试中");

    tokio::task::spawn_blocking(move || -> anyhow::Result<TestResult> {
        let bar = progress::Progress::new_with_callback("test", callback);
        match decompress::test_archive(&path, pwd.as_deref(), &bar) {
            Ok((entries, total_bytes)) => {
                let summary = format!(
                    "归档完整性测试通过: {} 个条目, {} 数据",
                    entries,
                    progress::format_bytes(total_bytes)
                );
                Ok(TestResult {
                    summary,
                    passed: true,
                })
            }
            Err(e) => Ok(TestResult {
                summary: format!("测试失败: {}", e),
                passed: false,
            }),
        }
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| e.to_string())
}

/// 上下文感知压缩格式推荐
///
/// 根据收件人/传输方式/目标系统/优先级自动推荐最优格式。
#[tauri::command]
async fn suggest_format(intent: IntentDto) -> Result<context::FormatSuggestion, String> {
    let core_intent = context::CompressionIntent {
        recipient: intent.recipient.into(),
        transport: intent.transport.into(),
        target_os: intent.target_os.into(),
        priority: intent.priority.into(),
        estimated_size: intent.estimated_size,
    };
    Ok(context::suggest_format(&core_intent))
}

/// 从会话钥匙串获取密码
#[tauri::command]
async fn keychain_get(
    key: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    let state = state
        .lock()
        .map_err(|e| format!("状态锁失败: {}", e))?;
    Ok(state.keychain.get(&key).map(|e| e.password))
}

/// 存储密码到会话钥匙串
#[tauri::command]
async fn keychain_set(
    key: String,
    value: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let state = state
        .lock()
        .map_err(|e| format!("状态锁失败: {}", e))?;
    state.keychain.set(&key, &value, &key);
    Ok(())
}

/// 从会话钥匙串删除密码
#[tauri::command]
async fn keychain_delete(
    key: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let state = state
        .lock()
        .map_err(|e| format!("状态锁失败: {}", e))?;
    state.keychain.delete(&key);
    Ok(())
}

/// 文件选择对话框
#[tauri::command]
async fn pick_file() -> Result<Option<String>, String> {
    let result = rfd::AsyncFileDialog::new().pick_file().await;
    Ok(result.map(|h| h.path().display().to_string()))
}

/// 文件夹选择对话框
#[tauri::command]
async fn pick_folder() -> Result<Option<String>, String> {
    let result = rfd::AsyncFileDialog::new().pick_folder().await;
    Ok(result.map(|h| h.path().display().to_string()))
}

/// 保存文件对话框
#[tauri::command]
async fn save_file() -> Result<Option<String>, String> {
    let result = rfd::AsyncFileDialog::new().save_file().await;
    Ok(result.map(|h| h.path().display().to_string()))
}

// ───────────────────────── Tauri 应用入口 ─────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Mutex::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            compress,
            decompress,
            encrypt,
            decrypt,
            list_archive,
            extract_partial,
            test_archive,
            suggest_format,
            keychain_get,
            keychain_set,
            keychain_delete,
            pick_file,
            pick_folder,
            save_file,
            cancel_task,
        ])
        // 拖放文件: 把路径发给前端
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::DragDrop(drag_drop) = event {
                use tauri::DragDropEvent;
                match drag_drop {
                    DragDropEvent::Enter { paths, .. } => {
                        if let Some(path) = paths.first() {
                            let _ = window.emit("file-hovered", path.to_string_lossy().to_string());
                        }
                    }
                    DragDropEvent::Drop { paths, .. } => {
                        if let Some(path) = paths.first() {
                            let _ = window.emit("file-opened", path.to_string_lossy().to_string());
                        }
                    }
                    DragDropEvent::Leave => {
                        let _ = window.emit("file-drop-left", ());
                    }
                    _ => {}
                }
            }
        })
        // macOS: 通过 argv 接收双击打开的文件 (启动时)
        .setup(|app| {
            if let Some(args) = std::env::args().nth(1) {
                let path = std::path::Path::new(&args);
                if path.exists() {
                    let _ = app.emit("file-opened", path.to_string_lossy().to_string());
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
