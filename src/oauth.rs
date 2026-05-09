//! OAuth refresh flow against Anthropic's token endpoint.
//!
//! Mirrors cswap's `oauth.py::refresh_oauth_credentials`. Refresh URL,
//! client ID, and request shape are taken from there (Claude Code uses
//! the same).

use crate::account::OAuthCredentials;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

const OAUTH_TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const USER_AGENT: &str = concat!("claude-swap-tray/", env!("CARGO_PKG_VERSION"));
const EXPIRY_BUFFER_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    grant_type: &'a str,
    refresh_token: &'a str,
    client_id: &'a str,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    expires_in: i64,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

pub fn is_expired(creds: &OAuthCredentials) -> bool {
    let now_ms = Utc::now().timestamp_millis();
    creds.expires_at <= now_ms + EXPIRY_BUFFER_MS
}

pub async fn refresh(creds: &OAuthCredentials) -> Result<OAuthCredentials> {
    if creds.refresh_token.is_empty() {
        anyhow::bail!("no refresh token on credentials");
    }
    let body = RefreshRequest {
        grant_type: "refresh_token",
        refresh_token: &creds.refresh_token,
        client_id: OAUTH_CLIENT_ID,
    };
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client
        .post(OAUTH_TOKEN_URL)
        .json(&body)
        .send()
        .await
        .context("oauth refresh request failed")?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("oauth refresh returned {status}: {text}");
    }
    let parsed: RefreshResponse = resp.json().await.context("oauth refresh response parse")?;

    let now_ms = Utc::now().timestamp_millis();
    let scopes = parsed
        .scope
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_else(|| creds.scopes.clone());
    Ok(OAuthCredentials {
        access_token: parsed.access_token,
        refresh_token: parsed
            .refresh_token
            .unwrap_or_else(|| creds.refresh_token.clone()),
        expires_at: now_ms + parsed.expires_in.saturating_mul(1000),
        scopes,
        extra: creds.extra.clone(),
    })
}
