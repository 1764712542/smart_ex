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
use crate::archive_list::ArchiveEntry;
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
    /// (bytes_done, bytes_total) — 用于显示速度/ETA
    ProgressDetail(u64, u64),
    /// 归档列表结果
    ArchiveList(Vec<ArchiveEntry>),
    /// 完整性测试结果 (摘要, 是否通过)
    TestResult(String, bool),
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

/// UI 主题
#[derive(Clone, Copy, Debug, PartialEq)]
enum Theme {
    Dark,
    Light,
}

impl Theme {
    fn toggle(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Theme::Dark => "🌙",
            Theme::Light => "☀️",
        }
    }
}

/// 主题色板 (根据主题动态返回)
struct Palette {
    bg: Color32,
    panel: Color32,
    text: Color32,
    text_dim: Color32,
}

impl Palette {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                bg: BG,
                panel: PANEL,
                text: TEXT,
                text_dim: TEXT_DIM,
            },
            Theme::Light => Self {
                bg: Color32::from_rgb(238, 240, 244),
                panel: Color32::from_rgb(248, 250, 252),
                text: Color32::from_rgb(30, 34, 42),
                text_dim: Color32::from_rgb(110, 120, 135),
            },
        }
    }
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
    /// 密码可见性切换
    show_password: bool,
    /// 文件排除规则 (通配符, 逗号分隔输入)
    exclude_patterns: String,
    /// 分卷大小 (例: 100M, 1G), 留空则不分卷
    split_size: String,

    logs: Vec<LogEntry>,
    progress: f32,
    /// 进度详情: (bytes_done, bytes_total)
    progress_detail: (u64, u64),
    working: bool,
    status_text: String,
    /// 取消标志 (工作线程通过此标志检测取消请求)
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,

    /// 归档内容浏览 (List 结果)
    archive_entries: Vec<ArchiveEntry>,
    /// 是否显示归档列表面板
    show_archive_panel: bool,

    /// 最近使用的文件路径
    recent_files: Vec<String>,

    /// Toast 通知 (消息, 显示截止时间)
    toast: Option<(String, MsgKind, Instant)>,

    /// 当前 UI 主题
    theme: Theme,
    /// 压缩/加密后安全删除源文件
    secure_delete: bool,

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
            show_password: false,
            exclude_patterns: String::new(),
            split_size: String::new(),
            logs: Vec::new(),
            progress: 0.0,
            progress_detail: (0, 0),
            working: false,
            status_text: i18n::t("ready").to_string(),
            cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            archive_entries: Vec::new(),
            show_archive_panel: false,
            recent_files: Vec::new(),
            toast: None,
            theme: Theme::Dark,
            secure_delete: false,
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
            let s = path.display().to_string();
            self.add_recent_file(&s);
            self.input_path = s;
            self.output_path.clear();
            self.auto_fill_output();
            self.maybe_switch_mode_by_input();
        }
    }

    fn pick_input_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            let s = path.display().to_string();
            self.add_recent_file(&s);
            self.input_path = s;
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
        let secure_delete = self.secure_delete;
        let mode = self.mode.clone();

        // 解析排除规则 (逗号分隔)
        let excludes: Vec<String> = if self.exclude_patterns.trim().is_empty() {
            Vec::new()
        } else {
            self.exclude_patterns
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };

        // 解析分卷大小
        let split = if self.split_size.trim().is_empty() {
            None
        } else {
            Some(self.split_size.trim().to_string())
        };

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
        self.progress_detail = (0, 0);
        self.working = true;
        self.cancel_flag.store(false, std::sync::atomic::Ordering::Relaxed);
        self.status_text = i18n::t("processing").to_string();
        self.log(
            &format!("{}{}", i18n::t("start_prefix"), mode.title()),
            MsgKind::Info,
        );
        self.log(&format!("  {}: {}", i18n::t("input"), input), MsgKind::Info);
        if !output.is_empty() {
            self.log(&format!("  {}: {}", i18n::t("output"), output), MsgKind::Info);
        }
        if !excludes.is_empty() {
            self.log(&format!("  {}: {}", i18n::t("exclude_label"), excludes.join(", ")), MsgKind::Info);
        }
        if let Some(s) = &split {
            self.log(&format!("  {}: {}", i18n::t("split_label"), s), MsgKind::Info);
        }

        let tx = self.tx.clone();
        let cancel = self.cancel_flag.clone();
        let handle = std::thread::spawn(move || {
            let result = run_task(
                mode, input, output, password, level, container, encrypt_after,
                excludes, split, secure_delete, cancel, tx.clone(),
            );
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

    /// 取消当前任务
    fn cancel_work(&mut self) {
        if !self.working {
            return;
        }
        self.cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        self.log(i18n::t("cancel_requested"), MsgKind::Warn);
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
                WorkMsg::ProgressDetail(done, total) => {
                    self.progress_detail = (done, total);
                }
                WorkMsg::ArchiveList(entries) => {
                    self.working = false;
                    self.archive_entries = entries;
                    self.show_archive_panel = true;
                    self.status_text = i18n::t("done").to_string();
                    let count = self.archive_entries.len();
                    self.show_toast(&format!("📋 {} {}", count, i18n::t("entries")), MsgKind::Success);
                }
                WorkMsg::TestResult(summary, ok) => {
                    self.working = false;
                    self.status_text = if ok { i18n::t("done").to_string() } else { i18n::t("failed").to_string() };
                    if ok {
                        self.log(&format!("✅ {}", summary), MsgKind::Success);
                        self.show_toast(&summary, MsgKind::Success);
                    } else {
                        self.log(&format!("❌ {}", summary), MsgKind::Error);
                        self.show_toast(&summary, MsgKind::Error);
                    }
                }
                WorkMsg::Done(ok, summary) => {
                    self.working = false;
                    self.progress_detail = (0, 0);
                    if ok {
                        self.status_text = i18n::t("done").to_string();
                        self.log(&format!("✅ {}", summary), MsgKind::Success);
                        self.show_toast(&summary, MsgKind::Success);
                    } else {
                        self.status_text = i18n::t("failed").to_string();
                        self.log(
                            &format!("{}{}", i18n::t("fail_prefix"), summary),
                            MsgKind::Error,
                        );
                        self.show_toast(&format!("{}{}", i18n::t("fail_prefix"), summary), MsgKind::Error);
                    }
                }
            }
        }
    }

    /// 显示 Toast 通知 (3秒后自动消失)
    fn show_toast(&mut self, msg: &str, kind: MsgKind) {
        self.toast = Some((msg.to_string(), kind, Instant::now() + Duration::from_secs(3)));
    }

    /// 添加最近文件
    fn add_recent_file(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_string());
        if self.recent_files.len() > 8 {
            self.recent_files.truncate(8);
        }
    }

    /// 浏览归档内容 (List)
    fn list_archive_gui(&mut self) {
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

        self.working = true;
        self.status_text = i18n::t("processing").to_string();
        self.log(&format!("📋 {}", i18n::t("list_archive")), MsgKind::Info);

        let password = if self.password.is_empty() { None } else { Some(self.password.clone()) };
        let tx = self.tx.clone();
        let handle = std::thread::spawn(move || {
            match crate::archive_list::list_archive(std::path::Path::new(&input), password.as_deref()) {
                Ok(entries) => {
                    let _ = tx.send(WorkMsg::ArchiveList(entries));
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

    /// 测试归档完整性
    fn test_archive_gui(&mut self) {
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

        self.working = true;
        self.status_text = i18n::t("processing").to_string();
        self.log(&format!("🧪 {}", i18n::t("test_archive")), MsgKind::Info);

        let password = if self.password.is_empty() { None } else { Some(self.password.clone()) };
        let tx = self.tx.clone();
        let handle = std::thread::spawn(move || {
            let bar = make_progress(&tx);
            match crate::decompress::test_archive(std::path::Path::new(&input), password.as_deref(), &bar) {
                Ok((entries, bytes)) => {
                    let summary = format!(
                        "{}: {} {}, {}",
                        i18n::t("test_pass"),
                        entries,
                        i18n::t("entries"),
                        crate::progress::format_bytes(bytes)
                    );
                    let _ = tx.send(WorkMsg::TestResult(summary, true));
                }
                Err(e) => {
                    let e = friendly_error(e);
                    let summary = format!("{}: {}", i18n::t("test_fail"), e);
                    let _ = tx.send(WorkMsg::TestResult(summary, false));
                }
            }
        });
        self.worker_handle = Some(handle);
    }

    /// 生成随机密码
    fn generate_password(&mut self) {
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        const UPPER: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
        const LOWER: &[u8] = b"abcdefghijkmnpqrstuvwxyz";
        const DIGIT: &[u8] = b"23456789";
        const SPECIAL: &[u8] = b"!@#$%^&*-_+=";
        let len = 16;
        let mut pwd = String::with_capacity(len);
        let mut state = seed;
        for i in 0..len {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let pool = match i % 4 {
                0 => UPPER,
                1 => LOWER,
                2 => DIGIT,
                _ => SPECIAL,
            };
            let idx = (state % pool.len() as u64) as usize;
            pwd.push(pool[idx] as char);
        }
        self.password = pwd;
        self.show_toast("🔐 密码已生成", MsgKind::Info);
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

/// 创建带 GUI 实时进度回调的 Progress
fn make_progress(tx: &Sender<WorkMsg>) -> crate::progress::Progress {
    let tx_clone = tx.clone();
    let callback: crate::progress::ProgressCallback = std::sync::Arc::new(
        move |cur, total, bytes_done, bytes_total| {
            if total > 0 {
                let pct = cur as f32 / total as f32;
                let _ = tx_clone.send(WorkMsg::Progress(pct.min(1.0)));
            }
            // 发送字节级详情 (速度/ETA)
            if bytes_total > 0 || bytes_done > 0 {
                let _ = tx_clone.send(WorkMsg::ProgressDetail(bytes_done, bytes_total));
            }
        },
    );
    crate::progress::Progress::new_with_callback("", callback)
}

fn run_task(
    mode: Mode,
    input: String,
    output: String,
    password: String,
    level: i32,
    container: Container,
    encrypt_after: bool,
    excludes: Vec<String>,
    split: Option<String>,
    secure_delete: bool,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    tx: Sender<WorkMsg>,
) -> anyhow::Result<String> {
    let inp = std::path::Path::new(&input);
    /// 检查取消标志
    macro_rules! check_cancel {
        () => {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(anyhow::anyhow!("{}", i18n::t("cancelled")));
            }
        };
    }

    match mode {
        Mode::Compress => {
            let out_path = if output.is_empty() {
                compress::default_output(inp, container)
            } else {
                PathBuf::from(output)
            };
            let start = Instant::now();
            let bar = make_progress(&tx);
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

            // 有排除规则 → 使用 compress_with_exclude
            if excludes.is_empty() {
                compress::compress(inp, &out_path, container, level, archive_pwd, &bar)?;
            } else {
                compress::compress_with_exclude(inp, &out_path, container, level, archive_pwd, &bar, &excludes)?;
            }

            check_cancel!();

            let mut final_path = out_path.clone();

            // 分卷切割
            if let Some(split_str) = &split {
                if let Ok(split_size) = compress::parse_split_size(split_str) {
                    let parts = compress::split_file(&out_path, split_size)?;
                    if parts.len() > 1 {
                        let _ = tx.send(WorkMsg::Log(
                            format!("✂️ {}: {} 个分卷", i18n::t("split_label"), parts.len()),
                            MsgKind::Info,
                        ));
                        let elapsed = start.elapsed();
                        return Ok(format!(
                            "{} ({:.2}s, {} 个分卷)",
                            i18n::t("compress_done"),
                            elapsed.as_secs_f64(),
                            parts.len()
                        ));
                    }
                }
            }

            // 对不支持内嵌加密的容器, 退回 .enc 包装
            if encrypt_after && !password.is_empty() && !container.supports_encryption() {
                check_cancel!();
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

            // 安全删除源文件
            if secure_delete {
                check_cancel!();
                let _ = tx.send(WorkMsg::Log(
                    i18n::t("secure_delete_done").to_string(),
                    MsgKind::Info,
                ));
                secure_delete_path(inp);
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

            check_cancel!();

            if detect(&archive_path).is_some() {
                std::fs::create_dir_all(&out_dir)?;
                let bar = make_progress(&tx);
                // 传递密码给归档解压 (zip/7z/rar 加密归档)
                decompress::decompress_with_password(&archive_path, &out_dir, pwd_opt, &bar)?;
                let _ = tx.send(WorkMsg::Progress(0.95));
                // 清理临时文件
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
            // 安全删除源文件
            if secure_delete {
                check_cancel!();
                let _ = tx.send(WorkMsg::Log(
                    i18n::t("secure_delete_done").to_string(),
                    MsgKind::Info,
                ));
                secure_delete_path(inp);
            }
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

            check_cancel!();

            if detect(&out_path).is_some() {
                let _ = tx.send(WorkMsg::Log(
                    i18n::t("continue_extract").to_string(),
                    MsgKind::Info,
                ));
                let extract_dir = out_path.with_extension("");
                std::fs::create_dir_all(&extract_dir)?;
                let bar = make_progress(&tx);
                decompress::decompress_with_password(&out_path, &extract_dir, None, &bar)?;
                // 清理临时解密文件
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
        tmp
    } else {
        inp.to_path_buf()
    };

    let start = Instant::now();
    if detect(&archive_path).is_some() {
        let bar = make_progress(&tx);
        decompress::decompress_with_password(&archive_path, &out_dir, pwd_opt, &bar)?;
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

/// 安全删除文件: 先用随机数据覆写 3 次, 再删除
/// 对目录则递归安全删除其中所有文件, 最后移除目录
fn secure_delete_path(path: &std::path::Path) {
    if path.is_dir() {
        // 递归安全删除目录内文件
        let mut stack = vec![path.to_path_buf()];
        let mut files = Vec::new();
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else {
                        files.push(p);
                    }
                }
            }
        }
        for f in &files {
            secure_delete_file(f);
        }
        // 删除空目录 (后序)
        let _ = std::fs::remove_dir_all(path);
    } else if path.is_file() {
        secure_delete_file(path);
    }
}

/// 安全删除单个文件: 覆写 3 次后删除
fn secure_delete_file(path: &std::path::Path) {
    let len = match std::fs::metadata(path) {
        Ok(m) => m.len() as usize,
        Err(_) => {
            let _ = std::fs::remove_file(path);
            return;
        }
    };
    // 覆写 3 次: 全 0xFF / 全 0x00 / 随机
    let patterns: [Vec<u8>; 3] = [
        vec![0xFF; len.min(1024 * 1024)],
        vec![0x00; len.min(1024 * 1024)],
        {
            use std::time::SystemTime;
            let mut seed = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0xCAFE);
            let mut buf = vec![0u8; len.min(1024 * 1024)];
            for b in &mut buf {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                *b = (seed & 0xFF) as u8;
            }
            buf
        },
    ];
    for pattern in &patterns {
        if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(path) {
            use std::io::Write;
            let mut remaining = len;
            while remaining > 0 {
                let chunk = remaining.min(pattern.len());
                if f.write_all(&pattern[..chunk]).is_err() {
                    break;
                }
                remaining -= chunk;
            }
            let _ = f.flush();
            let _ = f.sync_all();
        }
    }
    let _ = std::fs::remove_file(path);
}

// ───────────────────────── eframe 实现 ─────────────────────────

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump_messages();
        if self.working {
            ctx.request_repaint_after(Duration::from_millis(80));
        }

        // ── 拖放支持 ──
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            for file in &dropped {
                if let Some(path) = file.path.as_ref() {
                    self.input_path = path.display().to_string();
                    self.output_path.clear();
                    self.auto_fill_output();
                    self.maybe_switch_mode_by_input();
                    self.add_recent_file(&path.display().to_string());
                    break;
                }
            }
        }

        // ── 键盘快捷键 ──
        ctx.input(|i| {
            // Ctrl+Enter: 开始任务
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Enter) && !self.working {
                self.start_work();
            }
            // Escape: 取消任务 或 关闭归档面板
            if i.key_pressed(egui::Key::Escape) {
                if self.show_archive_panel {
                    self.show_archive_panel = false;
                } else if self.working {
                    self.cancel_work();
                }
            }
            // Ctrl+L: 清空日志
            if i.modifiers.ctrl && i.key_pressed(egui::Key::L) {
                self.logs.clear();
            }
        });

        // 根据主题设置 visuals
        let pal = Palette::from(self.theme);
        match self.theme {
            Theme::Dark => {
                let mut visuals = egui::Visuals::dark();
                visuals.panel_fill = pal.bg;
                visuals.window_fill = pal.panel;
                ctx.set_visuals(visuals);
            }
            Theme::Light => {
                let mut visuals = egui::Visuals::light();
                visuals.panel_fill = pal.bg;
                visuals.window_fill = pal.panel;
                ctx.set_visuals(visuals);
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::group(ctx.style().as_ref()).fill(pal.bg).inner_margin(egui::Margin::same(16.0)))
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

        // ── 归档列表面板 (浮动窗口) ──
        if self.show_archive_panel {
            archive_list_window(self, ctx);
        }

        // ── Toast 通知 ──
        show_toast_overlay(self, ctx);
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
            // 主题切换按钮
            if ui.add_sized(
                Vec2::new(40.0, 26.0),
                egui::Button::new(
                    RichText::new(app.theme.icon())
                        .font(FontId::proportional(14.0)),
                ),
            ).clicked() {
                app.theme = app.theme.toggle();
            }
            ui.add_space(4.0);
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

                // 浏览 + 测试 按钮
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let list_btn = egui::Button::new(
                        RichText::new(i18n::t("list_archive"))
                            .font(FontId::proportional(12.0)),
                    )
                    .min_size(Vec2::new((ui.available_width() - 8.0) / 2.0, 28.0));
                    if ui.add(list_btn).clicked() && !app.working {
                        app.list_archive_gui();
                    }
                    let test_btn = egui::Button::new(
                        RichText::new(i18n::t("test_archive"))
                            .font(FontId::proportional(12.0)),
                    )
                    .min_size(Vec2::new(ui.available_width(), 28.0));
                    if ui.add(test_btn).clicked() && !app.working {
                        app.test_archive_gui();
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

            // 安全删除源文件选项 (压缩/加密模式)
            if matches!(app.mode, Mode::Compress | Mode::Encrypt) {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let _ = ui.checkbox(&mut app.secure_delete, i18n::t("secure_delete"));
                    if app.secure_delete {
                        ui.label(RichText::new("🗑️").color(WARN));
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
                ui.horizontal(|ui| {
                    let _ = ui.add_sized(
                        Vec2::new(ui.available_width() - 90.0, 28.0),
                        egui::TextEdit::singleline(&mut app.password)
                            .password(!app.show_password)
                            .hint_text(i18n::t("password_hint")),
                    );
                    // 密码可见性切换按钮
                    let eye = if app.show_password { "🙈" } else { "👁" };
                    if ui.add_sized(Vec2::new(36.0, 28.0), egui::Button::new(eye)).clicked() {
                        app.show_password = !app.show_password;
                    }
                    // 密码生成器按钮
                    if ui.add_sized(Vec2::new(36.0, 28.0), egui::Button::new("🎲")).clicked() {
                        app.generate_password();
                    }
                });
            }

            // 最近文件 (下拉选择)
            if !app.recent_files.is_empty() && !app.working {
                ui.add_space(8.0);
                section_label(ui, i18n::t("recent_files"));
                egui::ComboBox::from_id_salt("recent_combo")
                    .selected_text("")
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for path in &app.recent_files.clone() {
                            let label = std::path::Path::new(path)
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.clone());
                            if ui.selectable_label(false, &label).clicked() {
                                app.input_path = path.clone();
                                app.output_path.clear();
                                app.auto_fill_output();
                                app.maybe_switch_mode_by_input();
                            }
                            ui.label(
                                RichText::new(path)
                                    .color(TEXT_DIM)
                                    .font(FontId::monospace(10.0)),
                            );
                        }
                    });
            }

            // 压缩模式: 排除规则 + 分卷大小
            if app.mode == Mode::Compress {
                ui.add_space(8.0);
                section_label(ui, i18n::t("exclude_label"));
                let _ = ui.add_sized(
                    Vec2::new(ui.available_width(), 28.0),
                    egui::TextEdit::singleline(&mut app.exclude_patterns)
                        .hint_text(i18n::t("exclude_hint")),
                );

                ui.add_space(8.0);
                section_label(ui, i18n::t("split_label"));
                let _ = ui.add_sized(
                    Vec2::new(ui.available_width(), 28.0),
                    egui::TextEdit::singleline(&mut app.split_size)
                        .hint_text(i18n::t("split_hint")),
                );
            }

            ui.add_space(16.0);

            // 主按钮 + 取消按钮
            ui.horizontal(|ui| {
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
                .min_size(Vec2::new(ui.available_width() - 80.0, 40.0));
                let resp = ui.add(btn);
                if resp.clicked() && !app.working {
                    app.start_work();
                }

                // 取消按钮
                if app.working {
                    let cancel_btn = egui::Button::new(
                        RichText::new("✕")
                            .color(Color32::WHITE)
                            .font(FontId::proportional(15.0)),
                    )
                    .fill(ERROR)
                    .min_size(Vec2::new(40.0, 40.0));
                    if ui.add(cancel_btn).clicked() {
                        app.cancel_work();
                    }
                }
            });
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

                // 进度详情: 字节数 + 速度 + ETA
                let (done, total) = app.progress_detail;
                if total > 0 && app.working {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "{} / {}",
                            format_size(done),
                            format_size(total)
                        ))
                        .color(TEXT_DIM)
                        .font(FontId::monospace(11.0)),
                    );
                }
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
                            // 时间戳 + 日志文本在同一行, 长文本自动换行
                            // 使用 horizontal_wrapped 确保超长行不会溢出边界
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.label(
                                    RichText::new(&entry.time)
                                        .color(TEXT_DIM)
                                        .font(FontId::monospace(11.0)),
                                );
                                // 日志文本占满剩余宽度, 自动换行
                                ui.label(
                                    RichText::new(&entry.text)
                                        .color(color)
                                        .font(FontId::monospace(12.0)),
                                );
                            });
                            ui.add_space(2.0);
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

/// 归档列表面板 (浮动窗口)
fn archive_list_window(app: &mut App, ctx: &egui::Context) {
    let mut open = true;
    egui::Window::new(format!("📋 {}", i18n::t("list_archive")))
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .min_width(500.0)
        .min_height(350.0)
        .default_width(640.0)
        .default_height(450.0)
        .show(ctx, |ui| {
            if app.archive_entries.is_empty() {
                ui.label(
                    RichText::new("(空)")
                        .color(TEXT_DIM)
                        .font(FontId::proportional(14.0)),
                );
                return;
            }

            // 统计
            let total_size: u64 = app.archive_entries.iter().map(|e| e.size).sum();
            let total_compressed: u64 = app.archive_entries.iter().map(|e| e.compressed_size).sum();
            let file_count = app.archive_entries.iter().filter(|e| !e.is_dir).count();
            let dir_count = app.archive_entries.iter().filter(|e| e.is_dir).count();

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{}: {} · {}: {} · {}: {}",
                        i18n::t("total"),
                        app.archive_entries.len(),
                        i18n::t("files"),
                        file_count,
                        i18n::t("dirs"),
                        dir_count
                    ))
                    .color(TEXT_DIM)
                    .font(FontId::proportional(12.0)),
                );
            });
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{}: {} · {}: {}",
                        i18n::t("uncompressed"),
                        format_size(total_size),
                        i18n::t("compressed"),
                        format_size(total_compressed)
                    ))
                    .color(TEXT_DIM)
                    .font(FontId::monospace(11.0)),
                );
                if total_size > 0 {
                    let ratio = (1.0 - total_compressed as f64 / total_size as f64) * 100.0;
                    ui.label(
                        RichText::new(format!("({:.1}% {})", ratio, i18n::t("saved")))
                            .color(SUCCESS)
                            .font(FontId::monospace(11.0)),
                    );
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // 表头
            ui.horizontal(|ui| {
                ui.label(RichText::new(i18n::t("name")).color(TEXT_DIM).font(FontId::proportional(12.0)).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(i18n::t("size")).color(TEXT_DIM).font(FontId::proportional(12.0)).strong());
                });
            });
            ui.separator();

            // 文件列表 (可滚动)
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for entry in &app.archive_entries {
                        ui.horizontal(|ui| {
                            let icon = if entry.is_dir { "📁" } else { "📄" };
                            let color = if entry.is_dir { TEXT_DIM } else { TEXT };
                            ui.label(
                                RichText::new(format!("{} {}", icon, entry.name))
                                    .color(color)
                                    .font(FontId::monospace(11.0)),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format_size(entry.size))
                                        .color(TEXT_DIM)
                                        .font(FontId::monospace(11.0)),
                                );
                            });
                        });
                    }
                });
        });

    if !open {
        app.show_archive_panel = false;
    }
}

/// Toast 通知覆盖层
fn show_toast_overlay(app: &mut App, ctx: &egui::Context) {
    // 检查 Toast 是否过期
    if let Some((_, _, expiry)) = &app.toast {
        if Instant::now() >= *expiry {
            app.toast = None;
            return;
        }
    } else {
        return;
    }

    let (msg, kind, _) = app.toast.as_ref().unwrap();
    let color = match kind {
        MsgKind::Info => ACCENT,
        MsgKind::Success => SUCCESS,
        MsgKind::Warn => WARN,
        MsgKind::Error => ERROR,
    };

    egui::Area::new(egui::Id::new("toast_overlay"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-20.0, 20.0))
        .show(ctx, |ui| {
            egui::Frame::group(ui.style())
                .fill(PANEL)
                .rounding(8.0)
                .stroke(egui::Stroke::new(1.0, color))
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(msg).color(color).font(FontId::proportional(13.0)));
                    });
                });
        });

    // 请求重绘以更新过期检查
    ctx.request_repaint_after(Duration::from_millis(500));
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
