pub use iced::widget::text_input::focus;
use iced::widget::{component, text_input, Component};

use super::{Element, Renderer};

pub type Id = text_input::Id;

mod command;

pub fn input<'a, Message>(
    id: Id,
    on_submit: impl Fn(String) -> Message + 'a,
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
pub enum Event {
    Input(String),
    Send,
}

pub struct Input<'a, Message> {
    id: Id,
    on_submit: Box<dyn Fn(String) -> Message + 'a>,
}

#[derive(Default)]
pub struct State {
    input: String,
}

impl<'a, Message> Component<Message, Renderer> for Input<'a, Message> {
    type State = State;
    type Event = Event;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<Message> {
        match event {
            Event::Input(input) => {
                state.input = input;
                None
            }
            Event::Send => {
                let input = std::mem::take(&mut state.input);
                Some((self.on_submit)(input))
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event> {
        text_input("Send message...", &state.input)
            .on_input(Event::Input)
            .on_submit(Event::Send)
            .id(self.id.clone())
            .padding(8)
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
