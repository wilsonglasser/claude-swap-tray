//! Add-account screen — orchestrates `claude login` on Windows and copies
//! the resulting credentials into every detected WSL distro.

use crate::app::{App, Message};
use crate::login::{self, AddOutcome};
use crate::platform::Location;
use crate::screens::Screen;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length, Task};

#[derive(Debug, Clone)]
pub enum Msg {
    StartLogin,
    LoginDone(Result<AddOutcomeView, String>),
    Reset,
}

#[derive(Debug, Clone)]
pub struct AddOutcomeView {
    pub email: String,
    pub replications: Vec<(String, Result<(), String>)>,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub status: LoginStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LoginStatus {
    #[default]
    Idle,
    Running(String),
    Success(AddOutcomeViewOk),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddOutcomeViewOk {
    pub email: String,
    pub replications: Vec<(String, Result<(), String>)>,
}

pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    match msg {
        Msg::StartLogin => {
            if !has_windows_location(&app.locations) {
                app.add_state.status = LoginStatus::Failed(
                    "Install Claude Code on Windows first — login flow runs there.".to_string(),
                );
                return Task::none();
            }
            app.add_state.status =
                LoginStatus::Running("launching `claude login` on Windows…".to_string());
            let locations = app.locations.clone();
            Task::perform(
                async move {
                    login::add_account_and_sync(locations)
                        .await
                        .map(map_outcome)
                        .map_err(|e| e.to_string())
                },
                |res| Message::AddAccount(Msg::LoginDone(res)),
            )
        }
        Msg::LoginDone(Ok(view)) => {
            app.add_state.status = LoginStatus::Success(AddOutcomeViewOk {
                email: view.email,
                replications: view.replications,
            });
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

fn has_windows_location(locations: &[Location]) -> bool {
    locations
        .iter()
        .any(|l| matches!(l, Location::Windows { .. }))
}

fn map_outcome(o: AddOutcome) -> AddOutcomeView {
    AddOutcomeView {
        email: o.account.email,
        replications: o
            .replications
            .into_iter()
            .map(|(label, res)| (label, res.map_err(|e| e.to_string())))
            .collect(),
    }
}

async fn reload_accounts() -> Vec<crate::account::Account> {
    let store = match crate::store::Store::open() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    store.list().unwrap_or_default()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let summary = location_summary(&app.locations);

    let status_widget: Element<'_, Message> = match &app.add_state.status {
        LoginStatus::Idle => text("Click \"Start login\" to launch `claude login` on Windows.")
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().background.weak.color),
            })
            .into(),
        LoginStatus::Running(msg) => row![text("⟳"), text(msg.clone())].spacing(8).into(),
        LoginStatus::Success(view) => success_widget(view),
        LoginStatus::Failed(e) => text(format!("✗ {e}"))
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().danger.weak.color),
            })
            .into(),
    };

    let can_start = has_windows_location(&app.locations)
        && !matches!(app.add_state.status, LoginStatus::Running(_));

    let start_btn = {
        let mut b = button(text("Start login"));
        if can_start {
            b = b.on_press(Message::AddAccount(Msg::StartLogin));
        }
        b.style(button::primary)
    };

    let actions: Vec<Element<'_, Message>> = match app.add_state.status {
        LoginStatus::Success(_) | LoginStatus::Failed(_) => vec![
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
            text("Login runs on the Windows-side `claude` CLI. After the OAuth flow completes the credentials are copied into every detected WSL distro automatically.")
                .size(13)
                .style(|t: &iced::Theme| text::Style {
                    color: Some(t.extended_palette().background.weak.color),
                }),
            Space::new().height(20),
            summary,
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

fn location_summary(locations: &[Location]) -> Element<'_, Message> {
    let has_windows = has_windows_location(locations);
    let wsl_count = locations
        .iter()
        .filter(|l| matches!(l, Location::Wsl { .. }))
        .count();

    let win_line: Element<'_, Message> = if has_windows {
        text("✓ Windows install detected (login source)")
            .size(14)
            .into()
    } else {
        text("✗ No Windows install — install Claude Code on Windows first")
            .size(14)
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().danger.weak.color),
            })
            .into()
    };

    let wsl_line: Element<'_, Message> = match wsl_count {
        0 => text("No WSL distros to sync (you can still add accounts).")
            .size(14)
            .into(),
        n => text(format!("{n} WSL distro(s) will receive the credentials"))
            .size(14)
            .into(),
    };

    container(column![win_line, wsl_line].spacing(4).padding(12))
        .style(container::bordered_box)
        .into()
}

fn success_widget(view: &AddOutcomeViewOk) -> Element<'_, Message> {
    let mut lines: Vec<Element<'_, Message>> = vec![
        text(format!("✓ Added {}", view.email))
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().success.weak.color),
            })
            .into(),
    ];
    for (label, res) in &view.replications {
        let line: Element<'_, Message> = match res {
            Ok(()) => text(format!("    ✓ Synced to {label}")).size(13).into(),
            Err(e) => text(format!("    ✗ {label}: {e}"))
                .size(13)
                .style(|t: &iced::Theme| text::Style {
                    color: Some(t.extended_palette().danger.weak.color),
                })
                .into(),
        };
        lines.push(line);
    }
    column(lines).spacing(4).into()
}
