//! User-configurable settings.

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_QUALIFIER: &str = "com";
const APP_ORG: &str = "wilsonglasser";
const APP_NAME: &str = "claude-swap-tray";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_threshold")]
    pub threshold_percent: f64,
    #[serde(default = "default_poll")]
    pub poll_interval_seconds: u64,
    #[serde(default = "default_true")]
    pub notify_sound: bool,
    #[serde(default)]
    pub auto_rotate: bool,
}

fn default_threshold() -> f64 {
    95.0
}
fn default_poll() -> u64 {
    60
}
fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            threshold_percent: default_threshold(),
            poll_interval_seconds: default_poll(),
            notify_sound: true,
            auto_rotate: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        match path() {
            Ok(p) if p.exists() => fs::read_to_string(&p)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            _ => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let p = path()?;
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let tmp = p.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &p).with_context(|| format!("rename to {}", p.display()))?;
        Ok(())
    }
}

fn path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME)
        .ok_or_else(|| anyhow::anyhow!("could not resolve project data directory"))?;
    Ok(dirs.config_dir().join("settings.json"))
}
