use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, overlay, renderer, widget,
};
use iced::{Event, Length, Point, Rectangle, Size, Vector, mouse};

use super::{Element, Renderer};
use crate::Theme;

pub fn anchored_overlay<'a, Message: 'a>(
    base: impl Into<Element<'a, Message>>,
    overlay: impl Into<Element<'a, Message>>,
    anchor: Anchor,
    offset: f32,
    // Emitted when a mouse press lands outside the overlay (e.g. to dismiss it).
    on_dismiss: Option<Box<dyn Fn() -> Message + 'a>>,
) -> Element<'a, Message> {
    AnchoredOverlay {
        base: base.into(),
        overlay: overlay.into(),
        anchor,
        offset,
        on_dismiss,
    }
    .into()
}

#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    AboveTop,
    BelowTopCentered,
}

struct AnchoredOverlay<'a, Message> {
    base: Element<'a, Message>,
    overlay: Element<'a, Message>,
    anchor: Anchor,
    offset: f32,
    on_dismiss: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<Message> Widget<Message, Theme, Renderer>
    for AnchoredOverlay<'_, Message>
{
    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.base.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.base.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.base),
            widget::Tree::new(&self.overlay),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.base, &self.overlay]);
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        self.base.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.base.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.base.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let (first, second) = tree.children.split_at_mut(1);

        let base = self.base.as_widget_mut().overlay(
            &mut first[0],
            layout,
            renderer,
            viewport,
            translation,
        );

        let overlay = overlay::Element::new(Box::new(Overlay {
            content: &mut self.overlay,
            tree: &mut second[0],
            anchor: self.anchor,
            offset: self.offset,
            on_dismiss: &self.on_dismiss,
            base_layout: layout.bounds(),
            // Apply the accumulated translation (e.g. a scrollable's offset)
            // so the overlay anchors to the base's on-screen position rather
            // than its position in unscrolled content space.
            position: layout.position() + translation,
            viewport: *viewport,
        }));

        Some(
            overlay::Group::with_children(
                base.into_iter().chain(Some(overlay)).collect(),
            )
            .overlay(),
        )
    }
}

impl<'a, Message> From<AnchoredOverlay<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(anchored_overlay: AnchoredOverlay<'a, Message>) -> Self {
        Element::new(anchored_overlay)
    }
}

struct Overlay<'a, 'b, Message> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut widget::Tree,
    anchor: Anchor,
    offset: f32,
    on_dismiss: &'b Option<Box<dyn Fn() -> Message + 'a>>,
    base_layout: Rectangle,
    position: Point,
    viewport: Rectangle,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'_, '_, Message>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let (width, height) = match self.anchor {
            // From top of base to top of viewport
            Anchor::AboveTop => (self.base_layout.width, self.position.y),
            // From top of base to bottom of viewport
            Anchor::BelowTopCentered => (bounds.width, bounds.height),
        };

        let limits = layout::Limits::new(Size::ZERO, Size { width, height })
            .width(Length::Fill)
            .height(Length::Fill);

        let node = self
            .content
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);

        let translation = match self.anchor {
            // Overlay height + offset above the top
            Anchor::AboveTop => {
                Vector::new(0.0, -(node.size().height + self.offset))
            }
            // Offset below the top and centered, pushed up just enough to stay
            // within the viewport when it would overflow the bottom edge.
            Anchor::BelowTopCentered => {
                let mut x =
                    self.base_layout.width / 2.0 - node.size().width / 2.0;

                // overlay may be wider than parent
                let left = self.position.x + x;
                if left < self.viewport.x {
                    x += self.viewport.x - left;
                }
                let right = self.position.x + x + node.size().width;
                let viewport_right = self.viewport.x + self.viewport.width;
                if right > viewport_right {
                    x -= right - viewport_right;
                }

                let mut y = self.offset;

                let overflow = (self.position.y + y + node.size().height)
                    - (self.viewport.y + self.viewport.height);
                if overflow > 0.0 {
                    y -= overflow;
                }

                // Never push above the top of the viewport.
                if self.position.y + y < self.viewport.y {
                    y = self.viewport.y - self.position.y;
                }

                Vector::new(x, y)
            }
        };

        node.move_to(self.position + translation)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        self.content
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        // A press outside the overlay dismisses it (and is consumed so it
        // doesn't also act on whatever is underneath).
        if let Some(on_dismiss) = self.on_dismiss.as_ref()
            && matches!(event, Event::Mouse(mouse::Event::ButtonPressed { .. }))
            && !cursor.is_over(layout.bounds())
        {
            shell.publish(on_dismiss());
            shell.capture_event();
            return;
        }

        let should_capture = matches!(event, Event::Mouse(_) | Event::Touch(_))
            && cursor.is_over(layout.bounds());

        self.content.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );

        if should_capture {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'c>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            self.tree,
            layout,
            renderer,
            &self.viewport,
            Vector::default(),
        )
    }
}
