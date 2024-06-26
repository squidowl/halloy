use data::environment::WIKI_WEBSITE;
use data::{config, Config};
use iced::widget::{button, column, container, text, vertical_space};
use iced::{alignment, Length};

use crate::widget::Element;
use crate::{icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    RefreshConfiguration,
    OpenConfigurationDirectory,
    OpenWikiWebsite,
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
            Message::OpenWikiWebsite => {
                let _ = open::that(WIKI_WEBSITE);

                None
            }
        }
    }

    pub fn view<'a>(&self) -> Element<'a, Message> {
        let config_button = button(
            container(text("Open Config Directory"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fixed(250.0))
        .style(theme::button::secondary)
        .on_press(Message::OpenConfigurationDirectory);

        let wiki_button = button(
            container(text("Open Wiki Website"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fill)
        .style(theme::button::secondary)
        .on_press(Message::OpenWikiWebsite);

        let refresh_button = button(
            container(text("Refresh Halloy"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fixed(250.0))
        .style(theme::button::primary)
        .on_press(Message::RefreshConfiguration);

        let content = column![]
            .push(icon::error().style(theme::text::error).size(35))
            .push(vertical_space().height(10))
            .push(text("Error reading configuration file"))
            .push(vertical_space().height(10))
            .push(text(self.error.to_string()).style(theme::text::error))
            .push(vertical_space().height(10))
            .push(
                column![]
                    .width(250)
                    .spacing(4)
                    .push(config_button)
                    .push(wiki_button)
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
