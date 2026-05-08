//! Install location discovery — Windows native + every detected WSL distro.

use anyhow::Result;
use std::fmt;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod wsl;

#[derive(Debug, Clone)]
pub enum Location {
    /// Native Windows install.
    Windows {
        /// Where `.credentials.json` lives — typically `%USERPROFILE%\.claude\`.
        config_dir: PathBuf,
        /// Home dir — `%USERPROFILE%`. `.claude.json` sits here.
        home_dir: PathBuf,
    },
    /// A WSL distro. Paths are UNC paths on the Windows side
    /// (`\\wsl$\<distro>\home\<user>\...`).
    Wsl {
        distro: String,
        config_dir: PathBuf,
        home_dir: PathBuf,
    },
}

impl Location {
    pub fn config_dir(&self) -> &Path {
        match self {
            Location::Windows { config_dir, .. } | Location::Wsl { config_dir, .. } => config_dir,
        }
    }

    pub fn home_dir(&self) -> &Path {
        match self {
            Location::Windows { home_dir, .. } | Location::Wsl { home_dir, .. } => home_dir,
        }
    }

    /// `.credentials.json` — sits inside `<config_dir>`.
    pub fn credentials_path(&self) -> PathBuf {
        self.config_dir().join(".credentials.json")
    }

    /// `.claude.json` (or legacy `<config_dir>/.config.json`) — holds
    /// `oauthAccount` with email + organization metadata. Resolution:
    /// legacy `<config_dir>/.config.json` if it exists, else
    /// `<home_dir>/.claude.json`.
    pub fn global_config_path(&self) -> PathBuf {
        let legacy = self.config_dir().join(".config.json");
        if legacy.exists() {
            return legacy;
        }
        self.home_dir().join(".claude.json")
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
    Ok(Vec::new())
}
