//! smart_ex GUI - 基于 eframe/egui 的优美界面 (双语 + 快捷解压)
//!
//! 设计: 深色玻璃拟态主题, 顶部 Tab 切换 "压缩 / 解压 / 加密 / 解密",
//! 左侧参数面板, 右侧日志/进度面板. 单二进制零外部资源.
//! 支持: 中英双语实时切换 / 解压到当前文件夹 / 解压另存为 / 加密归档密码.

use crate::compress;
use crate::crypto;
use crate::decompress;
use crate::format::{detect, Container};
use crate::i18n::{self, Lang};
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use eframe::egui::{Color32, FontId, RichText, Vec2};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ───────────────────────── 主题色 ─────────────────────────

const BG: Color32 = Color32::from_rgb(18, 22, 32);
const PANEL: Color32 = Color32::from_rgb(28, 34, 48);
const ACCENT: Color32 = Color32::from_rgb(120, 180, 255);
const ACCENT_DIM: Color32 = Color32::from_rgb(70, 120, 200);
const SUCCESS: Color32 = Color32::from_rgb(120, 220, 160);
const WARN: Color32 = Color32::from_rgb(255, 180, 90);
const ERROR: Color32 = Color32::from_rgb(255, 110, 110);
const TEXT: Color32 = Color32::from_rgb(230, 235, 245);
const TEXT_DIM: Color32 = Color32::from_rgb(150, 160, 180);

// ───────────────────────── 消息通道 ─────────────────────────

#[derive(Clone, Debug)]
enum WorkMsg {
    Log(String, MsgKind),
    Progress(f32),
    Done(bool, String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
enum MsgKind {
    Info,
    Success,
    Warn,
    Error,
}

#[derive(Clone, Debug)]
struct LogEntry {
    text: String,
    kind: MsgKind,
    time: String,
}

#[derive(Clone, Debug, PartialEq)]
enum Mode {
    Compress,
    Decompress,
    Encrypt,
    Decrypt,
}

impl Mode {
    fn title(&self) -> &'static str {
        match self {
            Mode::Compress => i18n::t("mode_compress"),
            Mode::Decompress => i18n::t("mode_decompress"),
            Mode::Encrypt => i18n::t("mode_encrypt"),
            Mode::Decrypt => i18n::t("mode_decrypt"),
        }
    }
}

pub struct App {
    mode: Mode,
    input_path: String,
    output_path: String,
    selected_container_idx: usize,
    level: i32,
    password: String,
    encrypt_archive: bool,

    logs: Vec<LogEntry>,
    progress: f32,
    working: bool,
    status_text: String,

    tx: Sender<WorkMsg>,
    rx: Receiver<WorkMsg>,
    worker_handle: Option<std::thread::JoinHandle<()>>,
}

impl Default for App {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self {
            mode: Mode::Compress,
            input_path: String::new(),
            output_path: String::new(),
            selected_container_idx: 4, // 默认 tar.zst 索引 (见 Container::all 顺序)
            level: 3,
            password: String::new(),
            encrypt_archive: false,
            logs: Vec::new(),
            progress: 0.0,
            working: false,
            status_text: i18n::t("ready").to_string(),
            tx,
            rx,
            worker_handle: None,
        }
    }
}

impl App {
    fn log(&mut self, text: &str, kind: MsgKind) {
        self.logs.push(LogEntry {
            text: text.to_string(),
            kind,
            time: chrono_like::now(),
        });
        if self.logs.len() > 500 {
            self.logs.drain(0..100);
        }
    }

