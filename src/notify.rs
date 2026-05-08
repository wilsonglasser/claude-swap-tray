//! Windows toast notifications via tauri-winrt-notification.
//!
//! AppId convention: any installed Windows app's AppUserModelID works as
//! the toast source. We register under the PowerShell AUMID for now so
//! toasts appear without an MSIX install. Once we ship MSIX/winget, we
//! switch to our own AppId.

#![cfg(target_os = "windows")]

use anyhow::{Context, Result};
use tauri_winrt_notification::{Sound, Toast};
use tracing::info;

const APP_ID: &str = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";

pub fn show_threshold_alert(slot: u32, email: &str, pct: f64, with_sound: bool) -> Result<()> {
    info!(slot, email, pct, "showing threshold toast");
    let mut toast = Toast::new(APP_ID)
        .title("Claude usage alert")
        .text1(&format!("{email} is at {pct:.0}% of its limit"))
        .text2("Switch to another account to keep working.");
    if with_sound {
        toast = toast.sound(Some(Sound::Reminder));
    } else {
        toast = toast.sound(None);
    }
    toast.show().context("toast show failed")?;
    Ok(())
}

pub fn show_swap_done(from_email: &str, to_email: &str) -> Result<()> {
    info!(from = from_email, to = to_email, "showing swap-done toast");
    Toast::new(APP_ID)
        .title("Claude account switched")
        .text1(&format!("{from_email} → {to_email}"))
        .text2("Close + reopen any running Claude Code so the new token takes effect.")
        .sound(Some(Sound::Default))
        .show()
        .context("toast show failed")?;
    Ok(())
}
