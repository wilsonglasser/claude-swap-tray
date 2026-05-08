//! System tray — Windows-only.
//!
//! Single-instance: a named Win32 mutex guards against double-launch. The
//! event loop runs on the main thread (required by tray-icon/tao); the
//! background monitor runs on a tokio task.

#![cfg(target_os = "windows")]

use crate::config::Settings;
use crate::monitor::Monitor;
use anyhow::Result;
use single_instance::SingleInstance;
use tracing::info;

const INSTANCE_KEY: &str = "claude-swap-tray-singleton";

pub async fn run() -> Result<()> {
    let instance = SingleInstance::new(INSTANCE_KEY)?;
    if !instance.is_single() {
        anyhow::bail!("claude-swap-tray is already running — `stop` first");
    }
    let settings = Settings::load();

    // Spawn monitor.
    let monitor = Monitor::new(settings.clone());
    tokio::spawn(async move {
        if let Err(e) = monitor.run().await {
            tracing::error!(error = ?e, "monitor crashed");
        }
    });

    info!("tray starting — menu: usage, switch, close");
    // TODO: tray-icon + tao event loop.
    //   - icon from assets/icon.ico (TODO add asset)
    //   - menu items: header (current account + worst pct), separator,
    //     "Switch to next", submenu of accounts, separator, "Settings…",
    //     "Quit"
    //   - tooltip updated each tick from monitor state (use a shared Mutex)
    //   - left-click: open quick switch popup
    //   - handle toast button activations (winrt-toast register_activator)

    // Placeholder block so command stays in foreground.
    tokio::signal::ctrl_c().await?;
    Ok(())
}

pub fn stop() -> Result<()> {
    // TODO: send WM_CLOSE / custom message to running instance window so it
    // exits cleanly. For now: tell the user.
    println!("not implemented — close from the tray icon menu, or kill the process");
    Ok(())
}
