use data::{command, Command};
pub use iced::widget::text_input::focus;
use iced::widget::{component, text_input, Component};

use crate::theme;

use super::{Element, Renderer};

pub type Id = text_input::Id;

pub fn input<'a, Message>(
    id: Id,
    on_submit: impl Fn(Content) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: 'a,
{
    Input {
        id,
        on_submit: Box::new(on_submit),
    }
    .into()
}

#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Command(Command),
}

#[derive(Debug, Clone)]
pub enum Event {
    Input(String),
    Send,
}

pub struct Input<'a, Message> {
    id: Id,
    on_submit: Box<dyn Fn(Content) -> Message + 'a>,
}

#[derive(Default)]
pub struct State {
    input: String,
    error: Option<String>,
}

impl<'a, Message> Component<Message, Renderer> for Input<'a, Message> {
    type State = State;
    type Event = Event;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<Message> {
        match event {
            Event::Input(input) => {
                state.error = None;
                state.input = input;
                None
            }
            Event::Send => {
                // Reset error state
                state.error = None;

                // Parse message
                let content = match state.input.parse::<Command>() {
                    Ok(command) => Content::Command(command),
                    Err(command::Error::MissingSlash) => Content::Text(state.input.clone()),
                    // TODO: Implement error handling
                    Err(error) => {
                        state.error = Some(error.to_string());
                        return None;
                    }
                };

                // Clear message, we parsed it succesfully
                state.input = String::new();

                Some((self.on_submit)(content))
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event> {
        let style = if state.error.is_some() {
            theme::TextInput::Error
        } else {
            theme::TextInput::Default
        };

        text_input("Send message...", &state.input)
            .on_input(Event::Input)
            .on_submit(Event::Send)
            .id(self.id.clone())
            .padding(8)
            .style(style)
            .into()
    }
}

impl<'a, Message> From<Input<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(input: Input<'a, Message>) -> Self {
        component(input)
    }
}
