//! claude-swap-tray entry point — iced GUI app.

// Hide the console window on Windows release builds. Debug builds keep
// the console attached so `cargo run` shows tracing output inline.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use std::fs::OpenOptions;
use std::io;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

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
    #[cfg(target_os = "windows")]
    tray::spawn();
    app::run()?;
    Ok(())
}

fn init_tracing() {
    // Default filter: our crate at info, third-party libs at warn.
    // Override via `RUST_LOG=...`.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "warn,claude_swap_tray=info,iced=warn,iced_wgpu=warn,wgpu_core=warn,wgpu_hal=warn,naga=warn",
        )
    });

    // In release on Windows there's no console — write logs to a file.
    // In debug or non-Windows, write to stderr (visible in `cargo run`).
    let writer = log_writer();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();
}

#[cfg(all(target_os = "windows", not(debug_assertions)))]
fn log_writer() -> BoxMakeWriter {
    use directories::ProjectDirs;
    let path = ProjectDirs::from("com", "wilsonglasser", "claude-swap-tray").map(|d| {
        let dir = d.data_local_dir().to_path_buf();
        std::fs::create_dir_all(&dir).ok();
        dir.join("claude-swap-tray.log")
    });
    if let Some(p) = path {
        if let Ok(file) = OpenOptions::new().create(true).append(true).open(&p) {
            return BoxMakeWriter::new(std::sync::Mutex::new(file));
        }
    }
    BoxMakeWriter::new(io::sink)
}

#[cfg(not(all(target_os = "windows", not(debug_assertions))))]
fn log_writer() -> BoxMakeWriter {
    BoxMakeWriter::new(io::stderr)
}
