//! Command-line surface.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Launch the tray icon + background monitor (Windows only).
    Start,
    /// Stop a running tray instance.
    Stop,
    /// Show current account, locations detected, monitor status.
    Status,
    /// Add the currently logged-in Claude Code account to the pool.
    Add {
        #[arg(long)]
        slot: Option<u32>,
    },
    /// List managed accounts with usage info.
    List,
    /// Switch to the next account (round-robin).
    Switch,
    /// Switch to a specific account by number or email.
    SwitchTo {
        identifier: String,
    },
    /// Remove an account from the pool.
    Remove {
        identifier: String,
    },
    /// Print detected install locations (Windows native + each WSL distro).
    Locations,
}

pub async fn dispatch(args: Cli) -> Result<()> {
    match args.command {
        None | Some(Command::Start) => start_tray().await,
        Some(Command::Stop) => stop_tray().await,
        Some(Command::Status) => switcher::status().await,
        Some(Command::Add { slot }) => switcher::add_account(slot).await,
        Some(Command::List) => switcher::list_accounts().await,
        Some(Command::Switch) => switcher::switch_next().await,
        Some(Command::SwitchTo { identifier }) => switcher::switch_to(&identifier).await,
        Some(Command::Remove { identifier }) => switcher::remove_account(&identifier).await,
        Some(Command::Locations) => print_locations().await,
    }
}

#[cfg(target_os = "windows")]
async fn start_tray() -> Result<()> {
    crate::tray::run().await
}

#[cfg(not(target_os = "windows"))]
async fn start_tray() -> Result<()> {
    anyhow::bail!("tray mode is Windows-only; build/run on Windows host")
}

#[cfg(target_os = "windows")]
async fn stop_tray() -> Result<()> {
    crate::tray::stop()
}

#[cfg(not(target_os = "windows"))]
async fn stop_tray() -> Result<()> {
    anyhow::bail!("tray mode is Windows-only")
}

async fn print_locations() -> Result<()> {
    let locations = crate::platform::discover_locations().await?;
    for loc in locations {
        println!("{loc}");
    }
    Ok(())
}

use crate::switcher;
