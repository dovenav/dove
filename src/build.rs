//! 构建与渲染模块：
//! - 解析主题模板并渲染首页、内网页与详情跳转页
//! - 生成 robots.txt、sitemap.xml
//! - 处理 slug/UTM/风险标签等

use anyhow::{bail, Context, Result};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tera::{Context as TContext, Tera};

use crate::{
    config::{ChangeFreq, ColorScheme, Config, Layout, RiskLevel, SearchEngine, Site, UtmParams},
    icons::{download_icons_concurrent, normalize_remote_icon},
    utils::{env_opt_string, env_opt_usize, hostname_from_url, safe_subpath},
};

/// 执行构建：拷贝资源、并发缓存远程图标、渲染页面、写出 sitemap/robots
#[allow(clippy::too_many_arguments)]
pub(crate) fn build(
    mut config: Config,
    out_dir: &Path,
    static_dir: Option<&Path>,
    theme_cli: Option<&Path>,
    base_path_cli: Option<String>,
    no_intranet: bool,
    generate_intermediate_page: bool,
    color_scheme_override: Option<ColorScheme>,
    title_override: Option<String>,
    desc_override: Option<String>,
    build_version_opt: Option<String>,
    icon_dir_cli: Option<String>,
    icon_threads_cli: Option<usize>,
) -> Result<()> {
    // 准备输出目录
    if !out_dir.exists() {
        fs::create_dir_all(out_dir)
            .with_context(|| format!("创建输出目录失败: {}", out_dir.display()))?;
    }
    // 计算站点根目录（支持 base_path 子路径），CLI 覆盖配置
    let base_path_effective = base_path_cli.or_else(|| config.site.base_path.clone());
    let site_dir = match &base_path_effective {
        Some(bp) => match safe_subpath(bp) {
            Some(sub) => out_dir.join(sub),
            None => out_dir.to_path_buf(),
        },
        None => out_dir.to_path_buf(),
    };
    if !site_dir.exists() {
        fs::create_dir_all(&site_dir)
            .with_context(|| format!("创建站点目录失败: {}", site_dir.display()))?;
    }

    // 解析主题目录：CLI --theme > 配置 site.theme_dir > 默认 themes/default
    let mut theme_dir = theme_cli
        .map(|p| p.to_path_buf())
        .or_else(|| config.site.theme_dir.as_ref().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("themes/default"));
    if !theme_dir.exists() {
        // 兼容在工作区根目录运行：尝试 dove/<theme_dir>
        let alt = Path::new("dove").join(&theme_dir);
        if alt.exists() {
            theme_dir = alt;
        }
    }
    if !theme_dir.exists() {
        bail!(
            "主题目录不存在: {}。可用 --theme 指定或在 dove.yaml 的 site.theme_dir 配置。",
            theme_dir.display()
        );
    }

    // 拷贝主题 assets -> site_dir/assets
    let theme_assets = theme_dir.join("assets");
    if theme_assets.exists() {
        let dest_assets = site_dir.join("assets");
        if !dest_assets.exists() {
            fs::create_dir_all(&dest_assets)?;
        }
        crate::init::copy_dir_all(&theme_assets, &dest_assets)?;

        // Copy sw.js to dist directory if it exists
        let sw_js_path = theme_assets.join("sw.js");
        if sw_js_path.exists() {
            let dist_sw_js_path = site_dir.join("sw.js");
            std::fs::copy(&sw_js_path, &dist_sw_js_path)?;
            println!("-> Copied sw.js to {}", dist_sw_js_path.display());
        }
    }

    // 复制用户静态资源（最后复制以便覆盖主题）
    if let Some(sd) = static_dir {
        if sd.exists() {
            crate::init::copy_dir_all(sd, &site_dir)?;
        } else {
            eprintln!("警告: 指定的静态目录不存在: {}", sd.display());
        }
    }

    // 并发预取远程图标，并回写为本地相对路径（失败则保持远程 URL）
    // 目标目录优先级：CLI > ENV > 默认；相对于站点根
    let icon_dir_rel: String = icon_dir_cli
        .or_else(|| env_opt_string("DOVE_ICON_DIR"))
        .unwrap_or_else(|| "assets/icons".to_string());
    let icon_threads: usize = icon_threads_cli
        .or_else(|| env_opt_usize("DOVE_ICON_THREADS"))
        .unwrap_or(8)
        .max(1);
    let icon_dir_abs = site_dir.join(icon_dir_rel.trim_start_matches('/'));
    if !icon_dir_abs.exists() {
        fs::create_dir_all(&icon_dir_abs)?;
    }

    // 收集需要下载的远程图标（去重）
    let mut targets: Vec<(String, String)> = Vec::new(); // (orig, fetch_url)
    let mut seen: HashSet<String> = HashSet::new();
    // 搜索引擎 icons
    if let Some(ref list) = config.site.search_engines {
        for e in list {
            if let Some(ref ic) = e.icon {
                if let Some((orig, fetch)) = normalize_remote_icon(ic) {
                    if seen.insert(orig.clone()) {
                        targets.push((orig, fetch));
                    }
                }
            }
        }
    }
    // 链接 icons
    for g in &config.groups {
        for l in &g.links {
            if let Some(ref ic) = l.icon {
                if let Some((orig, fetch)) = normalize_remote_icon(ic) {
                    if seen.insert(orig.clone()) {
                        targets.push((orig, fetch));
                    }
                }
            }
        }
    }

    // 执行下载（remote 功能启用时有效）并得到映射 orig -> 相对路径
    if targets.is_empty() {
        println!("ℹ️ 未发现需要下载的图标。");
    } else {
        println!(
            "⬇️ 下载图标: {} 个 -> {}（并发 {}）",
            targets.len(),
            icon_dir_rel,
            icon_threads
        );
    }
    let icon_map: HashMap<String, String> =
        download_icons_concurrent(&targets, &icon_dir_abs, &icon_dir_rel, icon_threads);

    // 回写配置中的 icon 字段（仅当下载成功时替换成本地相对路径）
    if let Some(ref mut engines) = config.site.search_engines {
        for e in engines.iter_mut() {
            if let Some(ref mut ic) = e.icon {
                if let Some(v) = icon_map.get(ic) {
                    *ic = v.clone();
                }
            }
        }
    }
    for g in config.groups.iter_mut() {
        for l in g.links.iter_mut() {
            if let Some(ref mut ic) = l.icon {
                if let Some(v) = icon_map.get(ic) {
                    *ic = v.clone();
                }
            }
        }
    }

    // 版本：CLI > ENV > crate
    let effective_build_version = build_version_opt
        .or_else(|| env_opt_string("DOVE_BUILD_VERSION"))
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    // 构建时间（UTC，ISO 8601 简化至秒）
    let build_time = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // 渲染 HTML via Tera 到 site_dir
    let externals = render_with_theme(
        &config,
        &theme_dir,
        &site_dir,
        !no_intranet,
        generate_intermediate_page,
        color_scheme_override,
        title_override,
        desc_override,
        &effective_build_version,
        &build_time,
    )?;

    // 生成 robots.txt 与 sitemap.xml（若提供 base_url 则写绝对 URL）
    write_robots(&site_dir)?;
    write_sitemap(
        &site_dir,
        &config.site,
        base_path_effective.as_deref(),
        &externals,
        &build_time,
    )?;

    println!("✅ 生成完成 -> {}", site_dir.display());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_with_theme(
    cfg: &Config,
    theme_dir: &Path,
    out_dir: &Path,
    generate_intranet: bool,
    generate_intermediate_page: bool,
    color_scheme_override: Option<ColorScheme>,
    title_override: Option<String>,
    desc_override: Option<String>,
    build_version: &str,
    build_time: &str,
) -> Result<Vec<LinkDetail>> {
    // 匹配主题模板目录
    let pattern = theme_dir.join("templates").join("**/*");
    let pattern_str = pattern.to_string_lossy().to_string();
    let tera = Tera::new(&pattern_str).with_context(|| format!("加载模板失败: {}", pattern_str))?;

    // 渲染外网(index.html)，按需渲染内网(intranet/index.html)
    let title_ref = title_override.as_deref();
    let desc_ref = desc_override.as_deref();
    let externals = render_one(
        &tera,
        cfg,
        out_dir,
        NetMode::External,
        generate_intranet,
        generate_intermediate_page,
        color_scheme_override,
        title_ref,
        desc_ref,
        build_version,
        build_time,
    )?;
    if !externals.is_empty() && generate_intermediate_page {
        render_link_details(
            &tera,
            cfg,
            out_dir,
            &externals,
            color_scheme_override,
            title_ref,
            desc_ref,
            build_version,
            build_time,
        )?;
    }
    if generate_intranet {
        let _internals = render_one(
            &tera,
            cfg,
            out_dir,
            NetMode::Intranet,
            generate_intranet,
            generate_intermediate_page,
            color_scheme_override,
            title_ref,
            desc_ref,
            build_version,
            build_time,
        )?;
    }
    Ok(externals)
}

#[derive(Clone, Copy)]
enum NetMode {
    External,
    Intranet,
}

#[derive(Clone)]
struct LinkDetail {
    slug: String,
    name: String,
    intro: String,
    details: Option<String>,
    icon: Option<String>,
    host: String,
    final_url: String,
    risk: Option<RiskLevel>,
    delay_seconds: u32,
    utm: Option<UtmParams>,
    s_lastmod: Option<String>,
    s_changefreq: Option<ChangeFreq>,
    s_priority: Option<f32>,
}

#[allow(clippy::too_many_arguments)]
fn render_one(
    tera: &Tera,
    cfg: &Config,
    out_dir: &Path,
    mode: NetMode,
    has_intranet: bool,
    generate_intermediate_page: bool,
    color_scheme_override: Option<ColorScheme>,
    title_override: Option<&str>,
    desc_override: Option<&str>,
    build_version: &str,
    build_time: &str,
) -> Result<Vec<LinkDetail>> {
    let mut ctx = TContext::new();
    // Build/version info from caller (CI/CLI), already resolved
    ctx.insert("build_version", &build_version);
    ctx.insert("build_time", &build_time);

    let site_title = title_override.unwrap_or(&cfg.site.title);
    let site_desc = desc_override.unwrap_or(&cfg.site.description);
    ctx.insert("site_title", &site_title);
    ctx.insert("site_desc", &site_desc);
    // 颜色模式
    let scheme = match color_scheme_override.unwrap_or(cfg.site.color_scheme) {
        ColorScheme::Auto => "auto",
        ColorScheme::Light => "light",
        ColorScheme::Dark => "dark",
    };
    ctx.insert("color_scheme", &scheme);
    // 是否存在内网
    ctx.insert("has_intranet", &has_intranet);
    // 是否生成中间页
    ctx.insert("generate_intermediate_page", &generate_intermediate_page);
    // 静态资源与根路径前缀
    let asset_prefix = match mode {
        NetMode::External => String::new(),
        NetMode::Intranet => String::from("../"),
    };
    let root_prefix = asset_prefix.clone();
    let service_worker_path = format!("{}sw.js", root_prefix);
    ctx.insert("asset_prefix", &asset_prefix);
    ctx.insert("root_prefix", &root_prefix);
    ctx.insert("service_worker_path", &service_worker_path);
    // 内/外网切换链接与标签
    let (network_switch_href, mode_other_label) = match mode {
        NetMode::External => ("intranet/", "内网"),
        NetMode::Intranet => ("../", "外网"),
    };
    ctx.insert("network_switch_href", &network_switch_href);
    ctx.insert("mode_other_label", &mode_other_label);

    // 搜索引擎与默认项
    let rengines: Vec<SearchEngine> = cfg.site.search_engines.clone().unwrap_or_default();
    let mut default_engine: String = cfg.site.default_engine.clone().unwrap_or_default();
    if default_engine.is_empty() && !rengines.is_empty() {
        default_engine = rengines[0].name.clone();
    }
    ctx.insert("search_engines", &rengines);
    ctx.insert("engine_default", &default_engine);
    // 布局
    let layout = match cfg.site.layout {
        Layout::Default => "default",
        Layout::Ntp => "ntp",
    };
    ctx.insert("layout", &layout);
    // 可选：百度统计（Tongji）站点 ID，用于注入 hm.js
    if let Some(ref id) = cfg.site.baidu_tongji_id {
        if !id.trim().is_empty() {
            ctx.insert("baidu_tongji_id", id);
        }
    }
    // 可选：Google Analytics（GA4）Measurement ID，用于注入 gtag.js
    if let Some(ref gid) = cfg.site.google_analytics_id {
        if !gid.trim().is_empty() {
            ctx.insert("google_analytics_id", gid);
        }
    }

    // Canonical 与 OG image（仅外网）
    if matches!(mode, NetMode::External) {
        if let Some(base) = cfg.site.base_url.as_deref() {
            let page = match mode {
                NetMode::External => "index.html",
                NetMode::Intranet => "intranet/index.html",
            };
            let canon = build_page_url(Some(base), cfg.site.base_path.as_deref(), page);
            ctx.insert("canonical_url", &canon);
        }
        if let Some(og) = og_image_url(cfg, false) {
            ctx.insert("og_image", &og);
        }
    }

    use serde::Serialize;
    #[derive(Serialize)]
    struct RLink {
        name: String,
        href: String,
        display_url: String,
        desc: String,
        icon: Option<String>,
        host: String,
    }
    #[derive(Serialize)]
    struct RGroup {
        name: String,
        category: String,
        display: String,
        links: Vec<RLink>,
    }

    let mut used_slugs: HashSet<String> = HashSet::new();
    let mut name_counts: HashMap<String, u32> = HashMap::new();
    let mut details: Vec<LinkDetail> = Vec::new();
    let mut rgroups: Vec<RGroup> = Vec::new();
    let mut categories: Vec<String> = Vec::new();
    for g in &cfg.groups {
        let mut rlinks = Vec::new();
        for l in &g.links {
            match mode {
                NetMode::External => {
                    // 仅当存在外网地址时参与外网页面与详情页
                    let final_url = match l.url.as_ref().and_then(|s| {
                        if s.trim().is_empty() {
                            None
                        } else {
                            Some(s)
                        }
                    }) {
                        Some(u) => u.to_string(),
                        None => {
                            continue;
                        }
                    };
                    let host = hostname_from_url(&final_url).unwrap_or_default();
                    let base_slug = if let Some(user_slug) = &l.slug {
                        slugify(user_slug)
                    } else {
                        // 默认：按 name 生成；若 name 重复，则使用 name+host 组合
                        let key = l.name.to_lowercase();
                        let entry = name_counts.entry(key).or_insert(0);
                        *entry += 1;
                        if *entry == 1 {
                            slugify(&l.name)
                        } else if !host.is_empty() {
                            slugify(&format!("{}-{}", l.name, host))
                        } else {
                            slugify(&l.name)
                        }
                    };
                    let slug = unique_slug(&base_slug, &mut used_slugs);
                    let href = if generate_intermediate_page {
                        format!("/go/{}/", slug)
                    } else {
                        final_url.clone()
                    };
                    let icon_res = l
                        .icon
                        .as_ref()
                        .map(|s| resolve_icon_for_page(s, &asset_prefix));
                    rlinks.push(RLink {
                        name: l.name.clone(),
                        href: href.clone(),
                        display_url: final_url.clone(),
                        desc: l.intro.clone(),
                        icon: icon_res,
                        host: host.clone(),
                    });
                    let delay = cfg
                        .site
                        .redirect
                        .as_ref()
                        .and_then(|r| r.delay_seconds)
                        .unwrap_or(0);
                    let risk = l
                        .risk
                        .or_else(|| cfg.site.redirect.as_ref().and_then(|r| r.default_risk));
                    let utm = l
                        .utm
                        .clone()
                        .or_else(|| cfg.site.redirect.as_ref().and_then(|r| r.utm.clone()));
                    details.push(LinkDetail {
                        slug,
                        name: l.name.clone(),
                        intro: l.intro.clone(),
                        details: l.details.clone(),
                        icon: l.icon.clone(),
                        host,
                        final_url,
                        risk,
                        delay_seconds: delay,
                        utm,
                        s_lastmod: l.lastmod.clone(),
                        s_changefreq: l.changefreq,
                        s_priority: l.priority,
                    });
                }
                NetMode::Intranet => {
                    let href = l
                        .intranet
                        .clone()
                        .or_else(|| l.url.clone())
                        .unwrap_or_default();
                    if href.trim().is_empty() {
                        continue;
                    }
                    let host = hostname_from_url(&href).unwrap_or_default();
                    let icon_res = l
                        .icon
                        .as_ref()
                        .map(|s| resolve_icon_for_page(s, &asset_prefix));
                    let display_url = href.clone();
                    rlinks.push(RLink {
                        name: l.name.clone(),
                        href,
                        display_url,
                        desc: l.intro.clone(),
                        icon: icon_res,
                        host,
                    });
                }
            }
        }
        // 仅当该分组有可展示链接时，才加入分组与分类列表
        if !rlinks.is_empty() {
            let cat = g.category.clone().unwrap_or_else(|| "全部".to_string());
            if !categories.contains(&cat) {
                categories.push(cat.clone());
            }
            let disp = resolve_display(g.display.as_deref(), &cfg.site, &cat);
            rgroups.push(RGroup {
                name: g.name.clone(),
                category: cat,
                display: disp,
                links: rlinks,
            });
        }
    }
    ctx.insert("groups", &rgroups);
    ctx.insert("categories", &categories);

    let html = tera
        .render("index.html.tera", &ctx)
        .context("渲染模板 index.html.tera 失败")?;
    let (target_path, display_name) = match mode {
        NetMode::External => (out_dir.join("index.html"), "index.html".to_string()),
        NetMode::Intranet => {
            let intranet_dir = out_dir.join("intranet");
            if !intranet_dir.exists() {
                fs::create_dir_all(&intranet_dir)?;
            }
            let legacy_path = out_dir.join("intranet.html");
            if legacy_path.exists() {
                if let Err(err) = fs::remove_file(&legacy_path) {
                    eprintln!("警告: 无法删除旧的 intranet.html: {}", err);
                }
            }
            (
                intranet_dir.join("index.html"),
                "intranet/index.html".to_string(),
            )
        }
    };
    fs::write(&target_path, html).with_context(|| format!("写入 {} 失败", display_name))?;
    Ok(details)
}

#[allow(clippy::too_many_arguments)]
fn render_link_details(
    tera: &Tera,
    cfg: &Config,
    out_dir: &Path,
    links: &[LinkDetail],
    color_scheme_override: Option<ColorScheme>,
    title_override: Option<&str>,
    desc_override: Option<&str>,
    build_version: &str,
    build_time: &str,
) -> Result<()> {
    let site_title = title_override.unwrap_or(&cfg.site.title);
    let site_desc = desc_override.unwrap_or(&cfg.site.description);
    let scheme = match color_scheme_override.unwrap_or(cfg.site.color_scheme) {
        ColorScheme::Auto => "auto",
        ColorScheme::Light => "light",
        ColorScheme::Dark => "dark",
    };

    // 预先计算分类（仅包含至少一个可展示外网链接的分组）
    let mut categories: Vec<String> = Vec::new();
    for g in &cfg.groups {
        let mut has_any = false;
        for l in &g.links {
            if let Some(u) = l.url.as_ref() {
                if !u.trim().is_empty() {
                    has_any = true;
                    break;
                }
            }
        }
        if has_any {
            let cat = g.category.clone().unwrap_or_else(|| "全部".to_string());
            if !categories.contains(&cat) {
                categories.push(cat);
            }
        }
    }

    for d in links {
        let mut ctx = TContext::new();
        ctx.insert("build_version", &build_version);
        ctx.insert("build_time", &build_time);
        ctx.insert("site_title", &site_title);
        ctx.insert("site_desc", &site_desc);
        ctx.insert("color_scheme", &scheme);

        // Open Graph相关变量
        if let Some(ref base_url) = cfg.site.base_url {
            ctx.insert("base_url", base_url);
            // 构建详情页的完整URL
            let detail_url = format!("{}/go/{}/", base_url.trim_end_matches('/'), d.slug);
            ctx.insert("site_url", &detail_url);
        }
        if let Some(og) = og_image_url(cfg, true) {
            ctx.insert("og_image", &og);
        }
        ctx.insert("categories", &categories);
        ctx.insert("link_name", &d.name);
        ctx.insert("link_intro", &d.intro);
        // 详情 HTML：若配置了 details，用原样 HTML；否则使用简介文本（将在模板中 escape）
        let details_html: Option<String> = d.details.clone();
        ctx.insert("link_details_html", &details_html);
        let icon_href: Option<String> = d.icon.as_ref().map(|s| resolve_icon_for_detail(s));
        ctx.insert("link_icon", &icon_href);
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
        // 可选：百度统计（Tongji）站点 ID，用于注入 hm.js
        if let Some(ref id) = cfg.site.baidu_tongji_id {
            if !id.trim().is_empty() {
                ctx.insert("baidu_tongji_id", id);
            }
        }
        // 可选：Google Analytics（GA4）Measurement ID，用于注入 gtag.js
        if let Some(ref gid) = cfg.site.google_analytics_id {
            if !gid.trim().is_empty() {
                ctx.insert("google_analytics_id", gid);
            }
        }
        let html = tera
            .render("detail.html.tera", &ctx)
            .context("渲染模板 detail.html.tera 失败")?;
        let dir = out_dir.join("go").join(&d.slug);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        fs::write(dir.join("index.html"), html)
            .with_context(|| format!("写入详情页失败: go/{}/index.html", d.slug))?;
    }
    Ok(())
}

fn slugify(input: &str) -> String {
    let mut s = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            s.push(c);
            prev_dash = false;
        } else if !prev_dash && !s.is_empty() {
            s.push('-');
            prev_dash = true;
        }
    }
    while s.ends_with('-') {
        s.pop();
    }
    if s.is_empty() {
        "link".to_string()
    } else {
        s
    }
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
    let Some(utm) = utm else {
        return url_str.to_string();
    };
    if utm.source.is_none()
        && utm.medium.is_none()
        && utm.campaign.is_none()
        && utm.term.is_none()
        && utm.content.is_none()
    {
        return url_str.to_string();
    }
    if let Ok(mut u) = url::Url::parse(url_str) {
        {
            let mut qp = u.query_pairs_mut();
            if let Some(ref v) = utm.source {
                qp.append_pair("utm_source", v);
            }
            if let Some(ref v) = utm.medium {
                qp.append_pair("utm_medium", v);
            }
            if let Some(ref v) = utm.campaign {
                qp.append_pair("utm_campaign", v);
            }
            if let Some(ref v) = utm.term {
                qp.append_pair("utm_term", v);
            }
            if let Some(ref v) = utm.content {
                qp.append_pair("utm_content", v);
            }
        }
        u.to_string()
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

fn write_robots(root: &Path) -> Result<()> {
    let content = "User-agent: *\nAllow: /\n";
    fs::write(root.join("robots.txt"), content.as_bytes()).context("写入 robots.txt 失败")?;
    Ok(())
}

fn write_sitemap(
    root: &Path,
    site: &Site,
    base_path: Option<&str>,
    details: &[LinkDetail],
    build_time: &str,
) -> Result<()> {
    // Helper to join base_url + base_path + subpath
    fn url_join(base_url: Option<&str>, base_path: Option<&str>, sub: &str) -> String {
        if let Some(b) = base_url {
            let mut out = String::new();
            out.push_str(b.trim_end_matches('/'));
            if let Some(bp) = base_path {
                out.push('/');
                out.push_str(bp.trim_matches('/'));
            }
            out.push('/');
            out.push_str(sub.trim_matches('/'));
            out
        } else {
            // 相对路径
            let mut out = String::new();
            if let Some(bp) = base_path {
                out.push_str(bp.trim_matches('/'));
                out.push('/');
            }
            out.push_str(sub.trim_matches('/'));
            out
        }
    }

    // 首页与内网页
    type UrlEntry = (String, Option<String>, Option<ChangeFreq>, Option<f32>);
    let mut urls: Vec<UrlEntry> = Vec::new();
    urls.push((
        url_join(site.base_url.as_deref(), base_path, "index.html"),
        None,
        site.sitemap.as_ref().and_then(|s| s.default_changefreq),
        site.sitemap.as_ref().and_then(|s| s.default_priority),
    ));
    urls.push((
        url_join(site.base_url.as_deref(), base_path, "intranet/index.html"),
        None,
        site.sitemap.as_ref().and_then(|s| s.default_changefreq),
        site.sitemap.as_ref().and_then(|s| s.default_priority),
    ));
    // 详情页
    for d in details {
        let sub = format!("go/{}/index.html", d.slug);
        urls.push((
            url_join(site.base_url.as_deref(), base_path, &sub),
            d.s_lastmod.clone(),
            d.s_changefreq,
            sanitize_priority(d.s_priority),
        ));
    }

    // 组装 XML
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for (loc, lastmod, cf, pr) in urls {
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", loc));
        if let Some(ts) = lastmod
            .or_else(|| site.sitemap.as_ref().and_then(|s| s.lastmod.clone()))
            .or_else(|| Some(build_time.to_string()))
        {
            xml.push_str(&format!("    <lastmod>{}</lastmod>\n", ts));
        }
        if let Some(c) = cf {
            xml.push_str(&format!(
                "    <changefreq>{}</changefreq>\n",
                changefreq_str(c)
            ));
        }
        if let Some(p) = pr {
            xml.push_str(&format!("    <priority>{:.1}</priority>\n", p));
        }
        xml.push_str("  </url>\n");
    }
    xml.push_str("</urlset>\n");
    fs::write(root.join("sitemap.xml"), xml.as_bytes()).context("写入 sitemap.xml 失败")?;
    Ok(())
}

fn changefreq_str(cf: ChangeFreq) -> &'static str {
    match cf {
        ChangeFreq::Always => "always",
        ChangeFreq::Hourly => "hourly",
        ChangeFreq::Daily => "daily",
        ChangeFreq::Weekly => "weekly",
        ChangeFreq::Monthly => "monthly",
        ChangeFreq::Yearly => "yearly",
        ChangeFreq::Never => "never",
    }
}

