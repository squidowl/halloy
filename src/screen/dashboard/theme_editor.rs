use iced::{color, widget::container, Color, Length};
use iced::{Task, Vector};

use crate::widget::{color_picker, Element};
use crate::window::{self, Window};

#[derive(Debug, Clone)]
pub enum Message {
    Color(Color),
}

#[derive(Debug, Clone)]
pub struct ThemeEditor {
    pub id: window::Id,
    color: Color,
}

impl ThemeEditor {
    pub fn open(main_window: &Window) -> (Self, Task<window::Id>) {
        let (id, task) = window::open(window::Settings {
            size: iced::Size::new(300.0, 200.0),
            position: main_window
                .position
                .map(|point| window::Position::Specific(point + Vector::new(20.0, 20.0)))
                .unwrap_or_default(),
            exit_on_close_request: false,
            ..window::settings()
        });

        (
            Self {
                id,
                color: color!(0x00FF00),
            },
            task,
        )
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
