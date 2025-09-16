//! 配置与加载模块：
//! - 定义 `Config`/`Site`/`Group`/`Link` 等数据结构
//! - 提供 `load_config` 支持本地文件/URL/Gist（三者按优先级）
//! - 暴露配置来源信息，便于日志打印

use std::{fs, path::{Path, PathBuf}};
use std::collections::{BTreeSet, HashSet};
use anyhow::{Result, Context, bail};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

#[cfg(feature = "remote")]
use ureq::Response;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) site: Site,
    pub(crate) groups: Vec<Group>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Site {
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) description: String,
    /// 颜色模式（auto|light|dark），兼容旧字段名 `theme`
    #[serde(default = "default_color_scheme", alias = "theme")]
    pub(crate) color_scheme: ColorScheme,
    /// 主题目录（相对/绝对），例如 `themes/default`
    #[serde(default)]
    pub(crate) theme_dir: Option<String>,
    /// 站点根路径（相对子路径），例如 `secretPath`，将输出到 `dist/secretPath/`
    /// 也支持多级 `a/b/c`。不允许 `.` 或 `..`。
    #[serde(default, alias = "root_path")]
    pub(crate) base_path: Option<String>,
    /// 跳转页设置（延迟倒计时、UTM 参数、默认风险等级）
    #[serde(default)]
    pub(crate) redirect: Option<RedirectSettings>,
    /// 可选：站点基础 URL（包含协议与域名，末尾可不带 `/`），用于 canonical、sitemap、OG。
    #[serde(default)]
    pub(crate) base_url: Option<String>,
    /// 可选：用于社交分享的图片地址（相对或绝对）。缺省使用 `assets/favicon.svg`。
    #[serde(default)]
    pub(crate) og_image: Option<String>,
    /// 站点地图默认设置
    #[serde(default)]
    pub(crate) sitemap: Option<SitemapSettings>,
    /// 搜索引擎列表（名称 + 模板，如 https://www.google.com/search?q={q}）
    #[serde(default)]
    pub(crate) search_engines: Option<Vec<SearchEngine>>,
    /// 默认搜索引擎名（匹配 search_engines[].name），未设置则使用第一个
    #[serde(default)]
    pub(crate) default_engine: Option<String>,
    /// 布局：default | ntp（Chrome 新标签页风格）
    #[serde(default = "default_layout")]
    pub(crate) layout: Layout,
    /// 可选：百度统计（百度站长平台 Tongji）的站点 ID（用于 hm.js）
    #[serde(default)]
    pub(crate) baidu_tongji_id: Option<String>,
    /// 可选：Google Analytics（推荐 GA4 Measurement ID，如 G-XXXX）
    #[serde(default)]
    pub(crate) google_analytics_id: Option<String>,
    /// 可选：分类显示模式配置（category -> display mode），例如：{"常用":"standard", "开发":"compact"}
    #[serde(default)]
    pub(crate) category_display: Option<std::collections::HashMap<String, String>>,
    /// 可选：默认分类显示模式（未显式配置的分类使用），可取：standard|compact|list|text
    #[serde(default)]
    pub(crate) default_category_display: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ColorScheme {
    Auto,
    Light,
    Dark,
}

pub(crate) fn default_color_scheme() -> ColorScheme { ColorScheme::Auto }

