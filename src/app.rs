//! Root iced application — Elm-style update/view loop with screen routing.

use crate::account::Account;
use crate::platform::Location;
use crate::screens::{Screen, accounts, add_account, settings};
use anyhow::Result;
use iced::{Element, Subscription, Task, Theme, time, window};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    /// Top-level navigation.
    NavigateTo(Screen),
    /// Periodic refresh tick.
    Tick,
    /// Locations re-discovered.
    LocationsRefreshed(Vec<Location>),
    /// Account list refreshed.
    AccountsRefreshed(Vec<Account>),
    /// User clicked the X on the window — hide instead of exit.
    WindowCloseRequested(window::Id),
    /// Background usage monitor produced an event.
    #[cfg(target_os = "windows")]
    MonitorEvent(crate::monitor::MonitorEvent),
    /// User picked something from the system tray menu.
    #[cfg(target_os = "windows")]
    TrayAction(crate::tray::TrayAction),

    AddAccount(add_account::Msg),
    Accounts(accounts::Msg),
    Settings(settings::Msg),
}

pub struct App {
    pub screen: Screen,
    pub accounts: Vec<Account>,
    pub locations: Vec<Location>,
    pub active_slot: Option<u32>,
    pub usage_pct: Option<f64>,
    pub add_state: add_account::State,
    pub settings_state: settings::State,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let app = Self {
            screen: Screen::Accounts,
            accounts: Vec::new(),
            locations: Vec::new(),
            active_slot: None,
            usage_pct: None,
            add_state: add_account::State::default(),
            settings_state: settings::State::default(),
        };
        let bootstrap = Task::batch(vec![
            Task::perform(load_locations(), Message::LocationsRefreshed),
            Task::perform(load_accounts(), Message::AccountsRefreshed),
        ]);
        (app, bootstrap)
    }

    fn title(&self) -> String {
        format!("claude-swap-tray — {}", self.screen.label())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NavigateTo(screen) => {
                self.screen = screen;
                Task::none()
            }
            Message::Tick => Task::batch(vec![
                Task::perform(load_locations(), Message::LocationsRefreshed),
                Task::perform(load_accounts(), Message::AccountsRefreshed),
            ]),
            Message::LocationsRefreshed(locs) => {
                self.locations = locs;
                Task::none()
            }
            Message::AccountsRefreshed(accts) => {
                self.accounts = accts;
                Task::none()
            }
            Message::WindowCloseRequested(id) => window::set_mode(id, window::Mode::Hidden),
            #[cfg(target_os = "windows")]
            Message::MonitorEvent(ev) => handle_monitor_event(self, ev),
            #[cfg(target_os = "windows")]
            Message::TrayAction(action) => handle_tray_action(action),
            Message::AddAccount(msg) => add_account::update(self, msg),
            Message::Accounts(msg) => accounts::update(self, msg),
            Message::Settings(msg) => settings::update(self, msg),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        crate::screens::root_view(self)
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs: Vec<Subscription<Message>> = vec![
            time::every(Duration::from_secs(30)).map(|_| Message::Tick),
            window::close_requests().map(Message::WindowCloseRequested),
        ];
        #[cfg(target_os = "windows")]
        {
            subs.push(monitor_subscription().map(Message::MonitorEvent));
            subs.push(crate::tray::subscription());
        }
        Subscription::batch(subs)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

#[cfg(target_os = "windows")]
fn handle_monitor_event(app: &mut App, ev: crate::monitor::MonitorEvent) -> Task<Message> {
    use crate::monitor::MonitorEvent;
    match ev {
        MonitorEvent::ThresholdCrossed { slot, email, pct } => {
            app.usage_pct = Some(pct);
            let with_sound = app.settings_state.draft.notify_sound;
            if let Err(e) = crate::notify::show_threshold_alert(slot, &email, pct, with_sound) {
                tracing::warn!(error = ?e, "threshold toast failed");
            }
        }
        MonitorEvent::UsageUpdated { pct, .. } => {
            app.usage_pct = Some(pct);
        }
    }
    Task::none()
}

#[cfg(target_os = "windows")]
fn handle_tray_action(action: crate::tray::TrayAction) -> Task<Message> {
    use crate::tray::TrayAction;
    match action {
        TrayAction::ShowWindow => window::oldest().then(|opt| match opt {
            Some(id) => Task::batch(vec![
                window::set_mode(id, window::Mode::Windowed),
                window::gain_focus(id),
            ]),
            None => Task::none(),
        }),
        TrayAction::HideWindow => window::oldest().then(|opt| match opt {
            Some(id) => window::set_mode(id, window::Mode::Hidden),
            None => Task::none(),
        }),
        TrayAction::Quit => iced::exit(),
    }
}

#[cfg(target_os = "windows")]
fn monitor_subscription() -> Subscription<crate::monitor::MonitorEvent> {
    use iced::stream;
    Subscription::run(|| {
        stream::channel(16, |mut output| async move {
            loop {
                let settings = crate::config::Settings::load();
                let mon = crate::monitor::Monitor::new(settings);
                if let Some(ev) = mon.poll_once().await {
                    let _ = iced::futures::SinkExt::send(&mut output, ev).await;
                }
                tokio::time::sleep(mon.poll_interval()).await;
            }
        })
    })
}

pub fn run() -> Result<()> {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .window_size((900.0, 600.0))
        .exit_on_close_request(false)
        .run()
        .map_err(|e| anyhow::anyhow!("iced runtime error: {e}"))
}

async fn load_locations() -> Vec<Location> {
    crate::platform::discover_locations()
        .await
        .unwrap_or_default()
}

async fn load_accounts() -> Vec<Account> {
    let store = match crate::store::Store::open() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    store.list().unwrap_or_default()
}
