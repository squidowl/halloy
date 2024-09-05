use iced::{event, keyboard, window, Subscription};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Copy,
    Escape,
    Home,
    End,
}

pub fn events() -> Subscription<(window::Id, Event)> {
    event::listen_with(filtered_events)
}

fn filtered_events(
    event: iced::Event,
    status: iced::event::Status,
    window: window::Id,
) -> Option<(window::Id, Event)> {
    let ignored = |status| matches!(status, iced::event::Status::Ignored);

    let event = match &event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            ..
        }) => Some(Event::Escape),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(c),
            modifiers,
            ..
        }) if c.as_str() == "c" && modifiers.command() => Some(Event::Copy),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Home),
            ..
        }) if ignored(status) => Some(Event::Home),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::End),
            ..
        }) if ignored(status) => Some(Event::End),
        _ => None,
    };

    event.map(|event| (window, event))
}
