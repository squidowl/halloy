use iced::advanced::{Clipboard, Layout, Shell, widget};
use iced::{Event, Padding, Rectangle, mouse, window};

use super::{Element, Renderer, decorate};

#[derive(Debug, Clone, Copy)]
pub enum When {
    /// Element bounds intersect the viewport (any part is visible, including fully)
    Intersecting,
    /// Element bounds are fully outside viewport
    Disjoint,
    /// Element bounds are fully contained within the viewport
    Contained,
    /// Element is not fully contained by the viewport (partially or fully outside)
    NotContained,
}

pub fn notify_visibility<'a, Message, Id>(
    content: impl Into<Element<'a, Message>>,
    margin: impl Into<Padding>,
    when: When,
    id: Id,
    message: Message,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
    Id: 'a + Copy + Eq + 'static,
{
    let margin = margin.into();

    decorate(content)
        .update(
            move |state: &mut (Option<Id>, bool),
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
                    if state.0 != Some(id) {
                        state.0 = Some(id);
                        state.1 = false;
                    }

                    let is_visible =
                        viewport.expand(margin).intersects(&layout.bounds());

                    let bounds = layout.bounds();
                    let is_fully_visible = viewport.x <= bounds.x
                        && viewport.y <= bounds.y
                        && viewport.x + viewport.width
                            >= bounds.x + bounds.width
                        && viewport.y + viewport.height
                            >= bounds.y + bounds.height;

                    let should_notify = match when {
                        When::Intersecting => is_visible,
                        When::Disjoint => !is_visible,
                        When::Contained => is_fully_visible,
                        When::NotContained => !is_fully_visible,
                    };

                    if should_notify && !state.1 {
                        shell.publish(message.clone());
                        state.1 = true;
                    } else if !should_notify {
                        state.1 = false;
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
