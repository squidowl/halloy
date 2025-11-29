use data::{Config, channel_list};
use iced::widget::{container, text};
use iced::{Size, Task};

use crate::Theme;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {}

pub enum Event {}

#[derive(Debug, Clone)]
pub struct ChannelList {}

impl ChannelList {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(
        &mut self,
        _message: Message,
        _config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        (Task::none(), None)
    }
}

pub fn view<'a>(
    _state: &'a ChannelList,
    manager: &'a channel_list::Manager,
    _config: &'a Config,
    _theme: &'a Theme,
) -> Element<'a, Message> {
    container(text("List")).into()
}
