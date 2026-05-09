//! WSL distro discovery.

use crate::platform::Location;
use anyhow::Result;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub async fn discover() -> Result<Vec<Location>> {
    let distros = match list_distros().await {
        Ok(d) => d,
        Err(e) => {
            warn!(error = ?e, "wsl -l -q failed; assuming no WSL distros");
            return Ok(Vec::new());
        }
    };
    info!(count = distros.len(), "WSL distros enumerated: {distros:?}");

    let mut out = Vec::new();
    for distro in distros {
        match wsl_user(&distro).await {
            Some(user) => {
                let home = PathBuf::from(format!(r"\\wsl$\{distro}\home\{user}"));
                let config_dir = home.join(".claude");
                let claude_json = home.join(".claude.json");
                let cd_exists = config_dir.exists();
                let cj_exists = claude_json.exists();
                debug!(
                    distro = %distro,
                    user = %user,
                    config_dir = %config_dir.display(),
                    config_dir_exists = cd_exists,
                    claude_json_exists = cj_exists,
                    "WSL probe"
                );
                if cd_exists || cj_exists {
                    out.push(Location::Wsl {
                        distro,
                        config_dir,
                        home_dir: home,
                    });
                } else {
                    info!(distro, "WSL distro has no Claude Code install — skipping");
                }
            }
            None => warn!(distro, "wsl whoami failed; skipping distro"),
        }
    }
    info!(count = out.len(), "WSL locations with Claude Code");
    Ok(out)
}

async fn list_distros() -> Result<Vec<String>> {
    let output = wsl_command(&["-l", "-q"]).output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("wsl -l -q exited {}: {}", output.status, err.trim());
    }
    let raw = decode_wsl_output(&output.stdout);
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect())
}

async fn wsl_user(distro: &str) -> Option<String> {
    let output = wsl_command(&["-d", distro, "-e", "whoami"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        warn!(
            distro,
            stderr = %String::from_utf8_lossy(&output.stderr).trim(),
            "wsl whoami failed"
        );
        return None;
    }
    let raw = decode_wsl_output(&output.stdout);
    Some(raw.trim().to_string()).filter(|s| !s.is_empty())
}

/// Build a `wsl.exe` invocation with `WSL_UTF8=1` (forces UTF-8 stdout) and
/// no console flash on Windows.
fn wsl_command(args: &[&str]) -> Command {
    let mut cmd = Command::new("wsl");
    cmd.args(args);
    cmd.env("WSL_UTF8", "1");
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt as _;

/// `wsl.exe` historically emits UTF-16LE without `WSL_UTF8=1`. We set that
/// env var, so output is plain UTF-8. Keep the legacy decoder as a fallback
/// in case an older Windows build ignores the env.
fn decode_wsl_output(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // Fallback: UTF-16LE with optional BOM.
    let body = if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        &bytes[2..]
    } else {
        bytes
    };
    let u16s: Vec<u16> = body
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&u16s)
}
