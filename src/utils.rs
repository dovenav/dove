//! 通用辅助函数：
//! - 环境变量读取与解析
//! - 安全的子路径处理、URL 主机名提取
//! - 文本到枚举的解析工具

use std::{env, path::PathBuf};
use crate::config::ColorScheme;

/// 将字符串转为安全子路径（过滤 `.` / `..` 等危险片段）。
pub(crate) fn safe_subpath(s: &str) -> Option<PathBuf> {
    let mut p = PathBuf::new();
    for seg in s.split('/') {
        let t = seg.trim();
        if t.is_empty() || t == "." || t == ".." { continue; }
        p.push(t);
    }
    if p.components().next().is_none() { None } else { Some(p) }
}

/// 可选读取 PATH 环境变量为 PathBuf。
pub(crate) fn env_opt_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).map(PathBuf::from)
}

/// 可选读取 String 环境变量。
pub(crate) fn env_opt_string(key: &str) -> Option<String> {
    env::var(key).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// 可选读取 usize 环境变量。
pub(crate) fn env_opt_usize(key: &str) -> Option<usize> {
    env::var(key).ok().and_then(|s| s.parse::<usize>().ok())
}

/// 读取布尔环境变量的真值（1/true/on/yes/y）。
pub(crate) fn env_bool_truthy(key: &str) -> Option<bool> {
    env::var(key).ok().map(|v| {
        match v.to_ascii_lowercase().as_str() {
            "1" | "true" | "on" | "yes" | "y" => true,
            "0" | "false" | "off" | "no" | "n" => false,
            _ => false,
        }
    })
}

/// 将字符串解析为 ColorScheme。
pub(crate) fn parse_color_scheme(s: String) -> Option<ColorScheme> {
    match s.to_ascii_lowercase().as_str() {
        "auto" => Some(ColorScheme::Auto),
        "light" => Some(ColorScheme::Light),
        "dark" => Some(ColorScheme::Dark),
        _ => None,
    }
}

/// 从 URL 字符串提取主机名（失败返回 None）。
pub(crate) fn hostname_from_url(u: &str) -> Option<String> {
    match url::Url::parse(u) {
        Ok(p) => p.host_str().map(|s| s.to_string()),
        Err(_) => None,
    }
}
