//! Native Windows install discovery.

use crate::platform::Location;
use anyhow::Result;
use std::env;
use std::path::PathBuf;

pub fn discover() -> Result<Option<Location>> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("could not resolve user home"))?;
    let config_dir = if let Ok(env_dir) = env::var("CLAUDE_CONFIG_DIR") {
        PathBuf::from(env_dir)
    } else {
        home.join(".claude")
    };
    Ok(Some(Location::Windows {
        config_dir,
        home_dir: home,
    }))
}
