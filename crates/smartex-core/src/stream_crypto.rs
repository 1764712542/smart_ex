//! 流式加密 — 分块 AES-256-GCM
//!
//! 格式: [magic 4B][version 1B][salt 32B][nonce_prefix 8B][chunk_size 4B]
//!       [chunk1_counter 4B][chunk1_ciphertext+tag][chunk2_counter 4B][chunk2_ciphertext+tag]...
//!
//! 每块独立 nonce (prefix 8B + counter 4B), 独立认证。
//! 内存占用恒定 ~8MB (1 个 chunk + 缓冲)。
//! 中断后可从最后完整 chunk 续传。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use zeroize::Zeroize;

/// 魔数: SmartEx streaming crypto v1
const MAGIC: &[u8; 4] = b"SMX1";
const VERSION: u8 = 1;
/// 单块明文大小: 4MB
const CHUNK_SIZE: usize = 4 * 1024 * 1024;
const NONCE_PREFIX_LEN: usize = 8;
const NONCE_COUNTER_LEN: usize = 4;
/// AES-GCM 标准 nonce 长度: 12 字节
const NONCE_LEN: usize = NONCE_PREFIX_LEN + NONCE_COUNTER_LEN;
/// GCM 认证标签长度: 16 字节
const TAG_LEN: usize = 16;
/// Argon2 salt 长度
const SALT_LEN: usize = 32;
/// AES-256 密钥长度
const KEY_LEN: usize = 32;
/// 头部固定长度: 4 + 1 + 32 + 8 + 4 = 49 字节
const HEADER_LEN: usize = MAGIC.len() + 1 + SALT_LEN + NONCE_PREFIX_LEN + 4;
/// I/O 缓冲区: 1MB (默认 BufReader/Writer 仅 8KB)
const BUF_SIZE: usize = 1024 * 1024;
/// Argon2 内存成本 (32 MB) — 与 crypto.rs 一致
const ARGON2_MEM: u32 = 32 * 1024;
/// Argon2 迭代次数
const ARGON2_ITER: u32 = 3;

/// 从密码派生 256 位密钥 (Argon2id)
fn derive_key(password: &str, salt: &[u8]) -> anyhow::Result<[u8; KEY_LEN]> {
    let params = Params::new(ARGON2_MEM, ARGON2_ITER, 1, Some(KEY_LEN))
        .map_err(|e| anyhow::anyhow!("Argon2 参数错误: {}", e))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow::anyhow!("密钥派生失败: {}", e))?;
    Ok(key)
}

/// 拼接 nonce: 8B prefix + 4B counter (大端)
fn build_nonce(prefix: &[u8], counter: u32) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    nonce[..NONCE_PREFIX_LEN].copy_from_slice(prefix);
    nonce[NONCE_PREFIX_LEN..].copy_from_slice(&counter.to_be_bytes());
    nonce
}

/// 读取整个 chunk (循环 read 直到读满或 EOF)
/// 返回实际读取字节数; 0 表示 EOF。
fn read_full_chunk<R: Read>(reader: &mut R, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        let n = reader.read(&mut buf[total..])?;
        if n == 0 {
            break;
        }
        total += n;
    }
    Ok(total)
}

