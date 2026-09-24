use iced::advanced::{
    Layout, Renderer as _, Shell, Widget, layout, overlay, renderer, widget,
};
use iced::{Event, Length, Rectangle, Size, Vector, mouse};

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

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) {
        self.base.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        );
        tree.size = tree.children[0].size;
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout,
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

    fn diff(&mut self, tree: &mut widget::Tree) {
        tree.diff_children(&mut [&mut self.base, &mut self.overlay]);
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        self.base.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            viewport,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.base.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout,
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
        layout: Layout,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
        window: Size,
    ) -> Vec<overlay::Element<'b, Message, Theme, Renderer>> {
        let (first, second) = tree.children.split_at_mut(1);

        let base = self.base.as_widget_mut().overlay(
            &mut first[0],
            layout,
            renderer,
            viewport,
            translation,
            window,
        );

        let position = layout.position() + translation;
        let viewport = *viewport + translation;

        let (width, height) = match self.anchor {
            // From top of base to top of viewport
            Anchor::AboveTop => (layout.bounds().width, position.y),
            // From top of base to bottom of viewport
            Anchor::BelowTopCentered => (window.width, window.height),
        };

        let limits = layout::Limits::new(Size::ZERO, Size { width, height })
            .width(Length::Fill)
            .height(Length::Fill);

        self.overlay
            .as_widget_mut()
            .layout(&mut second[0], renderer, &limits);

        let size = second[0].size;
        let translation = match self.anchor {
            // Overlay height + offset above the top
            Anchor::AboveTop => Vector::new(0.0, -(size.height + self.offset)),
            // Offset below the top and centered, pushed up just enough to stay
            // within the viewport when it would overflow the bottom edge.
            Anchor::BelowTopCentered => {
                let mut x = layout.bounds().width / 2.0 - size.width / 2.0;

                // overlay may be wider than parent
                let left = position.x + x;
                if left < viewport.x {
                    x += viewport.x - left;
                }
                let right = position.x + x + size.width;
                let viewport_right = viewport.x + viewport.width;
                if right > viewport_right {
                    x -= right - viewport_right;
                }

                let mut y = self.offset;

                let overflow = position.y + y + size.height
                    - (viewport.y + viewport.height);
                if overflow > 0.0 {
                    y -= overflow;
                }

                // Never push above the top of the viewport.
                if position.y + y < viewport.y {
                    y = viewport.y - position.y;
                }

                Vector::new(x, y)
            }
        };

        let overlay = overlay::Element::new(Box::new(Overlay {
            content: &mut self.overlay,
            tree: &mut second[0],
            on_dismiss: &self.on_dismiss,
            layout: Layout::new(size).move_to(position + translation),
            viewport,
            window,
        }));

        base.into_iter().chain(std::iter::once(overlay)).collect()
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
    on_dismiss: &'b Option<Box<dyn Fn() -> Message + 'a>>,
    layout: Layout,
    viewport: Rectangle,
    window: Size,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'_, '_, Message>
{
    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        cursor: mouse::Cursor,
    ) {
        renderer.with_layer(self.layout.bounds(), |renderer| {
            self.content.as_widget().draw(
                self.tree,
                renderer,
                theme,
                style,
                self.layout,
                cursor,
                &self.layout.bounds(),
            );
        });
    }

    fn operate(
        &mut self,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        self.content.as_widget_mut().operate(
            self.tree,
            self.layout,
            &self.layout.bounds(),
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        event: &Event,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        // A press outside the overlay dismisses it (and is consumed so it
        // doesn't also act on whatever is underneath).
        if let Some(on_dismiss) = self.on_dismiss.as_ref()
            && matches!(event, Event::Mouse(mouse::Event::ButtonPressed { .. }))
            && !cursor.is_over(self.layout.bounds())
        {
            shell.publish(on_dismiss());
            shell.capture_event();
            return;
        }

        let should_capture = matches!(event, Event::Mouse(_) | Event::Touch(_))
            && cursor.is_over(self.layout.bounds());

        self.content.as_widget_mut().update(
            self.tree,
            event,
            self.layout,
            cursor,
            renderer,
            shell,
            &self.layout.bounds(),
        );

        if should_capture {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        let interaction = self.content.as_widget().mouse_interaction(
            self.tree,
            self.layout,
            cursor,
            &self.viewport,
            renderer,
        );

        if interaction == mouse::Interaction::None
            && cursor.is_over(self.layout.bounds())
        {
            mouse::Interaction::Idle
        } else {
            interaction
        }
    }

    fn overlay<'c>(
        &'c mut self,
        renderer: &Renderer,
    ) -> Vec<overlay::Element<'c, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            self.tree,
            self.layout,
            renderer,
            &self.viewport,
            Vector::default(),
            self.window,
        )
    }
}
