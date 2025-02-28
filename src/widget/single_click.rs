use iced::advanced::widget::Tree;
use iced::advanced::{mouse, Clipboard, Layout, Shell};
use iced::event;

use crate::widget::{decorate, Renderer};
use crate::Element;

pub fn single_click<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    decorate(content)
        .update(
            move |_state: &mut State,
                  inner: &mut Element<'a, Message>,
                  tree: &mut Tree,
                  event: &iced::Event,
                  layout: Layout<'_>,
                  cursor: mouse::Cursor,
                  renderer: &Renderer,
                  clipboard: &mut dyn Clipboard,
                  shell: &mut Shell<'_, Message>,
                  viewport: &iced::Rectangle| {
                inner.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, clipboard, shell, viewport,
                );

                if shell.is_event_captured() {
                    return;
                }

                if !cursor.is_over(layout.bounds()) {
                    return;
                }

                let event::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
                else {
                    return;
                };

                shell.publish(message.clone());
                shell.capture_event();
            },
        )
        .into()
}

#[derive(Clone, Debug)]
struct State;

impl Default for State {
    fn default() -> Self {
        State
    }
}
