use data::Config;
use iced::widget::{button, column, container, image, row, text, vertical_space};
use iced::{alignment, Length};

use crate::widget::Element;
use crate::{font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    RefreshConfiguration,
    OpenConfigurationDirectory,
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

        Welcome::default()
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::RefreshConfiguration => Some(Event::RefreshConfiguration),
            Message::OpenConfigurationDirectory => {
                // Open config directory.
                let _ = open::that(Config::config_dir());

                None
            }
        }
    }

    pub fn view<'a>(&self) -> Element<'a, Message> {
        let config_dir = String::from(Config::config_dir().to_string_lossy());

        let config_button = button(
            container(text("Open Directory"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .style(theme::Button::Secondary)
        .on_press(Message::OpenConfigurationDirectory);

        let refresh_button = button(
            container(text("Refresh"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .style(theme::Button::Secondary)
        .on_press(Message::RefreshConfiguration);

        let content = column![]
            .spacing(1)
            .push(
                image(image::Handle::from_memory(include_bytes!(
                    "../../assets/logo.png"
                )))
                .width(150),
            )
            .push(vertical_space(10))
            .push(text("Welcome to Halloy!").font(font::MONO_BOLD.clone()))
            .push(vertical_space(4))
            .push(text(
                "No configuration file found. Please follow the steps below to proceed",
            ))
            .push(vertical_space(8))
            .push(
                column![]
                    .push(row![
                        text("1. ").style(theme::Text::Accent),
                        text("Go to "),
                        text(config_dir).style(theme::Text::Info)
                    ])
                    .push(row![
                        text("2. ").style(theme::Text::Accent),
                        text("Create "),
                        text("config.yaml").style(theme::Text::Info),
                        text(" - you can use "),
                        text("config.template.yaml").style(theme::Text::Info),
                        text(" as a starting point"),
                    ])
                    .push(row![
                        text("3. ").style(theme::Text::Accent),
                        text("Customize the file with your preferred servers, settings, and theme")
                    ])
                    .push(row![
                        text("4. ").style(theme::Text::Accent),
                        text("Join "),
                        text("#halloy").style(theme::Text::Info),
                        text(" on libera.chat if you have questions or looking for help"),
                    ])
                    .spacing(2)
                    .align_items(iced::Alignment::Start),
            )
            .push(vertical_space(10))
            .push(
                row![]
                    .width(250)
                    .spacing(4)
                    .push(config_button)
                    .push(refresh_button),
            )
            .align_items(iced::Alignment::Center);

        container(container(content).width(510))
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
