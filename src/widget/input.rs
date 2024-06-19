use std::collections::HashMap;

use data::user::User;
use data::{input, isupport, Buffer, Command};
use iced::advanced::widget::{self, Operation};
pub use iced::widget::text_input::{focus, move_cursor_to_end};
use iced::widget::{component, container, row, text, text_input, Component};
use iced::{Length, Rectangle};

use self::completion::Completion;
use super::{anchored_overlay, key_press, Element, Renderer};
use crate::theme::{self, Theme};

mod completion;

pub type Id = text_input::Id;

pub fn input<'a, Message>(
    id: Id,
    buffer: Buffer,
    input: &'a str,
    history: &'a [String],
    users: &'a [User],
    channels: &'a [String],
    isupport: HashMap<isupport::Kind, isupport::Parameter>,
    buffer_focused: bool,
    disabled: bool,
    on_input: impl Fn(input::Draft) -> Message + 'a,
    on_submit: impl Fn(data::Input) -> Message + 'a,
    on_completion: impl Fn(input::Draft) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    Input {
        id,
        buffer,
        input,
        users,
        channels,
        isupport,
        history,
        buffer_focused,
        disabled,
        on_input: Box::new(on_input),
        on_submit: Box::new(on_submit),
        on_completion: Box::new(on_completion),
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
    input: &'a str,
    users: &'a [User],
    channels: &'a [String],
    isupport: HashMap<isupport::Kind, isupport::Parameter>,
    history: &'a [String],
    buffer_focused: bool,
    disabled: bool,
    on_input: Box<dyn Fn(data::input::Draft) -> Message + 'a>,
    on_submit: Box<dyn Fn(data::Input) -> Message + 'a>,
    on_completion: Box<dyn Fn(data::input::Draft) -> Message + 'a>,
}

#[derive(Default)]
pub struct State {
    error: Option<String>,
    completion: Completion,
    selected_history: Option<usize>,
}

impl<'a, Message> Component<Message, Theme, Renderer> for Input<'a, Message>
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

                state
                    .completion
                    .process(&input, self.users, self.channels, &self.isupport);

                Some((self.on_input)(input::Draft {
                    buffer: self.buffer.clone(),
                    text: input,
                }))
            }
            Event::Send => {
                // Reset error state
                state.error = None;
                // Reset selected history
                state.selected_history = None;

                if let Some(entry) = state.completion.select() {
                    let new_input = entry.complete_input(self.input);

                    Some((self.on_completion)(input::Draft {
                        buffer: self.buffer.clone(),
                        text: new_input,
                    }))
                } else if !self.input.is_empty() {
                    state.completion.reset();

                    // Parse input
                    let input = match input::parse(self.buffer.clone(), self.input) {
                        Ok(input) => input,
                        Err(error) => {
                            state.error = Some(error.to_string());
                            return None;
                        }
                    };

                    Some((self.on_submit)(input))
                } else {
                    None
                }
            }
            Event::Tab => {
                if let Some(entry) = state.completion.tab() {
                    let new_input = entry.complete_input(self.input);

                    Some((self.on_completion)(input::Draft {
                        buffer: self.buffer.clone(),
                        text: new_input,
                    }))
                } else {
                    None
                }
            }
            Event::Up => {
                state.completion.reset();

                if !self.history.is_empty() {
                    if let Some(index) = state.selected_history.as_mut() {
                        *index = (*index + 1).min(self.history.len() - 1);
                    } else {
                        state.selected_history = Some(0);
                    }

                    let new_input = self
                        .history
                        .get(state.selected_history.unwrap())
                        .unwrap()
                        .clone();
                    state
                        .completion
                        .process(&new_input, self.users, self.channels, &self.isupport);

                    return Some((self.on_completion)(input::Draft {
                        buffer: self.buffer.clone(),
                        text: new_input,
                    }));
                }

                None
            }
            Event::Down => {
                state.completion.reset();

                if let Some(index) = state.selected_history.as_mut() {
                    let new_input = if *index == 0 {
                        state.selected_history = None;
                        String::new()
                    } else {
                        *index -= 1;
                        let new_input = self.history.get(*index).unwrap().clone();
                        state.completion.process(
                            &new_input,
                            self.users,
                            self.channels,
                            &self.isupport,
                        );
                        new_input
                    };

                    return Some((self.on_completion)(input::Draft {
                        buffer: self.buffer.clone(),
                        text: new_input,
                    }));
                }

                None
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event> {
        let style = if state.error.is_some() {
            theme::text_input::error
        } else {
            theme::text_input::primary
        };

        let mut text_input = text_input("Send message...", self.input)
            .on_submit(Event::Send)
            .id(self.id.clone())
            .padding(8)
            .style(style);

        if !self.disabled {
            text_input = text_input.on_input(Event::Input);
        }

        // Add tab support
        let mut input = key_press(
            text_input,
            key_press::Key::Named(key_press::Named::Tab),
            key_press::Modifiers::default(),
            Event::Tab,
        );

        // Add up / down support for history cycling
        if self.buffer_focused {
            input = key_press(
                key_press(
                    input,
                    key_press::Key::Named(key_press::Named::ArrowUp),
                    key_press::Modifiers::default(),
                    Event::Up,
                ),
                key_press::Key::Named(key_press::Named::ArrowDown),
                key_press::Modifiers::default(),
                Event::Down,
            );
        }

        let overlay = state
            .error
            .as_deref()
            .map(error)
            .or_else(|| state.completion.view(self.input))
            .unwrap_or_else(|| row![].into());

        anchored_overlay(input, overlay, anchored_overlay::Anchor::AboveTop, 4.0)
    }

    fn operate(&self, state: &mut State, operation: &mut dyn widget::Operation<()>) {
        operation.custom(state, Some(&self.id.clone().into()));
    }
}

fn error<'a, 'b, Message: 'a>(error: &'b str) -> Element<'a, Message> {
    container(text(error.to_string()).style(theme::text::error))
        .center_y(Length::Shrink)
        .padding(8)
        .style(theme::container::context)
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

pub fn reset<Message: Send + 'static>(id: impl Into<widget::Id>) -> iced::Task<Message> {
    struct Reset {
        id: widget::Id,
    }

    impl<T> Operation<T> for Reset {
        fn container(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
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

    iced::Task::widget(Reset { id: id.into() })
}
