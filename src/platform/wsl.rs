//! WSL distro discovery.
//!
//! Strategy:
//! 1. Run `wsl -l -q` to enumerate distros (one name per line; UTF-16LE on
//!    older Windows builds — needs decoding).
//! 2. For each distro, resolve the Linux user via `wsl -d <name> -e whoami`.
//! 3. Build UNC path `\\wsl$\<distro>\home\<user>\.claude` (or `.localhost`
//!    on newer Windows).
//! 4. Probe existence of the directory; skip distros where Claude Code is
//!    not installed.

use crate::platform::Location;
use anyhow::Result;
use std::path::PathBuf;
use tokio::process::Command;

pub async fn discover() -> Result<Vec<Location>> {
    let distros = list_distros().await.unwrap_or_default();
    let mut out = Vec::new();
    for distro in distros {
        if let Some(user) = wsl_user(&distro).await {
            let unc = PathBuf::from(format!(r"\\wsl$\{distro}\home\{user}\.claude"));
            if unc.exists() {
                out.push(Location::Wsl { distro, config_dir: unc });
            }
        }
    }
    Ok(out)
}

async fn list_distros() -> Result<Vec<String>> {
    let output = Command::new("wsl").args(["-l", "-q"]).output().await?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    // Older `wsl.exe` emits UTF-16LE; newer ones honor `WSL_UTF8=1`.
    let raw = decode_wsl_output(&output.stdout);
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect())
}

async fn wsl_user(distro: &str) -> Option<String> {
    let output = Command::new("wsl")
        .args(["-d", distro, "-e", "whoami"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = decode_wsl_output(&output.stdout);
    Some(raw.trim().to_string()).filter(|s| !s.is_empty())
}

fn decode_wsl_output(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else if bytes.iter().step_by(2).skip(1).take(8).all(|&b| b == 0) {
        let u16s: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}
