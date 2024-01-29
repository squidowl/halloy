use std::time;

use iced::advanced::widget::{self, tree, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
use iced::{advanced, event, Length, Rectangle, Size};

const TIMEOUT_MILLIS: u64 = 250;

use crate::widget::Renderer;
use crate::{Element, Theme};

pub struct DoubleClick<'a, Message> {
    content: Element<'a, Message>,
    message: Message,
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

impl<'a, Message> Widget<Message, Theme, Renderer> for DoubleClick<'a, Message>
where
    Message: Clone,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &advanced::layout::Limits,
    ) -> advanced::layout::Node {
        self.content.as_widget().layout(tree, renderer, limits)
    }

    fn tag(&self) -> widget::tree::Tag {
        tree::Tag::of::<Internal>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Internal::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.content]);
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        )
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: event::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        let status = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
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

        let event::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event else {
            return event::Status::Ignored;
        };

        let state = tree.state.downcast_mut::<Internal>();
        let now = time::Instant::now();
        let timeout = time::Duration::from_millis(TIMEOUT_MILLIS);
        let diff = now - state.instant;

        if diff <= timeout {
            shell.publish(self.message.clone());
            event::Status::Captured
        } else {
            state.instant = time::Instant::now();
            event::Status::Ignored
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<Message>,
    ) {
        self.content
            .as_widget()
            .operate(&mut tree.children[0], layout, renderer, operation)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer)
    }
}

impl<'a, Message> From<DoubleClick<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(double_click: DoubleClick<'a, Message>) -> Self {
        Element::new(double_click)
    }
}

pub fn double_click<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    message: Message,
) -> DoubleClick<'a, Message> {
    DoubleClick {
        content: content.into(),
        message,
    }
}
