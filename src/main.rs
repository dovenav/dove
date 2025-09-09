use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use tera::{Context as TContext, Tera};
use include_dir::{include_dir, Dir};
#[cfg(feature = "remote")]
use ureq::Response;
use std::collections::HashSet;
use url::Url;

// 内置示例（用于 init）
const SAMPLE_CONFIG: &str = include_str!("assets/sample.dove.yaml");
static DEFAULT_THEME_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/themes/default");

#[derive(Parser, Debug)]
#[command(name = "dove", about = "静态导航站生成器", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 生成静态站点
    Build {
        /// 配置文件路径，默认：dove.yaml / dove.yml
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// 配置文件 URL，支持 http/https（可用于 Gist raw 链接）
        #[arg(long, value_name = "URL")]
        input_url: Option<String>,
        /// 输出目录，默认：dist
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// 额外静态资源目录（可选），会复制到输出目录中
        #[arg(long, value_name = "DIR")]
        static_dir: Option<PathBuf>,
        /// 指定主题目录，覆盖配置中的 site.theme_dir
        #[arg(long, value_name = "DIR")]
        theme: Option<PathBuf>,
        /// 指定站点根路径，覆盖配置中的 site.base_path
        #[arg(long, value_name = "PATH")]
        base_path: Option<String>,
        /// 仅生成外网页面（不生成 intranet.html）
        #[arg(long)]
        no_intranet: bool,
    },
    /// 初始化示例配置与静态资源
    Init {
        /// 强制覆盖已存在文件
        #[arg(long)]
        force: bool,
        /// 目标目录（默认当前目录）
        #[arg(value_name = "DIR")] 
        dir: Option<PathBuf>,
    },
}

#[derive(Debug, Deserialize)]
struct Config {
    site: Site,
    groups: Vec<Group>,
}

