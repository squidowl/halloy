use crate::widget::Element;
use iced::widget::combo_box;

#[derive(Debug, Clone)]
pub struct CommandBar {
    input: combo_box::State<Command>,
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            input: combo_box::State::new(Command::list()),
        }
    }

    pub fn view<Message>(&self, on_command: fn(Command) -> Message) -> Element<Message>
    where
        Message: 'static + Clone,
    {
        combo_box(&self.input, "placeholder", None, move |x| on_command(x)).into()
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    OpenConfig,
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::OpenConfig => write!(f, "Configuration - Open Directory"),
        }
    }
}

impl Command {
    pub fn list() -> Vec<Self> {
        vec![Command::OpenConfig]
    }
}