    fn containers(&self) -> &'static [Container] {
        Container::all()
    }

    fn current_container(&self) -> Container {
        self.containers()[self.selected_container_idx]
    }

    fn pick_input_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.input_path = path.display().to_string();
            self.output_path.clear();
            self.auto_fill_output();
            self.maybe_switch_mode_by_input();
        }
    }

    fn pick_input_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.input_path = path.display().to_string();
            self.output_path.clear();
            self.auto_fill_output();
        }
    }

    fn pick_output_file(&mut self) {
        let mut dlg = rfd::FileDialog::new();
        if let Some(parent) = std::path::Path::new(&self.input_path).parent() {
            dlg = dlg.set_directory(parent);
        }
        if let Some(path) = dlg.save_file() {
            self.output_path = path.display().to_string();
        }
    }

    fn pick_output_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.output_path = path.display().to_string();
        }
    }

    fn auto_fill_output(&mut self) {
        if self.input_path.is_empty() {
            return;
        }
        let p = std::path::Path::new(&self.input_path);
        match self.mode {
            Mode::Compress => {
                let c = self.current_container();
                self.output_path = compress::default_output(p, c).display().to_string();
            }
            Mode::Decompress => {
                // 智能剥离所有归档扩展名: archive.tar.gz → archive
                let stem = archive_stem(p);
                let dir = p.parent().unwrap_or(std::path::Path::new("."));
                self.output_path = dir.join(stem).display().to_string();
            }
            Mode::Encrypt => {
                self.output_path = crypto::default_encrypt_output(p).display().to_string();
            }
            Mode::Decrypt => {
                self.output_path = crypto::default_decrypt_output(p).display().to_string();
            }
        }
    }

    fn maybe_switch_mode_by_input(&mut self) {
        let p = std::path::Path::new(&self.input_path);
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name.ends_with(".enc") {
            self.mode = Mode::Decrypt;
            self.output_path.clear();
            self.auto_fill_output();
        } else if detect(p).is_some() {
            self.mode = Mode::Decompress;
            self.output_path.clear();
            self.auto_fill_output();
        }
    }

    fn switch_mode(&mut self, m: Mode) {
        self.mode = m;
        self.output_path.clear();
        self.auto_fill_output();
    }

    /// 解压到当前文件夹 (输入归档所在目录)
    fn extract_here(&mut self) {
        if self.working {
            return;
        }
        let input = self.input_path.trim().to_string();
        if input.is_empty() {
            self.log(i18n::t("select_input"), MsgKind::Error);
            return;
        }
        let p = std::path::Path::new(&input);
        if !p.exists() {
            self.log(i18n::t("input_not_exist"), MsgKind::Error);
            return;
        }
        let parent = p
            .parent()
            .map(|x| x.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .to_string_lossy()
            .to_string();
        let password = self.password.clone();
        self.logs.clear();
        self.progress = 0.0;
        self.working = true;
        self.status_text = i18n::t("processing").to_string();
        self.log(
            &format!("{} {}", i18n::t("start_prefix"), i18n::t("extract_here")),
            MsgKind::Info,
        );
        self.log(&format!("  {}: {}", i18n::t("input"), input), MsgKind::Info);
        self.log(
            &format!("  {}: {}", i18n::t("output"), parent),
            MsgKind::Info,
        );

        let tx = self.tx.clone();
        let handle = std::thread::spawn(move || {
            let result = run_extract_task(input, parent, password, tx.clone(), false);
            match result {
                Ok(summary) => {
                    let _ = tx.send(WorkMsg::Progress(1.0));
                    let _ = tx.send(WorkMsg::Done(true, summary));
                }
                Err(e) => {
                    let e = friendly_error(e);
                    let _ = tx.send(WorkMsg::Log(
                        format!("{}{}", i18n::t("error_prefix"), e),
                        MsgKind::Error,
                    ));
                    let _ = tx.send(WorkMsg::Done(false, e.to_string()));
                }
            }
        });
        self.worker_handle = Some(handle);
    }

    /// 解压另存为 — 弹出目录选择器
    fn extract_as(&mut self) {
        if self.working {
            return;
        }
        let input = self.input_path.trim().to_string();
        if input.is_empty() {
            self.log(i18n::t("select_input"), MsgKind::Error);
            return;
        }
        let p = std::path::Path::new(&input);
        if !p.exists() {
            self.log(i18n::t("input_not_exist"), MsgKind::Error);
            return;
        }
        // 智能匹配: 默认建议在归档同级目录下创建以归档文件名命名的子目录
        // 例: /Downloads/archive.tar.gz → 建议解压到 /Downloads/archive/
        let suggested_name = archive_stem(p);
        let mut dlg = rfd::FileDialog::new();
        if let Some(parent) = p.parent() {
            // 尝试设置建议目录: parent/suggested_name (rfd 会以该路径为起点)
            let suggested_dir = parent.join(&suggested_name);
            dlg = dlg.set_directory(parent);
            // 保存建议路径供选择后使用
            dlg = dlg.set_title(&format!(
                "{}: {}",
                i18n::t("extract_to"),
                suggested_dir.display()
            ));
        }
        if let Some(out_dir) = dlg.pick_folder() {
            // 如果用户选择的目录与归档同级, 自动在其中创建以归档名命名的子目录
            let out = if out_dir == p.parent().unwrap_or(std::path::Path::new(".")) {
                out_dir.join(&suggested_name).display().to_string()
            } else {
                out_dir.join(&suggested_name).display().to_string()
            };
            let password = self.password.clone();
            self.logs.clear();
            self.progress = 0.0;
            self.working = true;
            self.status_text = i18n::t("processing").to_string();
            self.log(
                &format!("{} {}", i18n::t("start_prefix"), i18n::t("extract_to")),
                MsgKind::Info,
            );
            self.log(&format!("  {}: {}", i18n::t("input"), input), MsgKind::Info);
            self.log(&format!("  {}: {}", i18n::t("output"), out), MsgKind::Info);

            let tx = self.tx.clone();
            let handle = std::thread::spawn(move || {
                let result = run_extract_task(input, out, password, tx.clone(), true);
                match result {
                    Ok(summary) => {
                        let _ = tx.send(WorkMsg::Progress(1.0));
                        let _ = tx.send(WorkMsg::Done(true, summary));
                    }
                    Err(e) => {
                        let e = friendly_error(e);
                        let _ = tx.send(WorkMsg::Log(
                            format!("{}{}", i18n::t("error_prefix"), e),
                            MsgKind::Error,
                        ));
                        let _ = tx.send(WorkMsg::Done(false, e.to_string()));
                    }
                }
            });
            self.worker_handle = Some(handle);
        }
    }

    fn start_work(&mut self) {
        if self.working {
            return;
        }
        let input = self.input_path.trim().to_string();
        let output = self.output_path.trim().to_string();
        let password = self.password.clone();
        let level = self.level;
        let container = self.current_container();
        let encrypt_after = self.encrypt_archive;
        let mode = self.mode.clone();

        if input.is_empty() {
            self.log(i18n::t("select_input"), MsgKind::Error);
            return;
        }
        if !std::path::Path::new(&input).exists() {
            self.log(i18n::t("input_not_exist"), MsgKind::Error);
            return;
        }

        self.logs.clear();
        self.progress = 0.0;
        self.working = true;
        self.status_text = i18n::t("processing").to_string();
        self.log(
            &format!("{}{}", i18n::t("start_prefix"), mode.title()),
            MsgKind::Info,
        );
        self.log(&format!("  {}: {}", i18n::t("input"), input), MsgKind::Info);
        if !output.is_empty() {
            self.log(&format!("  {}: {}", i18n::t("output"), output), MsgKind::Info);
        }

        let tx = self.tx.clone();
        let handle = std::thread::spawn(move || {
            let result = run_task(mode, input, output, password, level, container, encrypt_after, tx.clone());
            match result {
                Ok(summary) => {
                    let _ = tx.send(WorkMsg::Progress(1.0));
                    let _ = tx.send(WorkMsg::Done(true, summary));
                }
                Err(e) => {
                    let e = friendly_error(e);
                    let _ = tx.send(WorkMsg::Log(
                        format!("{}{}", i18n::t("error_prefix"), e),
                        MsgKind::Error,
                    ));
                    let _ = tx.send(WorkMsg::Done(false, e.to_string()));
                }
            }
        });
        self.worker_handle = Some(handle);
    }

    fn pump_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkMsg::Log(text, kind) => {
                    self.logs.push(LogEntry {
                        text,
                        kind,
                        time: chrono_like::now(),
                    });
                }
                WorkMsg::Progress(p) => {
                    self.progress = p.clamp(0.0, 1.0);
                }
                WorkMsg::Done(ok, summary) => {
                    self.working = false;
                    if ok {
                        self.status_text = i18n::t("done").to_string();
                        self.log(&format!("✅ {}", summary), MsgKind::Success);
                    } else {
                        self.status_text = i18n::t("failed").to_string();
                        self.log(
                            &format!("{}{}", i18n::t("fail_prefix"), summary),
                            MsgKind::Error,
                        );
                    }
                }
            }
        }
    }
}

