//! Anthropic usage API client.
//!
//! `GET https://api.anthropic.com/api/oauth/usage` with bearer access token.
//! Response carries 5-hour and 7-day windows with used/limit + reset time.
//! Field shape is undocumented; we deserialize loosely and tolerate missing
//! windows.

use crate::account::OAuthCredentials;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageWindow {
    pub pct: f64,
    pub used: u64,
    pub limit: u64,
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageReport {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
}

impl UsageReport {
    pub fn worst_pct(&self) -> f64 {
        let h5 = self.five_hour.as_ref().map(|w| w.pct).unwrap_or(0.0);
        let d7 = self.seven_day.as_ref().map(|w| w.pct).unwrap_or(0.0);
        h5.max(d7)
    }
}

pub async fn fetch(creds: &OAuthCredentials) -> Result<UsageReport> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client
        .get(USAGE_URL)
        .bearer_auth(&creds.access_token)
        .send()
        .await
        .context("usage api request failed")?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("usage api returned {status}: {text}");
    }
    let raw: serde_json::Value = resp.json().await.context("usage api json parse failed")?;
    Ok(parse(&raw))
}

/// Parse the usage payload. The endpoint shape varies; we look for any
/// nested object with `used`/`limit`/`resets_at` keys under common
/// container names (`five_hour_window`, `5h`, `seven_day_window`, `7d`).
fn parse(raw: &serde_json::Value) -> UsageReport {
    UsageReport {
        five_hour: extract_window(raw, &["five_hour", "five_hour_window", "5h", "fiveHour"]),
        seven_day: extract_window(raw, &["seven_day", "seven_day_window", "7d", "sevenDay"]),
    }
}

fn extract_window(root: &serde_json::Value, keys: &[&str]) -> Option<UsageWindow> {
    for k in keys {
        if let Some(obj) = root.get(*k) {
            if let Some(w) = window_from_obj(obj) {
                return Some(w);
            }
        }
    }
    None
}

fn window_from_obj(obj: &serde_json::Value) -> Option<UsageWindow> {
    let used = obj.get("used").or_else(|| obj.get("utilization_used"))?.as_u64()?;
    let limit = obj.get("limit").or_else(|| obj.get("utilization_limit"))?.as_u64()?;
    if limit == 0 {
        return None;
    }
    let pct = (used as f64 / limit as f64) * 100.0;
    let resets_at = obj
        .get("resets_at")
        .or_else(|| obj.get("reset_at"))
        .or_else(|| obj.get("resetsAt"))
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    Some(UsageWindow { pct, used, limit, resets_at })
}
