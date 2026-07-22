use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 进度回调函数类型: (current, total, bytes_done, bytes_total) -> ()
/// 新增 bytes 参数用于计算速度和 ETA
pub type ProgressCallback = Arc<dyn Fn(u64, u64, u64, u64) + Send + Sync>;

/// 统一进度条封装
///
/// 同时支持 CLI 进度条 (indicatif) 和 GUI 实时进度回调.
/// 当 callback 被设置时, 每次 inc() 都会触发回调, 让 GUI 实时更新进度.
/// v0.5.0: 新增 bytes 级别进度 (用于计算速度和剩余时间)
pub struct Progress {
    bar: ProgressBar,
    total: Arc<AtomicU64>,
    current: Arc<AtomicU64>,
    /// 已处理字节数
    bytes_done: Arc<AtomicU64>,
    /// 总字节数 (若已知)
    bytes_total: Arc<AtomicU64>,
    /// 起始时间 (用于计算速度)
    start_time: Instant,
    callback: Option<ProgressCallback>,
}

impl Progress {
    pub fn new(message: &str) -> Self {
        let bar = ProgressBar::new(0);
        bar.set_message(message.to_string());
        bar.set_style(
            ProgressStyle::with_template(
                "{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {per_sec}",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        Self {
            bar,
            total: Arc::new(AtomicU64::new(0)),
            current: Arc::new(AtomicU64::new(0)),
            bytes_done: Arc::new(AtomicU64::new(0)),
            bytes_total: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            callback: None,
        }
    }

    /// 创建带 GUI 回调的 Progress
    pub fn new_with_callback(message: &str, callback: ProgressCallback) -> Self {
        let mut p = Self::new(message);
        p.callback = Some(callback);
        p
    }

    pub fn set_total(&self, total: u64) {
        self.total.store(total, Ordering::Relaxed);
        self.bar.set_length(total);
    }

    /// 设置总字节数 (用于计算速度和 ETA)
    pub fn set_bytes_total(&self, bytes: u64) {
        self.bytes_total.store(bytes, Ordering::Relaxed);
    }

    /// 增加已处理字节数
    pub fn inc_bytes(&self, bytes: u64) {
        self.bytes_done.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn inc(&self, n: u64) {
        let cur = self.current.fetch_add(n, Ordering::Relaxed) + n;
        let total = self.total.load(Ordering::Relaxed);
        self.bar.inc(n);
        // 触发 GUI 回调 (如果有) — 传递文件数和字节数
        if let Some(cb) = &self.callback {
            let bytes_done = self.bytes_done.load(Ordering::Relaxed);
            let bytes_total = self.bytes_total.load(Ordering::Relaxed);
            cb(cur, total, bytes_done, bytes_total);
        }
    }

    /// 获取进度详情字符串 (用于 GUI 显示)
    pub fn detail_string(&self) -> String {
        let bytes_done = self.bytes_done.load(Ordering::Relaxed);
        let bytes_total = self.bytes_total.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();

        if bytes_total > 0 && elapsed > 0.0 {
            let speed = bytes_done as f64 / elapsed;
            let remaining = if speed > 0.0 {
                (bytes_total - bytes_done) as f64 / speed
            } else {
                0.0
            };
            format!(
                "{} / {} · {}/s · ETA {}s",
                format_bytes(bytes_done),
                format_bytes(bytes_total),
                format_bytes(speed as u64),
                remaining as u64
            )
        } else if elapsed > 0.0 && bytes_done > 0 {
            let speed = bytes_done as f64 / elapsed;
            format!("{} · {}/s", format_bytes(bytes_done), format_bytes(speed as u64))
        } else {
            String::new()
        }
    }

    pub fn finish(&self, msg: &str) {
        self.bar.finish_with_message(msg.to_string());
        // 确保最终进度为 100%
        if let Some(cb) = &self.callback {
            let total = self.total.load(Ordering::Relaxed);
            let bytes_total = self.bytes_total.load(Ordering::Relaxed);
            cb(total, total, bytes_total, bytes_total);
        }
    }
}

/// 格式化字节数为人类可读字符串
pub fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.2} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{} B", n)
    }
}