/// 检测是否为 macOS TCC 权限拒绝 (EPERM)
/// 在受保护目录 (Downloads/Desktop/Documents) 操作时, macOS 会返回 EPERM
fn is_perm_denied(e: &anyhow::Error) -> bool {
    // 检查错误链中是否有 os error 1 (EPERM)
    let mut source: Option<&dyn std::error::Error> = Some(e.as_ref());
    while let Some(err) = source {
        let msg = err.to_string();
        if msg.contains("os error 1") || msg.contains("Operation not permitted") {
            return true;
        }
        source = err.source();
    }
    false
}

/// 将普通错误包装为更友好的权限提示 (如果是 EPERM)
fn friendly_error(e: anyhow::Error) -> anyhow::Error {
    if is_perm_denied(&e) {
        anyhow::anyhow!("{}\n\n原始错误: {}", i18n::t("perm_denied"), e)
    } else {
        e
    }
}

fn run_task(
    mode: Mode,
    input: String,
    output: String,
    password: String,
    level: i32,
    container: Container,
    encrypt_after: bool,
    tx: Sender<WorkMsg>,
) -> anyhow::Result<String> {
    let inp = std::path::Path::new(&input);
    match mode {
        Mode::Compress => {
            let out_path = if output.is_empty() {
                compress::default_output(inp, container)
            } else {
                PathBuf::from(output)
            };
            let start = Instant::now();
            let bar = crate::progress::Progress::new("");
            let _ = tx.send(WorkMsg::Log(
                format!(
                    "  {}: {}, {}: {}",
                    i18n::t("format"),
                    container.display_name(),
                    i18n::t("compress_level"),
                    level
                ),
                MsgKind::Info,
            ));
            // 若容器支持内嵌加密且有密码, 直接用归档加密
            let archive_pwd = if container.supports_encryption() && encrypt_after && !password.is_empty() {
                Some(password.as_str())
            } else {
                None
            };
            compress::compress(inp, &out_path, container, level, archive_pwd, &bar)?;
            let _ = tx.send(WorkMsg::Progress(0.6));

            let mut final_path = out_path.clone();
            // 对不支持内嵌加密的容器, 退回 .enc 包装
            if encrypt_after && !password.is_empty() && !container.supports_encryption() {
                let _ = tx.send(WorkMsg::Log(
                    i18n::t("encrypting").to_string(),
                    MsgKind::Info,
                ));
                let enc_out = PathBuf::from(format!("{}.enc", out_path.display()));
                crypto::encrypt_file(&out_path, &enc_out, &password)?;
                let _ = std::fs::remove_file(&out_path);
                final_path = enc_out;
                let _ = tx.send(WorkMsg::Progress(0.95));
            }

            let elapsed = start.elapsed();
            let size = std::fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
            Ok(format!(
                "{} ({:.2}s, {})",
                i18n::t("compress_done"),
                elapsed.as_secs_f64(),
                format_size(size)
            ))
        }
        Mode::Decompress => {
            let out_dir = if output.is_empty() {
                PathBuf::from(".")
            } else {
                PathBuf::from(output)
            };
            let start = Instant::now();

            let name = inp
                .file_name()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            let pwd_opt = if password.is_empty() {
                None
            } else {
                Some(password.as_str())
            };

            let archive_path = if name.ends_with(".enc") {
                if pwd_opt.is_none() {
                    return Err(anyhow::anyhow!("{}", i18n::t("decrypt_need_password")));
                }
                let _ = tx.send(WorkMsg::Log(
                    i18n::t("decrypting").to_string(),
                    MsgKind::Info,
                ));
                let tmp = PathBuf::from(format!("{}.tmp", inp.display()));
                crypto::decrypt_file(inp, &tmp, pwd_opt.unwrap())?;
                let _ = tx.send(WorkMsg::Progress(0.5));
                tmp
            } else {
                inp.to_path_buf()
            };

            if detect(&archive_path).is_some() {
                std::fs::create_dir_all(&out_dir)?;
                let bar = crate::progress::Progress::new("");
                // 传递密码给归档解压 (zip/7z/rar 加密归档)
                decompress::decompress_with_password(&archive_path, &out_dir, pwd_opt, &bar)?;
                let _ = tx.send(WorkMsg::Progress(0.95));
                if archive_path != inp {
                    let _ = std::fs::remove_file(&archive_path);
                }
            }
            let elapsed = start.elapsed();
            Ok(format!(
                "{} ({:.2}s)",
                i18n::t("decompress_done"),
                elapsed.as_secs_f64()
            ))
        }
        Mode::Encrypt => {
            let out_path = if output.is_empty() {
                crypto::default_encrypt_output(inp)
            } else {
                PathBuf::from(output)
            };
            if password.is_empty() {
                return Err(anyhow::anyhow!("{}", i18n::t("encrypt_need_password")));
            }
            let start = Instant::now();
            crypto::encrypt_file(inp, &out_path, &password)?;
            let _ = tx.send(WorkMsg::Progress(0.95));
            let elapsed = start.elapsed();
            let size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
            Ok(format!(
                "{} ({:.2}s, {})",
                i18n::t("encrypt_done"),
                elapsed.as_secs_f64(),
                format_size(size)
            ))
        }
        Mode::Decrypt => {
            let out_path = if output.is_empty() {
                crypto::default_decrypt_output(inp)
            } else {
                PathBuf::from(output)
            };
            if password.is_empty() {
                return Err(anyhow::anyhow!("{}", i18n::t("decrypt_need_password")));
            }
            let start = Instant::now();
            crypto::decrypt_file(inp, &out_path, &password)?;
            let _ = tx.send(WorkMsg::Progress(0.95));

            if detect(&out_path).is_some() {
                let _ = tx.send(WorkMsg::Log(
                    i18n::t("continue_extract").to_string(),
                    MsgKind::Info,
                ));
                let extract_dir = out_path.with_extension("");
                std::fs::create_dir_all(&extract_dir)?;
                let bar = crate::progress::Progress::new("");
                decompress::decompress_with_password(&out_path, &extract_dir, None, &bar)?;
                let _ = std::fs::remove_file(&out_path);
            }
            let elapsed = start.elapsed();
            Ok(format!(
                "{} ({:.2}s)",
                i18n::t("decrypt_done"),
                elapsed.as_secs_f64()
            ))
        }
    }
}

