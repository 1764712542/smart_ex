//! SmartEx Tauri 后端入口
//!
//! IPC 命令将在后续迭代中添加

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    smartex_tauri_lib::run()
}
