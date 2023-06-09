use data::{command, Command};
pub use iced::widget::text_input::{focus, move_cursor_to_end};
use iced::widget::{component, container, row, text, text_input, Component};

use self::completion::Completion;
use super::{anchored_overlay, key_press, Element, Renderer};
use crate::theme;

mod completion;

pub type Id = text_input::Id;

pub fn input<'a, Message>(
    id: Id,
    on_submit: impl Fn(Content) -> Message + 'a,
    on_completion: Message,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    Input {
        id,
        on_submit: Box::new(on_submit),
        on_completion,
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
    Tab,
}

pub struct Input<'a, Message> {
    id: Id,
    on_submit: Box<dyn Fn(Content) -> Message + 'a>,
    on_completion: Message,
}

#[derive(Default)]
pub struct State {
    input: String,
    error: Option<String>,
    completion: Completion,
}

impl<'a, Message> Component<Message, Renderer> for Input<'a, Message>
where
    Message: Clone,
{
    type State = State;
    type Event = Event;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<Message> {
        match event {
            Event::Input(input) => {
                state.error = None;
                state.input = input;

                state.completion.process(&state.input);

                None
            }
            Event::Send => {
                // Reset error state
                state.error = None;

                if let Some(command) = state.completion.select() {
                    state.input = command;
                    Some(self.on_completion.clone())
                } else {
                    state.completion.reset();

                    // Parse message
                    let content = match state.input.parse::<Command>() {
                        Ok(command) => Content::Command(command),
                        Err(command::Error::MissingSlash) => Content::Text(state.input.clone()),
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
            Event::Tab => {
                state.completion.tab();
                None
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event> {
        let style = if state.error.is_some() {
            theme::TextInput::Error
        } else {
            theme::TextInput::Default
        };

        let text_input = text_input("Send message...", &state.input)
            .on_input(Event::Input)
            .on_submit(Event::Send)
            .id(self.id.clone())
            .padding(8)
            .style(style)
            .into();

        let input = if state.completion.is_selecting() {
            key_press(
                text_input,
                key_press::KeyCode::Tab,
                key_press::Modifiers::default(),
                Event::Tab,
            )
        } else {
            text_input
        };

        let overlay = state
            .error
            .as_ref()
            .map(error)
            .or_else(|| state.completion.view(&state.input))
            .unwrap_or_else(|| row![].into());

        anchored_overlay(input, overlay)
    }
}

fn error<'a, Message: 'a>(error: impl ToString) -> Element<'a, Message> {
    container(text(error).style(theme::Text::Error))
        .center_y()
        .padding(8)
        .style(theme::Container::Primary)
        .into()
}

impl<'a, Message> From<Input<'a, Message>> for Element<'a, Message>
where
    Message: 'a + Clone,
{
    fn from(input: Input<'a, Message>) -> Self {
        component(input)
    }
}