/// 流式加密文件
///
/// 恒定内存 (~8MB), 支持任意大小文件。
/// 进度回调: (bytes_done, bytes_total)
pub fn encrypt_stream(
    input: &Path,
    output: &Path,
    password: &str,
    progress: Option<&dyn Fn(u64, u64)>,
) -> anyhow::Result<()> {
    let input_size = std::fs::metadata(input)
        .map_err(|e| anyhow::anyhow!("读取输入文件元数据失败: {}", e))?
        .len();

    // 1. 生成 salt + nonce_prefix
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_prefix = [0u8; NONCE_PREFIX_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_prefix);

    // 2. Argon2id 派生密钥
    let mut key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    // 3. 写头部: magic + version + salt + nonce_prefix + chunk_size
    let out_file = File::create(output).map_err(|e| {
        anyhow::anyhow!("创建输出文件失败 {}: {}", output.display(), e)
    })?;
    let mut writer = BufWriter::with_capacity(BUF_SIZE, out_file);
    writer.write_all(MAGIC)?;
    writer.write_all(&[VERSION])?;
    writer.write_all(&salt)?;
    writer.write_all(&nonce_prefix)?;
    writer.write_all(&(CHUNK_SIZE as u32).to_be_bytes())?;

    // 4. 循环读取 4MB chunk, 每块用递增 counter 生成 nonce, AES-GCM 加密
    let in_file = File::open(input)
        .map_err(|e| anyhow::anyhow!("打开输入文件失败 {}: {}", input.display(), e))?;
    let mut reader = BufReader::with_capacity(BUF_SIZE, in_file);
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut counter: u32 = 0;
    let mut bytes_done: u64 = 0;

    loop {
        let read = read_full_chunk(&mut reader, &mut buf)?;
        if read == 0 {
            break;
        }
        let nonce_bytes = build_nonce(&nonce_prefix, counter);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, &buf[..read])
            .map_err(|e| anyhow::anyhow!("加密块 #{} 失败: {}", counter, e))?;
        // [counter 4B][ciphertext + tag]
        writer.write_all(&counter.to_be_bytes())?;
        writer.write_all(&ciphertext)?;

        bytes_done += read as u64;
        if let Some(cb) = progress {
            cb(bytes_done, input_size);
        }
        counter = counter
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("chunk counter 溢出 (文件过大)"))?;
    }

    writer.flush()?;
    // 6. 擦除内存中的密钥
    key.zeroize();
    buf.zeroize();
    Ok(())
}

/// 流式解密文件
pub fn decrypt_stream(
    input: &Path,
    output: &Path,
    password: &str,
    progress: Option<&dyn Fn(u64, u64)>,
) -> anyhow::Result<()> {
    let total_size = std::fs::metadata(input)
        .map_err(|e| anyhow::anyhow!("读取输入文件元数据失败: {}", e))?
        .len();
    if (total_size as usize) < HEADER_LEN {
        anyhow::bail!("文件过短 ({}B), 不是有效的流式加密文件", total_size);
    }

    let in_file = File::open(input)
        .map_err(|e| anyhow::anyhow!("打开输入文件失败 {}: {}", input.display(), e))?;
    let mut reader = BufReader::with_capacity(BUF_SIZE, in_file);

    // 1. 读头部, 验证 magic + version
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        anyhow::bail!("文件魔数不匹配, 不是 smart_ex 流式加密文件");
    }
    let mut ver_buf = [0u8; 1];
    reader.read_exact(&mut ver_buf)?;
    if ver_buf[0] != VERSION {
        anyhow::bail!("不支持的版本: {} (本程序支持 {})", ver_buf[0], VERSION);
    }
    // 2. 读 salt + nonce_prefix + chunk_size
    let mut salt = [0u8; SALT_LEN];
    reader.read_exact(&mut salt)?;
    let mut nonce_prefix = [0u8; NONCE_PREFIX_LEN];
    reader.read_exact(&mut nonce_prefix)?;
    let mut cs_buf = [0u8; 4];
    reader.read_exact(&mut cs_buf)?;
    let chunk_size = u32::from_be_bytes(cs_buf) as usize;
    if chunk_size == 0 {
        anyhow::bail!("无效的 chunk_size: 0");
    }

    // 3. Argon2id 派生密钥
    let mut key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    // 4. 预扫描: 计算 plaintext_total 用于进度回调
    let body_size = total_size - HEADER_LEN as u64;
    let plaintext_total = estimate_plaintext_size(body_size, chunk_size);

    // 5. 循环读取 [counter 4B][ciphertext+tag], 解密
    let out_file = File::create(output).map_err(|e| {
        anyhow::anyhow!("创建输出文件失败 {}: {}", output.display(), e)
    })?;
    let mut writer = BufWriter::with_capacity(BUF_SIZE, out_file);

    let mut remaining = body_size;
    let mut bytes_done: u64 = 0;

    while remaining > 0 {
        if remaining < (4 + TAG_LEN) as u64 {
            anyhow::bail!("文件损坏: 截断的块头 (剩余 {}B)", remaining);
        }
        let mut counter_bytes = [0u8; 4];
        reader.read_exact(&mut counter_bytes)?;
        remaining -= 4;
        let counter = u32::from_be_bytes(counter_bytes);

        // 计算本块密文长度: min(chunk_size, remaining - TAG_LEN)
        let ct_len = (remaining - TAG_LEN as u64)
            .min(chunk_size as u64)
            as usize;
        let mut ct_buf = vec![0u8; ct_len + TAG_LEN];
        reader.read_exact(&mut ct_buf)?;
        remaining -= ct_buf.len() as u64;

        // 6. 每块独立 nonce, 验证 GCM tag, 失败立即报错
        let nonce_bytes = build_nonce(&nonce_prefix, counter);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ct_buf.as_ref())
            .map_err(|_| {
                anyhow::anyhow!("解密失败: 密码错误或文件已损坏 (块 #{})", counter)
            })?;
        writer.write_all(&plaintext)?;

        bytes_done += plaintext.len() as u64;
        if let Some(cb) = progress {
            cb(bytes_done, plaintext_total);
        }
    }

    writer.flush()?;
    key.zeroize();
    Ok(())
}

