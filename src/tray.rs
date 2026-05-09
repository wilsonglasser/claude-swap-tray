//! System tray icon — Windows only.
//!
//! Lives on a dedicated thread because `tray-icon` requires a Win32
//! message pump on the thread that owns the tray. We use a `PeekMessageW`
//! loop so the same iteration can also drain commands from iced (via a
//! crossbeam channel) and rebuild the menu when accounts/usage change.
//!
//! Menu structure:
//! ```text
//!   Show window
//!   Hide to tray
//!   ---
//!   Switch to >
//!     ● user@a.com  (54%)
//!       user@b.com  (12%)
//!     ---
//!     Refresh now
//!   ---
//!   Quit
//! ```
//! The `usage_pct` for the active account is also reflected in the tray
//! tooltip: `claude-swap-tray — user@example.com (54%)`.

#![cfg(target_os = "windows")]

use crate::app::Message;
use anyhow::Result;
use iced::Subscription;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Mutex, OnceLock};
use std::thread;
use tracing::{info, warn};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};

const APP_NAME: &str = "claude-swap-tray";

#[derive(Debug, Clone, PartialEq)]
pub enum TrayAction {
    ShowWindow,
    HideWindow,
    SwitchTo(u32),
    RefreshUsage,
    Quit,
}

#[derive(Debug, Clone, Default)]
pub struct MenuModel {
    pub accounts: Vec<MenuAccount>,
    pub active_slot: Option<u32>,
    pub usage_pct: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MenuAccount {
    pub slot: u32,
    pub email: String,
    pub usage_pct: Option<f64>,
}

/// Channel iced uses to ask the tray thread to rebuild its menu.
static REBUILD_TX: OnceLock<SyncSender<MenuModel>> = OnceLock::new();
/// Lookup from menu-item-id → tray action. Repopulated on every menu rebuild.
static ID_MAP: Mutex<Option<HashMap<String, TrayAction>>> = Mutex::new(None);

pub fn spawn() {
    let (tx, rx) = sync_channel::<MenuModel>(8);
    let _ = REBUILD_TX.set(tx);
    thread::Builder::new()
        .name("tray".to_string())
        .spawn(|| {
            if let Err(e) = run_tray_thread(rx) {
                warn!(error = ?e, "tray thread crashed");
            }
        })
        .expect("spawn tray thread");
}

/// Tell the tray thread to rebuild its menu with the given model. Called
/// from iced whenever accounts list / active slot / usage changes.
pub fn push_menu(model: MenuModel) {
    if let Some(tx) = REBUILD_TX.get() {
        let _ = tx.try_send(model);
    }
}

fn run_tray_thread(rx: Receiver<MenuModel>) -> Result<()> {
    // Initial empty menu: just Show / Hide / Quit until iced sends a model.
    let (menu, ids) = build_menu(&MenuModel::default());
    *ID_MAP.lock().unwrap() = Some(ids);

    let icon = build_default_icon();
    let tray: TrayIcon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(APP_NAME)
        .with_icon(icon)
        .build()?;
    info!("tray icon ready");

    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        loop {
            // Drain pending Win32 messages without blocking.
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                if msg.message == WM_QUIT {
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            // Drain pending menu rebuilds from iced.
            while let Ok(model) = rx.try_recv() {
                let (m, ids) = build_menu(&model);
                tray.set_menu(Some(Box::new(m)));
                *ID_MAP.lock().unwrap() = Some(ids);
                let _ = tray.set_tooltip(Some(tooltip_for(&model)));
            }
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    }
}

fn tooltip_for(model: &MenuModel) -> String {
    let active = model
        .active_slot
        .and_then(|s| model.accounts.iter().find(|a| a.slot == s));
    match (active, model.usage_pct) {
        (Some(a), Some(p)) => format!("{APP_NAME} — {} ({p:.0}%)", a.email),
        (Some(a), None) => format!("{APP_NAME} — {}", a.email),
        _ => APP_NAME.to_string(),
    }
}

fn build_menu(model: &MenuModel) -> (Menu, HashMap<String, TrayAction>) {
    let mut ids: HashMap<String, TrayAction> = HashMap::new();

    let menu = Menu::new();
    let show = MenuItem::new("Show window", true, None);
    let hide = MenuItem::new("Hide to tray", true, None);
    let quit = MenuItem::new("Quit", true, None);
    ids.insert(show.id().0.clone(), TrayAction::ShowWindow);
    ids.insert(hide.id().0.clone(), TrayAction::HideWindow);
    ids.insert(quit.id().0.clone(), TrayAction::Quit);

    menu.append(&show).ok();
    menu.append(&hide).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();

    if !model.accounts.is_empty() {
        let switch_label = match (model.active_slot, model.usage_pct) {
            (Some(_), Some(p)) => format!("Switch to…  ({p:.0}%)"),
            _ => "Switch to…".to_string(),
        };
        let submenu = Submenu::new(switch_label, true);
        for acct in &model.accounts {
            let is_active = Some(acct.slot) == model.active_slot;
            let label = render_account_label(acct, is_active);
            // Active item: present but disabled (informational).
            let item = MenuItem::new(label, !is_active, None);
            ids.insert(item.id().0.clone(), TrayAction::SwitchTo(acct.slot));
            submenu.append(&item).ok();
        }
        submenu.append(&PredefinedMenuItem::separator()).ok();
        let refresh = MenuItem::new("Refresh usage", true, None);
        ids.insert(refresh.id().0.clone(), TrayAction::RefreshUsage);
        submenu.append(&refresh).ok();
        menu.append(&submenu).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
    }

    menu.append(&quit).ok();
    (menu, ids)
}

fn render_account_label(acct: &MenuAccount, is_active: bool) -> String {
    let prefix = if is_active { "●  " } else { "    " };
    match acct.usage_pct {
        Some(p) => format!("{prefix}{}  ({p:.0}%)", acct.email),
        None => format!("{prefix}{}", acct.email),
    }
}

/// Procedural 32×32 RGBA — claude-orange disc on dark ring on transparent.
/// Public so the iced window can use the same image as its taskbar icon.
pub fn build_default_icon() -> Icon {
    let (rgba, w, h) = build_default_icon_rgba();
    Icon::from_rgba(rgba, w, h).expect("icon build")
}

pub fn build_default_icon_rgba() -> (Vec<u8>, u32, u32) {
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
                rgba.extend_from_slice(&[0xCC, 0x78, 0x4A, 0xFF]);
            } else if dist <= 14.5 {
                rgba.extend_from_slice(&[0x1A, 0x1A, 0x1A, 0xFF]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    (rgba, size, size)
}

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
    let map = ID_MAP.lock().ok()?;
    map.as_ref()?.get(id).cloned()
}
