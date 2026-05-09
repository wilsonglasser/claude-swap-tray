//! Background usage monitor — Windows-only.
//!
//! Runs as an iced `Subscription`. Each tick: load active account + creds,
//! refresh if expired, query Anthropic usage API, compare worst window
//! against the configured threshold, emit a `MonitorEvent` if crossed.
//! Anti-spam: each (slot, window) tuple is suppressed for 30 minutes after
//! firing.

#![cfg(target_os = "windows")]

use crate::config::Settings;
use crate::oauth;
use crate::store::Store;
use crate::usage;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

const SUPPRESS_SECS: i64 = 30 * 60;

#[derive(Debug, Clone)]
pub enum MonitorEvent {
    /// Usage of the active account crossed the threshold.
    ThresholdCrossed { slot: u32, email: String, pct: f64 },
    /// Usage updated for the active account (quiet refresh of UI state).
    UsageUpdated { slot: u32, pct: f64 },
}

#[derive(Debug, Default)]
struct AlertState {
    /// slot -> last alert epoch s
    alerted_slots: HashMap<u32, i64>,
}

pub struct Monitor {
    settings: Mutex<Settings>,
    state: Arc<Mutex<AlertState>>,
}

impl Monitor {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: Mutex::new(settings),
            state: Arc::new(Mutex::new(AlertState::default())),
        }
    }

    pub fn set_settings(&self, settings: Settings) {
        if let Ok(mut g) = self.settings.try_lock() {
            *g = settings;
        }
    }

    async fn threshold(&self) -> f64 {
        self.settings.lock().await.threshold_percent
    }

    /// Run a single poll cycle. Returns the event to emit, if any.
    pub async fn poll_once(&self) -> Option<MonitorEvent> {
        match self.poll_inner().await {
            Ok(ev) => ev,
            Err(e) => {
                warn!(error = ?e, "monitor poll failed");
                None
            }
        }
    }

    async fn poll_inner(&self) -> Result<Option<MonitorEvent>> {
        let store = Store::open()?;
        let Some(slot) = store.active_slot()? else {
            return Ok(None);
        };
        let Some(account) = store.account(slot)? else {
            return Ok(None);
        };
        let mut creds = store.load_credentials(slot)?;
        if oauth::is_expired(&creds) {
            match oauth::refresh(&creds).await {
                Ok(fresh) => {
                    store.save_account(&account, &fresh)?;
                    creds = fresh;
                }
                Err(e) => {
                    warn!(slot, error = ?e, "monitor: refresh failed");
                    return Ok(None);
                }
            }
        }
        let report = usage::fetch(&creds).await?;
        let pct = report.worst_pct();
        let threshold = self.threshold().await;
        if pct >= threshold {
            let mut st = self.state.lock().await;
            let now = chrono::Utc::now().timestamp();
            let suppressed = st
                .alerted_slots
                .get(&slot)
                .map(|&last| now - last < SUPPRESS_SECS)
                .unwrap_or(false);
            if !suppressed {
                st.alerted_slots.insert(slot, now);
                return Ok(Some(MonitorEvent::ThresholdCrossed {
                    slot,
                    email: account.email,
                    pct,
                }));
            }
        }
        Ok(Some(MonitorEvent::UsageUpdated { slot, pct }))
    }

    pub async fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.settings.lock().await.poll_interval_seconds.max(15))
    }
}
