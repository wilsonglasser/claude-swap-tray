//! Anthropic usage API client.
//!
//! Endpoint: `GET https://api.anthropic.com/api/oauth/usage` with the OAuth
//! access token in `Authorization: Bearer ...`. Returns 5h + 7d windows with
//! used/limit and reset timestamps.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageWindow {
    pub pct: f64,
    pub used: u64,
    pub limit: u64,
    pub resets_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
}

impl UsageReport {
    /// Return the highest usage % across all windows, used for threshold checks.
    pub fn worst_pct(&self) -> f64 {
        let h5 = self.five_hour.as_ref().map(|w| w.pct).unwrap_or(0.0);
        let d7 = self.seven_day.as_ref().map(|w| w.pct).unwrap_or(0.0);
        h5.max(d7)
    }
}

pub async fn fetch(access_token: &str) -> Result<UsageReport> {
    let client = reqwest::Client::new();
    let resp = client
        .get(USAGE_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .context("usage api request failed")?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("usage api returned {status}");
    }
    let raw: serde_json::Value = resp.json().await.context("usage api json parse failed")?;
    parse(raw)
}

fn parse(raw: serde_json::Value) -> Result<UsageReport> {
    // TODO: shape may differ per response; verify against real fixture and
    // implement strict deserialization.
    let _ = raw;
    Ok(UsageReport { five_hour: None, seven_day: None })
}
