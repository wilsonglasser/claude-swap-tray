//! claude-swap-tray entry point — iced GUI app.

use anyhow::Result;

mod account;
mod app;
mod config;
mod login;
mod oauth;
mod platform;
mod screens;
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
    app::run()?;
    Ok(())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
