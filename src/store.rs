//! Account persistence.
//!
//! Sensitive tokens go into the OS credential store via the `keyring` crate
//! (Windows Credential Manager backend). Non-sensitive metadata (slot, email,
//! org info, sequence state) goes into a JSON manifest under
//! `directories::ProjectDirs::data_dir`.
//!
//! TODO: implement read/write/delete + migration from cswap layout if user
//! had it installed.

use crate::account::{Account, OAuthCredentials};
use anyhow::Result;

pub struct Store {
    // TODO: paths, keyring service name
}

impl Store {
    pub fn open() -> Result<Self> {
        Ok(Self {})
    }

    pub fn list(&self) -> Result<Vec<Account>> {
        // TODO: read manifest, return accounts
        Ok(vec![])
    }

    pub fn save_account(&self, _acct: &Account, _creds: &OAuthCredentials) -> Result<()> {
        // TODO: write metadata to manifest, tokens to keyring
        Ok(())
    }

    pub fn load_credentials(&self, _slot: u32) -> Result<OAuthCredentials> {
        anyhow::bail!("not implemented")
    }

    pub fn delete_account(&self, _slot: u32) -> Result<()> {
        // TODO: remove from manifest + keyring
        Ok(())
    }

    pub fn active_slot(&self) -> Result<Option<u32>> {
        Ok(None)
    }

    pub fn set_active_slot(&self, _slot: u32) -> Result<()> {
        Ok(())
    }
}
