//! 预览与热重载静态文件服务模块
//! - 监视主题/静态/本地配置变更并增量重建
//! - 内置极简 HTTP 静态文件服务器，支持热刷新

use std::{fs, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}}, thread, time::Duration};
use anyhow::Result;
use notify::{RecommendedWatcher, Watcher, RecursiveMode};

use crate::{build::build, config::{Config, load_config, describe_source}, config::ColorScheme};

/// 监视并服务指定目录，按需重建与热刷新
pub(crate) fn preview_watch_and_serve(
    root: PathBuf,
    addr: String,
    input: Option<PathBuf>,
    input_url: Option<String>,
    gist_id: Option<String>,
    gist_file: Option<String>,
    token: Option<String>,
    auth_scheme: Option<String>,
    out: PathBuf,
    static_dir: Option<PathBuf>,
    theme_dir: Option<PathBuf>,
    base_path: Option<String>,
    no_intranet: bool,
    generate_intermediate_page: bool,
    color_scheme: Option<ColorScheme>,
    title: Option<String>,
    desc: Option<String>,
    open: bool,
    build_version: Option<String>,
    icon_dir: Option<String>,
    icon_threads: Option<usize>,
) -> Result<()> {
    if !root.exists() { anyhow::bail!("预览目录不存在: {}", root.display()); }
    println!("🔎 预览目录: {}", root.display());
    println!("🚀 访问: http://{}", addr);
    if open { let _ = webbrowser::open(&format!("http://{}", addr)); }

    // 版本号与变更标记
    let version = Arc::new(AtomicU64::new(0));
    let dirty = Arc::new(AtomicBool::new(false));

    // 监视（主题目录、静态目录、本地配置文件）
    {
        let dirty = dirty.clone();
        let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if res.is_ok() { dirty.store(true, Ordering::SeqCst); }
        })?;
        if let Some(td) = theme_dir.as_ref() { if td.exists() { watcher.watch(td, RecursiveMode::Recursive)?; } }
        if let Some(sd) = static_dir.as_ref() { if sd.exists() { watcher.watch(sd, RecursiveMode::Recursive)?; } }
        if let Some(ip) = input.as_ref() {
            if ip.exists() {
                let watch_target = if ip.is_dir() {
                    ip.clone()
                } else {
                    ip.parent().unwrap_or(Path::new(".")).to_path_buf()
                };
                if watch_target.exists() { watcher.watch(&watch_target, RecursiveMode::Recursive)?; }
            }
        }
        // 保持 watcher 活到生命周期末尾
        std::mem::forget(watcher);
    }

    // 后台重建线程
    {
        let version = version.clone();
        let dirty = dirty.clone();
        let build_version = build_version.clone();
        let icon_dir = icon_dir.clone();
        let icon_threads = icon_threads.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(400));
                if dirty.swap(false, Ordering::SeqCst) {
                    // 重新加载配置并构建
                    if let Ok(loaded) = load_config(
                        input.as_deref(), input_url.as_deref(), gist_id.as_deref(), gist_file.as_deref(), token.as_deref(), auth_scheme.as_deref(),
                    ) {
                        if let Ok(cfg) = serde_yaml::from_str::<Config>(&loaded.text) {
                            let _ = build(cfg, &out, static_dir.as_deref(), theme_dir.as_deref(), base_path.clone(), no_intranet, generate_intermediate_page, color_scheme, title.clone(), desc.clone(), build_version.clone(), icon_dir.clone(), icon_threads);
                            version.fetch_add(1, Ordering::SeqCst);
                            println!("🔁 已重建，version = {} · 配置来源: {}", version.load(Ordering::SeqCst), describe_source(&loaded.source));
                        }
                    }
                }
            }
        });
    }

    // 启动服务
    serve_with_reload(&root, &addr, version)
}

fn serve_with_reload(root: &Path, addr: &str, version: Arc<AtomicU64>) -> Result<()> {
    let server = tiny_http::Server::http(addr).map_err(|e| anyhow::anyhow!("绑定地址失败: {}: {}", addr, e))?;
    for rq in server.incoming_requests() {
        let url = rq.url();
        if url == "/__dove__/version" {
            let body = version.load(Ordering::SeqCst).to_string();
            let _ = rq.respond(tiny_http::Response::from_string(body).with_status_code(200));
            continue;
        }
        let path_only = url.split('?').next().unwrap_or("/");
        let mut segs = Vec::new();
        for s in path_only.split('/') { let t = s.trim(); if t.is_empty() || t=="." || t==".." { continue; } segs.push(t); }
        let mut fpath = root.to_path_buf();
        for s in &segs { fpath.push(s); }
        let is_dir_req = path_only.ends_with('/') || segs.is_empty();
        if is_dir_req { fpath.push("index.html"); }
        let mut status = 200;
        if !fpath.exists() || fpath.is_dir() { status = 404; }
        let content_type = content_type_for_path(&fpath);
        let resp = if status == 200 {
            if content_type.starts_with("text/html") {
                match fs::read_to_string(&fpath) {
                    Ok(mut s) => {
                        s.push_str("\n<script>(function(){var c=null;async function t(){try{var r=await fetch('/__dove__/version',{cache:'no-store'});var v=await r.text();if(c===null)c=v;else if(v!==c) location.reload();}catch(e){} setTimeout(t,1000);} t();})();</script>\n");
                        tiny_http::Response::from_string(s).with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap())
                    }
                    Err(_) => tiny_http::Response::from_string("Not Found").with_status_code(404)
                }
            } else {
                match fs::read(&fpath) {
                    Ok(bytes) => tiny_http::Response::from_data(bytes).with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap()),
                    Err(_) => tiny_http::Response::from_string("Not Found").with_status_code(404),
                }
            }
        } else {
            tiny_http::Response::from_string("Not Found")
        };
        let _ = rq.respond(resp.with_status_code(status));
    }
    Ok(())
}

fn content_type_for_path(p: &Path) -> String {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8".into(),
        "css" => "text/css; charset=utf-8".into(),
        "js" => "application/javascript; charset=utf-8".into(),
        "mjs" => "application/javascript; charset=utf-8".into(),
        "map" => "application/json; charset=utf-8".into(),
        "json" => "application/json; charset=utf-8".into(),
        "txt" => "text/plain; charset=utf-8".into(),
        "svg" => "image/svg+xml".into(),
        "png" => "image/png".into(),
        "jpg" | "jpeg" => "image/jpeg".into(),
        "gif" => "image/gif".into(),
        "webp" => "image/webp".into(),
        "avif" => "image/avif".into(),
        "ico" => "image/x-icon".into(),
        "woff" => "font/woff".into(),
        "woff2" => "font/woff2".into(),
        "ttf" => "font/ttf".into(),
        "otf" => "font/otf".into(),
        "eot" => "application/vnd.ms-fontobject".into(),
        "wasm" => "application/wasm".into(),
        _ => "application/octet-stream".into(),
    }
}
