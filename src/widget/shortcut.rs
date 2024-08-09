use data::shortcut;
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell};
use iced::{event, keyboard, mouse, Event};

pub use data::shortcut::Command;

use super::{wrap, Element, Renderer};

pub fn shortcut<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    shortcuts: Vec<data::Shortcut>,
    on_press: impl Fn(Command) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: 'a,
{
    wrap(base)
        .state::<shortcut::Modifiers>()
        .on_event(
            move |modifiers: &mut shortcut::Modifiers,
                  inner: &mut Element<'a, Message>,
                  tree: &mut Tree,
                  event: iced::Event,
                  layout: Layout<'_>,
                  cursor: mouse::Cursor,
                  renderer: &Renderer,
                  clipboard: &mut dyn Clipboard,
                  shell: &mut Shell<'_, Message>,
                  viewport: &iced::Rectangle| {
                match &event {
                    Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                        let key_bind = shortcut::KeyBind::from((key.clone(), *modifiers));

                        if let Some(command) = shortcuts
                            .iter()
                            .find_map(|shortcut| shortcut.execute(&key_bind))
                        {
                            shell.publish((on_press)(command));
                            return event::Status::Captured;
                        }
                    }
                    Event::Keyboard(keyboard::Event::ModifiersChanged(new_modifiers)) => {
                        *modifiers = (*new_modifiers).into();
                    }
                    _ => {}
                }

                inner.as_widget_mut().on_event(
                    tree, event, layout, cursor, renderer, clipboard, shell, viewport,
                )
            },
        )
        .into()
}