/// 解压任务 (extract-here / extract-as 共用)
fn run_extract_task(
    input: String,
    output: String,
    password: String,
    tx: Sender<WorkMsg>,
    create_dir: bool,
) -> anyhow::Result<String> {
    let inp = std::path::Path::new(&input);
    let out_dir = PathBuf::from(&output);

    if create_dir {
        std::fs::create_dir_all(&out_dir)?;
    } else if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir)?;
    }

    let pwd_opt = if password.is_empty() {
        None
    } else {
        Some(password.as_str())
    };

    let name = inp
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let archive_path = if name.ends_with(".enc") {
        if pwd_opt.is_none() {
            return Err(anyhow::anyhow!("{}", i18n::t("decrypt_need_password")));
        }
        let _ = tx.send(WorkMsg::Log(
            i18n::t("decrypting").to_string(),
            MsgKind::Info,
        ));
        let tmp = PathBuf::from(format!("{}.tmp", inp.display()));
        crypto::decrypt_file(inp, &tmp, pwd_opt.unwrap())?;
        let _ = tx.send(WorkMsg::Progress(0.5));
        tmp
    } else {
        inp.to_path_buf()
    };

    let start = Instant::now();
    if detect(&archive_path).is_some() {
        let bar = crate::progress::Progress::new("");
        decompress::decompress_with_password(&archive_path, &out_dir, pwd_opt, &bar)?;
        let _ = tx.send(WorkMsg::Progress(0.95));
        if archive_path != inp {
            let _ = std::fs::remove_file(&archive_path);
        }
    } else {
        return Err(anyhow::anyhow!(
            "{}: {}",
            i18n::t("input_not_exist"),
            archive_path.display()
        ));
    }
    let elapsed = start.elapsed();
    Ok(format!(
        "{} ({:.2}s)",
        i18n::t("decompress_done"),
        elapsed.as_secs_f64()
    ))
}

