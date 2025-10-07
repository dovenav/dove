// 程序入口：保持精简，仅负责解析 CLI 和分发命令。
// 其余逻辑均拆分到独立模块，便于维护与测试。

mod build;
mod cli;
mod commands;
mod config;
mod icons;
mod init;
mod preview;
mod utils;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    commands::run(cli)
}
