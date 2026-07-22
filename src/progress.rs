use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// 统一进度条封装
pub struct Progress {
    bar: ProgressBar,
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
        Self { bar }
    }

    pub fn set_total(&self, total: u64) {
        self.bar.set_length(total);
    }

    pub fn inc(&self, n: u64) {
        self.bar.inc(n);
    }

    pub fn finish(&self, msg: &str) {
        self.bar.finish_with_message(msg.to_string());
    }
}