/// 智能剥离归档文件的所有扩展名, 返回核心文件名
/// 例: "archive.tar.gz" → "archive", "data.tar.bz2" → "data",
///     "file.zip" → "file", "noext" → "noext"
fn archive_stem(path: &std::path::Path) -> String {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // 已知的复合扩展名 (从长到短排序, 优先匹配)
    const COMPOUND_EXTS: &[&str] = &[
        ".tar.gz", ".tar.xz", ".tar.zst", ".tar.bz2", ".tar.lz4", ".tar.lz",
        ".tar.lzma", ".tar.7z", ".tar.gz2",
    ];

    let lower = name.to_lowercase();
    for ext in COMPOUND_EXTS {
        if lower.ends_with(ext) {
            return name[..name.len() - ext.len()].to_string();
        }
    }

    // 单一扩展名: 使用 file_stem
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or(name)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ───────────────────────── eframe 实现 ─────────────────────────

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump_messages();
        if self.working {
            ctx.request_repaint_after(Duration::from_millis(80));
        }

        // 自定义深色背景
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = PANEL;
        ctx.set_visuals(visuals);

        egui::CentralPanel::default()
            .frame(egui::Frame::group(ctx.style().as_ref()).fill(BG).inner_margin(egui::Margin::same(16.0)))
            .show(ctx, |ui| {
                title_bar(ui, self);

                ui.add_space(8.0);

                mode_tabs(self, ui);

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    let left_width = (ui.available_width() * 0.46).max(360.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(left_width, ui.available_height()),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| left_panel(self, ui),
                    );

                    ui.separator();

                    right_panel(self, ui);
                });
            });
    }
}

