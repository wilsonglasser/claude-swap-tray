//! Account model + serde shapes that match Claude Code's on-disk JSON.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Public account record stored in our manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub slot: u32,
    pub email: String,
    pub uuid: String,
    #[serde(default)]
    pub organization_uuid: String,
    #[serde(default)]
    pub organization_name: String,
    pub added_at: DateTime<Utc>,
}

impl Account {
    pub fn display_tag(&self) -> String {
        if self.organization_name.is_empty() {
            self.email.clone()
        } else {
            format!("{} ({})", self.email, self.organization_name)
        }
    }
}

/// OAuth credentials stored in the OS credential store.
///
/// Mirrors the `claudeAiOauth` block of Claude Code's `.credentials.json`,
/// in camelCase for round-trip compatibility when we write the file back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: String,
    /// Epoch milliseconds when access_token expires.
    pub expires_at: i64,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl OAuthCredentials {
    pub fn expires_at_utc(&self) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(self.expires_at)
            .single()
            .unwrap_or_else(Utc::now)
    }
}

/// Top-level shape of `.credentials.json` written by Claude Code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: OAuthCredentials,
}

/// Subset of `~/.claude.json` we care about — the `oauthAccount` block.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthAccount {
    #[serde(default)]
    pub email_address: String,
    #[serde(default)]
    pub account_uuid: String,
    #[serde(default)]
    pub organization_uuid: String,
    #[serde(default)]
    pub organization_name: String,
}

/// Top-level shape of `~/.claude.json` we read for account metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeGlobalConfig {
    #[serde(default)]
    pub oauth_account: OAuthAccount,
}
