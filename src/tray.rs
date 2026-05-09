//! System tray icon — Windows only.
//!
//! Lives on a dedicated thread because `tray-icon` requires a Win32
//! message loop on the thread that owns the tray. The thread runs
//! `GetMessageW` until a `WM_QUIT` is posted (when the user picks Quit
//! from the menu and we exit the iced runtime via `iced::exit`).
//!
//! Menu events bubble out via the `muda` global static channel
//! (`MenuEvent::receiver()`); iced reads them with a `Subscription` that
//! polls the channel from inside `iced::stream::channel`.

#![cfg(target_os = "windows")]

use crate::app::Message;
use anyhow::Result;
use iced::Subscription;
use std::sync::OnceLock;
use std::thread;
use tracing::{info, warn};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, TranslateMessage,
};

const TOOLTIP: &str = "claude-swap-tray";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ShowWindow,
    HideWindow,
    Quit,
}

static SHOW_ID: OnceLock<String> = OnceLock::new();
static HIDE_ID: OnceLock<String> = OnceLock::new();
static QUIT_ID: OnceLock<String> = OnceLock::new();

/// Fire-and-forget — spawn the tray on its own OS thread. Safe to call
/// once per process.
pub fn spawn() {
    thread::Builder::new()
        .name("tray".to_string())
        .spawn(|| {
            if let Err(e) = run_tray_thread() {
                warn!(error = ?e, "tray thread crashed");
            }
        })
        .expect("spawn tray thread");
}

fn run_tray_thread() -> Result<()> {
    let menu = Menu::new();
    let show = MenuItem::new("Show window", true, None);
    let hide = MenuItem::new("Hide to tray", true, None);
    let quit = MenuItem::new("Quit", true, None);
    let _ = SHOW_ID.set(show.id().0.clone());
    let _ = HIDE_ID.set(hide.id().0.clone());
    let _ = QUIT_ID.set(quit.id().0.clone());
    menu.append_items(&[
        &show,
        &hide,
        &PredefinedMenuItem::separator(),
        &quit,
    ])?;

    let icon = build_default_icon();
    let _tray: TrayIcon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(TOOLTIP)
        .with_icon(icon)
        .build()?;
    info!("tray icon ready");

    // Win32 message pump — blocks until a WM_QUIT is posted.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

/// Procedural 32×32 icon — claude-orange disc on dark ring on transparent.
/// Replace by reading `assets/icon.ico` once we ship a designed asset.
fn build_default_icon() -> Icon {
    let size: u32 = 32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    let cx = size as f32 / 2.0 - 0.5;
    let cy = size as f32 / 2.0 - 0.5;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= 8.5 {
                rgba.extend_from_slice(&[0xCC, 0x78, 0x4A, 0xFF]); // claude orange
            } else if dist <= 14.5 {
                rgba.extend_from_slice(&[0x1A, 0x1A, 0x1A, 0xFF]); // dark ring
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Icon::from_rgba(rgba, size, size).expect("icon build")
}

/// iced subscription that streams [`TrayAction`]s from the menu thread.
pub fn subscription() -> Subscription<Message> {
    use iced::stream;
    Subscription::run(|| {
        stream::channel(16, |mut output| async move {
            let rx = MenuEvent::receiver();
            loop {
                match rx.try_recv() {
                    Ok(ev) => {
                        if let Some(action) = classify(&ev) {
                            let _ = iced::futures::SinkExt::send(
                                &mut output,
                                Message::TrayAction(action),
                            )
                            .await;
                        }
                    }
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                    }
                }
            }
        })
    })
}

fn classify(ev: &MenuEvent) -> Option<TrayAction> {
    let id = &ev.id.0;
    if SHOW_ID.get().map(|s| s == id).unwrap_or(false) {
        return Some(TrayAction::ShowWindow);
    }
    if HIDE_ID.get().map(|s| s == id).unwrap_or(false) {
        return Some(TrayAction::HideWindow);
    }
    if QUIT_ID.get().map(|s| s == id).unwrap_or(false) {
        return Some(TrayAction::Quit);
    }
    None
}
