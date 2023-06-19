use iced::{keyboard, subscription, window, Subscription};

#[derive(Debug, Clone, Copy)]
pub enum Event {
    CloseRequested,
    Copy,
    Escape,
    Home,
    End,
}

pub fn events() -> Subscription<Event> {
    subscription::events_with(filtered_events)
}

fn filtered_events(event: iced::Event, _status: iced::event::Status) -> Option<Event> {
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
            modifiers,
        }) if modifiers.command() => Some(Event::Home),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key_code: keyboard::KeyCode::End,
            modifiers,
        }) if modifiers.command() => Some(Event::End),
        iced::Event::Window(window::Event::CloseRequested) => Some(Event::CloseRequested),
        _ => None,
    }
}