fn title_bar(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(i18n::t("app_name"))
                .color(ACCENT)
                .font(FontId::proportional(24.0))
                .strong(),
        );
        ui.label(
            RichText::new(i18n::t("app_tagline"))
                .color(TEXT_DIM)
                .font(FontId::proportional(13.0)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // 语言切换按钮
            let lang = i18n::current_lang();
            let btn_label = match lang {
                Lang::Zh => "🌐 EN",
                Lang::En => "🌐 中文",
            };
            if ui.add_sized(
                Vec2::new(72.0, 26.0),
                egui::Button::new(
                    RichText::new(btn_label)
                        .color(TEXT)
                        .font(FontId::proportional(12.0)),
                ),
            ).clicked() {
                let new_lang = lang.toggle();
                i18n::set_lang(new_lang);
                app.status_text = i18n::t("ready").to_string();
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(i18n::t("app_version"))
                    .color(TEXT_DIM)
                    .font(FontId::proportional(11.0)),
            );
        });
    });
}

fn mode_tabs(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let modes = [Mode::Compress, Mode::Decompress, Mode::Encrypt, Mode::Decrypt];
        for m in modes {
            let selected = app.mode == m;
            let txt = RichText::new(m.title()).font(FontId::proportional(14.0)).strong();
            let btn = egui::SelectableLabel::new(selected, txt);
            let resp = ui.add_sized(Vec2::new(130.0, 30.0), btn);
            if resp.clicked() && !app.working {
                app.switch_mode(m);
            }
            if selected {
                resp.highlight();
            }
            ui.add_space(4.0);
        }
    });
}

fn left_panel(app: &mut App, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style())
        .fill(PANEL)
        .rounding(10.0)
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            section_label(ui, i18n::t("input"));
            ui.horizontal(|ui| {
                let _ = ui.add_sized(
                    Vec2::new(ui.available_width() - 150.0, 28.0),
                    egui::TextEdit::singleline(&mut app.input_path)
                        .hint_text(i18n::t("input_hint")),
                );
                if app.mode == Mode::Compress {
                    if ui.add_sized(Vec2::new(68.0, 28.0), egui::Button::new(i18n::t("pick_file"))).clicked() {
                        app.pick_input_file();
                    }
                    if ui.add_sized(Vec2::new(68.0, 28.0), egui::Button::new(i18n::t("pick_dir"))).clicked() {
                        app.pick_input_dir();
                    }
                } else {
                    if ui.add_sized(Vec2::new(140.0, 28.0), egui::Button::new(i18n::t("pick_file_btn"))).clicked() {
                        app.pick_input_file();
                    }
                }
            });

            ui.add_space(10.0);

            section_label(ui, i18n::t("output"));
            ui.horizontal(|ui| {
                let _ = ui.add_sized(
                    Vec2::new(ui.available_width() - 80.0, 28.0),
                    egui::TextEdit::singleline(&mut app.output_path)
                        .hint_text(i18n::t("output_hint")),
                );
                if ui.add_sized(Vec2::new(72.0, 28.0), egui::Button::new(i18n::t("browse"))).clicked() {
                    match app.mode {
                        Mode::Compress | Mode::Encrypt => app.pick_output_file(),
                        Mode::Decompress | Mode::Decrypt => app.pick_output_dir(),
                    }
                }
            });

            // ── 解压快捷按钮 ──
            if app.mode == Mode::Decompress {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let here_btn = egui::Button::new(
                        RichText::new(i18n::t("extract_here"))
                            .color(Color32::WHITE)
                            .font(FontId::proportional(12.0))
                            .strong(),
                    )
                    .fill(ACCENT_DIM)
                    .min_size(Vec2::new((ui.available_width() - 8.0) / 2.0, 30.0));
                    if ui.add(here_btn).clicked() && !app.working {
                        app.extract_here();
                    }
                    let to_btn = egui::Button::new(
                        RichText::new(i18n::t("extract_to"))
                            .color(Color32::WHITE)
                            .font(FontId::proportional(12.0))
                            .strong(),
                    )
                    .fill(ACCENT_DIM)
                    .min_size(Vec2::new(ui.available_width(), 30.0));
                    if ui.add(to_btn).clicked() && !app.working {
                        app.extract_as();
                    }
                });
            }

            ui.add_space(10.0);

            if app.mode == Mode::Compress {
                section_label(ui, i18n::t("format"));
                let containers = Container::all();
                let mut selected = app.selected_container_idx;
                egui::ComboBox::from_id_salt("fmt_combo")
                    .selected_text(containers[selected].display_name())
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for (i, c) in containers.iter().enumerate() {
                            ui.selectable_value(&mut selected, i, c.display_name());
                        }
                    });
                app.selected_container_idx = selected;

                ui.add_space(10.0);

                section_label(ui, &format!("{}: {}", i18n::t("compress_level"), app.level));
                let level_range = match app.current_container() {
                    Container::Zip => 0..=9,
                    Container::SevenZ => 1..=9,
                    Container::TarLz4 => 1..=12,
                    _ => 1..=9,
                };
                let _ = ui.add_sized(
                    Vec2::new(ui.available_width(), 20.0),
                    egui::Slider::new(&mut app.level, level_range).clamping(egui::SliderClamping::Always),
                );
                level_hint(ui, app.level, app.current_container());

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    let _ = ui.checkbox(&mut app.encrypt_archive, i18n::t("encrypt_after"));
                    if app.encrypt_archive {
                        ui.label(RichText::new("🔒").color(WARN));
                        // 若容器不支持内嵌加密, 提示将使用 .enc 包装
                        if !app.current_container().supports_encryption() {
                            ui.label(
                                RichText::new("(.enc)")
                                    .color(TEXT_DIM)
                                    .font(FontId::proportional(10.0)),
                            );
                        }
                    }
                });
            }

            let need_password = matches!(app.mode, Mode::Encrypt | Mode::Decrypt)
                || (app.mode == Mode::Compress && app.encrypt_archive)
                || (app.mode == Mode::Decompress
                    && (std::path::Path::new(&app.input_path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_lowercase().ends_with(".enc"))
                        .unwrap_or(false)
                        || is_likely_encrypted_archive(&app.input_path)));

            if need_password {
                ui.add_space(8.0);
                section_label(ui, i18n::t("password"));
                let _ = ui.add_sized(
                    Vec2::new(ui.available_width(), 28.0),
                    egui::TextEdit::singleline(&mut app.password)
                        .password(true)
                        .hint_text(i18n::t("password_hint")),
                );
            }

            ui.add_space(16.0);

            let btn_text = if app.working {
                app.status_text.clone()
            } else {
                format!("▶ {}", app.mode.title())
            };
            let btn_color = if app.working { ACCENT_DIM } else { ACCENT };
            let btn = egui::Button::new(
                RichText::new(btn_text)
                    .color(Color32::WHITE)
                    .font(FontId::proportional(15.0))
                    .strong(),
            )
            .fill(btn_color)
            .min_size(Vec2::new(ui.available_width(), 40.0));
            let resp = ui.add(btn);
            if resp.clicked() && !app.working {
                app.start_work();
            }
        });
}

