//! Orchestrate `claude login` to add a fresh account.
//!
//! Flow:
//! 1. Snapshot any existing `.credentials.json` at the chosen location.
//! 2. Spawn `claude login` (Windows native) or `wsl -d <distro> -e claude login`.
//! 3. Poll the credentials file every 500ms while the subprocess is alive
//!    (timeout 5 min). When the file changes, parse it.
//! 4. Read account metadata (email, org, uuid) from the OAuth payload.
//! 5. Persist into our store, replicate to other locations on user request.

use crate::account::{Account, OAuthCredentials};
use crate::platform::Location;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;

const LOGIN_TIMEOUT_SECS: u64 = 300;
const POLL_INTERVAL_MS: u64 = 500;

pub async fn add_via_login(location: Location) -> Result<Account> {
    let creds_path = location.credentials_path();
    let before = read_file_bytes(&creds_path).await.ok();

    let mut child = spawn_login(&location)?;

    let deadline = Instant::now() + Duration::from_secs(LOGIN_TIMEOUT_SECS);
    loop {
        if Instant::now() > deadline {
            let _ = child.kill().await;
            anyhow::bail!("`claude login` timed out after {LOGIN_TIMEOUT_SECS}s");
        }

        if let Ok(now) = read_file_bytes(&creds_path).await {
            if Some(&now) != before.as_ref() {
                let acct = parse_account_from_credentials(&now, &location)?;
                let _ = child.wait().await;
                return Ok(acct);
            }
        }

        if let Ok(Some(status)) = child.try_wait() {
            if let Ok(now) = read_file_bytes(&creds_path).await {
                if Some(&now) != before.as_ref() {
                    return parse_account_from_credentials(&now, &location);
                }
            }
            anyhow::bail!(
                "`claude login` exited (code {:?}) without writing credentials",
                status.code()
            );
        }

        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
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

async fn read_file_bytes(path: &PathBuf) -> Result<Vec<u8>> {
    Ok(tokio::fs::read(path).await?)
}

fn parse_account_from_credentials(bytes: &[u8], location: &Location) -> Result<Account> {
    let _ = (bytes, location);
    // TODO: Claude Code's `.credentials.json` shape — extract:
    //   - claudeAiOauth: { accessToken, refreshToken, expiresAt, scopes,
    //                      organizationUuid?, organizationName?, accountUuid? }
    // The exact field names vary by CLI version. Inspect a real file.
    // For org/email metadata, may need a separate API call to /api/oauth/profile.
    anyhow::bail!("parse_account_from_credentials: not implemented — inspect a real .credentials.json first")
}

#[allow(dead_code)]
pub async fn replicate_credentials_to_locations(
    creds: &OAuthCredentials,
    locations: &[Location],
) -> Result<Vec<(Location, Result<()>)>> {
    let mut out = Vec::with_capacity(locations.len());
    for loc in locations {
        let res = write_credentials(creds, loc).await;
        out.push((loc.clone(), res));
    }
    Ok(out)
}

async fn write_credentials(_creds: &OAuthCredentials, _location: &Location) -> Result<()> {
    // TODO: serialize creds in claude-code's expected JSON shape and write
    // atomically (write to .tmp, fsync, rename) at location.credentials_path().
    Ok(())
}
