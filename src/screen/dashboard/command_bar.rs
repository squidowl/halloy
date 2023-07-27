use iced::widget::{column, combo_box, text};

use crate::theme;
use crate::widget::{double_pass, Element};

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
        double_pass(
            // Layout should be based on the Shrink text size width of largest option
            column(
                std::iter::once(text("Type a command..."))
                    .chain(Command::list().iter().map(ToString::to_string).map(text))
                    .map(Element::from)
                    .collect(),
            )
            // Give it some extra width
            .padding([0, 20]),
            combo_box(&self.state, "Type a command...", None, Message::Command)
                .on_close(Message::Unfocused)
                .style(theme::ComboBox::Default)
                .padding([4, 8]),
        )
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