/// 简单判断: 输入若是 zip/7z/rar 归档, 可能是加密的 — 显示密码框
fn is_likely_encrypted_archive(path: &str) -> bool {
    let name = std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name.ends_with(".zip") || name.ends_with(".7z") || name.ends_with(".rar")
}

fn level_hint(ui: &mut egui::Ui, level: i32, container: Container) {
    let (text, color) = match container {
        Container::TarLz4 => (
            if level <= 3 { "⚡ 极速 (LZ4 ~3GB/s)" } else { "LZ4 高压缩" },
            if level <= 3 { SUCCESS } else { WARN },
        ),
        Container::TarZst => (
            if level <= 2 { "⚡ 极速 Zstandard" } else if level <= 5 { "均衡" } else { "高压缩" },
            if level <= 5 { SUCCESS } else { WARN },
        ),
        Container::TarXz => ("LZMA 高压缩 (较慢)", WARN),
        Container::SevenZ => (
            if level <= 3 { "LZMA2 快速" } else { "LZMA2 高压缩 (较慢)" },
            if level <= 3 { SUCCESS } else { WARN },
        ),
        Container::Zip => (
            if level <= 3 { "⚡ Zstd 快速" } else if level <= 6 { "均衡" } else { "高压缩" },
            if level <= 6 { SUCCESS } else { WARN },
        ),
        _ => (
            if level <= 3 { "快速" } else if level <= 6 { "均衡" } else { "高压缩" },
            if level <= 6 { SUCCESS } else { WARN },
        ),
    };
    let _ = level;
    ui.label(RichText::new(text).color(color).font(FontId::proportional(11.0)));
}

