use data::Config;
use iced::{
    alignment,
    widget::{
        column, container, horizontal_space, opaque, row, stack, text, vertical_space, Rule
    },
    Length, Task, Vector
};

mod buffer;
mod scale_factor;

use crate::window::{self, Window};
use crate::{
    appearance::theme,
    widget::{tooltip, Element},
};

#[derive(Debug, Clone)]
pub enum Message {
    Open(Section),
    ScaleFactor(scale_factor::Message),
}

#[derive(Debug, Clone)]
pub enum Event {}

#[derive(Debug, Clone)]
pub struct Settings {
    pub window: window::Id,
    section: Section,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Buffer,
    ScaleFactor,
}

impl Section {
    fn list() -> Vec<Self> {
        vec![Section::Buffer, Section::ScaleFactor]
    }
}

impl std::fmt::Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Section::Buffer => "Buffer",
                Section::ScaleFactor => "Scale Factor",
            }
        )
    }
}

impl Settings {
    pub fn open(main_window: &Window) -> (Self, Task<window::Id>) {
        let (window, task) = window::open(window::Settings {
            size: iced::Size::new(625.0, 700.0),
            resizable: false,
            position: main_window
                .position
                .map(|point| window::Position::Specific(point + Vector::new(20.0, 20.0)))
                .unwrap_or_default(),
            exit_on_close_request: false,
            ..window::settings()
        });

        (
            Self {
                window,
                section: Section::Buffer,
            },
            task,
        )
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::Open(section) => {
                self.section = section;
            }
            Message::ScaleFactor(message) => match message {
                scale_factor::Message::Change(change) => println!("change {change}"),
            },
        }

        None
    }

    pub fn view<'a>(&self, config: &Config) -> Element<'a, Message> {
        container(row![
            sidebar::view(self.section),
            content::view(config, self.section),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
    }
}

mod content {
    use data::Config;
    use iced::{
        widget::{container, scrollable, Scrollable},
        Length,
    };

    use super::{buffer, scale_factor, Message, Section};

    use crate::{appearance::theme, widget::Element};

    pub fn view<'a>(config: &Config, section: Section) -> Element<'a, Message> {
        container(
            Scrollable::new(
                container(match section {
                    Section::Buffer => buffer::view(),
                    Section::ScaleFactor => scale_factor::view(config).map(Message::ScaleFactor),
                })
                .padding(8),
            )
            .direction(scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::default()
                    .width(0)
                    .scroller_width(0),
            )),
        )
        .style(|theme| theme::container::buffer(theme, false))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
    }
}

mod sidebar {
    use iced::{
        padding,
        widget::{button, container, scrollable, text, Column, Scrollable},
        Length,
    };

    use super::{Message, Section};

    use crate::{appearance::theme, widget::Element};

    pub fn view<'a>(selected: Section) -> Element<'a, Message> {
        let sections = Section::list()
            .into_iter()
            .map(|section| {
                button(text(section.to_string()))
                    .width(Length::Fill)
                    .on_press(Message::Open(section))
                    .padding(padding::left(8).right(8).top(4).bottom(4))
                    .style(move |theme, status| {
                        theme::button::sidebar_buffer(theme, status, false, section == selected)
                    })
                    .into()
            })
            .collect::<Vec<_>>();

        container(
            Scrollable::new(Column::with_children(sections).spacing(1)).direction(
                scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::default()
                        .width(0)
                        .scroller_width(0),
                ),
            ),
        )
        .width(125)
        .padding(padding::right(6).top(1))
        .into()
    }
}

fn wrap_with_disabled<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    disabled: bool,
) -> Element<'a, Message> {
    if disabled {
        stack![
            content.into(),
            tooltip(
                opaque(
                    container(vertical_space())
                        .style(theme::container::disabled_setting)
                        .width(Length::Fill),
                ),
                Some("Disabled. Configuration is defined in local config."),
                iced::widget::tooltip::Position::Left,
            )
        ]
        .into()
    } else {
        content.into()
    }
}

pub fn setting_row<'a, Message: 'a>(
    title: &'a str,
    description: &'a str,
    content: impl Into<Element<'a, Message>>,
    is_disabled: bool,
) -> Element<'a, Message> {
    column![
        row![
            column![
                text(title),
                text(description).style(theme::text::secondary),
            ]
            .max_width(200)
            .spacing(2),
            horizontal_space(),
            wrap_with_disabled(content.into(), is_disabled),
        ]
        .align_y(alignment::Vertical::Center),
        Rule::horizontal(1)
    ]
    .spacing(8)
    .into()
}
