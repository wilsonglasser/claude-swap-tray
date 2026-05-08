//! claude-swap-tray entry point.
//!
//! Single-binary CLI + tray app. With no subcommand, launches the tray icon
//! (Windows-only). Subcommands work headless on any host that can reach the
//! credential locations.

use anyhow::Result;
use clap::Parser;

mod account;
mod cli;
mod config;
mod oauth;
mod platform;
mod store;
mod switcher;
mod usage;

#[cfg(target_os = "windows")]
mod monitor;
#[cfg(target_os = "windows")]
mod notify;
#[cfg(target_os = "windows")]
mod tray;

fn main() -> Result<()> {
    init_tracing();
    let args = cli::Cli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(cli::dispatch(args))
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
