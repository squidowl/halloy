use data::shortcut::KeyBind;
use iced::{keyboard, subscription, window, Subscription};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    CloseRequested,
    Copy,
    Escape,
    Home,
    End,
    KeyBind(KeyBind),
}

pub fn events() -> Subscription<Event> {
    subscription::events_with(filtered_events)
}

fn filtered_events(event: iced::Event, status: iced::event::Status) -> Option<Event> {
    let ignored = |status| matches!(status, iced::event::Status::Ignored);

    match &event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key_code: keyboard::KeyCode::Escape,
            ..
        }) => Some(Event::Escape),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key_code: keyboard::KeyCode::C,
            modifiers,
        }) if modifiers.command() => Some(Event::Copy),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key_code: keyboard::KeyCode::Home,
            ..
        }) if ignored(status) => Some(Event::Home),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key_code: keyboard::KeyCode::End,
            ..
        }) if ignored(status) => Some(Event::End),
        iced::Event::Window(window::Event::CloseRequested) => Some(Event::CloseRequested),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key_code,
            modifiers,
        }) => Some(Event::KeyBind(KeyBind::from((*key_code, *modifiers)))),
        _ => None,
    }
}
