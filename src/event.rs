use iced::{Subscription, event, keyboard, mouse, window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Copy,
    Escape,
    LeftClick,
    UpdatePrimaryClipboard,
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
        iced::Event::Mouse(mouse::Event::ButtonPressed {
            button: mouse::Button::Left,
            ..
        }) if ignored(status) => Some(Event::LeftClick),
        iced::Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        )) if cfg!(target_os = "linux") && ignored(status) => {
            Some(Event::UpdatePrimaryClipboard)
        }
        _ => None,
    };

    event.map(|event| (window, event))
}
