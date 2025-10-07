//! 图标下载与规范化模块：
//! - 识别远程 icon 链接
//!   -（启用 remote 特性时）并发下载与本地缓存
//! - 根据 Content-Type 或 URL 推断扩展名

use std::{collections::HashMap, path::Path};

#[cfg(feature = "remote")]
use std::{fs, io::Read};

#[cfg(feature = "remote")]
use crate::config::load_config; // not used directly; keep feature parity

#[cfg(feature = "remote")]
use anyhow::Result;

/// 将可能的远程 icon 文本标准化为 (原始值, 可下载 URL)
pub(crate) fn normalize_remote_icon(s: &str) -> Option<(String, String)> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Some((t.to_string(), t.to_string()))
    } else if lower.starts_with("//") {
        Some((t.to_string(), format!("https:{}", t)))
    } else {
        None
    }
}

#[cfg(feature = "remote")]
use std::sync::mpsc;

#[cfg(feature = "remote")]
pub(crate) fn download_icons_concurrent(
    targets: &[(String, String)],
    dest_dir: &Path,
    rel_dir: &str,
    threads: usize,
) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    if targets.is_empty() {
        return map;
    }

    // 结果通道
    let (txr, rxr) = mpsc::channel::<(String, Option<String>)>();
    let total = targets.len();
    let workers = threads.min(total.max(1));
    let chunk_size = (total + workers - 1) / workers; // 向上取整
    for chunk_idx in 0..workers {
        let start = chunk_idx * chunk_size;
        let end = (start + chunk_size).min(total);
        if start >= end {
            break;
        }
        let slice: Vec<(String, String)> = targets[start..end].to_vec();
        let txr = txr.clone();
        let dest = dest_dir.to_path_buf();
        let rel = rel_dir.trim_matches('/').to_string();
        std::thread::spawn(move || {
            for (orig, fetch) in slice {
                let res = download_one_icon(&fetch, &dest).map(|fname| {
                    if rel.is_empty() {
                        fname
                    } else {
                        format!("{}/{}", rel, fname)
                    }
                });
                let _ = txr.send((orig, res));
            }
        });
    }
    drop(txr);

    // 收集结果并输出日志
    for _ in 0..total {
        if let Ok((orig, res)) = rxr.recv() {
            match res {
                Some(path_rel) => {
                    println!("✅ 图标已缓存: {} -> {}", orig, path_rel);
                    map.insert(orig, path_rel);
                }
                None => {
                    println!("⚠️ 图标下载失败: {}", orig);
                }
            }
        }
    }
    map
}

#[cfg(not(feature = "remote"))]
pub(crate) fn download_icons_concurrent(
    _targets: &[(String, String)],
    _dest_dir: &Path,
    _rel_dir: &str,
    _threads: usize,
) -> HashMap<String, String> {
    HashMap::new()
}

#[cfg(feature = "remote")]
fn download_one_icon(url: &str, dest_dir: &Path) -> Option<String> {
    // 发送请求
    let call = ureq::get(url).set("User-Agent", "dove/0.1").call();
    let resp = match ensure_success(call, url) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("⚠️ 请求失败: {} -> {}", url, e);
            return None;
        }
    };
    // 内容类型 -> 扩展名
    let ct = resp.header("Content-Type").unwrap_or("");
    let ext = ext_from_headers_or_url(ct, url);
    // 读入字节
    let mut reader = resp.into_reader();
    let mut buf: Vec<u8> = Vec::new();
    if let Err(e) = reader.read_to_end(&mut buf) {
        eprintln!("⚠️ 读取响应失败: {} -> {}", url, e);
        return None;
    }
    // 文件名：对 URL 做 FNV-1a 64 哈希
    let hash = fnv1a64(url.as_bytes());
    let fname = format!("i_{:016x}.{}", hash, ext);
    let fpath = dest_dir.join(&fname);
    if !fpath.exists() {
        if let Some(parent) = fpath.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&fpath, &buf) {
            eprintln!("⚠️ 写入失败: {} -> {}", fpath.display(), e);
            return None;
        }
    }
    Some(fname)
}

#[cfg(feature = "remote")]
fn ensure_success(resp: Result<ureq::Response, ureq::Error>, url: &str) -> Result<ureq::Response> {
    match resp {
        Ok(r) => Ok(r),
        Err(e) => anyhow::bail!("HTTP 请求失败 {}: {}", url, e),
    }
}

#[cfg(feature = "remote")]
fn ext_from_headers_or_url(content_type: &str, url: &str) -> &'static str {
    let ct = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match ct.as_str() {
        "image/svg+xml" => "svg",
        "image/png" => "png",
        "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        _ => {
            // 尝试从 URL path 提取
            if let Ok(u) = url::Url::parse(url) {
                if let Some(seg) = u.path_segments().and_then(|it| it.last()) {
                    if let Some(idx) = seg.rfind('.') {
                        return match &seg[idx + 1..].to_ascii_lowercase()[..] {
                            "svg" => "svg",
                            "png" => "png",
                            "ico" => "ico",
                            "jpg" | "jpeg" => "jpg",
                            "gif" => "gif",
                            "webp" => "webp",
                            "avif" => "avif",
                            _ => "bin",
                        };
                    }
                }
            }
            "bin"
        }
    }
}

#[cfg(feature = "remote")]
fn fnv1a64(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    for b in data {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
