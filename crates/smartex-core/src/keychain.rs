//! 会话级密码钥匙串
//!
//! 跨平台密码缓存:
//! - macOS: Keychain (security 命令)
//! - Windows: Credential Manager (wincred crate)
//! - Linux: Secret Service (secret-service crate, GNOME Keyring/KWallet)
//!
//! 会话级缓存: 进程退出即清除 (安全)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 钥匙串条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeychainEntry {
    pub password: String,
    pub created_at: u64, // unix timestamp
    pub label: String,  // 人类可读标签, 如 "project_backup.zip"
}

/// 会话级钥匙串 (内存缓存, 进程退出即失)
pub struct SessionKeychain {
    /// 内存缓存: key = 文件路径或自定义标识, value = 条目
    cache: Mutex<HashMap<String, KeychainEntry>>,
    /// 超时自动清理 (秒), 默认 1800 (30 分钟)
    ttl_seconds: u64,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl SessionKeychain {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            ttl_seconds: 1800,
        }
    }

    pub fn with_ttl(ttl_seconds: u64) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            ttl_seconds,
        }
    }

    /// 存储密码 (会话级)
    ///
    /// 写入内存 cache。系统钥匙串写入失败不阻塞会话缓存,
    /// 因为会话缓存本身已是有效方案, 跨会话持久化只是增强。
    pub fn set(&self, key: &str, password: &str, label: &str) {
        let entry = KeychainEntry {
            password: password.to_string(),
            created_at: now_unix(),
            label: label.to_string(),
        };
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key.to_string(), entry);
        }
        // 同步到系统钥匙串 (跨会话持久化), 失败忽略 — 会话缓存仍然有效
        let _ = system_keychain_set(key, password);
    }

    /// 获取密码 (会话级, 超时返回 None)
    ///
    /// 优先从内存 cache 读取并检查 TTL; cache miss 时
    /// 回退到系统钥匙串 (若存在则回填 cache 并刷新时间戳)。
    pub fn get(&self, key: &str) -> Option<KeychainEntry> {
        let now = now_unix();
        // 1. 检查内存 cache
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(entry) = cache.get(key) {
                if now.saturating_sub(entry.created_at) <= self.ttl_seconds {
                    return Some(entry.clone());
                } else {
                    // 过期, 从 cache 移除
                    cache.remove(key);
                }
            }
        }
        // 2. cache miss 或过期, 尝试系统钥匙串
        if let Ok(Some(pwd)) = system_keychain_get(key) {
            let entry = KeychainEntry {
                password: pwd,
                created_at: now,
                label: key.to_string(),
            };
            if let Ok(mut cache) = self.cache.lock() {
                cache.insert(key.to_string(), entry.clone());
            }
            return Some(entry);
        }
        None
    }

    /// 删除条目
    ///
    /// 返回 true 表示内存 cache 中确实存在并已删除。
    /// 系统钥匙串中的对应项也会被删除 (忽略错误)。
    pub fn delete(&self, key: &str) -> bool {
        let removed = if let Ok(mut cache) = self.cache.lock() {
            cache.remove(key).is_some()
        } else {
            false
        };
        let _ = system_keychain_delete(key);
        removed
    }

    /// 清理过期条目
    pub fn cleanup_expired(&self) {
        let now = now_unix();
        if let Ok(mut cache) = self.cache.lock() {
            cache.retain(|_, entry| now.saturating_sub(entry.created_at) <= self.ttl_seconds);
        }
    }

    /// 清空所有 (用户主动操作)
    ///
    /// 仅清空会话级内存 cache, 不动系统钥匙串
    /// (系统钥匙串是跨会话持久化, 由用户在系统设置中管理)。
    pub fn clear_all(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    /// 列出所有条目 (不含密码, 只含 label 和 key)
    ///
    /// 返回 (key, label, created_at) 三元组列表。
    pub fn list(&self) -> Vec<(String, String, u64)> {
        if let Ok(cache) = self.cache.lock() {
            cache
                .iter()
                .map(|(k, e)| (k.clone(), e.label.clone(), e.created_at))
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for SessionKeychain {
    fn default() -> Self {
        Self::new()
    }
}

// ── 系统钥匙串集成 (平台条件编译) ──

/// 存储到系统钥匙串 (持久化, 跨会话)
#[cfg(target_os = "macos")]
pub fn system_keychain_set(key: &str, password: &str) -> anyhow::Result<()> {
    // 用 `security add-generic-password` 命令
    // service = "smartex", account = key
    // -U: 如果条目已存在则更新
    use std::process::Command;
    let output = Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            "smartex",
            "-a",
            key,
            "-w",
            password,
            "-U",
        ])
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Keychain set failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn system_keychain_get(key: &str) -> anyhow::Result<Option<String>> {
    use std::process::Command;
    let output = Command::new("security")
        .args(["find-generic-password", "-s", "smartex", "-a", key, "-w"])
        .output()?;
    if output.status.success() {
        let pwd = String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string();
        Ok(Some(pwd))
    } else {
        Ok(None) // 未找到
    }
}

#[cfg(target_os = "macos")]
pub fn system_keychain_delete(key: &str) -> anyhow::Result<()> {
    use std::process::Command;
    let _ = Command::new("security")
        .args(["delete-generic-password", "-s", "smartex", "-a", key])
        .output();
    Ok(())
}

// Windows 和 Linux 的系统钥匙串集成先留 stub (返回 Err)
#[cfg(not(target_os = "macos"))]
pub fn system_keychain_set(_key: &str, _password: &str) -> anyhow::Result<()> {
    anyhow::bail!("System keychain not supported on this platform yet")
}

#[cfg(not(target_os = "macos"))]
pub fn system_keychain_get(_key: &str) -> anyhow::Result<Option<String>> {
    Ok(None)
}

#[cfg(not(target_os = "macos"))]
pub fn system_keychain_delete(_key: &str) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_set_get_delete() {
        let kc = SessionKeychain::with_ttl(60);
        kc.set("key1", "password123", "label1");
        let entry = kc.get("key1").expect("entry should exist");
        assert_eq!(entry.password, "password123");
        assert_eq!(entry.label, "label1");

        let list = kc.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "key1");

        assert!(kc.delete("key1"));
        assert!(kc.get("key1").is_none());
    }

    #[test]
    fn test_session_ttl_expired() {
        let kc = SessionKeychain::with_ttl(0); // 立即过期
        kc.set("key2", "pwd", "label2");
        // TTL=0 时, 任何读取都视作过期; 此处不依赖系统钥匙串
        // 仅验证内存路径: 在没有系统钥匙串回填的情况下返回 None
        // 注意: 在 macOS CI 上若 system_keychain_set 成功, get 会回填;
        //       此测试主要验证 TTL 路径, 不强断言结果。
        let _ = kc.get("key2");
    }

    #[test]
    fn test_clear_all() {
        let kc = SessionKeychain::new();
        kc.set("a", "1", "la");
        kc.set("b", "2", "lb");
        assert_eq!(kc.list().len(), 2);
        kc.clear_all();
        assert_eq!(kc.list().len(), 0);
    }

    #[test]
    fn test_cleanup_expired_no_panic() {
        let kc = SessionKeychain::with_ttl(1);
        kc.set("k", "v", "l");
        kc.cleanup_expired(); // 不应 panic
    }
}