#[derive(Debug, Deserialize)]
struct Site {
    title: String,
    #[serde(default)]
    description: String,
    /// 颜色模式（auto|light|dark），兼容旧字段名 `theme`
    #[serde(default = "default_color_scheme", alias = "theme")]
    color_scheme: ColorScheme,
    /// 主题目录（相对/绝对），例如 `themes/default`
    #[serde(default)]
    theme_dir: Option<String>,
    /// 站点根路径（相对子路径），例如 `secretPath`，将输出到 `dist/secretPath/`
    /// 也支持多级 `a/b/c`。不允许 `.` 或 `..`。
    #[serde(default, alias = "root_path")]
    base_path: Option<String>,
    /// 跳转页设置（延迟倒计时、UTM 参数、默认风险等级）
    #[serde(default)]
    redirect: Option<RedirectSettings>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum ColorScheme {
    Auto,
    Light,
    Dark,
}

fn default_color_scheme() -> ColorScheme { ColorScheme::Auto }

#[derive(Debug, Deserialize)]
struct Group {
    name: String,
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Debug, Deserialize)]
struct Link {
    name: String,
    url: String,
    #[serde(default)]
    desc: String,
    /// 可选：图标 URL（相对/绝对）
    #[serde(default)]
    icon: Option<String>,
    /// 可选：内网地址
    #[serde(default)]
    intranet: Option<String>,
    /// 可选：风险等级（low|medium|high），用于外网跳转页提示。若未配置，回退到 site.redirect.default_risk
    #[serde(default)]
    risk: Option<RiskLevel>,
    /// 可选：UTM 参数（若设置，将覆盖 site.redirect.utm；只对外网跳转页生效）
    #[serde(default)]
    utm: Option<UtmParams>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RiskLevel { Low, Medium, High }

#[derive(Debug, Deserialize, Clone, Default)]
struct RedirectSettings {
    /// 跳转延迟秒数（为 0 或缺省则不自动跳转）
    #[serde(default)]
    delay_seconds: Option<u32>,
    /// 默认风险等级（链接未设置 risk 时使用）
    #[serde(default)]
    default_risk: Option<RiskLevel>,
    /// 站点级 UTM 参数（链接未设置 utm 时使用）
    #[serde(default)]
    utm: Option<UtmParams>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct UtmParams {
    #[serde(default)] source: Option<String>,
    #[serde(default)] medium: Option<String>,
    #[serde(default)] campaign: Option<String>,
    #[serde(default)] term: Option<String>,
    #[serde(default)] content: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { input, input_url, out, static_dir, theme, base_path, no_intranet } => {
            // 环境变量覆盖（若 CLI 未指定）
            let env_input = env_opt_path("DOVE_INPUT");
            let env_input_url = env_opt_string("DOVE_INPUT_URL").or(env_opt_string("DOVE_GIST_URL"));
            let env_gist_id = env_opt_string("DOVE_GIST_ID");
            let env_gist_file = env_opt_string("DOVE_GIST_FILE");
            let env_out = env_opt_path("DOVE_OUT");
            let env_static = env_opt_path("DOVE_STATIC");
            let env_theme = env_opt_path("DOVE_THEME");
            let env_theme_dir = env_opt_path("DOVE_THEME_DIR");
            let env_base_path = env_opt_string("DOVE_BASE_PATH");
            let env_no_intranet = env_bool_truthy("DOVE_NO_INTRANET").unwrap_or(false);
            let env_color_scheme = env_opt_string("DOVE_COLOR_SCHEME").and_then(parse_color_scheme);
            let env_title = env_opt_string("DOVE_TITLE");
            let env_description = env_opt_string("DOVE_DESCRIPTION");
            let env_github_token = env_opt_string("DOVE_GITHUB_TOKEN");
            let env_auth_scheme = env_opt_string("DOVE_AUTH_SCHEME");

            let effective_input = input.or(env_input);
            let effective_input_url = input_url.or(env_input_url);
            let effective_out = out.or(env_out).unwrap_or_else(|| PathBuf::from("dist"));
            let effective_static = static_dir.or(env_static);
            let effective_theme = theme.or(env_theme).or(env_theme_dir);
            let effective_base_path = base_path.or(env_base_path);
            let effective_no_intranet = if no_intranet { true } else { env_no_intranet };
            let effective_color_scheme = env_color_scheme;
            let effective_title = env_title;
            let effective_desc = env_description;

            // 加载配置（本地/URL/Gist）
            let raw_cfg = load_config_text(
                effective_input.as_deref(),
                effective_input_url.as_deref(),
                env_gist_id.as_deref(),
                env_gist_file.as_deref(),
                env_github_token.as_deref(),
                env_auth_scheme.as_deref(),
            )?;
            let config: Config = serde_yaml::from_str(&raw_cfg)
                .with_context(|| "解析 YAML 失败（来自本地/URL/Gist）")?;

            let out_dir = effective_out;
            build(
                config,
                &out_dir,
                effective_static.as_deref(),
                effective_theme.as_deref(),
                effective_base_path,
                effective_no_intranet,
                effective_color_scheme,
                effective_title,
                effective_desc,
            )
        }
        Command::Init { force, dir } => {
            let dir = dir.unwrap_or_else(|| PathBuf::from("."));
            init_scaffold(&dir, force)
        }
    }
}

fn _resolve_local_config_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit { return Some(p.to_path_buf()); }
    for cand in ["dove.yaml", "dove.yml", "config.yaml", "config.yml"] {
        let p = Path::new(cand);
        if p.exists() { return Some(p.to_path_buf()); }
    }
    None
}

#[cfg(feature = "remote")]
fn load_config_text(
    input_path: Option<&Path>,
    input_url: Option<&str>,
    gist_id: Option<&str>,
    gist_file: Option<&str>,
    token: Option<&str>,
    auth_scheme: Option<&str>,
) -> Result<String> {
    // 1) 本地优先
    if let Some(path) = _resolve_local_config_path(input_path) {
        return fs::read_to_string(&path).with_context(|| format!("读取配置失败: {}", path.display()));
    }
    // 2) URL
    if let Some(url) = input_url {
        return http_get_text(url, token, auth_scheme).with_context(|| format!("下载配置失败: {}", url));
    }
    // 3) 本地自动查找
    if let Some(path) = _resolve_local_config_path(None) {
        return fs::read_to_string(&path).with_context(|| format!("读取配置失败: {}", path.display()));
    }
    // 4) Gist by ID
    if let Some(id) = gist_id {
        let (raw_url, chosen) = gist_resolve_raw_url(id, gist_file, token, auth_scheme)?;
        return http_get_text(&raw_url, token, auth_scheme)
            .with_context(|| format!("下载配置失败: Gist {} 文件 {}", id, chosen.unwrap_or("<auto>")));
    }
    bail!("未找到配置：请提供 --input 或 --input-url，或设置 DOVE_INPUT/DOVE_INPUT_URL/DOVE_GIST_ID，或在当前目录放置 dove.yaml");
}