#[derive(Debug, Deserialize)]
pub(crate) struct Group {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) links: Vec<Link>,
    /// 一级分类（侧边栏）。未设置时默认使用 "全部"。
    #[serde(default)]
    pub(crate) category: Option<String>,
    /// 可选：分组显示模式（优先级高于 site.category_display），standard|compact|list|text；也接受中文别名
    #[serde(default, alias = "display_mode")]
    pub(crate) display: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Link {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) url: Option<String>,
    /// 简介（用于列表页显示）。兼容旧字段名 `desc`。
    #[serde(default, alias = "desc")]
    pub(crate) intro: String,
    /// 详情（用于详情页，可写富文本 HTML）。未填写时默认回退为简介。
    #[serde(default)]
    pub(crate) details: Option<String>,
    /// 可选：显式指定 slug（将用于外网详情页路径 go/<slug>/）
    #[serde(default)]
    pub(crate) slug: Option<String>,
    /// 可选：图标 URL（相对/绝对）
    #[serde(default)]
    pub(crate) icon: Option<String>,
    /// 可选：内网地址
    #[serde(default)]
    pub(crate) intranet: Option<String>,
    /// 可选：风险等级（low|medium|high），用于外网跳转页提示。若未配置，回退到 site.redirect.default_risk
    #[serde(default)]
    pub(crate) risk: Option<RiskLevel>,
    /// 可选：UTM 参数（若设置，将覆盖 site.redirect.utm；只对外网跳转页生效）
    #[serde(default)]
    pub(crate) utm: Option<UtmParams>,
    /// 站点地图：最近修改时间（ISO 8601/RFC3339 或 YYYY-MM-DD）
    #[serde(default)]
    pub(crate) lastmod: Option<String>,
    /// 站点地图：变更频率（always/hourly/daily/weekly/monthly/yearly/never）
    #[serde(default)]
    pub(crate) changefreq: Option<ChangeFreq>,
    /// 站点地图：优先级（0.0 - 1.0）
    #[serde(default)]
    pub(crate) priority: Option<f32>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RiskLevel { Low, Medium, High }

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct RedirectSettings {
    /// 跳转延迟秒数（为 0 或缺省则不自动跳转）
    #[serde(default)]
    pub(crate) delay_seconds: Option<u32>,
    /// 默认风险等级（链接未设置 risk 时使用）
    #[serde(default)]
    pub(crate) default_risk: Option<RiskLevel>,
    /// 站点级 UTM 参数（链接未设置 utm 时使用）
    #[serde(default)]
    pub(crate) utm: Option<UtmParams>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct UtmParams {
    #[serde(default)] pub(crate) source: Option<String>,
    #[serde(default)] pub(crate) medium: Option<String>,
    #[serde(default)] pub(crate) campaign: Option<String>,
    #[serde(default)] pub(crate) term: Option<String>,
    #[serde(default)] pub(crate) content: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub(crate) struct SearchEngine {
    pub(crate) name: String,
    pub(crate) template: String,
    #[serde(default)]
    pub(crate) icon: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Layout { Default, Ntp }

pub(crate) fn default_layout() -> Layout { Layout::Default }

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ChangeFreq { Always, Hourly, Daily, Weekly, Monthly, Yearly, Never }

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct SitemapSettings {
    #[serde(default)]
    pub(crate) default_changefreq: Option<ChangeFreq>,
    #[serde(default)]
    pub(crate) default_priority: Option<f32>,
    #[serde(default)]
    pub(crate) lastmod: Option<String>,
}

/// 配置来源（用于打印和调试）
#[derive(Debug, Clone)]
pub(crate) enum ConfigSource {
    LocalExplicit(String),
    LocalAuto(String),
    #[cfg(feature = "remote")]
    Url(String),
    #[cfg(feature = "remote")]
    Gist { id: String, file: Option<String>, raw_url: String },
}

/// 加载后的配置文本及其来源
#[derive(Debug, Clone)]
pub(crate) struct LoadedConfig { pub(crate) text: String, pub(crate) source: ConfigSource }

/// 人类可读的来源描述
pub(crate) fn describe_source(src: &ConfigSource) -> String {
    match src {
        ConfigSource::LocalExplicit(p) => format!("本地文件: {}", p),
        ConfigSource::LocalAuto(p) => format!("本地文件(自动发现): {}", p),
        #[cfg(feature = "remote")]
        ConfigSource::Url(u) => format!("远程 URL: {}", u),
        #[cfg(feature = "remote")]
        ConfigSource::Gist { id, file, raw_url } => {
            match file {
                Some(f) => format!("Gist {} / {} (raw: {})", id, f, raw_url),
                None => format!("Gist {} (raw: {})", id, raw_url),
            }
        }
    }
}

// 自动发现本地配置：dove.yaml / dove.yml / config.yaml / config.yml
fn _resolve_local_config_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit { if p.exists() { return Some(p.to_path_buf()); } }
    for cand in ["dove.yaml", "dove.yml", "config.yaml", "config.yml"] {
        let p = Path::new(cand);
        if p.exists() { return Some(p.to_path_buf()); }
    }
    // 兼容在工作区根目录运行：尝试 dove/ 子目录中寻找
    let dove_dir = Path::new("dove");
    if dove_dir.is_dir() {
        for cand in ["dove.yaml", "dove.yml", "config.yaml", "config.yml"] {
            let p = dove_dir.join(cand);
            if p.exists() { return Some(p); }
        }
    }
    None
}

// 仅解析显式提供的本地路径；不做自动发现
fn _resolve_explicit_config_path(explicit: Option<&Path>) -> Option<PathBuf> {
    match explicit {
        Some(p) if p.exists() => Some(p.to_path_buf()),
        _ => None,
    }
}

#[cfg(feature = "remote")]
pub(crate) fn load_config(
    input_path: Option<&Path>,
    input_url: Option<&str>,
    gist_id: Option<&str>,
    gist_file: Option<&str>,
    token: Option<&str>,
    auth_scheme: Option<&str>,
) -> Result<LoadedConfig> {
    // 1) 显式本地路径（仅当明确提供）
    if let Some(path) = _resolve_explicit_config_path(input_path) {
        let raw = fs::read_to_string(&path).with_context(|| format!("读取配置失败: {}", path.display()))?;
        let text = expand_includes_text(&raw, Some(&path), None, token, auth_scheme)
            .with_context(|| format!("展开 include 失败: {}", path.display()))?;
        return Ok(LoadedConfig { text, source: ConfigSource::LocalExplicit(path.display().to_string()) });
    }
    // 2) URL
    if let Some(url) = input_url {
        let raw = http_get_text(url, token, auth_scheme).with_context(|| format!("下载配置失败: {}", url))?;
        let text = expand_includes_text(&raw, None, Some(url), token, auth_scheme)
            .with_context(|| format!("展开 include 失败: {}", url))?;
        return Ok(LoadedConfig { text, source: ConfigSource::Url(url.to_string()) });
    }
    // 3) Gist by ID（若提供则优先于本地自动发现）
    if let Some(id) = gist_id {
        let (raw_url, chosen) = gist_resolve_raw_url(id, gist_file, token, auth_scheme)?;
        let raw = http_get_text(&raw_url, token, auth_scheme)
            .with_context(|| format!("下载配置失败: Gist {} 文件 {}", id, chosen.as_deref().unwrap_or("<auto>")))?;
        let text = expand_includes_text(&raw, None, Some(&raw_url), token, auth_scheme)
            .with_context(|| format!("展开 include 失败: Gist {} 文件 {}", id, chosen.as_deref().unwrap_or("<auto>")))?;
        return Ok(LoadedConfig { text, source: ConfigSource::Gist { id: id.to_string(), file: chosen, raw_url } });
    }
    // 4) 本地自动查找
    if let Some(path) = _resolve_local_config_path(None) {
        let raw = fs::read_to_string(&path).with_context(|| format!("读取配置失败: {}", path.display()))?;
        let text = expand_includes_text(&raw, Some(&path), None, token, auth_scheme)
            .with_context(|| format!("展开 include 失败: {}", path.display()))?;
        return Ok(LoadedConfig { text, source: ConfigSource::LocalAuto(path.display().to_string()) });
    }
    bail!("未找到配置：请提供 --input 或 --input-url，或设置 DOVE_INPUT/DOVE_INPUT_URL/DOVE_GIST_ID，或在当前目录放置 dove.yaml");
}

#[cfg(not(feature = "remote"))]
pub(crate) fn load_config(
    input_path: Option<&Path>,
    _input_url: Option<&str>,
    _gist_id: Option<&str>,
    _gist_file: Option<&str>,
    _token: Option<&str>,
    _auth_scheme: Option<&str>,
) -> Result<LoadedConfig> {
    if let Some(path) = _resolve_explicit_config_path(input_path) {
        let raw = fs::read_to_string(&path).with_context(|| format!("读取配置失败: {}", path.display()))?;
        let text = expand_includes_text(&raw, Some(&path), None)
            .with_context(|| format!("展开 include 失败: {}", path.display()))?;
        return Ok(LoadedConfig { text, source: ConfigSource::LocalExplicit(path.display().to_string()) });
    }
    if let Some(path) = _resolve_local_config_path(None) {
        let raw = fs::read_to_string(&path).with_context(|| format!("读取配置失败: {}", path.display()))?;
        let text = expand_includes_text(&raw, Some(&path), None)
            .with_context(|| format!("展开 include 失败: {}", path.display()))?;
        return Ok(LoadedConfig { text, source: ConfigSource::LocalAuto(path.display().to_string()) });
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
    let resp = ensure_success(req.call(), url)?;
    resp.into_string()
        .with_context(|| format!("读取响应文本失败: {}", url))
}

#[cfg(feature = "remote")]
fn ensure_success(resp: Result<Response, ureq::Error>, url: &str) -> Result<Response> {
    match resp {
        Ok(r) => Ok(r),
        Err(e) => bail!("HTTP 请求失败 {}: {}", url, e),
    }
}

#[cfg(feature = "remote")]
fn gist_resolve_raw_url(id: &str, file_name: Option<&str>, token: Option<&str>, auth_scheme: Option<&str>) -> Result<(String, Option<String>)> {
    let api = format!("https://api.github.com/gists/{}", id);
    let mut req = ureq::get(&api)
        .set("User-Agent", "dove/0.1")
        .set("Accept", "application/vnd.github+json");
    if let Some(t) = token {
        let scheme = auth_scheme.map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("token");
        req = req.set("Authorization", &format!("{} {}", scheme, t));
    }
    let resp = ensure_success(req.call(), &api)?;
    let v: serde_json::Value = resp.into_json().context("解析 Gist API 响应失败")?;
    let files = v.get("files").and_then(|x| x.as_object()).ok_or_else(|| anyhow::anyhow!("Gist 无文件"))?;
    if let Some(target) = file_name {
        if let Some(file_obj) = files.get(target) { 
            if let Some(raw) = file_obj.get("raw_url").and_then(|r| r.as_str()) {
                return Ok((raw.to_string(), Some(target.to_string())));
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

// ===== Include 支持 =====

#[derive(Clone, Debug)]
enum IncludeBase {
    LocalDir(PathBuf),
    #[cfg(feature = "remote")]
    UrlBase(String),
}

#[cfg(feature = "remote")]
fn is_url_like(s: &str) -> bool {
    let t = s.trim();
    t.starts_with("http://") || t.starts_with("https://")
}

fn yaml_merge(base: Value, overlay: Value) -> Value {
    match (base, overlay) {
        (Value::Mapping(mut a), Value::Mapping(b)) => {
            for (k, v_b) in b {
                if let Some(v_a) = a.get_mut(&k) {
                    let merged = yaml_merge(v_a.clone(), v_b);
                    *v_a = merged;
                } else {
                    a.insert(k, v_b);
                }
            }
            Value::Mapping(a)
        }
        (Value::Sequence(a), Value::Sequence(b)) => {
            let mut a2 = Vec::new();
            a2.extend(a);
            a2.extend(b);
            Value::Sequence(a2)
        }
        (_a, b) => b, // 标量或类型不同：覆盖
    }
}

fn mapping_remove_includes(m: &mut Mapping) -> Option<Vec<String>> {
    // 支持 include/includes 两种键名
    let mut includes: Vec<String> = Vec::new();
    // 收集
    for key in ["include", "includes"] {
        let k = Value::String(key.to_string());
        if let Some(v) = m.remove(&k) {
            match v {
                Value::String(s) => { if !s.trim().is_empty() { includes.push(s); } }
                Value::Sequence(arr) => {
                    for item in arr.into_iter() {
                        if let Value::String(s) = item { if !s.trim().is_empty() { includes.push(s); } }
                    }
                }
                _ => {}
            }
        }
    }
    if includes.is_empty() { None } else { Some(includes) }
}

#[cfg(feature = "remote")]
fn value_dir_of_url(url: &str) -> String {
    match url.rfind('/') {
        Some(idx) => url[..idx+1].to_string(),
        None => url.to_string(),
    }
}

#[cfg(feature = "remote")]
fn join_url(base: &str, rel: &str) -> String {
    if is_url_like(rel) { return rel.to_string(); }
    let has_slash = base.ends_with('/');
    if has_slash { format!("{}{}", base, rel) } else { format!("{}/{}", base, rel) }
}

fn expand_includes_value(
    mut root: Value,
    base: &IncludeBase,
    visited: &mut HashSet<String>,
    #[cfg(feature = "remote")] token: Option<&str>,
    #[cfg(feature = "remote")] auth_scheme: Option<&str>,
) -> Result<Value> {
    // 仅在映射的最外层处理 include；并允许递归 include
    if let Value::Mapping(ref mut m) = root {
        let includes = mapping_remove_includes(m).unwrap_or_default();
        if !includes.is_empty() {
            // 为确定性，对本地通配展开后排序
            let mut expanded_values: Vec<Value> = Vec::new();
            for inc in includes {
                let inc = inc.trim();
                match base {
                    IncludeBase::LocalDir(dir) => {
                        // 支持通配符（glob）。
                        let pattern_path = dir.join(inc);
                        let pattern_str = pattern_path.to_string_lossy().to_string();
                        let mut matched: BTreeSet<String> = BTreeSet::new();
                        if let Ok(paths) = glob::glob(&pattern_str) {
                            for p in paths.flatten() { matched.insert(p.to_string_lossy().to_string()); }
                        }
                        // 若未匹配通配，则按普通文件处理
                        if matched.is_empty() {
                            let p = dir.join(inc);
                            matched.insert(p.to_string_lossy().to_string());
                        }
                        for p_str in matched {
                            let p = PathBuf::from(&p_str);
                            if !p.exists() { bail!("include 文件不存在: {}", p.display()); }
                            let abs = p.canonicalize().unwrap_or(p.clone());
                            let key = format!("local::{}", abs.display());
                            if !visited.insert(key.clone()) { bail!("检测到循环 include: {}", abs.display()); }
                            let text = fs::read_to_string(&abs)
                                .with_context(|| format!("读取 include 失败: {}", abs.display()))?;
                            let mut v: Value = serde_yaml::from_str(&text)
                                .with_context(|| format!("解析 YAML 失败: {}", abs.display()))?;
                            let new_base = IncludeBase::LocalDir(abs.parent().unwrap_or(Path::new(".")).to_path_buf());
                            v = expand_includes_value(
                                v,
                                &new_base,
                                visited,
                                #[cfg(feature = "remote")] token,
                                #[cfg(feature = "remote")] auth_scheme,
                            )?;
                            // 若 include 根是序列，视为 groups 片段
                            if let Value::Sequence(seq) = v {
                                let mut m = Mapping::new();
                                m.insert(Value::String("groups".to_string()), Value::Sequence(seq));
                                v = Value::Mapping(m);
                            }
                            expanded_values.push(v);
                        }
                    }
                    #[cfg(feature = "remote")]
                    IncludeBase::UrlBase(base_url) => {
                        let target = if is_url_like(inc) { inc.to_string() } else { join_url(base_url, inc) };
                        let key = format!("url::{}", target);
                        if !visited.insert(key.clone()) { bail!("检测到循环 include: {}", target); }
                        let text = http_get_text(&target, token, auth_scheme)
                            .with_context(|| format!("下载 include 失败: {}", target))?;
                        let mut v: Value = serde_yaml::from_str(&text)
                            .with_context(|| format!("解析 YAML 失败: {}", target))?;
                        let new_base = IncludeBase::UrlBase(value_dir_of_url(&target));
                        v = expand_includes_value(
                            v,
                            &new_base,
                            visited,
                            token,
                            auth_scheme,
                        )?;
                        if let Value::Sequence(seq) = v {
                            let mut m = Mapping::new();
                            m.insert(Value::String("groups".to_string()), Value::Sequence(seq));
                            v = Value::Mapping(m);
                        }
                        expanded_values.push(v);
                    }
                }
            }

            // 合并展开的 include 内容到 root
            let mut acc = Value::Mapping(Mapping::new());
            for v in expanded_values.into_iter() {
                acc = yaml_merge(acc, v);
            }
            let current = Value::Mapping(m.clone());
            root = yaml_merge(acc, current);
        }
    }
    Ok(root)
}

fn expand_includes_text(
    text: &str,
    base_path: Option<&Path>,
    #[allow(unused_variables)] base_url: Option<&str>,
    #[cfg(feature = "remote")] token: Option<&str>,
    #[cfg(feature = "remote")] auth_scheme: Option<&str>,
) -> Result<String> {
    let mut v: Value = serde_yaml::from_str(text)?;
    let mut visited: HashSet<String> = HashSet::new();
    let base = if let Some(p) = base_path {
        IncludeBase::LocalDir(p.parent().unwrap_or(Path::new(".")).to_path_buf())
    } else {
        #[cfg(feature = "remote")]
        {
            let url = base_url.unwrap_or("");
            IncludeBase::UrlBase(value_dir_of_url(url))
        }
        #[cfg(not(feature = "remote"))]
        {
            // 无 remote 特性时，不支持 URL include。这里仅占位，实际不会走到。
            IncludeBase::LocalDir(PathBuf::from("."))
        }
    };
    v = expand_includes_value(
        v,
        &base,
        &mut visited,
        #[cfg(feature = "remote")] token,
        #[cfg(feature = "remote")] auth_scheme,
    )?;
    let s = serde_yaml::to_string(&v)?;
    Ok(s)
}
