//! CLI 定义模块：仅负责命令行参数结构体与解析
//! 将 clap 的声明与业务逻辑解耦，便于在其它模块中复用参数。

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// 顶层 CLI 入口
#[derive(Parser, Debug)]
#[command(name = "dove", about = "静态导航站生成器", version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// 子命令定义
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// 生成静态站点
    Build {
        /// 配置文件路径，默认：dove.yaml / dove.yml
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// 配置文件 URL，支持 http/https（可用于 Gist raw 链接）
        #[arg(long, value_name = "URL")]
        input_url: Option<String>,
        /// 从 Gist 加载配置：Gist ID（与 --input-url 二选一，存在时忽略本地 input）
        #[cfg(feature = "remote")]
        #[arg(long, value_name = "ID")]
        gist_id: Option<String>,
        /// 从 Gist 加载配置：文件名（可选，不填则取第一个文件）
        #[cfg(feature = "remote")]
        #[arg(long, value_name = "NAME")]
        gist_file: Option<String>,
        /// 访问私有 Gist 或需要授权的 URL 的 token
        #[cfg(feature = "remote")]
        #[arg(long, value_name = "TOKEN")]
        github_token: Option<String>,
        /// 授权方案（默认 token，可设为 Bearer 等）
        #[cfg(feature = "remote")]
        #[arg(long, value_name = "SCHEME")]
        auth_scheme: Option<String>,
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
        /// 仅生成外网页面（不生成 intranet/）
        #[arg(long)]
        no_intranet: bool,
        /// 覆盖页面配色方案（auto|light|dark）
        #[arg(long, value_name = "SCHEME")]
        color_scheme: Option<String>,
        /// 覆盖站点标题（不修改配置文件）
        #[arg(long, value_name = "TITLE")]
        title: Option<String>,
        /// 覆盖站点描述（不修改配置文件）
        #[arg(long, value_name = "DESC")]
        description: Option<String>,
        /// 构建版本号（优先于环境变量 DOVE_BUILD_VERSION）
        #[arg(long, value_name = "VER")]
        build_version: Option<String>,
        /// 下载的图标保存目录（相对站点根）。默认 assets/icons
        #[arg(long, value_name = "DIR")]
        icon_dir: Option<String>,
        /// 图标下载并发数。默认 8
        #[arg(long, value_name = "N")]
        icon_threads: Option<usize>,
        /// 是否生成中间页（默认生成）。如果设置为 false，则链接直接跳转目标地址
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        generate_intermediate_page: bool,
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
    /// 预览生成结果（本地静态文件服务）
    Preview {
        /// 指定服务目录（优先于根据配置推导的 dist/<base_path>）
        #[arg(long, value_name = "DIR")]
        dir: Option<PathBuf>,
        /// 监听地址，默认 127.0.0.1:8787
        #[arg(long, value_name = "ADDR")]
        addr: Option<String>,
        /// 启动前触发一次构建
        #[arg(long)]
        build_first: bool,
        /// 以下参数用于可选构建（与 build 子命令相同）
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(long, value_name = "URL")]
        input_url: Option<String>,
        /// 从 Gist 加载配置：Gist ID（与 --input-url 二选一，存在时忽略本地 input）
        #[cfg(feature = "remote")]
        #[arg(long, value_name = "ID")]
        gist_id: Option<String>,
        /// 从 Gist 加载配置：文件名（可选，不填则取第一个文件）
        #[cfg(feature = "remote")]
        #[arg(long, value_name = "NAME")]
        gist_file: Option<String>,
        /// 访问私有 Gist 或需要授权的 URL 的 token
        #[cfg(feature = "remote")]
        #[arg(long, value_name = "TOKEN")]
        github_token: Option<String>,
        /// 授权方案（默认 token，可设为 Bearer 等）
        #[cfg(feature = "remote")]
        #[arg(long, value_name = "SCHEME")]
        auth_scheme: Option<String>,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        static_dir: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        theme: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        base_path: Option<String>,
        #[arg(long)]
        no_intranet: bool,
        /// 启动后自动在浏览器打开
        #[arg(long)]
        open: bool,
        /// 覆盖页面配色方案（auto|light|dark）
        #[arg(long, value_name = "SCHEME")]
        color_scheme: Option<String>,
        /// 覆盖站点标题（不修改配置文件）
        #[arg(long, value_name = "TITLE")]
        title: Option<String>,
        /// 覆盖站点描述（不修改配置文件）
        #[arg(long, value_name = "DESC")]
        description: Option<String>,
        /// 构建版本号（优先于环境变量 DOVE_BUILD_VERSION）
        #[arg(long, value_name = "VER")]
        build_version: Option<String>,
        /// 下载的图标保存目录（相对站点根）。默认 assets/icons
        #[arg(long, value_name = "DIR")]
        icon_dir: Option<String>,
        /// 图标下载并发数。默认 8
        #[arg(long, value_name = "N")]
        icon_threads: Option<usize>,
        /// 是否生成中间页（默认生成）。如果设置为 false，则链接直接跳转目标地址
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        generate_intermediate_page: bool,
    },
}
