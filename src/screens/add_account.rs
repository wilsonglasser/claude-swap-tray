//! Add-account screen — orchestrates `claude login` in a chosen location.

use crate::app::{App, Message};
use crate::login;
use crate::platform::Location;
use crate::screens::Screen;
use iced::widget::{Space, button, column, container, pick_list, row, text};
use iced::{Element, Length, Task};

#[derive(Debug, Clone)]
pub enum Msg {
    SelectLocation(LocationOption),
    StartLogin,
    LoginProgress(String),
    LoginDone(Result<crate::account::Account, String>),
    Reset,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub selected: Option<LocationOption>,
    pub status: LoginStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LoginStatus {
    #[default]
    Idle,
    Running(String),
    Success,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationOption {
    pub label: String,
    pub kind: LocationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationKind {
    Windows,
    Wsl(String),
}

impl std::fmt::Display for LocationOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

impl LocationOption {
    pub fn from_location(loc: &Location) -> Self {
        match loc {
            Location::Windows { .. } => Self {
                label: "Windows native".to_string(),
                kind: LocationKind::Windows,
            },
            Location::Wsl { distro, .. } => Self {
                label: format!("WSL: {distro}"),
                kind: LocationKind::Wsl(distro.clone()),
            },
        }
    }
}

pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    match msg {
        Msg::SelectLocation(opt) => {
            app.add_state.selected = Some(opt);
            Task::none()
        }
        Msg::StartLogin => {
            let Some(opt) = app.add_state.selected.clone() else {
                return Task::none();
            };
            let Some(loc) = app
                .locations
                .iter()
                .find(|l| matches_kind(l, &opt.kind))
                .cloned()
            else {
                app.add_state.status = LoginStatus::Failed(
                    "selected location no longer detected; refresh and retry".to_string(),
                );
                return Task::none();
            };
            app.add_state.status = LoginStatus::Running("launching `claude login`…".to_string());
            Task::perform(login::add_via_login(loc), |res| {
                Message::AddAccount(Msg::LoginDone(res.map_err(|e| e.to_string())))
            })
        }
        Msg::LoginProgress(s) => {
            app.add_state.status = LoginStatus::Running(s);
            Task::none()
        }
        Msg::LoginDone(Ok(_acct)) => {
            app.add_state.status = LoginStatus::Success;
            Task::perform(reload_accounts(), Message::AccountsRefreshed)
        }
        Msg::LoginDone(Err(e)) => {
            app.add_state.status = LoginStatus::Failed(e);
            Task::none()
        }
        Msg::Reset => {
            app.add_state = State::default();
            Task::none()
        }
    }
}

fn matches_kind(loc: &Location, kind: &LocationKind) -> bool {
    matches!(
        (loc, kind),
        (Location::Windows { .. }, LocationKind::Windows)
    ) || matches!(
        (loc, kind),
        (Location::Wsl { distro: a, .. }, LocationKind::Wsl(b)) if a == b
    )
}

async fn reload_accounts() -> Vec<crate::account::Account> {
    let store = match crate::store::Store::open() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    store.list().unwrap_or_default()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let options: Vec<LocationOption> = app
        .locations
        .iter()
        .map(LocationOption::from_location)
        .collect();

    let location_picker: Element<'_, Message> = if options.is_empty() {
        text("No Claude Code installations detected on this host.").into()
    } else {
        pick_list(
            options,
            app.add_state.selected.clone(),
            |opt| Message::AddAccount(Msg::SelectLocation(opt)),
        )
        .placeholder("Choose a location to log in via")
        .into()
    };

    let status_widget: Element<'_, Message> = match &app.add_state.status {
        LoginStatus::Idle => text("Pick a location and click \"Start login\".")
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().background.weak.color),
            })
            .into(),
        LoginStatus::Running(msg) => row![text("⟳"), text(msg.clone())].spacing(8).into(),
        LoginStatus::Success => text("✓ Account added. Sync to other locations from the Accounts screen.")
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().success.weak.color),
            })
            .into(),
        LoginStatus::Failed(e) => text(format!("✗ {e}"))
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().danger.weak.color),
            })
            .into(),
    };

    let start_btn = {
        let mut b = button(text("Start login"));
        let can_start = app.add_state.selected.is_some()
            && !matches!(app.add_state.status, LoginStatus::Running(_));
        if can_start {
            b = b.on_press(Message::AddAccount(Msg::StartLogin));
        }
        b.style(button::primary)
    };

    let actions: Vec<Element<'_, Message>> = match app.add_state.status {
        LoginStatus::Success | LoginStatus::Failed(_) => vec![
            button(text("Reset"))
                .on_press(Message::AddAccount(Msg::Reset))
                .style(button::secondary)
                .into(),
            button(text("Back to Accounts"))
                .on_press(Message::NavigateTo(Screen::Accounts))
                .style(button::primary)
                .into(),
        ],
        _ => vec![start_btn.into()],
    };

    container(
        column![
            text("Add a Claude Code account").size(24),
            Space::new().height(8),
            text("This will spawn `claude login` in the chosen location and capture the credentials when the OAuth flow completes.")
                .size(13)
                .style(|t: &iced::Theme| text::Style {
                    color: Some(t.extended_palette().background.weak.color),
                }),
            Space::new().height(20),
            location_picker,
            Space::new().height(20),
            status_widget,
            Space::new().height(20),
            row(actions).spacing(8),
        ]
        .max_width(640)
        .spacing(4),
    )
    .center_x(Length::Fill)
    .into()
}
