//! Account persistence.
//!
//! - Sensitive: OAuth tokens go into Windows Credential Manager via the
//!   `keyring` crate (service `claude-swap-tray-credentials`, user `slot-N`).
//! - Non-sensitive: a JSON manifest at `<data_dir>/manifest.json` keeps
//!   slot/email/org/active pointer.

use crate::account::{Account, OAuthCredentials};
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const APP_QUALIFIER: &str = "com";
const APP_ORG: &str = "wilsonglasser";
const APP_NAME: &str = "claude-swap-tray";

#[cfg(target_os = "windows")]
const KEYRING_SERVICE: &str = "claude-swap-tray-credentials";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    active_slot: Option<u32>,
    #[serde(default)]
    accounts: Vec<Account>,
}

fn default_version() -> u32 {
    1
}

impl Default for Manifest {
    fn default() -> Self {
        Self { version: 1, active_slot: None, accounts: Vec::new() }
    }
}

pub struct Store {
    manifest_path: PathBuf,
}

impl Store {
    pub fn open() -> Result<Self> {
        let dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME)
            .ok_or_else(|| anyhow::anyhow!("could not resolve project data directory"))?;
        let data_dir = dirs.data_dir();
        fs::create_dir_all(data_dir)
            .with_context(|| format!("failed to create {}", data_dir.display()))?;
        Ok(Self { manifest_path: data_dir.join("manifest.json") })
    }

    fn read_manifest(&self) -> Result<Manifest> {
        if !self.manifest_path.exists() {
            return Ok(Manifest::default());
        }
        let raw = fs::read_to_string(&self.manifest_path)
            .with_context(|| format!("read {}", self.manifest_path.display()))?;
        let m: Manifest = serde_json::from_str(&raw)
            .with_context(|| format!("parse {}", self.manifest_path.display()))?;
        Ok(m)
    }

    fn write_manifest(&self, m: &Manifest) -> Result<()> {
        let tmp = self.manifest_path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(m)?;
        fs::write(&tmp, json).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &self.manifest_path)
            .with_context(|| format!("rename to {}", self.manifest_path.display()))?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Account>> {
        Ok(self.read_manifest()?.accounts)
    }

    pub fn active_slot(&self) -> Result<Option<u32>> {
        Ok(self.read_manifest()?.active_slot)
    }

    pub fn set_active_slot(&self, slot: u32) -> Result<()> {
        let mut m = self.read_manifest()?;
        m.active_slot = Some(slot);
        self.write_manifest(&m)
    }

    /// Allocate a new slot number (lowest unused positive integer).
    pub fn next_slot(&self) -> Result<u32> {
        let m = self.read_manifest()?;
        let used: std::collections::BTreeSet<u32> =
            m.accounts.iter().map(|a| a.slot).collect();
        Ok((1u32..).find(|s| !used.contains(s)).unwrap())
    }

    pub fn save_account(&self, acct: &Account, creds: &OAuthCredentials) -> Result<()> {
        let mut m = self.read_manifest()?;
        if let Some(existing) = m.accounts.iter_mut().find(|a| a.slot == acct.slot) {
            *existing = acct.clone();
        } else {
            m.accounts.push(acct.clone());
            m.accounts.sort_by_key(|a| a.slot);
        }
        if m.active_slot.is_none() {
            m.active_slot = Some(acct.slot);
        }
        self.write_manifest(&m)?;
        write_credentials_to_keyring(acct.slot, creds)?;
        Ok(())
    }

    pub fn load_credentials(&self, slot: u32) -> Result<OAuthCredentials> {
        read_credentials_from_keyring(slot)
    }

    pub fn delete_account(&self, slot: u32) -> Result<()> {
        let mut m = self.read_manifest()?;
        m.accounts.retain(|a| a.slot != slot);
        if m.active_slot == Some(slot) {
            m.active_slot = m.accounts.first().map(|a| a.slot);
        }
        self.write_manifest(&m)?;
        delete_credentials_from_keyring(slot).ok();
        Ok(())
    }

    pub fn account(&self, slot: u32) -> Result<Option<Account>> {
        Ok(self
            .read_manifest()?
            .accounts
            .into_iter()
            .find(|a| a.slot == slot))
    }

    #[allow(dead_code)]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }
}

#[cfg(target_os = "windows")]
fn write_credentials_to_keyring(slot: u32, creds: &OAuthCredentials) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &format!("slot-{slot}"))?;
    let json = serde_json::to_string(creds)?;
    entry.set_password(&json)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn read_credentials_from_keyring(slot: u32) -> Result<OAuthCredentials> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &format!("slot-{slot}"))?;
    let json = entry.get_password()?;
    Ok(serde_json::from_str(&json)?)
}

#[cfg(target_os = "windows")]
fn delete_credentials_from_keyring(slot: u32) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &format!("slot-{slot}"))?;
    entry.delete_credential()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn write_credentials_to_keyring(_slot: u32, _creds: &OAuthCredentials) -> Result<()> {
    anyhow::bail!("keyring storage is Windows-only")
}

#[cfg(not(target_os = "windows"))]
fn read_credentials_from_keyring(_slot: u32) -> Result<OAuthCredentials> {
    anyhow::bail!("keyring storage is Windows-only")
}

#[cfg(not(target_os = "windows"))]
fn delete_credentials_from_keyring(_slot: u32) -> Result<()> {
    Ok(())
}
