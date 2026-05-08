//! Account model — a logical Claude Code identity (email + OAuth tokens).
//!
//! Tokens (sensitive) live in the OS credential store; non-sensitive metadata
//! (email, org, slot number) lives in a JSON manifest. See [`store`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub slot: u32,
    pub email: String,
    pub uuid: String,
    pub organization_uuid: String,
    pub organization_name: String,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
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
