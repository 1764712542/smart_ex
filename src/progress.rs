use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 进度回调函数类型: (current, total) -> ()
pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

/// 统一进度条封装
///
/// 同时支持 CLI 进度条 (indicatif) 和 GUI 实时进度回调.
/// 当 callback 被设置时, 每次 inc() 都会触发回调, 让 GUI 实时更新进度.
pub struct Progress {
    bar: ProgressBar,
    total: Arc<AtomicU64>,
    current: Arc<AtomicU64>,
    callback: Option<ProgressCallback>,
}

impl Progress {
    pub fn new(message: &str) -> Self {
        let bar = ProgressBar::new(0);
        bar.set_message(message.to_string());
        bar.set_style(
            ProgressStyle::with_template(
                "{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        Self {
            bar,
            total: Arc::new(AtomicU64::new(0)),
            current: Arc::new(AtomicU64::new(0)),
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

    pub fn inc(&self, n: u64) {
        let cur = self.current.fetch_add(n, Ordering::Relaxed) + n;
        let total = self.total.load(Ordering::Relaxed);
        self.bar.inc(n);
        // 触发 GUI 回调 (如果有)
        if let Some(cb) = &self.callback {
            cb(cur, total);
        }
    }

    pub fn finish(&self, msg: &str) {
        self.bar.finish_with_message(msg.to_string());
        // 确保最终进度为 100%
        if let Some(cb) = &self.callback {
            let total = self.total.load(Ordering::Relaxed);
            cb(total, total);
        }
    }
}
