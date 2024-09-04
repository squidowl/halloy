use iced::{color, widget::container, Color, Length};

use crate::widget::{color_picker, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Color(Color),
}

#[derive(Debug, Clone)]
pub struct ThemeEditor {
    color: Color,
}

impl Default for ThemeEditor {
    fn default() -> Self {
        Self {
            color: color!(0x00FF00),
        }
    }
}

impl ThemeEditor {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Color(color) => self.color = color,
        }
    }

    pub fn view(&self) -> Element<Message> {
        container(color_picker(self.color, Message::Color))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20.0)
            .into()
    }
}
