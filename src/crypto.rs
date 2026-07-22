use anyhow::{Context, Result};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

/// 加密文件魔数: b"SMEX1"
const MAGIC: &[u8; 5] = b"SMEX1";
/// Argon2 salt 长度
const SALT_LEN: usize = 16;
/// AES-GCM Nonce 长度
const NONCE_LEN: usize = 12;
/// 密钥长度 (AES-256)
const KEY_LEN: usize = 32;
/// 内存成本 (32 MB)
const ARGON2_MEM: u32 = 32 * 1024;
/// 迭代次数
const ARGON2_ITER: u32 = 3;

/// 从密码派生 256 位密钥 (Argon2id)
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let params = Params::new(ARGON2_MEM, ARGON2_ITER, 1, Some(KEY_LEN))
        .map_err(|e| anyhow::anyhow!("Argon2 参数错误: {}", e))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow::anyhow!("密钥派生失败: {}", e))?;
    Ok(key)
}

/// 加密文件
///
/// 文件格式:
/// [MAGIC 5B][salt 16B][nonce 12B][ciphertext + GCM tag]
pub fn encrypt_file(input: &Path, output: &Path, password: &str) -> Result<()> {
    let plaintext = read_file_bytes(input)?;

    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| anyhow::anyhow!("加密失败: {}", e))?;

    let mut out = File::create(output)
        .with_context(|| format!("创建输出文件失败: {}", output.display()))?;
    out.write_all(MAGIC)?;
    out.write_all(&salt)?;
    out.write_all(&nonce_bytes)?;
    out.write_all(&ciphertext)?;
    out.flush()?;

    // 安全清零
    let mut key_zero = key;
    key_zero.zeroize();
    Ok(())
}

/// 解密文件
pub fn decrypt_file(input: &Path, output: &Path, password: &str) -> Result<()> {
    let data = read_file_bytes(input)?;

    if data.len() < MAGIC.len() + SALT_LEN + NONCE_LEN {
        return Err(anyhow::anyhow!("文件过短或格式不正确"));
    }
    if &data[..MAGIC.len()] != MAGIC {
        return Err(anyhow::anyhow!("文件魔数不匹配, 不是 smart_ex 加密文件"));
    }

    let salt = &data[MAGIC.len()..MAGIC.len() + SALT_LEN];
    let nonce_bytes = &data[MAGIC.len() + SALT_LEN..MAGIC.len() + SALT_LEN + NONCE_LEN];
    let ciphertext = &data[MAGIC.len() + SALT_LEN + NONCE_LEN..];

    let mut key = derive_key(password, salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("解密失败: 密码错误或文件已损坏"))?;

    let mut out = File::create(output)
        .with_context(|| format!("创建输出文件失败: {}", output.display()))?;
    out.write_all(&plaintext)?;
    out.flush()?;

    key.zeroize();
    Ok(())
}

/// 默认加密输出路径: input + .enc
pub fn default_encrypt_output(input: &Path) -> PathBuf {
    let mut s = input.to_string_lossy().to_string();
    s.push_str(".enc");
    PathBuf::from(s)
}

/// 默认解密输出路径: 移除 .enc 后缀, 否则加 .dec
pub fn default_decrypt_output(input: &Path) -> PathBuf {
    let name = input.to_string_lossy().to_string();
    if let Some(stripped) = name.strip_suffix(".enc") {
        PathBuf::from(stripped)
    } else {
        PathBuf::from(format!("{}.dec", name))
    }
}

fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut f = File::open(path).with_context(|| format!("打开文件失败: {}", path.display()))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}
