//! Screen routing — each screen exposes its own `Msg`, `update`, `view`.

use crate::app::{App, Message};
use iced::Element;
use iced::widget::{Space, button, column, container, row, text};

pub mod accounts;
pub mod add_account;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Accounts,
    AddAccount,
    Settings,
}

impl Screen {
    pub fn label(self) -> &'static str {
        match self {
            Screen::Accounts => "Accounts",
            Screen::AddAccount => "Add account",
            Screen::Settings => "Settings",
        }
    }
}

pub fn root_view(app: &App) -> Element<'_, Message> {
    let nav = nav_bar(app.screen);
    let body: Element<'_, Message> = match app.screen {
        Screen::Accounts => accounts::view(app),
        Screen::AddAccount => add_account::view(app),
        Screen::Settings => settings::view(app),
    };
    column![
        nav,
        container(body)
            .padding(24)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
    ]
    .spacing(0)
    .into()
}

fn nav_bar(active: Screen) -> Element<'static, Message> {
    let make_btn = |screen: Screen| {
        let label = screen.label();
        let mut b = button(text(label));
        if screen == active {
            b = b.style(button::primary);
        } else {
            b = b.style(button::secondary);
        }
        b.on_press(Message::NavigateTo(screen)).into()
    };
    let buttons: Vec<Element<'static, Message>> = vec![
        make_btn(Screen::Accounts),
        make_btn(Screen::AddAccount),
        make_btn(Screen::Settings),
    ];
    container(
        row![
            text("claude-swap-tray").size(18),
            Space::new().width(iced::Length::Fill),
            row(buttons).spacing(8),
        ]
        .align_y(iced::alignment::Vertical::Center)
        .padding(12)
        .spacing(12),
    )
    .style(container::bordered_box)
    .into()
}
