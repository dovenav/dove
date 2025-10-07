//! 初始化脚手架与默认主题写出模块
//! - `dove init` 写出示例配置与内置默认主题

use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};
use std::{fs, path::Path};

// 内置示例（用于 init）
const SAMPLE_CONFIG: &str = include_str!("assets/sample.dove.yaml");
static DEFAULT_THEME_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/themes/default");

/// 初始化示例配置与默认主题目录
pub(crate) fn init_scaffold(dir: &Path, force: bool) -> Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }

    // 写入示例配置
    let cfg_path = dir.join("dove.yaml");
    if cfg_path.exists() && !force {
        eprintln!("跳过: {} 已存在，使用 --force 可覆盖", cfg_path.display());
    } else {
        fs::write(&cfg_path, SAMPLE_CONFIG.as_bytes())
            .with_context(|| format!("写入示例配置失败: {}", cfg_path.display()))?;
        println!("写入: {}", cfg_path.display());
    }

    // 写入默认主题目录
    let theme_root = dir.join("themes").join("default");
    if theme_root.exists() && !force {
        println!("跳过: {} 已存在，使用 --force 可覆盖", theme_root.display());
    } else {
        write_default_theme(&theme_root)?;
        println!("写入: {}", theme_root.display());
    }

    // 提示完成
    println!("✅ 初始化完成，在根目录运行: cargo run -- build");
    Ok(())
}

/// 递归复制目录
pub(crate) fn copy_dir_all(from: &Path, to: &Path) -> Result<()> {
    if !from.is_dir() {
        anyhow::bail!("{} 不是目录", from.display());
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let fpath = entry.path();
        let tpath = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            if !tpath.exists() {
                fs::create_dir_all(&tpath)?;
            }
            copy_dir_all(&fpath, &tpath)?;
        } else {
            if let Some(parent) = tpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::copy(&fpath, &tpath)
                .with_context(|| format!("复制失败: {} -> {}", fpath.display(), tpath.display()))?;
        }
    }
    Ok(())
}

/// 将内置默认主题写出到指定目录
pub(crate) fn write_default_theme(target_dir: &Path) -> Result<()> {
    for f in DEFAULT_THEME_DIR.files() {
        let rel = f.path();
        let out_path = target_dir.join(rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out_path, f.contents())
            .with_context(|| format!("写出默认主题文件失败: {}", out_path.display()))?;
    }
    Ok(())
}
