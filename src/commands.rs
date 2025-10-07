//! 命令调度模块：
//! - 接收解析好的 CLI 参数，计算“有效参数”
//! - 调用配置加载、构建、预览、初始化等模块

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::{
    build::build,
    cli::{Cli, Command},
    config::{self, Config},
    init::init_scaffold,
    preview::preview_watch_and_serve,
    utils::{env_bool_truthy, env_opt_path, env_opt_string, env_opt_usize, parse_color_scheme},
};

/// 运行指定的子命令
pub(crate) fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Build {
            input,
            input_url,
            #[cfg(feature = "remote")]
            gist_id,
            #[cfg(feature = "remote")]
            gist_file,
            #[cfg(feature = "remote")]
            github_token,
            #[cfg(feature = "remote")]
            auth_scheme,
            out,
            static_dir,
            theme,
            base_path,
            no_intranet,
            color_scheme,
            title,
            description,
            build_version,
            icon_dir,
            icon_threads,
            generate_intermediate_page: generate_intermediate_page_cli,
        } => {
            // 环境变量覆盖（若 CLI 未指定）
            let env_input = env_opt_path("DOVE_INPUT");
            let env_input_url =
                env_opt_string("DOVE_INPUT_URL").or(env_opt_string("DOVE_GIST_URL"));
            #[cfg(feature = "remote")]
            let env_gist_id = env_opt_string("DOVE_GIST_ID");
            #[cfg(feature = "remote")]
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
            #[cfg(feature = "remote")]
            let env_github_token = env_opt_string("DOVE_GITHUB_TOKEN");
            #[cfg(feature = "remote")]
            let env_auth_scheme = env_opt_string("DOVE_AUTH_SCHEME");
            let env_icon_dir = env_opt_string("DOVE_ICON_DIR");
            let env_icon_threads = env_opt_usize("DOVE_ICON_THREADS");
            let env_generate_intermediate_page = env_bool_truthy("DOVE_GENERATE_INTERMEDIATE_PAGE");

            let mut effective_input = input.or(env_input);
            let effective_input_url = input_url.or(env_input_url);
            #[cfg(feature = "remote")]
            let effective_gist_id = gist_id.or(env_gist_id);
            #[cfg(not(feature = "remote"))]
            let effective_gist_id: Option<String> = None;
            #[cfg(feature = "remote")]
            let effective_gist_file = gist_file.or(env_gist_file);
            #[cfg(not(feature = "remote"))]
            let effective_gist_file: Option<String> = None;
            let effective_out = out.or(env_out).unwrap_or_else(|| PathBuf::from("dist"));
            let effective_static = static_dir.or(env_static);
            let effective_theme = theme.or(env_theme).or(env_theme_dir);
            let effective_base_path = base_path.or(env_base_path);
            let effective_no_intranet = if no_intranet { true } else { env_no_intranet };
            let cli_color = color_scheme.and_then(parse_color_scheme);
            let effective_color_scheme = cli_color.or(env_color_scheme);
            let effective_title = title.or(env_title);
            let effective_desc = description.or(env_description);
            #[cfg(feature = "remote")]
            let effective_github_token = github_token.or(env_github_token);
            #[cfg(not(feature = "remote"))]
            let effective_github_token: Option<String> = None;
            #[cfg(feature = "remote")]
            let effective_auth_scheme = auth_scheme.or(env_auth_scheme);
            #[cfg(not(feature = "remote"))]
            let effective_auth_scheme: Option<String> = None;
            let effective_icon_dir = icon_dir.or(env_icon_dir);
            let effective_icon_threads = icon_threads.or(env_icon_threads);
            let effective_generate_intermediate_page = generate_intermediate_page_cli
                .or(env_generate_intermediate_page)
                .unwrap_or(true);

            // 当提供了 URL/Gist 时，忽略显式/环境的本地 input 路径，使 URL/Gist 优先生效
            if effective_input_url.is_some() || effective_gist_id.is_some() {
                effective_input = None;
            }

            // 加载配置（本地/URL/Gist）
            let loaded_cfg = config::load_config(
                effective_input.as_deref(),
                effective_input_url.as_deref(),
                effective_gist_id.as_deref(),
                effective_gist_file.as_deref(),
                effective_github_token.as_deref(),
                effective_auth_scheme.as_deref(),
            )?;
            println!(
                "ℹ️ 本次使用的配置来源: {}",
                config::describe_source(&loaded_cfg.source)
            );
            let config: Config = serde_yaml::from_str(&loaded_cfg.text)
                .with_context(|| "解析 YAML 失败（来自本地/URL/Gist）")?;

            let out_dir = effective_out;
            build(
                config,
                &out_dir,
                effective_static.as_deref(),
                effective_theme.as_deref(),
                effective_base_path,
                effective_no_intranet,
                effective_generate_intermediate_page,
                effective_color_scheme,
                effective_title,
                effective_desc,
                build_version,
                effective_icon_dir,
                effective_icon_threads,
            )
        }
        Command::Init { force, dir } => {
            let dir = dir.unwrap_or_else(|| PathBuf::from("."));
            init_scaffold(&dir, force)
        }
        Command::Preview {
            dir,
            addr,
            build_first,
            input,
            input_url,
            #[cfg(feature = "remote")]
            gist_id,
            #[cfg(feature = "remote")]
            gist_file,
            #[cfg(feature = "remote")]
            github_token,
            #[cfg(feature = "remote")]
            auth_scheme,
            out,
            static_dir,
            theme,
            base_path,
            no_intranet,
            open,
            color_scheme,
            title,
            description,
            build_version,
            icon_dir,
            icon_threads,
            generate_intermediate_page: generate_intermediate_page_cli,
        } => {
            // 环境变量
            let env_addr = env_opt_string("DOVE_PREVIEW_ADDR");
            let env_input = env_opt_path("DOVE_INPUT");
            let env_input_url =
                env_opt_string("DOVE_INPUT_URL").or(env_opt_string("DOVE_GIST_URL"));
            #[cfg(feature = "remote")]
            let env_gist_id = env_opt_string("DOVE_GIST_ID");
            #[cfg(feature = "remote")]
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
            #[cfg(feature = "remote")]
            let env_github_token = env_opt_string("DOVE_GITHUB_TOKEN");
            #[cfg(feature = "remote")]
            let env_auth_scheme = env_opt_string("DOVE_AUTH_SCHEME");
            let env_icon_dir = env_opt_string("DOVE_ICON_DIR");
            let env_icon_threads = env_opt_usize("DOVE_ICON_THREADS");
            let env_generate_intermediate_page = env_bool_truthy("DOVE_GENERATE_INTERMEDIATE_PAGE");

            let effective_addr = addr
                .or(env_addr)
                .unwrap_or_else(|| "127.0.0.1:8787".to_string());
            let mut effective_input = input.or(env_input);
            let effective_input_url = input_url.or(env_input_url);
            #[cfg(feature = "remote")]
            let effective_gist_id = gist_id.or(env_gist_id);
            #[cfg(not(feature = "remote"))]
            let effective_gist_id: Option<String> = None;
            #[cfg(feature = "remote")]
            let effective_gist_file = gist_file.or(env_gist_file);
            #[cfg(not(feature = "remote"))]
            let effective_gist_file: Option<String> = None;
            let effective_out = out.or(env_out).unwrap_or_else(|| PathBuf::from("dist"));
            let effective_static = static_dir.or(env_static);
            let effective_theme = theme.or(env_theme).or(env_theme_dir);
            let effective_base_path = base_path.or(env_base_path);
            let effective_no_intranet = if no_intranet { true } else { env_no_intranet };
            let cli_color = color_scheme.and_then(parse_color_scheme);
            let effective_color_scheme = cli_color.or(env_color_scheme);
            let effective_title = title.or(env_title);
            let effective_desc = description.or(env_description);
            #[cfg(feature = "remote")]
            let effective_github_token = github_token.or(env_github_token);
            #[cfg(not(feature = "remote"))]
            let effective_github_token: Option<String> = None;
            #[cfg(feature = "remote")]
            let effective_auth_scheme = auth_scheme.or(env_auth_scheme);
            #[cfg(not(feature = "remote"))]
            let effective_auth_scheme: Option<String> = None;
            let effective_icon_dir = icon_dir.or(env_icon_dir);
            let effective_icon_threads = icon_threads.or(env_icon_threads);
            let effective_generate_intermediate_page = generate_intermediate_page_cli
                .or(env_generate_intermediate_page)
                .unwrap_or(true);

            // 当提供了 URL/Gist 时，忽略显式/环境的本地 input 路径，使 URL/Gist 优先生效
            if effective_input_url.is_some() || effective_gist_id.is_some() {
                effective_input = None;
            }

            // 可选构建
            if build_first {
                let loaded_cfg = config::load_config(
                    effective_input.as_deref(),
                    effective_input_url.as_deref(),
                    effective_gist_id.as_deref(),
                    effective_gist_file.as_deref(),
                    effective_github_token.as_deref(),
                    effective_auth_scheme.as_deref(),
                )?;
                println!(
                    "ℹ️ 本次使用的配置来源: {}",
                    config::describe_source(&loaded_cfg.source)
                );
                let config: Config = serde_yaml::from_str(&loaded_cfg.text)
                    .with_context(|| "解析 YAML 失败（预览构建）")?;
                build(
                    config,
                    &effective_out,
                    effective_static.as_deref(),
                    effective_theme.as_deref(),
                    effective_base_path.clone(),
                    effective_no_intranet,
                    effective_generate_intermediate_page,
                    effective_color_scheme,
                    effective_title.clone(),
                    effective_desc.clone(),
                    build_version.clone(),
                    effective_icon_dir.clone(),
                    effective_icon_threads,
                )?;
            }

            // 计算服务目录
            let serve_dir = if let Some(d) = dir {
                d
            } else {
                // 尝试从配置推导 base_path
                let loaded_opt = config::load_config(
                    effective_input.as_deref(),
                    effective_input_url.as_deref(),
                    effective_gist_id.as_deref(),
                    effective_gist_file.as_deref(),
                    effective_github_token.as_deref(),
                    effective_auth_scheme.as_deref(),
                );
                match loaded_opt.and_then(|lc| {
                    serde_yaml::from_str::<Config>(&lc.text)
                        .map(|c| (lc, c))
                        .map_err(anyhow::Error::from)
                }) {
                    Ok((_lc, cfg)) => {
                        let mut d = effective_out.clone();
                        if let Some(bp) = cfg.site.base_path.as_deref() {
                            for seg in bp.split('/') {
                                let t = seg.trim();
                                if t.is_empty() || t == "." || t == ".." {
                                    continue;
                                }
                                d.push(t);
                            }
                        }
                        d
                    }
                    Err(_) => effective_out.clone(),
                }
            };

            // 启动预览
            preview_watch_and_serve(
                serve_dir,
                effective_addr,
                effective_input,
                effective_input_url,
                effective_gist_id,
                effective_gist_file,
                effective_github_token,
                effective_auth_scheme,
                effective_out,
                effective_static,
                effective_theme,
                effective_base_path,
                effective_no_intranet,
                effective_generate_intermediate_page,
                effective_color_scheme,
                effective_title,
                effective_desc,
                open,
                build_version,
                effective_icon_dir,
                effective_icon_threads,
            )
        }
    }
}