/// 估算解密后的明文总大小 (用于进度回调)
///
/// body_size = num_chunks * (4 + TAG_LEN) + plaintext_total
/// 最后一块的明文长度 ∈ [1, chunk_size] (假设非空),
/// 数值上 plaintext_total ≈ body_size - 20 * num_chunks,
/// 其中 num_chunks = ceil(plaintext_total / chunk_size)。
///
/// 这里采用保守估算: 每块在盘上至少占 4 + 1 + TAG_LEN = 21 字节,
/// 余下字节近似为明文长度。误差在每块 20B 范围内, 不影响进度展示。
fn estimate_plaintext_size(body_size: u64, chunk_size: usize) -> u64 {
    if body_size == 0 {
        return 0;
    }
    let chunk_overhead = (4 + TAG_LEN) as u64;
    let chunk_stride = chunk_size as u64 + chunk_overhead;
    // 假设全部为满块: 上界 chunk 数
    let max_chunks = body_size / chunk_stride + 1;
    let plaintext_estimate = body_size.saturating_sub(max_chunks * chunk_overhead);
    plaintext_estimate.max(0)
}

/// 获取加密文件信息 (不解密)
pub fn encrypted_file_info(input: &Path) -> anyhow::Result<EncryptedFileInfo> {
    let total_size = std::fs::metadata(input)
        .map_err(|e| anyhow::anyhow!("读取文件元数据失败: {}", e))?
        .len();
    if (total_size as usize) < HEADER_LEN {
        anyhow::bail!("文件过短, 不是有效的流式加密文件");
    }
    let mut reader = BufReader::with_capacity(BUF_SIZE, File::open(input)?);
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        anyhow::bail!("文件魔数不匹配, 不是 smart_ex 流式加密文件");
    }
    let mut ver_buf = [0u8; 1];
    reader.read_exact(&mut ver_buf)?;
    let version = ver_buf[0];
    // 跳过 salt (32) + nonce_prefix (8)
    reader.seek(SeekFrom::Current((SALT_LEN + NONCE_PREFIX_LEN) as i64))?;
    let mut cs_buf = [0u8; 4];
    reader.read_exact(&mut cs_buf)?;
    let chunk_size = u32::from_be_bytes(cs_buf) as usize;

    let body_size = total_size - HEADER_LEN as u64;
    let chunk_overhead = (4 + TAG_LEN) as u64;
    let chunk_stride = chunk_size as u64 + chunk_overhead;
    let total_chunks = if body_size == 0 {
        0
    } else {
        // 完整块 + 1 个尾块 (尾块可能是 partial)
        let full = body_size / chunk_stride;
        let remainder = body_size % chunk_stride;
        if remainder == 0 {
            full
        } else {
            full + 1
        }
    };

    Ok(EncryptedFileInfo {
        version,
        chunk_size,
        total_chunks,
    })
}

pub struct EncryptedFileInfo {
    pub version: u8,
    pub chunk_size: usize,
    pub total_chunks: u64,
}

