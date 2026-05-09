//! Anthropic usage API client.
//!
//! `GET https://api.anthropic.com/api/oauth/usage` with bearer access token.
//! Response shape (verified against cswap source): each window object
//! carries a `utilization` percentage (0–100) and a `resets_at` ISO 8601
//! timestamp. The endpoint may omit one or both windows depending on
//! account state; missing windows are treated as 0%.

use crate::account::OAuthCredentials;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageWindow {
    pub pct: f64,
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

fn parse(raw: &serde_json::Value) -> UsageReport {
    UsageReport {
        five_hour: window_from(raw.get("five_hour")),
        seven_day: window_from(raw.get("seven_day")),
    }
}

fn window_from(obj: Option<&serde_json::Value>) -> Option<UsageWindow> {
    let obj = obj?;
    let pct = obj.get("utilization")?.as_f64()?;
    let resets_at = obj
        .get("resets_at")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    Some(UsageWindow { pct, resets_at })
}
