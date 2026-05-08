//! Install location discovery — Windows native + every detected WSL distro.

use anyhow::Result;
use std::fmt;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod wsl;

#[derive(Debug, Clone)]
pub enum Location {
    /// Native Windows install — `C:\Users\<user>\.claude\`.
    Windows { config_dir: PathBuf },
    /// A WSL distro — `\\wsl$\<distro>\home\<user>\.claude\`.
    Wsl { distro: String, config_dir: PathBuf },
}

impl Location {
    pub fn credentials_path(&self) -> PathBuf {
        let dir = match self {
            Location::Windows { config_dir } => config_dir,
            Location::Wsl { config_dir, .. } => config_dir,
        };
        dir.join(".credentials.json")
    }

    pub fn label(&self) -> String {
        match self {
            Location::Windows { .. } => "Windows".to_string(),
            Location::Wsl { distro, .. } => format!("WSL: {distro}"),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} → {}", self.label(), self.credentials_path().display())
    }
}

#[cfg(target_os = "windows")]
pub async fn discover_locations() -> Result<Vec<Location>> {
    let mut out = Vec::new();
    if let Some(loc) = windows::discover()? {
        out.push(loc);
    }
    out.extend(wsl::discover().await?);
    Ok(out)
}

#[cfg(not(target_os = "windows"))]
pub async fn discover_locations() -> Result<Vec<Location>> {
    // Non-Windows builds exist only for `cargo check` convenience.
    // Discovery requires `\\wsl$\` UNC access which is Windows-only.
    Ok(Vec::new())
}
