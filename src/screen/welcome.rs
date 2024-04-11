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
        // Create initial config file.
        Config::create_initial_config();

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
            container(text(config_dir))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Shrink),
        )
        .padding([5, 20])
        .width(Length::Shrink)
        .style(theme::button::secondary)
        .on_press(Message::OpenConfigurationDirectory);

        let documentation_button = button(
            container(text("Open Documentation Website"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fill)
        .style(theme::button::primary)
        .on_press(Message::OpenWikiWebsite);

        let reload_button = button(
            container(text("Reload Config File"))
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
            .push(text("Halloy is configured through a config file."))
            .push(
                row![
                        text("You can find the "),
                        text("config.toml").style(theme::text::info),
                        text(" file at the following path:"),
                    ]
            )
            .push(vertical_space().height(8))
            .push(config_button)
            .push(vertical_space().height(10))
            .push(text("To begin and view config options, see below."))
            .push(vertical_space().height(10))
            .push(
                column![]
                    .width(250)
                    .spacing(4)
                    .push(documentation_button)
                    .push(reload_button),
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
