use iced::advanced::graphics::futures::MaybeSend;
use iced::widget::text_editor;
use iced::{Task, clipboard};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kill {
    WordForward,
    WordBackward,
    ToEnd,
    ToStart,
}

impl Kill {
    pub fn motion(self) -> text_editor::Motion {
        match self {
            Self::WordForward => text_editor::Motion::WordRight,
            Self::WordBackward => text_editor::Motion::WordLeft,
            Self::ToEnd => text_editor::Motion::End,
            Self::ToStart => text_editor::Motion::Home,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Emacs {
    Move(text_editor::Motion),
    Select(text_editor::Motion),
    Delete,
    Kill(Kill),
}

impl Emacs {
    fn into_binding<Message>(
        self,
        kill_binding: impl FnOnce(Kill) -> text_editor::Binding<Message>,
    ) -> text_editor::Binding<Message> {
        match self {
            Self::Move(motion) => text_editor::Binding::Move(motion),
            Self::Select(motion) => text_editor::Binding::Select(motion),
            Self::Delete => text_editor::Binding::Delete,
            Self::Kill(kill) => kill_binding(kill),
        }
    }
}

pub fn emacs<Message>(
    key_press: &text_editor::KeyPress,
    kill_binding: impl FnOnce(Kill) -> text_editor::Binding<Message>,
) -> Option<text_editor::Binding<Message>> {
    emacs_action(key_press).map(|action| action.into_binding(kill_binding))
}

/// Word/line deletion shortcuts (alt/cmd/ctrl + backspace/delete).
pub fn platform_kill<Message>(
    key_press: &text_editor::KeyPress,
    has_selection: bool,
    kill_binding: impl FnOnce(Kill) -> text_editor::Binding<Message>,
) -> Option<text_editor::Binding<Message>> {
    platform_kill_action(key_press, has_selection).map(kill_binding)
}

#[cfg(target_os = "macos")]
fn platform_kill_action(
    key_press: &text_editor::KeyPress,
    has_selection: bool,
) -> Option<Kill> {
    use iced::keyboard::key::Named;

    match key_press.key.as_ref() {
        iced::keyboard::Key::Named(Named::Backspace)
            if key_press.modifiers.alt() && !has_selection =>
        {
            Some(Kill::WordBackward)
        }
        iced::keyboard::Key::Named(Named::Backspace)
            if key_press.modifiers.logo() && !has_selection =>
        {
            Some(Kill::ToStart)
        }
        iced::keyboard::Key::Named(Named::Delete)
            if key_press.modifiers.alt() =>
        {
            Some(Kill::WordForward)
        }
        iced::keyboard::Key::Named(Named::Delete)
            if key_press.modifiers.logo() =>
        {
            Some(Kill::ToEnd)
        }
        _ => None,
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_kill_action(
    key_press: &text_editor::KeyPress,
    has_selection: bool,
) -> Option<Kill> {
    use iced::keyboard::key::Named;

    match key_press.key.as_ref() {
        iced::keyboard::Key::Named(Named::Backspace)
            if key_press.modifiers.control() && !has_selection =>
        {
            Some(if key_press.modifiers.shift() {
                Kill::ToStart
            } else {
                Kill::WordBackward
            })
        }
        iced::keyboard::Key::Named(Named::Delete)
            if key_press.modifiers.control() =>
        {
            Some(if key_press.modifiers.shift() {
                Kill::ToEnd
            } else {
                Kill::WordForward
            })
        }
        _ => None,
    }
}

pub fn perform_kill<T>(
    content: &mut text_editor::Content,
    kill: Kill,
    save_to_clipboard: bool,
    kill_to_clipboard: bool,
) -> Task<T>
where
    T: MaybeSend + 'static,
{
    content.perform(text_editor::Action::Select(kill.motion()));

    let task = if save_to_clipboard && kill_to_clipboard {
        content.selection().map_or_else(Task::none, |content| {
            clipboard::write(content).discard()
        })
    } else {
        Task::none()
    };

    content.perform(text_editor::Action::Edit(text_editor::Edit::Delete));

    task
}

fn emacs_action(key_press: &text_editor::KeyPress) -> Option<Emacs> {
    match key_press.key.as_ref() {
        iced::keyboard::Key::Character("e")
            if key_press.modifiers.control() =>
        {
            Some(move_or_select(key_press, text_editor::Motion::End))
        }
        iced::keyboard::Key::Character("a")
            if key_press.modifiers.control() =>
        {
            Some(move_or_select(key_press, text_editor::Motion::Home))
        }
        iced::keyboard::Key::Character("b") if key_press.modifiers.alt() => {
            Some(move_or_select(key_press, text_editor::Motion::WordLeft))
        }
        iced::keyboard::Key::Character("b")
            if key_press.modifiers.control() =>
        {
            Some(move_or_select(key_press, text_editor::Motion::Left))
        }
        iced::keyboard::Key::Character("f") if key_press.modifiers.alt() => {
            Some(move_or_select(key_press, text_editor::Motion::WordRight))
        }
        iced::keyboard::Key::Character("f")
            if key_press.modifiers.control() =>
        {
            Some(move_or_select(key_press, text_editor::Motion::Right))
        }
        iced::keyboard::Key::Character("d")
            if key_press.modifiers.control() =>
        {
            Some(Emacs::Delete)
        }
        iced::keyboard::Key::Character("d") if key_press.modifiers.alt() => {
            Some(Emacs::Kill(Kill::WordForward))
        }
        iced::keyboard::Key::Character("k")
            if key_press.modifiers.control() =>
        {
            Some(Emacs::Kill(Kill::ToEnd))
        }
        iced::keyboard::Key::Character("u")
            if key_press.modifiers.control() =>
        {
            Some(Emacs::Kill(Kill::ToStart))
        }
        iced::keyboard::Key::Character("w")
            if key_press.modifiers.control() =>
        {
            Some(Emacs::Kill(Kill::WordBackward))
        }
        _ => None,
    }
}

fn move_or_select(
    key_press: &text_editor::KeyPress,
    motion: text_editor::Motion,
) -> Emacs {
    if key_press.modifiers.shift() {
        Emacs::Select(motion)
    } else {
        Emacs::Move(motion)
    }
}
