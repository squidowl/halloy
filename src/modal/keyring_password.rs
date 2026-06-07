use data::config;
use iced::widget::{button, column, container, text, text_input};
use iced::{Length, Task, alignment};

use super::{Event, Message};
use crate::widget::Element;
use crate::{Theme, font, theme};

#[derive(Debug)]
pub struct KeyringPassword {
    pub label: String,
    pub context: String,
    pub key: String,
    password: String,
    error: Option<String>,
    saving: bool,
}

#[derive(Debug, Clone)]
pub enum Action {
    PasswordChanged(String),
    Save,
    Saved(Result<(), String>),
}

impl KeyringPassword {
    pub fn new(label: String, context: String, key: String) -> Self {
        Self {
            label,
            context,
            key,
            password: String::new(),
            error: None,
            saving: false,
        }
    }

    pub fn update(&mut self, action: Action) -> (Task<Message>, Option<Event>) {
        match action {
            Action::PasswordChanged(password) => {
                self.password = password;
                self.error = None;
                (Task::none(), None)
            }
            Action::Save => {
                if self.password.is_empty() || self.saving {
                    return (Task::none(), None);
                }

                self.saving = true;
                self.error = None;

                let key = self.key.clone();
                let password = self.password.clone();

                (
                    Task::perform(
                        async move {
                            config::keyring::set_password(&key, &password)
                                .map_err(|error| error.to_string())
                        },
                        |result| {
                            Message::KeyringPassword(Action::Saved(result))
                        },
                    ),
                    None,
                )
            }
            Action::Saved(Ok(())) => {
                self.password.clear();
                self.saving = false;
                (Task::none(), Some(Event::KeyringPasswordStored))
            }
            Action::Saved(Err(error)) => {
                self.error = Some(error);
                self.saving = false;
                (Task::none(), None)
            }
        }
    }

    pub fn view<'a>(&'a self, theme: &Theme) -> Element<'a, Message> {
        let can_save = !self.password.is_empty() && !self.saving;
        let save = can_save.then_some(Message::KeyringPassword(Action::Save));

        let mut content = column![
            text(format!("{} for {}", self.label, self.context)),
            text(&self.key)
                .style(theme::text::tertiary)
                .font_maybe(theme::font_style::tertiary(theme).map(font::get)),
            text_input("Password", &self.password)
                .secure(true)
                .on_input(|password| {
                    Message::KeyringPassword(Action::PasswordChanged(password))
                })
                .on_submit_maybe(save.clone())
                .width(Length::Fixed(300.0)),
        ]
        .spacing(12)
        .align_x(iced::Alignment::Center);

        if let Some(error) = &self.error {
            content =
                content.push(text(error).style(theme::text::error).font_maybe(
                    theme::font_style::error(theme).map(font::get),
                ));
        }

        let label = if self.saving { "Saving..." } else { "Save" };

        content = content
            .push(
                button(
                    container(text(label))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(|theme, status| {
                    theme::button::secondary(theme, status, false)
                })
                .on_press_maybe(save),
            )
            .push(
                button(
                    container(text("Cancel"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(|theme, status| {
                    theme::button::secondary(theme, status, false)
                })
                .on_press(Message::Cancel),
            );

        container(content)
            .width(Length::Shrink)
            .style(theme::container::tooltip)
            .padding(25)
            .into()
    }
}
