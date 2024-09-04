use data::Config;
use iced::widget::{column, container, text};
use iced::{alignment, Length};

use crate::screen::dashboard::sidebar;
use crate::widget::Element;

pub fn view<'a, Message: 'a>(
    config: &'a Config,
    sidebar: &'a sidebar::Sidebar,
) -> Element<'a, Message> {
    let arrow = if sidebar.hidden {
        ' '
    } else {
        match config.sidebar.position {
            data::config::sidebar::Position::Left => '⟵',
            data::config::sidebar::Position::Right => '⟶',
            data::config::sidebar::Position::Top => '↑',
            data::config::sidebar::Position::Bottom => '↓',
        }
    };

    let content = column![]
        .push(text(format!("{arrow} select buffer")).shaping(text::Shaping::Advanced))
        .align_x(iced::Alignment::Center);

    container(content)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
