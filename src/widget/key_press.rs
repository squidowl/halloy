use iced::advanced::{Clipboard, Layout, Shell, widget};
pub use iced::keyboard::key::Named;
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
