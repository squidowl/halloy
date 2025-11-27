use iced::advanced::{Clipboard, Layout, Shell, widget};
pub use iced::keyboard::key::{self, Named, Physical};
pub use iced::keyboard::{Key, Modifiers};
use iced::{Event, Rectangle, keyboard, mouse};

use super::{Element, Renderer, decorate};

pub fn key_press<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    key: Key,
    modifiers: Modifiers,
    on_press: Message,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    decorate(base)
        .update(
            move |_state: &mut (),
                  inner: &mut Element<'a, Message>,
                  tree: &mut widget::Tree,
                  event: &Event,
                  layout: Layout<'_>,
                  cursor: mouse::Cursor,
                  renderer: &Renderer,
                  clipboard: &mut dyn Clipboard,
                  shell: &mut Shell<'_, Message>,
                  viewport: &Rectangle| {
                if let Event::Keyboard(keyboard::Event::KeyPressed {
                    key: k,
                    modifiers: m,
                    ..
                }) = &event
                    && key == *k
                    && modifiers == *m
                {
                    shell.publish(on_press.clone());
                    shell.capture_event();
                    return;
                }

                inner.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, clipboard, shell,
                    viewport,
                );
            },
        )
        .into()
}

pub fn is_numpad(physical_key: &Physical) -> bool {
    matches!(
        physical_key,
        Physical::Code(key::Code::Numpad0)
            | Physical::Code(key::Code::Numpad1)
            | Physical::Code(key::Code::Numpad2)
            | Physical::Code(key::Code::Numpad3)
            | Physical::Code(key::Code::Numpad4)
            | Physical::Code(key::Code::Numpad5)
            | Physical::Code(key::Code::Numpad6)
            | Physical::Code(key::Code::Numpad7)
            | Physical::Code(key::Code::Numpad8)
            | Physical::Code(key::Code::Numpad9)
            | Physical::Code(key::Code::NumpadAdd)
            | Physical::Code(key::Code::NumpadBackspace)
            | Physical::Code(key::Code::NumpadClear)
            | Physical::Code(key::Code::NumpadClearEntry)
            | Physical::Code(key::Code::NumpadComma)
            | Physical::Code(key::Code::NumpadDecimal)
            | Physical::Code(key::Code::NumpadDivide)
            | Physical::Code(key::Code::NumpadEnter)
            | Physical::Code(key::Code::NumpadEqual)
            | Physical::Code(key::Code::NumpadHash)
            | Physical::Code(key::Code::NumpadMemoryAdd)
            | Physical::Code(key::Code::NumpadMemoryClear)
            | Physical::Code(key::Code::NumpadMemoryRecall)
            | Physical::Code(key::Code::NumpadMemoryStore)
            | Physical::Code(key::Code::NumpadMemorySubtract)
            | Physical::Code(key::Code::NumpadMultiply)
            | Physical::Code(key::Code::NumpadParenLeft)
            | Physical::Code(key::Code::NumpadParenRight)
            | Physical::Code(key::Code::NumpadStar)
            | Physical::Code(key::Code::NumpadSubtract)
    )
}
