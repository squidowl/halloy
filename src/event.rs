use iced::{event, keyboard, window, Subscription};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    CloseRequested(window::Id),
    Copy,
    Escape,
    Home,
    End,
    Focused(window::Id),
    Unfocused(window::Id),
}

pub fn events() -> Subscription<Event> {
    event::listen_with(filtered_events)
}

fn filtered_events(
    event: iced::Event,
    status: iced::event::Status,
    window: window::Id,
) -> Option<Event> {
    let ignored = |status| matches!(status, iced::event::Status::Ignored);

    match &event {
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
        iced::Event::Window(window::Event::CloseRequested) => Some(Event::CloseRequested(window)),
        iced::Event::Window(window::Event::Focused) => Some(Event::Focused(window)),
        iced::Event::Window(window::Event::Unfocused) => Some(Event::Unfocused(window)),
        _ => None,
    }
}