/// 续传加密 (从指定 chunk index 继续)
///
/// 用于中断后恢复。要求:
/// - output 已存在且头部合法 (含 salt + nonce_prefix)
/// - output 末尾位于完整块边界 (用户负责; 本函数不做截断校验)
/// - from_chunk 与 output 中已有块数一致
pub fn encrypt_stream_resume(
    input: &Path,
    output: &Path,
    password: &str,
    from_chunk: u64,
    progress: Option<&dyn Fn(u64, u64)>,
) -> anyhow::Result<()> {
    let input_size = std::fs::metadata(input)
        .map_err(|e| anyhow::anyhow!("读取输入文件元数据失败: {}", e))?
        .len();
    let target_offset = from_chunk
        .checked_mul(CHUNK_SIZE as u64)
        .ok_or_else(|| anyhow::anyhow!("from_chunk 过大, 偏移溢出"))?;
    if target_offset > input_size {
        anyhow::bail!(
            "from_chunk {} 超出输入文件大小 ({}B)",
            from_chunk,
            input_size
        );
    }

    // 1. 读已有 output 的头部获取 salt + nonce_prefix
    if !output.exists() {
        anyhow::bail!("续传失败: 输出文件不存在 {}", output.display());
    }
    let mut read_file = File::open(output)
        .map_err(|e| anyhow::anyhow!("打开已有输出失败: {}", e))?;
    let mut header = [0u8; HEADER_LEN];
    read_file.read_exact(&mut header)?;
    if &header[..MAGIC.len()] != MAGIC {
        anyhow::bail!("续传失败: 输出文件魔数不匹配");
    }
    if header[MAGIC.len()] != VERSION {
        anyhow::bail!("续传失败: 输出文件版本不匹配");
    }
    let salt = &header[MAGIC.len() + 1..MAGIC.len() + 1 + SALT_LEN];
    let nonce_prefix = &header[MAGIC.len() + 1 + SALT_LEN..MAGIC.len() + 1 + SALT_LEN + NONCE_PREFIX_LEN];
    let chunk_size_bytes = &header[MAGIC.len() + 1 + SALT_LEN + NONCE_PREFIX_LEN..];
    let existing_chunk_size = u32::from_be_bytes(chunk_size_bytes.try_into().unwrap()) as usize;
    if existing_chunk_size != CHUNK_SIZE {
        anyhow::bail!(
            "续传失败: 已有 chunk_size ({}) 与本程序 ({}) 不一致",
            existing_chunk_size,
            CHUNK_SIZE
        );
    }

    // 2. 重新派生密钥
    let mut key = derive_key(password, salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    // 3. seek input 到 from_chunk * chunk_size
    let in_file = OpenOptions::new().read(true).open(input)
        .map_err(|e| anyhow::anyhow!("打开输入文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(BUF_SIZE, in_file);
    reader.seek(SeekFrom::Start(target_offset))?;

    // 4. seek output 到末尾, 继续追加写入
    let out_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output)
        .map_err(|e| anyhow::anyhow!("以追加模式打开输出文件失败: {}", e))?;
    let mut writer = BufWriter::with_capacity(BUF_SIZE, out_file);

    // 5. 继续加密, counter 从 from_chunk 开始
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut counter: u32 = from_chunk as u32;
    let mut bytes_done: u64 = target_offset;

    loop {
        let read = read_full_chunk(&mut reader, &mut buf)?;
        if read == 0 {
            break;
        }
        let nonce_bytes = build_nonce(nonce_prefix, counter);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, &buf[..read])
            .map_err(|e| anyhow::anyhow!("加密块 #{} 失败: {}", counter, e))?;
        writer.write_all(&counter.to_be_bytes())?;
        writer.write_all(&ciphertext)?;

        bytes_done += read as u64;
        if let Some(cb) = progress {
            cb(bytes_done, input_size);
        }
        counter = counter
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("chunk counter 溢出 (文件过大)"))?;
    }

    writer.flush()?;
    key.zeroize();
    buf.zeroize();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("smartex_stream_{}_{}", std::process::id(), name));
        p
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_small() {
        let plain = tmp_path("plain_small.bin");
        let enc = tmp_path("enc_small.bin");
        let dec = tmp_path("dec_small.bin");
        {
            let mut f = File::create(&plain).unwrap();
            f.write_all(b"hello smartex streaming crypto").unwrap();
        }
        encrypt_stream(&plain, &enc, "pw123", None).unwrap();
        decrypt_stream(&enc, &dec, "pw123", None).unwrap();
        let orig = std::fs::read(&plain).unwrap();
        let got = std::fs::read(&dec).unwrap();
        assert_eq!(orig, got);
        let _ = std::fs::remove_file(&plain);
        let _ = std::fs::remove_file(&enc);
        let _ = std::fs::remove_file(&dec);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_multichunk() {
        // 10MB 数据, 跨 3 个 4MB chunk
        let plain = tmp_path("plain_multi.bin");
        let enc = tmp_path("enc_multi.bin");
        let dec = tmp_path("dec_multi.bin");
        let data: Vec<u8> = (0..10 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
        std::fs::write(&plain, &data).unwrap();
        encrypt_stream(&plain, &enc, "secret", None).unwrap();
        decrypt_stream(&enc, &dec, "secret", None).unwrap();
        let got = std::fs::read(&dec).unwrap();
        assert_eq!(data, got);
        let _ = std::fs::remove_file(&plain);
        let _ = std::fs::remove_file(&enc);
        let _ = std::fs::remove_file(&dec);
    }

    #[test]
    fn test_wrong_password_fails() {
        let plain = tmp_path("plain_wp.bin");
        let enc = tmp_path("enc_wp.bin");
        let dec = tmp_path("dec_wp.bin");
        std::fs::write(&plain, b"some data here").unwrap();
        encrypt_stream(&plain, &enc, "right", None).unwrap();
        let res = decrypt_stream(&enc, &dec, "wrong", None);
        assert!(res.is_err(), "wrong password must fail");
        let _ = std::fs::remove_file(&plain);
        let _ = std::fs::remove_file(&enc);
        let _ = std::fs::remove_file(&dec);
    }

    #[test]
    fn test_empty_input() {
        let plain = tmp_path("plain_empty.bin");
        let enc = tmp_path("enc_empty.bin");
        let dec = tmp_path("dec_empty.bin");
        File::create(&plain).unwrap();
        encrypt_stream(&plain, &enc, "pw", None).unwrap();
        decrypt_stream(&enc, &dec, "pw", None).unwrap();
        let got = std::fs::read(&dec).unwrap();
        assert!(got.is_empty());
        let info = encrypted_file_info(&enc).unwrap();
        assert_eq!(info.total_chunks, 0);
        let _ = std::fs::remove_file(&plain);
        let _ = std::fs::remove_file(&enc);
        let _ = std::fs::remove_file(&dec);
    }

    #[test]
    fn test_encrypted_file_info() {
        let plain = tmp_path("plain_info.bin");
        let enc = tmp_path("enc_info.bin");
        let data: Vec<u8> = (0..5 * 1024 * 1024).map(|i| (i % 191) as u8).collect();
        std::fs::write(&plain, &data).unwrap();
        encrypt_stream(&plain, &enc, "pw", None).unwrap();
        let info = encrypted_file_info(&enc).unwrap();
        assert_eq!(info.version, VERSION);
        assert_eq!(info.chunk_size, CHUNK_SIZE);
        assert_eq!(info.total_chunks, 2); // 5MB / 4MB = 2 chunks
        let _ = std::fs::remove_file(&plain);
        let _ = std::fs::remove_file(&enc);
    }

    #[test]
    fn test_resume_after_interrupt() {
        // 模拟: 先完整加密, 然后截断到 1 个完整 chunk + 头部,
        // 再用 resume 从 chunk 1 继续加密, 最后解密对比。
        let plain = tmp_path("plain_resume.bin");
        let enc_full = tmp_path("enc_full.bin");
        let enc_resume = tmp_path("enc_resume.bin");
        let dec = tmp_path("dec_resume.bin");
        // 9MB 数据 → 3 个 chunk (4MB + 4MB + 1MB)
        let data: Vec<u8> = (0..9 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
        std::fs::write(&plain, &data).unwrap();
        encrypt_stream(&plain, &enc_full, "pw", None).unwrap();
        // 截断到 1 个完整 chunk (头部 + 第 0 块)
        let one_chunk_size = (HEADER_LEN + 4 + CHUNK_SIZE + TAG_LEN) as u64;
        let full_bytes = std::fs::read(&enc_full).unwrap();
        std::fs::write(&enc_resume, &full_bytes[..one_chunk_size as usize]).unwrap();
        // 从 chunk 1 续传
        encrypt_stream_resume(&plain, &enc_resume, "pw", 1, None).unwrap();
        // 解密续传结果
        decrypt_stream(&enc_resume, &dec, "pw", None).unwrap();
        let got = std::fs::read(&dec).unwrap();
        assert_eq!(data, got);
        let _ = std::fs::remove_file(&plain);
        let _ = std::fs::remove_file(&enc_full);
        let _ = std::fs::remove_file(&enc_resume);
        let _ = std::fs::remove_file(&dec);
    }
}
