//! High-level account operations across all install locations.

use crate::account::Account;
use crate::login::replicate_credentials_to_locations;
use crate::oauth;
use crate::platform::Location;
use crate::store::Store;
use anyhow::Result;
use tracing::warn;

/// Make `slot` the active account, writing its credentials to every
/// detected location. Refreshes the access token first if it's expired or
/// near expiry.
pub async fn switch_to(slot: u32, locations: &[Location]) -> Result<Account> {
    let store = Store::open()?;
    let account = store
        .account(slot)?
        .ok_or_else(|| anyhow::anyhow!("no account in slot {slot}"))?;
    let mut creds = store.load_credentials(slot)?;

    if oauth::is_expired(&creds) {
        match oauth::refresh(&creds).await {
            Ok(fresh) => {
                store.save_account(&account, &fresh)?;
                creds = fresh;
            }
            Err(e) => warn!(slot, error = ?e, "refresh failed; writing stale token"),
        }
    }

    let results = replicate_credentials_to_locations(&creds, locations).await;
    for (loc, res) in &results {
        if let Err(e) = res {
            warn!(location = %loc, error = ?e, "credential write failed");
        }
    }
    store.set_active_slot(slot)?;
    Ok(account)
}

/// Round-robin to the next account after the active one.
pub async fn switch_next(locations: &[Location]) -> Result<Account> {
    let store = Store::open()?;
    let accounts = store.list()?;
    if accounts.is_empty() {
        anyhow::bail!("no accounts to switch to");
    }
    let active = store.active_slot()?;
    let next_slot = match active {
        None => accounts[0].slot,
        Some(curr) => {
            let idx = accounts.iter().position(|a| a.slot == curr).unwrap_or(0);
            accounts[(idx + 1) % accounts.len()].slot
        }
    };
    switch_to(next_slot, locations).await
}

pub async fn remove(slot: u32) -> Result<()> {
    Store::open()?.delete_account(slot)
}
