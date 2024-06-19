//! A widget that uses a two pass layout.
//!
//! Layout from first pass is used as the limits for the second pass

use iced::advanced::widget::tree;
use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::{event, mouse, Event, Length, Rectangle, Size, Vector};

use super::{Element, Renderer};
use crate::Theme;

/// Layout from first pass is used as the limits for the second pass
pub fn double_pass<'a, Message>(
    first_pass: impl Into<Element<'a, Message>>,
    second_pass: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    DoublePass {
        first_pass: first_pass.into(),
        second_pass: second_pass.into(),
    }
    .into()
}

struct DoublePass<'a, Message> {
    first_pass: Element<'a, Message>,
    second_pass: Element<'a, Message>,
}

impl<'a, Message> Widget<Message, Theme, Renderer> for DoublePass<'a, Message> {
    fn size(&self) -> Size<Length> {
        self.second_pass.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.second_pass.as_widget().size_hint()
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let layout = self.first_pass.as_widget().layout(
            &mut widget::Tree::new(&self.first_pass),
            renderer,
            limits,
        );

        let new_limits = layout::Limits::new(
            Size::ZERO,
            layout
                .size()
                // eliminate float precision issues if second pass
                // is fill
                .expand(Size::new(horizontal_expansion(), 1.0)),
        );

        self.second_pass
            .as_widget()
            .layout(tree, renderer, &new_limits)
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
        self.second_pass
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport)
    }

    fn tag(&self) -> tree::Tag {
        self.second_pass.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.second_pass.as_widget().state()
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.second_pass.as_widget().children()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        self.second_pass.as_widget().diff(tree);
    }

    fn operate(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        self.second_pass
            .as_widget()
            .operate(tree, layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        tree: &mut widget::Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        self.second_pass.as_widget_mut().on_event(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.second_pass
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.second_pass
            .as_widget_mut()
            .overlay(tree, layout, renderer, translation)
    }
}

impl<'a, Message> From<DoublePass<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(double_pass: DoublePass<'a, Message>) -> Self {
        Element::new(double_pass)
    }
}

pub fn horizontal_expansion() -> f32 {
    1.0
}