#[cfg(not(feature = "remote"))]
fn load_config_text(
    input_path: Option<&Path>,
    _input_url: Option<&str>,
    _gist_id: Option<&str>,
    _gist_file: Option<&str>,
    _token: Option<&str>,
    _auth_scheme: Option<&str>,
) -> Result<String> {
    if let Some(path) = _resolve_local_config_path(input_path) {
        return fs::read_to_string(&path).with_context(|| format!("读取配置失败: {}", path.display()));
    }
    if let Some(path) = _resolve_local_config_path(None) {
        return fs::read_to_string(&path).with_context(|| format!("读取配置失败: {}", path.display()));
    }
    bail!("未找到本地配置：在禁用 remote 功能时，无法使用 URL/Gist。请启用 feature `remote` 或在当前目录提供 dove.yaml");
}

#[cfg(feature = "remote")]
fn http_get_text(url: &str, token: Option<&str>, auth_scheme: Option<&str>) -> Result<String> {
    let mut req = ureq::get(url).set("User-Agent", "dove/0.1");
    if let Some(t) = token {
        let scheme = auth_scheme.map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("token");
        req = req.set("Authorization", &format!("{} {}", scheme, t));
    }
    let resp = req.call();
    ensure_success(resp, url)?
        .into_string()
        .with_context(|| format!("读取响应文本失败: {}", url))
}

#[cfg(feature = "remote")]
fn ensure_success(resp: Response, url: &str) -> Result<Response> {
    if resp.error() {
        bail!("HTTP 请求失败 {}: {}", url, resp.status());
    }
    Ok(resp)
}

#[cfg(feature = "remote")]
fn gist_resolve_raw_url(id: &str, file_name: Option<&str>, token: Option<&str>, auth_scheme: Option<&str>) -> Result<(String, Option<&str>)> {
    let api = format!("https://api.github.com/gists/{}", id);
    let mut req = ureq::get(&api)
        .set("User-Agent", "dove/0.1")
        .set("Accept", "application/vnd.github+json");
    if let Some(t) = token {
        let scheme = auth_scheme.map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("token");
        req = req.set("Authorization", &format!("{} {}", scheme, t));
    }
    let resp = req.call();
    let resp = ensure_success(resp, &api)?;
    let v: serde_json::Value = resp.into_json().context("解析 Gist API 响应失败")?;
    let files = v.get("files").and_then(|x| x.as_object()).ok_or_else(|| anyhow::anyhow!("Gist 无文件"))?;
    if let Some(target) = file_name {
        if let Some(file_obj) = files.get(target) { 
            if let Some(raw) = file_obj.get("raw_url").and_then(|r| r.as_str()) {
                return Ok((raw.to_string(), Some(target)));
            }
        }
        bail!("Gist {} 中未找到文件: {}", id, target);
    } else {
        // 取第一个文件
        if let Some((_name, file_obj)) = files.iter().next() {
            if let Some(raw) = file_obj.get("raw_url").and_then(|r| r.as_str()) {
                return Ok((raw.to_string(), None));
            }
        }
        bail!("Gist {} 没有可用的 raw_url", id);
    }
}

fn build(
    config: Config,
    out_dir: &Path,
    static_dir: Option<&Path>,
    theme_cli: Option<&Path>,
    base_path_cli: Option<String>,
    no_intranet: bool,
    color_scheme_override: Option<ColorScheme>,
    title_override: Option<String>,
    desc_override: Option<String>,
) -> Result<()> {
    // 准备输出目录
    if !out_dir.exists() { fs::create_dir_all(out_dir).with_context(|| format!("创建输出目录失败: {}", out_dir.display()))?; }
    // 计算站点根目录（支持 base_path 子路径），CLI 覆盖配置
    let base_path_effective = base_path_cli.or_else(|| config.site.base_path.clone());
    let site_dir = match &base_path_effective {
        Some(bp) => match safe_subpath(bp) {
            Some(sub) => out_dir.join(sub),
            None => out_dir.to_path_buf(),
        },
        None => out_dir.to_path_buf(),
    };
    if !site_dir.exists() { fs::create_dir_all(&site_dir).with_context(|| format!("创建站点目录失败: {}", site_dir.display()))?; }

    // 解析主题目录：CLI --theme > 配置 site.theme_dir > 默认 themes/default
    let theme_dir = theme_cli
        .map(|p| p.to_path_buf())
        .or_else(|| config.site.theme_dir.as_ref().map(|s| PathBuf::from(s)))
        .unwrap_or_else(|| PathBuf::from("themes/default"));
    if !theme_dir.exists() {
        bail!("主题目录不存在: {}。可用 --theme 指定或在 dove.yaml 的 site.theme_dir 配置。", theme_dir.display());
    }

    // 拷贝主题 assets -> site_dir/assets
    let theme_assets = theme_dir.join("assets");
    if theme_assets.exists() {
        let dest_assets = site_dir.join("assets");
        if !dest_assets.exists() { fs::create_dir_all(&dest_assets)?; }
        copy_dir_all(&theme_assets, &dest_assets)?;
    }

    // 复制用户静态资源（最后复制以便覆盖主题）
    if let Some(sd) = static_dir {
        if sd.exists() {
            copy_dir_all(sd, &site_dir)?;
        } else {
            eprintln!("警告: 指定的静态目录不存在: {}", sd.display());
        }
    }

    // 渲染 HTML via Tera 到 site_dir
    render_with_theme(
        &config,
        &theme_dir,
        &site_dir,
        !no_intranet,
        color_scheme_override,
        title_override,
        desc_override,
    )?;

    println!("✅ 生成完成 -> {}", site_dir.display());
    Ok(())
}

