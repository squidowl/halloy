use std::time::Duration;

use iced::widget::{
    Space, button, center, column, container, image, row, rule, text,
};
use iced::{Length, Task, alignment, clipboard};
use tokio::time;

use super::Message as ModalMessage;
use crate::widget::Element;
use crate::{Theme, font, icon, theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Version,
    Commit,
    GpuBackend,
    GpuAdapter,
    All,
    Mail,
}

#[derive(Debug, Clone)]
pub enum Action {
    Copy(Field, String),
    Clear(Field),
}

#[derive(Debug, Clone)]
pub struct About {
    logo: image::Handle,
    version: String,
    commit: String,
    system_information: Option<iced::system::Information>,
    copied: Option<Field>,
}

impl About {
    pub fn new(
        version: String,
        commit: String,
        system_information: Option<iced::system::Information>,
    ) -> Self {
        let logo_bytes = include_bytes!("../../assets/logo-256.png").to_vec();
        let logo = image::Handle::from_bytes(logo_bytes);

        Self {
            logo,
            version,
            commit,
            system_information,
            copied: None,
        }
    }

    pub fn update(&mut self, action: Action) -> Task<ModalMessage> {
        match action {
            Action::Copy(field, value) => {
                self.copied = Some(field);

                Task::batch(vec![
                    clipboard::write(value),
                    Task::perform(
                        async move {
                            time::sleep(Duration::from_secs(1)).await;
                            field
                        },
                        |field| ModalMessage::About(Action::Clear(field)),
                    ),
                ])
            }
            Action::Clear(field) => {
                if self.copied == Some(field) {
                    self.copied = None;
                }

                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self, theme: &'a Theme) -> Element<'a, ModalMessage> {
        let logo = image(self.logo.clone()).width(128);

        let gpu_backend = self
            .system_information
            .as_ref()
            .map(|info| info.graphics_backend.trim())
            .filter(|backend| !backend.is_empty())
            .unwrap_or("Unknown");

        let gpu_adapter = self
            .system_information
            .as_ref()
            .map(|info| info.graphics_adapter.trim())
            .filter(|adapter| !adapter.is_empty())
            .unwrap_or("Unknown");

        let item = |label: &'a str, value: &'a str, field: Field| {
            let icon = if self.copied == Some(field) {
                icon::checkmark().style(theme::text::success)
            } else {
                icon::copy().style(theme::text::secondary)
            };

            let message =
                ModalMessage::About(Action::Copy(field, value.to_string()));

            row![
                button(
                    row![
                        text(label).style(theme::text::secondary).font_maybe(
                            theme::font_style::secondary(theme).map(font::get)
                        ),
                        text(value)
                            .style(theme::text::primary)
                            .font_maybe(
                                theme::font_style::primary(theme)
                                    .map(font::get)
                            )
                            .width(Length::Fill)
                            .align_x(iced::widget::text::Alignment::Right)
                    ]
                    .width(Length::Fill)
                )
                .on_press(message.clone())
                .style(theme::button::bare),
                Space::new().width(4),
                button(center(icon))
                    .width(22)
                    .height(22)
                    .padding(2)
                    .style(move |theme, status| {
                        theme::button::secondary(theme, status, false)
                    })
                    .on_press(message),
            ]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center)
        };

        let copy_all_payload = format!(
            "Version: {}\nCommit: {}\nGPU Backend: {}\nGPU Adapter: {}",
            self.version, self.commit, gpu_backend, gpu_adapter
        );

        let copy_all_content = if self.copied == Some(Field::All) {
            container(
                row![
                    text("Copied"),
                    Space::new().width(6),
                    icon::checkmark().style(theme::text::success)
                ]
                .align_y(iced::Alignment::Center),
            )
            .align_x(alignment::Horizontal::Center)
            .width(Length::Fill)
        } else {
            container(text("Copy all"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill)
        };

        let copy_all = button(copy_all_content)
            .padding(5)
            .width(Length::Fixed(250.0))
            .style(|theme, status| {
                theme::button::secondary(theme, status, false)
            })
            .on_press(ModalMessage::About(Action::Copy(
                Field::All,
                copy_all_payload,
            )));

        let close = button(
            container(text("Close"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fixed(250.0))
        .style(|theme, status| theme::button::secondary(theme, status, false))
        .on_press(ModalMessage::Cancel);

        container(
            column![
                logo,
                column![
                    item("Version", &self.version, Field::Version),
                    item("Commit", &self.commit, Field::Commit),
                    item("GPU Backend", gpu_backend, Field::GpuBackend),
                    item("GPU Adapter", gpu_adapter, Field::GpuAdapter),
                    container(rule::horizontal(1))
                        .padding([6, 0])
                        .width(Length::Fill),
                    item("Contact", data::environment::EMAIL, Field::Mail),
                ],
                column![copy_all, close]
                    .spacing(8)
                    .align_x(iced::Alignment::Center),
            ]
            .spacing(20)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fixed(380.0))
        .style(theme::container::tooltip)
        .padding(25)
        .into()
    }
}
