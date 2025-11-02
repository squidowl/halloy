use data::Config;
use data::environment::WIKI_WEBSITE;
use iced::widget::{button, column, container, image, row, space, text};
use iced::{Length, alignment};

use crate::widget::Element;
use crate::{Theme, font, theme};

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
pub struct Welcome {
    logo: image::Handle,
}

impl Welcome {
    pub fn new() -> Self {
        // Create initial config file.
        Config::create_initial_config();

        let logo_bytes = include_bytes!("../../assets/logo.png").to_vec();
        let logo = image::Handle::from_bytes(logo_bytes);

        Welcome { logo }
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::RefreshConfiguration => Some(Event::RefreshConfiguration),
            Message::OpenConfigurationDirectory => {
                let _ = open::that_detached(Config::config_dir());

                None
            }
            Message::OpenWikiWebsite => {
                let _ = open::that_detached(WIKI_WEBSITE);

                None
            }
        }
    }

    pub fn view<'a>(&self, theme: &Theme) -> Element<'a, Message> {
        let config_dir = String::from(Config::config_dir().to_string_lossy());

        let config_button = button(
            container(text(config_dir))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Shrink),
        )
        .padding([5, 20])
        .width(Length::Shrink)
        .style(|theme, status| theme::button::secondary(theme, status, false))
        .on_press(Message::OpenConfigurationDirectory);

        let documentation_button = button(
            container(text("Open Documentation Website"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fill)
        .style(|theme, status| theme::button::secondary(theme, status, false))
        .on_press(Message::OpenWikiWebsite);

        let reload_button = button(
            container(text("Reload Config File"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fill)
        .style(|theme, status| theme::button::secondary(theme, status, false))
        .on_press(Message::RefreshConfiguration);

        let content = column![]
            .spacing(1)
            .push(image(self.logo.clone()).width(150))
            .push(space::vertical().height(10))
            .push(text("Welcome to Halloy!").font(font::MONO_BOLD.clone()))
            .push(space::vertical().height(4))
            .push(text("Halloy is configured through a config file."))
            .push(row![
                text("You can find the "),
                text("config.toml").style(theme::text::action).font_maybe(
                    theme::font_style::action(theme).map(font::get)
                ),
                text(" file at the following path:"),
            ])
            .push(space::vertical().height(8))
            .push(config_button)
            .push(space::vertical().height(10))
            .push(text("To begin and view config options, see below."))
            .push(space::vertical().height(10))
            .push(
                column![]
                    .width(250)
                    .spacing(4)
                    .push(documentation_button)
                    .push(reload_button),
            )
            .align_x(iced::Alignment::Center);

        container(content)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