fn sanitize_priority(p: Option<f32>) -> Option<f32> {
    p.map(|v| v.clamp(0.0, 1.0))
}

fn build_page_url(base_url: Option<&str>, base_path: Option<&str>, page: &str) -> String {
    if let Some(base) = base_url {
        let mut s = String::new();
        s.push_str(base.trim_end_matches('/'));
        if let Some(bp) = base_path {
            s.push('/');
            s.push_str(bp.trim_matches('/'));
        }
        s.push('/');
        s.push_str(page.trim_matches('/'));
        s
    } else {
        page.to_string()
    }
}

fn og_image_url(cfg: &Config, _detail_page: bool) -> Option<String> {
    if let Some(s) = cfg.site.og_image.as_deref() {
        return Some(s.to_string());
    }
    // 默认图：站点 favicon
    Some("assets/favicon.svg".to_string())
}

// Group display mode: prefer group.display, then site.category_display, then site.default_category_display
fn resolve_display(group_display: Option<&str>, site: &Site, category: &str) -> String {
    fn norm(s: &str) -> &str {
        match s.trim().to_ascii_lowercase().as_str() {
            // English
            "standard" => "standard",
            "compact" => "compact",
            "list" => "list",
            "text" => "text",
            // Chinese aliases
            "标准" => "standard",
            "简洁" => "compact",
            "列表" => "list",
            "文本" => "text",
            _ => "standard",
        }
    }
    if let Some(d) = group_display {
        return norm(d).to_string();
    }
    if let Some(map) = site.category_display.as_ref() {
        if let Some(v) = map.get(category) {
            return norm(v).to_string();
        }
    }
    if let Some(def) = site.default_category_display.as_deref() {
        return norm(def).to_string();
    }
    "standard".to_string()
}

fn resolve_icon_for_detail(icon: &str) -> String {
    let s = icon.trim();
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("//")
        || lower.starts_with("data:")
    {
        s.to_string()
    } else if s.starts_with('/') {
        // 将站点根相对路径转为页面相对（详情页位于 /go/<slug>/）
        let trimmed = s.trim_start_matches('/');
        format!("../../{}", trimmed)
    } else {
        // 普通相对路径，按站点根相对资源处理
        format!("../../{}", s)
    }
}

fn resolve_icon_for_page(icon: &str, asset_prefix: &str) -> String {
    let s = icon.trim();
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("//")
        || lower.starts_with("data:")
    {
        s.to_string()
    } else if s.starts_with('/') {
        let trimmed = s.trim_start_matches('/');
        if asset_prefix.is_empty() {
            trimmed.to_string()
        } else {
            format!("{}{}", asset_prefix, trimmed)
        }
    } else if s.starts_with("../") || s.starts_with("./") {
        s.to_string()
    } else if asset_prefix.is_empty() {
        s.to_string()
    } else {
        format!("{}{}", asset_prefix, s)
    }
}
