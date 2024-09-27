use std::time;

use iced::advanced::widget::Tree;
use iced::advanced::{mouse, Clipboard, Layout, Shell};
use iced::event;

const TIMEOUT_MILLIS: u64 = 250;

use crate::widget::{decorate, Renderer};
use crate::Element;

pub fn double_click<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    decorate(content)
        .on_event(
            move |state: &mut Internal,
                  inner: &mut Element<'a, Message>,
                  tree: &mut Tree,
                  event: iced::Event,
                  layout: Layout<'_>,
                  cursor: mouse::Cursor,
                  renderer: &Renderer,
                  clipboard: &mut dyn Clipboard,
                  shell: &mut Shell<'_, Message>,
                  viewport: &iced::Rectangle| {
                let status = inner.as_widget_mut().on_event(
                    tree,
                    event.clone(),
                    layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );

                if matches!(status, event::Status::Captured) {
                    return event::Status::Captured;
                }

                if !cursor.is_over(layout.bounds()) {
                    return event::Status::Ignored;
                }

                let event::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
                else {
                    return event::Status::Ignored;
                };

                let now = time::Instant::now();
                let timeout = time::Duration::from_millis(TIMEOUT_MILLIS);
                let diff = now - state.instant;

                if diff <= timeout {
                    shell.publish(message.clone());
                    event::Status::Captured
                } else {
                    state.instant = time::Instant::now();
                    event::Status::Ignored
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
