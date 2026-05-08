//! Root iced application — Elm-style update/view loop with screen routing.

use crate::account::Account;
use crate::platform::Location;
use crate::screens::{Screen, accounts, add_account, settings};
use anyhow::Result;
use iced::{Element, Subscription, Task, Theme, time};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    /// Top-level navigation.
    NavigateTo(Screen),
    /// Periodic refresh tick.
    Tick,
    /// Locations re-discovered (in case user installed new WSL distro).
    LocationsRefreshed(Vec<Location>),
    /// Account list refreshed.
    AccountsRefreshed(Vec<Account>),

    // Add-account screen.
    AddAccount(add_account::Msg),
    // Accounts screen.
    Accounts(accounts::Msg),
    // Settings screen.
    Settings(settings::Msg),
}

pub struct App {
    pub screen: Screen,
    pub accounts: Vec<Account>,
    pub locations: Vec<Location>,
    pub active_slot: Option<u32>,
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
            Message::AddAccount(msg) => add_account::update(self, msg),
            Message::Accounts(msg) => accounts::update(self, msg),
            Message::Settings(msg) => settings::update(self, msg),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        crate::screens::root_view(self)
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_secs(30)).map(|_| Message::Tick)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

pub fn run() -> Result<()> {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .window_size((900.0, 600.0))
        .run()
        .map_err(|e| anyhow::anyhow!("iced runtime error: {e}"))
}

async fn load_locations() -> Vec<Location> {
    crate::platform::discover_locations().await.unwrap_or_default()
}

async fn load_accounts() -> Vec<Account> {
    let store = match crate::store::Store::open() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    store.list().unwrap_or_default()
}
