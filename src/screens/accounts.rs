//! Accounts screen — list managed accounts, switch active, remove.

use crate::account::Account;
use crate::app::{App, Message};
use crate::platform::Location;
use crate::screens::Screen;
use crate::switcher;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length, Task};

#[derive(Debug, Clone)]
pub enum Msg {
    SwitchTo(u32),
    Remove(u32),
    SwitchCompleted(Result<u32, String>),
    RemoveCompleted(Result<u32, String>),
}

pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    match msg {
        Msg::SwitchTo(slot) => {
            let locations = app.locations.clone();
            Task::perform(
                async move {
                    switcher::switch_to(slot, &locations)
                        .await
                        .map(|_| slot)
                        .map_err(|e| e.to_string())
                },
                |res| Message::Accounts(Msg::SwitchCompleted(res)),
            )
        }
        Msg::Remove(slot) => Task::perform(
            async move {
                switcher::remove(slot)
                    .await
                    .map(|_| slot)
                    .map_err(|e| e.to_string())
            },
            |res| Message::Accounts(Msg::RemoveCompleted(res)),
        ),
        Msg::SwitchCompleted(Ok(slot)) => {
            app.active_slot = Some(slot);
            Task::perform(reload_accounts(), Message::AccountsRefreshed)
        }
        Msg::SwitchCompleted(Err(e)) => {
            tracing::warn!(error = %e, "switch failed");
            Task::none()
        }
        Msg::RemoveCompleted(Ok(_)) => Task::perform(reload_accounts(), Message::AccountsRefreshed),
        Msg::RemoveCompleted(Err(e)) => {
            tracing::warn!(error = %e, "remove failed");
            Task::none()
        }
    }
}

async fn reload_accounts() -> Vec<Account> {
    let store = match crate::store::Store::open() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    store.list().unwrap_or_default()
}

pub fn view(app: &App) -> Element<'_, Message> {
    if app.accounts.is_empty() {
        return container(
            column![
                text("No accounts yet").size(28),
                Space::new().height(8),
                text("Log into Claude Code, then add the account here.")
                    .size(14)
                    .style(|t: &iced::Theme| text::Style {
                        color: Some(iced::Color {
                            a: 0.65,
                            ..t.extended_palette().background.base.text
                        }),
                    }),
                Space::new().height(20),
                button(text("Add account"))
                    .style(button::primary)
                    .on_press(Message::NavigateTo(Screen::AddAccount)),
            ]
            .align_x(iced::alignment::Horizontal::Center)
            .spacing(4),
        )
        .center(Length::Fill)
        .into();
    }

    let active = app.active_slot;
    let rows: Vec<Element<'_, Message>> = app
        .accounts
        .iter()
        .map(|a| account_row(a, active == Some(a.slot)))
        .collect();

    let header = match app.usage_pct {
        Some(pct) => format!("{} account(s) — active at {pct:.0}%", app.accounts.len()),
        None => format!("{} account(s)", app.accounts.len()),
    };

    column![
        text(header).size(22),
        Space::new().height(12),
        column(rows).spacing(8),
        Space::new().height(20),
        locations_summary(&app.locations),
    ]
    .into()
}

fn account_row(acct: &Account, is_active: bool) -> Element<'_, Message> {
    let org_label = if acct.organization_name.is_empty() {
        "personal".to_string()
    } else {
        acct.organization_name.clone()
    };

    let identity = column![
        text(acct.email.clone()).size(15),
        text(org_label).size(12).style(muted_text),
    ]
    .spacing(2);

    let action: Element<'_, Message> = if is_active {
        text("● Active")
            .size(13)
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().success.base.color),
            })
            .into()
    } else {
        button(text("Switch"))
            .on_press(Message::Accounts(Msg::SwitchTo(acct.slot)))
            .style(button::primary)
            .into()
    };

    container(
        row![
            identity,
            Space::new().width(Length::Fill),
            action,
            button(text("Remove"))
                .on_press(Message::Accounts(Msg::Remove(acct.slot)))
                .style(button::danger),
        ]
        .align_y(iced::alignment::Vertical::Center)
        .spacing(12)
        .padding(12),
    )
    .style(if is_active {
        active_row_style
    } else {
        container::bordered_box
    })
    .into()
}

fn active_row_style(theme: &iced::Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(iced::Background::Color(palette.success.weak.color)),
        text_color: Some(palette.success.weak.text),
        border: iced::Border {
            color: palette.success.strong.color,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

/// Mid-gray that's readable on both Dark and Light themes. Replaces
/// `extended_palette().background.weak.color` (which is a divider tone
/// — too washed-out for body text on dark bg).
fn muted_text(theme: &iced::Theme) -> text::Style {
    let palette = theme.extended_palette();
    let base = palette.background.base.text;
    text::Style {
        color: Some(iced::Color { a: 0.65, ..base }),
    }
}

fn locations_summary(locations: &[Location]) -> Element<'_, Message> {
    if locations.is_empty() {
        return text("No Claude Code installations detected.")
            .size(13)
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().danger.base.color),
            })
            .into();
    }
    let labels: String = locations
        .iter()
        .map(|l| l.label())
        .collect::<Vec<_>>()
        .join(", ");
    text(format!("{} location(s): {labels}", locations.len()))
        .size(13)
        .style(muted_text)
        .into()
}
