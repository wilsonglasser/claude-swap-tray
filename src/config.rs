//! User-configurable settings — threshold, poll interval, sound on/off.
//!
//! Persisted as JSON in the cswap data dir.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub threshold_percent: f64,
    pub poll_interval_seconds: u64,
    pub notify_sound: bool,
    pub auto_rotate: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            threshold_percent: 95.0,
            poll_interval_seconds: 60,
            notify_sound: true,
            auto_rotate: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        // TODO: read from data_dir/settings.json, fallback to default
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        // TODO
        Ok(())
    }
}
