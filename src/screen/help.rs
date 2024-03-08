use data::{config, Config};
use iced::widget::{button, column, container, text, vertical_space};
use iced::{alignment, Length};

use crate::widget::Element;
use crate::{font, icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    RefreshConfiguration,
    OpenConfigurationDirectory,
}

#[derive(Debug, Clone)]
pub enum Event {
    RefreshConfiguration,
}

#[derive(Debug, Clone)]
pub struct Help {
    error: config::Error,
}

impl Help {
    pub fn new(error: config::Error) -> Self {
        Help { error }
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::RefreshConfiguration => Some(Event::RefreshConfiguration),
            Message::OpenConfigurationDirectory => {
                let _ = open::that(Config::config_dir());

                None
            }
        }
    }

    pub fn view<'a>(&self) -> Element<'a, Message> {
        let config_button = button(
            container(text("Open Directory"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .width(Length::Fixed(250.0))
        .style(theme::Button::Secondary)
        .on_press(Message::OpenConfigurationDirectory);

        let refresh_button = button(
            container(text("Refresh"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .width(Length::Fixed(250.0))
        .style(theme::Button::Secondary)
        .on_press(Message::RefreshConfiguration);

        let content = column![]
            .spacing(1)
            .push(icon::error().size(45))
            .push(vertical_space(10))
            .push(text("Error reading configuration file").font(font::MONO_BOLD.clone()))
            .push(vertical_space(3))
            .push(text(self.error.to_string()).style(theme::Text::Error))
            .push(vertical_space(10))
            .push(
                column![]
                    .width(250)
                    .spacing(4)
                    .push(config_button)
                    .push(refresh_button),
            )
            .align_items(iced::Alignment::Center);

        container(content)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
