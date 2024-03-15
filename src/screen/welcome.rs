use data::environment::WIKI_WEBSITE;
use data::Config;
use iced::widget::{button, column, container, image, row, text, vertical_space};
use iced::{alignment, Length};

use crate::widget::Element;
use crate::{font, theme};

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

#[derive(Debug, Default, Clone)]
pub struct Welcome;

impl Welcome {
    pub fn new() -> Self {
        // Create template config file.
        Config::create_template_config();

        Welcome
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
        let config_dir = String::from(Config::config_dir().to_string_lossy());

        let config_button = button(
            container(text("Open Config Directory"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fill)
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
        .width(Length::Fill)
        .style(theme::button::primary)
        .on_press(Message::RefreshConfiguration);

        let content = column![]
            .spacing(1)
            .push(
                image(image::Handle::from_memory(include_bytes!(
                    "../../assets/logo.png"
                )))
                .width(150),
            )
            .push(vertical_space().height(10))
            .push(text("Welcome to Halloy!").font(font::MONO_BOLD.clone()))
            .push(vertical_space().height(4))
            .push(text(
                "To get started with, simply follow the steps below",
            ))
            .push(vertical_space().height(8))
            .push(
                column![]
                    .push(row![
                        text("1. ").style(theme::text::accent),
                        text("Go to "),
                        text(config_dir).style(theme::text::info)
                    ])
                    .push(row![
                        text("2. ").style(theme::text::accent),
                        text("Create "),
                        text("config.toml").style(theme::text::info),
                        text(" using "),
                        text("config.template.toml").style(theme::text::info),
                        text(" as a base"),
                    ])
                    .push(row![
                        text("3. ").style(theme::text::accent),
                        text("Join "),
                        text("#halloy").style(theme::text::info),
                        text(" on "),
                        text("libera.chat").style(theme::text::info),
                        text(" and say hello"),
                    ])
                    .spacing(2)
                    .align_items(iced::Alignment::Start),
            )
            .push(vertical_space().height(10))
            .push(text(
                "For more information, please visit the Wiki website",
            ))
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
