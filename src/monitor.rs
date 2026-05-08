//! Background usage monitor — Windows-only.
//!
//! Polls Anthropic's usage API for the active account at a configurable
//! interval. When usage crosses the threshold, fires a toast notification
//! with a "Switch now" action button. Anti-spam: each threshold-crossed
//! account is suppressed until either the window resets or a swap occurs.

#![cfg(target_os = "windows")]

use crate::config::Settings;
use crate::store::Store;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone, Default)]
struct AlertState {
    /// slot -> last time (epoch s) we already alerted for this account
    alerted_slots: HashMap<u32, i64>,
}

pub struct Monitor {
    settings: Settings,
    state: Arc<Mutex<AlertState>>,
}

impl Monitor {
    pub fn new(settings: Settings) -> Self {
        Self { settings, state: Arc::new(Mutex::new(AlertState::default())) }
    }

    pub async fn run(&self) -> Result<()> {
        let interval = Duration::from_secs(self.settings.poll_interval_seconds.max(10));
        info!(interval_s = self.settings.poll_interval_seconds, threshold = self.settings.threshold_percent, "monitor started");
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            if let Err(e) = self.tick().await {
                warn!(error = ?e, "monitor tick failed");
            }
        }
    }

    async fn tick(&self) -> Result<()> {
        let store = Store::open()?;
        let Some(active) = store.active_slot()? else { return Ok(()) };
        let creds = store.load_credentials(active)?;
        // TODO: refresh creds if expired before fetching usage
        let usage = crate::usage::fetch(&creds.access_token).await?;
        let pct = usage.worst_pct();
        if pct >= self.settings.threshold_percent {
            self.maybe_alert(active, pct).await?;
        }
        Ok(())
    }

    async fn maybe_alert(&self, slot: u32, pct: f64) -> Result<()> {
        let mut state = self.state.lock().await;
        let now = chrono::Utc::now().timestamp();
        // Suppress same-slot re-alerts within 30 min.
        if let Some(last) = state.alerted_slots.get(&slot) {
            if now - last < 30 * 60 {
                return Ok(());
            }
        }
        state.alerted_slots.insert(slot, now);
        drop(state);
        crate::notify::show_threshold_alert(slot, pct).await?;
        Ok(())
    }
}
