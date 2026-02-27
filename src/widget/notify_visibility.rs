use iced::advanced::{Clipboard, Layout, Shell, widget};
use iced::{Event, Padding, Rectangle, mouse, window};

use super::{Element, Renderer, decorate};

#[derive(Debug, Clone, Copy)]
pub enum When {
    Visible,
    NotVisible,
}

pub fn notify_visibility<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    margin: impl Into<Padding>,
    when: When,
    message: Message,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    let margin = margin.into();

    decorate(content)
        .update(
            move |sent: &mut bool,
                  inner: &mut Element<'a, Message>,
                  tree: &mut widget::Tree,
                  event: &Event,
                  layout: Layout<'_>,
                  cursor: mouse::Cursor,
                  renderer: &Renderer,
                  clipboard: &mut dyn Clipboard,
                  shell: &mut Shell<'_, Message>,
                  viewport: &Rectangle| {
                if let Event::Window(window::Event::RedrawRequested(_)) = &event
                {
                    let is_visible =
                        viewport.expand(margin).intersects(&layout.bounds());

                    let should_notify = match when {
                        When::Visible => is_visible,
                        When::NotVisible => !is_visible,
                    };

                    if should_notify && !*sent {
                        shell.publish(message.clone());
                        *sent = true;
                    }
                }

                inner.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, clipboard, shell,
                    viewport,
                );
            },
        )
        .into()
}