fn right_panel(app: &mut App, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        egui::Frame::group(ui.style())
            .fill(PANEL)
            .rounding(10.0)
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(i18n::t("progress")).color(TEXT_DIM).font(FontId::proportional(13.0)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let pct = (app.progress * 100.0) as u32;
                        let done_label = i18n::t("done");
                        let status_color = if app.working {
                            ACCENT
                        } else if app.status_text == done_label {
                            SUCCESS
                        } else {
                            TEXT_DIM
                        };
                        ui.label(
                            RichText::new(format!("{} · {}%", app.status_text, pct))
                                .color(status_color)
                                .font(FontId::proportional(13.0)),
                        );
                    });
                });
                ui.add_space(6.0);
                let bar = egui::ProgressBar::new(app.progress)
                    .fill(ACCENT)
                    .desired_width(ui.available_width());
                ui.add(bar);
            });

        ui.add_space(12.0);

        egui::Frame::group(ui.style())
            .fill(PANEL)
            .rounding(10.0)
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(280.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(i18n::t("log")).color(TEXT_DIM).font(FontId::proportional(13.0)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(i18n::t("clear")).clicked() {
                            app.logs.clear();
                        }
                    });
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for entry in &app.logs {
                            let color = match entry.kind {
                                MsgKind::Info => TEXT,
                                MsgKind::Success => SUCCESS,
                                MsgKind::Warn => WARN,
                                MsgKind::Error => ERROR,
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(&entry.time)
                                        .color(TEXT_DIM)
                                        .font(FontId::monospace(11.0)),
                                );
                                ui.label(
                                    RichText::new(&entry.text)
                                        .color(color)
                                        .font(FontId::monospace(12.0)),
                                );
                            });
                        }
                    });
            });
    });
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .color(TEXT_DIM)
            .font(FontId::proportional(12.0))
            .strong(),
    );
}

mod chrono_like {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn now() -> String {
        let dur = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = dur.as_secs();
        let h = (secs / 3600) % 24;
        let m = (secs / 60) % 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02}", h, m, s)
    }
}

/// 启动 GUI
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(960.0, 640.0))
            .with_min_inner_size(Vec2::new(720.0, 520.0))
            .with_title("smart_ex"),
        ..Default::default()
    };
    eframe::run_native(
        "smart_ex",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
}

/// 配置中文字体: 尝试加载系统 CJK 字体, 失败则回退到默认
///
/// egui 默认字体不包含中文字形, 导致 GUI 中文显示为方框 (豆腐块).
/// 本函数按平台查找系统已有中文字体并注入 egui 字体表,
/// 保证单二进制无需外部资源即可正常显示中文.
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 按平台查找系统中文字体路径 (优先级从高到低)
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",           // PingFang (macOS 10.11+)
            "/System/Library/Fonts/STHeiti Medium.ttc",      // 黑体
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",    // 冬青黑体
            "/System/Library/Fonts/Supplemental/Songti.ttc", // 宋体
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\msyh.ttc",      // 微软雅黑
            "C:\\Windows\\Fonts\\msyh.ttf",
            "C:\\Windows\\Fonts\\msyhbd.ttc",    // 微软雅黑粗体
            "C:\\Windows\\Fonts\\simhei.ttf",     // 黑体
            "C:\\Windows\\Fonts\\simsun.ttc",     // 宋体
            "C:\\Windows\\Fonts\\Deng.ttf",        // 等线
        ]
    } else {
        // Linux: 常见 CJK 字体路径
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/wqy-zenhei/wqy-zenhei.ttc",
            "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/droid-fallback/DroidSansFallback.ttf",
        ]
    };

    // 尝试加载第一个可用的中文字体
    for path in candidates {
        if let Ok(font_data) = std::fs::read(path) {
            // 注入为名为 "CJK" 的字体族
            fonts.font_data.insert(
                "cjk".to_owned(),
                egui::FontData::from_owned(font_data),
            );

            // 将 CJK 字体插入到 Proportional 和 Monospace 字体族的开头
            // 这样 egui 优先用 CJK 字体渲染中文, 回退到默认字体渲染拉丁字符
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk".to_owned());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk".to_owned());

            ctx.set_fonts(fonts);
            return;
        }
    }

    // 未找到系统 CJK 字体: 使用默认字体 (中文可能显示为方框)
    // 用户可安装任一字体: macOS 自带 PingFang / Windows 自带微软雅黑 / Linux 安装 fonts-noto-cjk
    eprintln!("[smart_ex] 警告: 未找到系统中文字体, GUI 中文可能无法正常显示");
    eprintln!("[smart_ex] Linux 用户请安装: sudo apt install fonts-noto-cjk");
}
