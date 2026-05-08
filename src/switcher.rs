//! High-level account operations: add/remove/list/switch.
//!
//! On switch, writes credentials to **all** detected install locations
//! (Windows native + each WSL distro) so every Claude Code on the host picks
//! up the new account on next start. User must restart Claude Code processes
//! manually — this tool does not hot-reload.

use crate::platform::Location;
use crate::store::Store;
use anyhow::Result;

pub async fn add_account(_slot: Option<u32>) -> Result<()> {
    // TODO: read current `.credentials.json` from each location, dedupe via
    // refresh_token, prompt user to log in if nothing found, save to store.
    println!("add_account: not implemented");
    Ok(())
}

pub async fn list_accounts() -> Result<()> {
    let store = Store::open()?;
    let accounts = store.list()?;
    let active = store.active_slot()?;
    if accounts.is_empty() {
        println!("(no accounts) — log into Claude Code, then run `claude-swap-tray add`");
        return Ok(());
    }
    for acct in accounts {
        let marker = if Some(acct.slot) == active { "*" } else { " " };
        println!("{marker} [{}] {}", acct.slot, acct.display_tag());
    }
    Ok(())
}

pub async fn status() -> Result<()> {
    let store = Store::open()?;
    let active = store.active_slot()?;
    println!("active slot: {active:?}");
    let locations = crate::platform::discover_locations().await?;
    println!("locations:");
    for loc in &locations {
        println!("  - {loc}");
    }
    Ok(())
}

pub async fn switch_next() -> Result<()> {
    println!("switch_next: not implemented");
    Ok(())
}

pub async fn switch_to(_identifier: &str) -> Result<()> {
    println!("switch_to: not implemented");
    Ok(())
}

pub async fn remove_account(_identifier: &str) -> Result<()> {
    println!("remove_account: not implemented");
    Ok(())
}

/// Write the given account's credentials into every install location.
#[allow(dead_code)]
pub async fn apply_credentials_to_all(
    _creds: &crate::account::OAuthCredentials,
    locations: &[Location],
) -> Result<()> {
    for _loc in locations {
        // TODO: serialize OAuth creds in claude-code's expected JSON shape,
        // write to loc.credentials_path() atomically.
    }
    Ok(())
}
