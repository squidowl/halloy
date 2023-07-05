use std::collections::VecDeque;

use data::{input, Buffer, Command};
use iced::advanced::widget::{self, Operation};
pub use iced::widget::text_input::{focus, move_cursor_to_end};
use iced::widget::{component, container, row, text, text_input, Component};

use self::completion::Completion;
use super::{anchored_overlay, key_press, Element, Renderer};
use crate::theme;

mod completion;

pub const HISTORY_LENGTH: usize = 100;

pub type Id = text_input::Id;

pub fn input<'a, Message>(
    id: Id,
    buffer: Buffer,
    on_submit: impl Fn(data::Input) -> Message + 'a,
    on_completion: Message,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    Input {
        id,
        buffer,
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
    Up,
    Down,
}

pub struct Input<'a, Message> {
    id: Id,
    buffer: Buffer,
    on_submit: Box<dyn Fn(data::Input) -> Message + 'a>,
    on_completion: Message,
}

#[derive(Default)]
pub struct State {
    input: String,
    error: Option<String>,
    completion: Completion,
    history: VecDeque<String>,
    selected_history: Option<usize>,
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
                // Reset error state
                state.error = None;
                // Reset selected history
                state.selected_history = None;

                state.input = input;

                state.completion.process(&state.input);

                None
            }
            Event::Send => {
                // Reset error state
                state.error = None;
                // Reset selected history
                state.selected_history = None;

                if let Some(command) = state.completion.select() {
                    state.input = command;
                    Some(self.on_completion.clone())
                } else if !state.input.is_empty() {
                    state.completion.reset();

                    // Parse input
                    let input = match input::parse(self.buffer.clone(), &state.input) {
                        Ok(input) => input,
                        Err(error) => {
                            state.error = Some(error.to_string());
                            return None;
                        }
                    };

                    // Clear message and add it to history
                    state.history.push_front(std::mem::take(&mut state.input));
                    state.history.truncate(HISTORY_LENGTH);

                    Some((self.on_submit)(input))
                } else {
                    None
                }
            }
            Event::Tab => {
                state.completion.tab();
                None
            }
            Event::Up => {
                state.completion.reset();

                if !state.history.is_empty() {
                    if let Some(index) = state.selected_history.as_mut() {
                        *index = (*index + 1).min(state.history.len() - 1);
                    } else {
                        state.selected_history = Some(0);
                    }

                    state.input = state
                        .history
                        .get(state.selected_history.unwrap())
                        .unwrap()
                        .clone();
                    state.completion.process(&state.input);

                    return Some(self.on_completion.clone());
                }

                None
            }
            Event::Down => {
                state.completion.reset();

                if let Some(index) = state.selected_history.as_mut() {
                    if *index == 0 {
                        state.selected_history = None;
                        state.input.clear();
                    } else {
                        *index -= 1;
                        state.input = state.history.get(*index).unwrap().clone();
                        state.completion.process(&state.input);
                    }

                    return Some(self.on_completion.clone());
                }

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

        // Add tab support if selecting a completion
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

        // Add up / down support for history cycling
        let input = key_press(
            key_press(
                input,
                key_press::KeyCode::Up,
                key_press::Modifiers::default(),
                Event::Up,
            ),
            key_press::KeyCode::Down,
            key_press::Modifiers::default(),
            Event::Down,
        );

        let overlay = state
            .error
            .as_ref()
            .map(error)
            .or_else(|| state.completion.view(&state.input))
            .unwrap_or_else(|| row![].into());

        anchored_overlay(input, overlay)
    }

    fn operate(&self, state: &mut State, operation: &mut dyn widget::Operation<Message>) {
        operation.custom(state, Some(&self.id.clone().into()));
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

pub fn reset<Message: 'static>(id: impl Into<widget::Id>) -> iced::Command<Message> {
    struct Reset {
        id: widget::Id,
    }

    impl<T> Operation<T> for Reset {
        fn container(
            &mut self,
            _id: Option<&widget::Id>,
            operate_on_children: &mut dyn FnMut(&mut dyn Operation<T>),
        ) {
            operate_on_children(self)
        }

        fn custom(&mut self, state: &mut dyn std::any::Any, id: Option<&widget::Id>) {
            if Some(&self.id) == id {
                if let Some(state) = state.downcast_mut::<State>() {
                    *state = State::default();
                }
            }
        }
    }

    iced::Command::widget(Reset { id: id.into() })
}
