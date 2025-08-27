use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell};
use iced::{Size, mouse};

use super::{Element, Renderer, decorate};

pub fn on_resize<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    on_resize: impl Fn(Size) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: 'a,
{
    #[derive(Default)]
    struct State {
        last_size: Option<Size>,
    }

    decorate(base)
        .update(
            move |state: &mut State,
                  inner: &mut Element<'a, Message>,
                  tree: &mut Tree,
                  event: &iced::Event,
                  layout: Layout<'_>,
                  cursor: mouse::Cursor,
                  renderer: &Renderer,
                  clipboard: &mut dyn Clipboard,
                  shell: &mut Shell<'_, Message>,
                  viewport: &iced::Rectangle| {
                let new_size = layout.bounds().size();

                if state.last_size.is_none_or(|size| size != new_size) {
                    state.last_size = Some(new_size);
                    shell.publish(on_resize(new_size));
                }

                inner.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, clipboard, shell,
                    viewport,
                );
            },
        )
        .into()
}
