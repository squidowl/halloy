use iced::widget::combo_box;

use crate::theme;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub struct CommandBar {
    state: combo_box::State<Command>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Command(Command),
    Unfocused,
}

impl CommandBar {
    pub fn new() -> Self {
        let state = combo_box::State::new(Command::list());
        state.focus();

        Self { state }
    }

    pub fn update(&mut self, message: Message) -> Event {
        match message {
            Message::Command(command) => Event::Command(command),
            Message::Unfocused => Event::Unfocused,
        }
    }

    pub fn view(&self) -> Element<Message> {
        combo_box(&self.state, "Type a command...", None, Message::Command)
            .on_close(Message::Unfocused)
            .style(theme::ComboBox::Default)
            .padding([4, 8])
            .into()
    }
}

pub enum Event {
    Command(Command),
    Unfocused,
}

#[derive(Debug, Clone)]
pub enum Command {
    OpenConfig,
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::OpenConfig => write!(f, "Configuration: Open Directory"),
        }
    }
}

impl Command {
    pub fn list() -> Vec<Self> {
        vec![Command::OpenConfig]
    }
}
