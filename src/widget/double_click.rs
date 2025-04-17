use std::time;

use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, mouse};
use iced::event;

const TIMEOUT_MILLIS: u64 = 250;

use crate::Element;
use crate::widget::{Renderer, decorate};

pub fn double_click<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    decorate(content)
        .update(
            move |state: &mut Internal,
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
                    tree, event, layout, cursor, renderer, clipboard, shell,
                    viewport,
                );

                if shell.is_event_captured() {
                    return;
                }

                if !cursor.is_over(layout.bounds()) {
                    return;
                }

                let event::Event::Mouse(mouse::Event::ButtonPressed(
                    mouse::Button::Left,
                )) = event
                else {
                    return;
                };

                let now = time::Instant::now();
                let timeout = time::Duration::from_millis(TIMEOUT_MILLIS);
                let diff = now - state.instant;

                if diff <= timeout {
                    shell.publish(message.clone());
                    shell.capture_event();
                } else {
                    state.instant = time::Instant::now();
                }
            },
        )
        .into()
}

#[derive(Clone, Debug)]
struct Internal {
    instant: time::Instant,
}

impl Default for Internal {
    fn default() -> Self {
        Internal {
            instant: time::Instant::now(),
        }
    }
}
