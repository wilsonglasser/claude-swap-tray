//! Windows toast notification wrapper.
//!
//! Uses `tauri-winrt-notification` for native Windows ToastNotificationManager.
//! Toasts support action buttons that raise an event our tray event loop
//! catches via the activation handler.

#![cfg(target_os = "windows")]

use anyhow::Result;
use tracing::info;

const APP_ID: &str = "com.wilsonglasser.claude-swap-tray";

pub async fn show_threshold_alert(slot: u32, pct: f64) -> Result<()> {
    info!(slot, pct, "showing threshold toast");
    // TODO: build Toast with title, message, two action buttons
    // ("Switch now" -> emit tray event, "Snooze 30m"), sound, app icon.
    // Register activation handler to receive button click.
    let _ = (APP_ID, slot, pct);
    Ok(())
}

pub async fn show_swap_done(from_email: &str, to_email: &str) -> Result<()> {
    info!(from = from_email, to = to_email, "showing swap-done toast");
    // TODO: confirmation toast with instruction "Close + reopen Claude Code,
    // then `claude --resume`".
    Ok(())
}
