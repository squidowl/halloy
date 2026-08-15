use data::shortcut::KeyBind;
use iced::{Subscription, event, keyboard, mouse, window};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Copy,
    Escape,
    LeftClick,
    ResetFocus,
    UpdatePrimaryClipboard,
    /// An ignored key press for the dashboard to match on
    Key(KeyBind),
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

    let recognized = match &event {
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
        // Any other mouse press (a captured left click, or a right click)
        // exits message focus
        iced::Event::Mouse(mouse::Event::ButtonPressed {
            button: mouse::Button::Left | mouse::Button::Right,
            ..
        }) => Some(Event::ResetFocus),
        // Forward ignored key presses so the dashboard can match them
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modifiers,
            ..
        }) if ignored(status) => {
            Some(Event::Key(KeyBind::from((key.clone(), *modifiers))))
        }
        _ => None,
    };

    recognized.map(|event| (window, event))
}
