//! Native Windows install discovery.
//!
//! Resolves `CLAUDE_CONFIG_DIR` env or falls back to `%USERPROFILE%\.claude`.

use crate::platform::Location;
use anyhow::Result;
use std::env;
use std::path::PathBuf;

pub fn discover() -> Result<Option<Location>> {
    let dir = if let Ok(env_dir) = env::var("CLAUDE_CONFIG_DIR") {
        PathBuf::from(env_dir)
    } else {
        let home = directories::UserDirs::new()
            .map(|u| u.home_dir().to_path_buf())
            .ok_or_else(|| anyhow::anyhow!("could not resolve user home"))?;
        home.join(".claude")
    };
    Ok(Some(Location::Windows { config_dir: dir }))
}
