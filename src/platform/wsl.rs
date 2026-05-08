//! WSL distro discovery.

use crate::platform::Location;
use anyhow::Result;
use std::path::PathBuf;
use tokio::process::Command;

pub async fn discover() -> Result<Vec<Location>> {
    let distros = list_distros().await.unwrap_or_default();
    let mut out = Vec::new();
    for distro in distros {
        if let Some(user) = wsl_user(&distro).await {
            let home = PathBuf::from(format!(r"\\wsl$\{distro}\home\{user}"));
            let config_dir = home.join(".claude");
            if config_dir.exists() || home.join(".claude.json").exists() {
                out.push(Location::Wsl {
                    distro,
                    config_dir,
                    home_dir: home,
                });
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
