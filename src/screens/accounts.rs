//! Accounts screen — list managed accounts with usage info, switch button.

use crate::app::{App, Message};
use crate::screens::Screen;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length, Task};

#[derive(Debug, Clone)]
pub enum Msg {
    SwitchTo(u32),
    Remove(u32),
    SwitchCompleted(Result<(), String>),
}

#[allow(unused_variables)]
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    match msg {
        Msg::SwitchTo(slot) => {
            // TODO: dispatch async switcher::switch_to and refresh on completion
            Task::none()
        }
        Msg::Remove(_slot) => {
            // TODO: dispatch async removal
            Task::none()
        }
        Msg::SwitchCompleted(_) => Task::none(),
    }
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
                        color: Some(t.extended_palette().background.weak.color),
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

    column![
        text(format!("{} account(s)", app.accounts.len())).size(22),
        Space::new().height(12),
        column(rows).spacing(8),
        Space::new().height(20),
        text(format!("{} location(s) detected", app.locations.len()))
            .size(13)
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().background.weak.color),
            }),
    ]
    .into()
}

fn account_row(acct: &crate::account::Account, is_active: bool) -> Element<'_, Message> {
    let marker = if is_active { "●" } else { " " };
    container(
        row![
            text(marker).size(20),
            column![
                text(acct.email.clone()).size(15),
                text(acct.organization_name.clone())
                    .size(12)
                    .style(|t: &iced::Theme| text::Style {
                        color: Some(t.extended_palette().background.weak.color),
                    }),
            ]
            .spacing(2),
            Space::new().width(Length::Fill),
            button(text("Switch"))
                .on_press(Message::Accounts(Msg::SwitchTo(acct.slot)))
                .style(button::primary),
            button(text("Remove"))
                .on_press(Message::Accounts(Msg::Remove(acct.slot)))
                .style(button::danger),
        ]
        .align_y(iced::alignment::Vertical::Center)
        .spacing(12)
        .padding(12),
    )
    .style(container::bordered_box)
    .into()
}