fn init_scaffold(dir: &Path, force: bool) -> Result<()> {
    if !dir.exists() { fs::create_dir_all(dir)?; }

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
        if theme_root.exists() { fs::remove_dir_all(&theme_root)?; }
        write_default_theme(&theme_root)?;
        println!("写入默认主题: {}", theme_root.display());
    }
    println!("✅ 初始化完成，在根目录运行: cargo run -- build");
    Ok(())
}

fn copy_dir_all(from: &Path, to: &Path) -> Result<()> {
    if !from.is_dir() { bail!("{} 不是目录", from.display()); }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let fpath = entry.path();
        let tpath = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            if !tpath.exists() { fs::create_dir_all(&tpath)?; }
            copy_dir_all(&fpath, &tpath)?;
        } else {
            if let Some(parent) = tpath.parent() { if !parent.exists() { fs::create_dir_all(parent)?; } }
            fs::copy(&fpath, &tpath).with_context(|| format!("复制失败: {} -> {}", fpath.display(), tpath.display()))?;
        }
    }
    Ok(())
}

// 规范化相对子路径，过滤空段/./..，避免越界
fn safe_subpath(s: &str) -> Option<PathBuf> {
    let mut buf = PathBuf::new();
    for seg in s.split('/') {
        let t = seg.trim();
        if t.is_empty() || t == "." || t == ".." { continue; }
        buf.push(t);
    }
    if buf.as_os_str().is_empty() { None } else { Some(buf) }
}

fn env_opt_path(key: &str) -> Option<PathBuf> {
    match env::var(key) {
        Ok(val) => {
            let t = val.trim();
            if t.is_empty() { None } else { Some(PathBuf::from(t)) }
        }
        Err(_) => None,
    }
}

fn env_opt_string(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(val) => {
            let t = val.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        }
        Err(_) => None,
    }
}

fn env_bool_truthy(key: &str) -> Option<bool> {
    match env::var(key) {
        Ok(val) => {
            let v = val.trim().to_ascii_lowercase();
            if ["1","true","yes","y","on"].contains(&v.as_str()) { Some(true) }
            else if ["0","false","no","n","off"].contains(&v.as_str()) { Some(false) }
            else { None }
        }
        Err(_) => None,
    }
}

fn parse_color_scheme(s: String) -> Option<ColorScheme> {
    match s.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(ColorScheme::Auto),
        "light" => Some(ColorScheme::Light),
        "dark" => Some(ColorScheme::Dark),
        _ => None,
    }
}

fn hostname_from_url(u: &str) -> Option<String> {
    match url::Url::parse(u) {
        Ok(parsed) => parsed.host_str().map(|s| s.to_string()),
        Err(_) => None,
    }
}

