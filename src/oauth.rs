//! OAuth token handling — decode JWT-ish access tokens, refresh expired ones.
//!
//! Mirrors the relevant parts of cswap's `oauth.py`. The Anthropic OAuth
//! refresh endpoint is undocumented but stable (used by Claude Code itself).

use crate::account::OAuthCredentials;
use anyhow::Result;
use chrono::Utc;

const REFRESH_URL: &str = "https://console.anthropic.com/v1/oauth/token";

pub fn is_expired(creds: &OAuthCredentials) -> bool {
    creds.expires_at <= Utc::now()
}

pub async fn refresh(_creds: &OAuthCredentials) -> Result<OAuthCredentials> {
    // TODO: POST to REFRESH_URL with refresh_token, parse new access_token + expires_at
    let _ = REFRESH_URL;
    anyhow::bail!("not implemented")
}
