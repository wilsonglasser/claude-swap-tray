//! Settings screen — threshold, poll interval, sound toggle.

use crate::app::{App, Message};
use crate::config::Settings;
use iced::widget::{Space, button, checkbox, column, container, row, slider, text};
use iced::{Element, Length, Task};

#[derive(Debug, Clone)]
pub enum Msg {
    ThresholdChanged(f64),
    PollIntervalChanged(u64),
    SoundToggled(bool),
    AutoRotateToggled(bool),
    Save,
    Saved(Result<(), String>),
}

#[derive(Debug, Clone)]
pub struct State {
    pub draft: Settings,
    pub last_save: Option<Result<(), String>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            draft: Settings::load(),
            last_save: None,
        }
    }
}

pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    match msg {
        Msg::ThresholdChanged(v) => {
            app.settings_state.draft.threshold_percent = v;
            Task::none()
        }
        Msg::PollIntervalChanged(v) => {
            app.settings_state.draft.poll_interval_seconds = v;
            Task::none()
        }
        Msg::SoundToggled(v) => {
            app.settings_state.draft.notify_sound = v;
            Task::none()
        }
        Msg::AutoRotateToggled(v) => {
            app.settings_state.draft.auto_rotate = v;
            Task::none()
        }
        Msg::Save => {
            let draft = app.settings_state.draft.clone();
            Task::perform(
                async move { draft.save().map_err(|e| e.to_string()) },
                |res| Message::Settings(Msg::Saved(res)),
            )
        }
        Msg::Saved(res) => {
            app.settings_state.last_save = Some(res);
            Task::none()
        }
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.settings_state.draft;
    container(
        column![
            text("Settings").size(24),
            Space::new().height(20),
            field(
                "Threshold (%)",
                "Alert when usage on the active account crosses this %.",
                row![
                    slider(50.0..=99.0, s.threshold_percent, |v| {
                        Message::Settings(Msg::ThresholdChanged(v))
                    })
                    .step(1.0)
                    .width(280),
                    text(format!("{:.0}%", s.threshold_percent)).width(60),
                ]
                .spacing(12)
                .into(),
            ),
            Space::new().height(16),
            field(
                "Poll interval (s)",
                "How often to check usage. Lower = more responsive, more API calls.",
                row![
                    slider(15.0..=600.0, s.poll_interval_seconds as f64, |v| {
                        Message::Settings(Msg::PollIntervalChanged(v as u64))
                    })
                    .step(15.0)
                    .width(280),
                    text(format!("{} s", s.poll_interval_seconds)).width(60),
                ]
                .spacing(12)
                .into(),
            ),
            Space::new().height(16),
            checkbox(s.notify_sound)
                .label("Play sound on threshold alert")
                .on_toggle(|v| Message::Settings(Msg::SoundToggled(v))),
            Space::new().height(8),
            checkbox(s.auto_rotate)
                .label("Auto-rotate to next account when threshold crossed")
                .on_toggle(|v| Message::Settings(Msg::AutoRotateToggled(v))),
            Space::new().height(20),
            row![
                button(text("Save"))
                    .on_press(Message::Settings(Msg::Save))
                    .style(button::primary),
                save_status(&app.settings_state.last_save),
            ]
            .spacing(12)
            .align_y(iced::alignment::Vertical::Center),
        ]
        .max_width(640)
        .spacing(4),
    )
    .center_x(Length::Fill)
    .into()
}

fn field<'a>(label: &'a str, help: &'a str, control: Element<'a, Message>) -> Element<'a, Message> {
    column![
        text(label).size(15),
        text(help).size(12).style(|t: &iced::Theme| text::Style {
            color: Some(t.extended_palette().background.weak.color),
        }),
        Space::new().height(6),
        control,
    ]
    .into()
}

fn save_status(state: &Option<Result<(), String>>) -> Element<'_, Message> {
    match state {
        None => Element::from(text("")),
        Some(Ok(())) => text("✓ Saved")
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().success.weak.color),
            })
            .into(),
        Some(Err(e)) => text(format!("✗ {e}"))
            .style(|t: &iced::Theme| text::Style {
                color: Some(t.extended_palette().danger.weak.color),
            })
            .into(),
    }
}