fn render_with_theme(
    cfg: &Config,
    theme_dir: &Path,
    out_dir: &Path,
    generate_intranet: bool,
    color_scheme_override: Option<ColorScheme>,
    title_override: Option<String>,
    desc_override: Option<String>,
) -> Result<()> {
    // 匹配主题模板目录
    let pattern = theme_dir.join("templates").join("**/*");
    let pattern_str = pattern.to_string_lossy().to_string();
    let tera = Tera::new(&pattern_str)
        .with_context(|| format!("加载模板失败: {}", pattern_str))?;

    // 渲染外网(index.html)，按需渲染内网(intranet.html)
    let title_ref = title_override.as_deref();
    let desc_ref = desc_override.as_deref();
    let externals = render_one(&tera, cfg, out_dir, NetMode::External, generate_intranet, color_scheme_override, title_ref, desc_ref)?;
    if !externals.is_empty() {
        render_link_details(&tera, cfg, out_dir, &externals, color_scheme_override, title_ref, desc_ref)?;
    }
    if generate_intranet {
        let _internals = render_one(&tera, cfg, out_dir, NetMode::Intranet, generate_intranet, color_scheme_override, title_ref, desc_ref)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum NetMode { External, Intranet }

#[derive(Clone)]
struct LinkDetail { slug: String, name: String, desc: String, icon: Option<String>, host: String, final_url: String, risk: Option<RiskLevel>, delay_seconds: u32, utm: Option<UtmParams> }

fn render_one(
    tera: &Tera,
    cfg: &Config,
    out_dir: &Path,
    mode: NetMode,
    has_intranet: bool,
    color_scheme_override: Option<ColorScheme>,
    title_override: Option<&str>,
    desc_override: Option<&str>,
) -> Result<Vec<LinkDetail>> {
    let mut ctx = TContext::new();
    let site_title = title_override.unwrap_or(&cfg.site.title);
    let site_desc = desc_override.unwrap_or(&cfg.site.description);
    ctx.insert("site_title", &site_title);
    ctx.insert("site_desc", &site_desc);
    let scheme = match color_scheme_override.unwrap_or(cfg.site.color_scheme) { ColorScheme::Auto => "auto", ColorScheme::Light => "light", ColorScheme::Dark => "dark" };
    ctx.insert("color_scheme", &scheme);

    // 网络模式与切换按钮
    let (mode_str, other_label, switch_href) = match mode {
        NetMode::External => ("external", "内网", "intranet.html"),
        NetMode::Intranet => ("intranet", "外网", "index.html"),
    };
    ctx.insert("mode", &mode_str);
    ctx.insert("mode_other_label", &other_label);
    ctx.insert("network_switch_href", &switch_href);
    ctx.insert("has_intranet", &has_intranet);

    use serde::Serialize;
    #[derive(Serialize)]
    struct RLink { name: String, href: String, desc: String, icon: Option<String>, host: String }
    #[derive(Serialize)]
    struct RGroup { name: String, links: Vec<RLink> }

    let mut used_slugs: HashSet<String> = HashSet::new();
    let mut details: Vec<LinkDetail> = Vec::new();
    let mut rgroups: Vec<RGroup> = Vec::new();
    for g in &cfg.groups {
        let mut rlinks = Vec::new();
        for l in &g.links {
            match mode {
                NetMode::External => {
                    // 最终跳转目标为外网地址
                    let final_url = l.url.clone();
                    let host = hostname_from_url(&final_url).unwrap_or_default();
                    let base_slug = slugify(&l.name);
                    let slug = unique_slug(&base_slug, &mut used_slugs);
                    let href = format!("go/{}/", slug);
                    rlinks.push(RLink { name: l.name.clone(), href: href.clone(), desc: l.desc.clone(), icon: l.icon.clone(), host: host.clone() });
                    let delay = cfg.site.redirect.as_ref().and_then(|r| r.delay_seconds).unwrap_or(0);
                    let risk = l.risk.or_else(|| cfg.site.redirect.as_ref().and_then(|r| r.default_risk));
                    let utm = l.utm.clone().or_else(|| cfg.site.redirect.as_ref().and_then(|r| r.utm.clone()));
                    details.push(LinkDetail { slug, name: l.name.clone(), desc: l.desc.clone(), icon: l.icon.clone(), host, final_url, risk, delay_seconds: delay, utm });
                }
                NetMode::Intranet => {
                    let href = l.intranet.clone().unwrap_or_else(|| l.url.clone());
                    let host = hostname_from_url(&href).unwrap_or_default();
                    rlinks.push(RLink { name: l.name.clone(), href, desc: l.desc.clone(), icon: l.icon.clone(), host });
                }
            }
        }
        rgroups.push(RGroup { name: g.name.clone(), links: rlinks });
    }
    ctx.insert("groups", &rgroups);

    let html = tera.render("index.html.tera", &ctx)
        .context("渲染模板 index.html.tera 失败")?;
    let filename = match mode { NetMode::External => "index.html", NetMode::Intranet => "intranet.html" };
    fs::write(out_dir.join(filename), html).with_context(|| format!("写入 {} 失败", filename))?;
    Ok(details)
}

fn render_link_details(
    tera: &Tera,
    cfg: &Config,
    out_dir: &Path,
    links: &[LinkDetail],
    color_scheme_override: Option<ColorScheme>,
    title_override: Option<&str>,
    desc_override: Option<&str>,
) -> Result<()> {
    let site_title = title_override.unwrap_or(&cfg.site.title);
    let site_desc = desc_override.unwrap_or(&cfg.site.description);
    let scheme = match color_scheme_override.unwrap_or(cfg.site.color_scheme) { ColorScheme::Auto => "auto", ColorScheme::Light => "light", ColorScheme::Dark => "dark" };

    for d in links {
        let mut ctx = TContext::new();
        ctx.insert("site_title", &site_title);
        ctx.insert("site_desc", &site_desc);
        ctx.insert("color_scheme", &scheme);
        ctx.insert("link_name", &d.name);
        ctx.insert("link_desc", &d.desc);
        ctx.insert("link_icon", &d.icon);
        ctx.insert("link_host", &d.host);
        let final_url = apply_utm(&d.final_url, d.utm.as_ref());
        ctx.insert("link_url", &final_url);
        // 风险等级
        let (risk_class, risk_label) = risk_meta(d.risk);
        ctx.insert("risk_class", &risk_class);
        ctx.insert("risk_label", &risk_label);
        // 延迟
        ctx.insert("delay_seconds", &d.delay_seconds);
        ctx.insert("has_delay", &(d.delay_seconds > 0));
        let html = tera.render("detail.html.tera", &ctx)
            .context("渲染模板 detail.html.tera 失败")?;
        let dir = out_dir.join("go").join(&d.slug);
        if !dir.exists() { fs::create_dir_all(&dir)?; }
        fs::write(dir.join("index.html"), html).with_context(|| format!("写入详情页失败: go/{}/index.html", d.slug))?;
    }
    Ok(())
}

fn slugify(input: &str) -> String {
    let mut s = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars() {
        let c = ch.to_ascii_lowercase();
        if (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') {
            s.push(c);
            prev_dash = false;
        } else {
            if !prev_dash && !s.is_empty() { s.push('-'); prev_dash = true; }
        }
    }
    while s.ends_with('-') { s.pop(); }
    if s.is_empty() { "link".to_string() } else { s }
}

fn unique_slug(base: &str, used: &mut HashSet<String>) -> String {
    let mut slug = base.to_string();
    let mut i = 2;
    while used.contains(&slug) {
        slug = format!("{}-{}", base, i);
        i += 1;
    }
    used.insert(slug.clone());
    slug
}

fn apply_utm(url_str: &str, utm: Option<&UtmParams>) -> String {
    let Some(utm) = utm else { return url_str.to_string() };
    if utm.source.is_none() && utm.medium.is_none() && utm.campaign.is_none() && utm.term.is_none() && utm.content.is_none() { return url_str.to_string(); }
    if let Ok(mut u) = Url::parse(url_str) {
        {
            let mut qp = u.query_pairs_mut();
            if let Some(ref v) = utm.source { qp.append_pair("utm_source", v); }
            if let Some(ref v) = utm.medium { qp.append_pair("utm_medium", v); }
            if let Some(ref v) = utm.campaign { qp.append_pair("utm_campaign", v); }
            if let Some(ref v) = utm.term { qp.append_pair("utm_term", v); }
            if let Some(ref v) = utm.content { qp.append_pair("utm_content", v); }
        }
        u.into_string()
    } else {
        url_str.to_string()
    }
}

fn risk_meta(r: Option<RiskLevel>) -> (String, String) {
    match r.unwrap_or(RiskLevel::Low) {
        RiskLevel::Low => ("low".into(), "低风险".into()),
        RiskLevel::Medium => ("medium".into(), "中风险".into()),
        RiskLevel::High => ("high".into(), "高风险".into()),
    }
}
fn write_default_theme(target_dir: &Path) -> Result<()> {
    for f in DEFAULT_THEME_DIR.files() {
        let rel = f.path();
        let out_path = target_dir.join(rel);
        if let Some(parent) = out_path.parent() { fs::create_dir_all(parent)?; }
        fs::write(&out_path, f.contents())
            .with_context(|| format!("写出默认主题文件失败: {}", out_path.display()))?;
    }
    Ok(())
}
