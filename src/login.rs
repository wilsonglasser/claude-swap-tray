//! Orchestrate `claude login` on the Windows host and replicate the
//! resulting credentials to every detected install location.
//!
//! Design choice: `claude login` always runs on Windows native (where the
//! browser flow + localhost callback are most reliable). WSL distros never
//! get their own login — we just write the credentials file into each
//! distro's `\\wsl$\<distro>\home\<user>\.claude\` from the Windows side.

use crate::account::{Account, ClaudeGlobalConfig, CredentialsFile, OAuthCredentials};
use crate::platform::Location;
use crate::store::Store;
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::process::Command;

const LOGIN_TIMEOUT_SECS: u64 = 300;
const POLL_INTERVAL_MS: u64 = 500;

/// Result of an add-account attempt: which Account was captured plus a
/// per-location replication outcome (so the UI can show "✓ synced to
/// Ubuntu / ✗ Debian failed: <reason>").
pub struct AddOutcome {
    pub account: Account,
    pub replications: Vec<(String, Result<()>)>,
}

/// Run `claude login` on the Windows install, capture the new credentials,
/// persist them, and replicate to every WSL location detected.
pub async fn add_account_and_sync(locations: Vec<Location>) -> Result<AddOutcome> {
    let windows_loc = locations
        .iter()
        .find(|l| matches!(l, Location::Windows { .. }))
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no Windows Claude Code installation detected — install Claude Code on Windows first"
            )
        })?;

    let (account, creds) = login_on_windows(&windows_loc).await?;

    let store = Store::open()?;
    store.save_account(&account, &creds)?;

    let wsl_locations: Vec<Location> = locations
        .into_iter()
        .filter(|l| matches!(l, Location::Wsl { .. }))
        .collect();

    let mut replications = Vec::with_capacity(wsl_locations.len());
    for loc in &wsl_locations {
        let label = loc.label();
        let res = write_credentials(&creds, loc).await;
        replications.push((label, res));
    }

    Ok(AddOutcome {
        account,
        replications,
    })
}

async fn login_on_windows(location: &Location) -> Result<(Account, OAuthCredentials)> {
    let creds_path = location.credentials_path();
    let before = read_file_bytes(&creds_path).await.ok();

    let mut child = spawn_login_windows()?;

    let deadline = Instant::now() + Duration::from_secs(LOGIN_TIMEOUT_SECS);
    let new_bytes = loop {
        if Instant::now() > deadline {
            let _ = child.kill().await;
            anyhow::bail!("`claude login` timed out after {LOGIN_TIMEOUT_SECS}s");
        }

        if let Ok(now) = read_file_bytes(&creds_path).await {
            if Some(&now) != before.as_ref() && !now.is_empty() {
                let _ = child.wait().await;
                break now;
            }
        }

        if let Ok(Some(status)) = child.try_wait() {
            match read_file_bytes(&creds_path).await {
                Ok(now) if Some(&now) != before.as_ref() && !now.is_empty() => break now,
                _ => anyhow::bail!(
                    "`claude login` exited (code {:?}) without writing credentials",
                    status.code()
                ),
            }
        }

        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    };

    let creds = parse_oauth_credentials(&new_bytes)?;
    let meta = read_account_metadata(location).await.unwrap_or_default();

    let store = Store::open()?;
    let slot = store.next_slot()?;
    let account = Account {
        slot,
        email: meta.email_address,
        uuid: meta.account_uuid,
        organization_uuid: meta.organization_uuid,
        organization_name: meta.organization_name,
        added_at: Utc::now(),
    };
    Ok((account, creds))
}

fn spawn_login_windows() -> Result<tokio::process::Child> {
    Command::new("claude")
        .arg("login")
        .spawn()
        .context("failed to spawn `claude login` on Windows host")
}

async fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
    Ok(tokio::fs::read(path).await?)
}

fn parse_oauth_credentials(bytes: &[u8]) -> Result<OAuthCredentials> {
    let parsed: CredentialsFile =
        serde_json::from_slice(bytes).context("parse .credentials.json")?;
    Ok(parsed.claude_ai_oauth)
}

async fn read_account_metadata(location: &Location) -> Result<crate::account::OAuthAccount> {
    let path = location.global_config_path();
    let raw = tokio::fs::read(&path)
        .await
        .with_context(|| format!("read {}", path.display()))?;
    let parsed: ClaudeGlobalConfig = serde_json::from_slice(&raw).context("parse .claude.json")?;
    Ok(parsed.oauth_account)
}

/// Write the given OAuth credentials into the `.credentials.json` of every
/// target location (atomic temp+rename). Used after a switch to propagate
/// the active account across Windows + WSL installs.
pub async fn replicate_credentials_to_locations(
    creds: &OAuthCredentials,
    locations: &[Location],
) -> Vec<(Location, Result<()>)> {
    let mut out = Vec::with_capacity(locations.len());
    for loc in locations {
        let res = write_credentials(creds, loc).await;
        out.push((loc.clone(), res));
    }
    out
}

async fn write_credentials(creds: &OAuthCredentials, location: &Location) -> Result<()> {
    let payload = CredentialsFile {
        claude_ai_oauth: creds.clone(),
    };
    let json = serde_json::to_vec_pretty(&payload)?;
    let target = location.credentials_path();
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let tmp = target.with_extension("json.tmp");
    tokio::fs::write(&tmp, &json)
        .await
        .with_context(|| format!("write {}", tmp.display()))?;
    tokio::fs::rename(&tmp, &target)
        .await
        .with_context(|| format!("rename to {}", target.display()))?;
    Ok(())
}
