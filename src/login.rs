//! Orchestrate `claude login` to add a fresh account.
//!
//! Flow:
//! 1. Snapshot any existing `.credentials.json` at the chosen location.
//! 2. Spawn `claude login` (Windows native) or `wsl -d <distro> -e claude login`.
//! 3. Poll the credentials file every 500ms while the subprocess is alive
//!    (timeout 5 min). When the file changes, parse it.
//! 4. Read account metadata (email, org) from the same location's
//!    `.claude.json` (or legacy `.config.json`).
//! 5. Persist to our `Store` — keyring + manifest.

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

pub async fn add_via_login(location: Location) -> Result<Account> {
    let creds_path = location.credentials_path();
    let before = read_file_bytes(&creds_path).await.ok();

    let mut child = spawn_login(&location)?;

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
    let meta = read_account_metadata(&location).await.unwrap_or_default();

    let store = Store::open()?;
    let slot = store.next_slot()?;
    let account = Account {
        slot,
        email: meta.email_address.clone(),
        uuid: meta.account_uuid.clone(),
        organization_uuid: meta.organization_uuid.clone(),
        organization_name: meta.organization_name.clone(),
        added_at: Utc::now(),
    };
    store.save_account(&account, &creds)?;
    Ok(account)
}

fn spawn_login(location: &Location) -> Result<tokio::process::Child> {
    let mut cmd = match location {
        Location::Windows { .. } => {
            let mut c = Command::new("claude");
            c.arg("login");
            c
        }
        Location::Wsl { distro, .. } => {
            let mut c = Command::new("wsl");
            c.args(["-d", distro, "-e", "claude", "login"]);
            c
        }
    };
    cmd.spawn()
        .with_context(|| format!("failed to spawn `claude login` for {location}"))
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
    let parsed: ClaudeGlobalConfig =
        serde_json::from_slice(&raw).context("parse .claude.json")?;
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
    let payload = CredentialsFile { claude_ai_oauth: creds.clone() };
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
