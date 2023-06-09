use iced::advanced::widget::tree;
use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
pub use iced::keyboard::{KeyCode, Modifiers};
use iced::{event, keyboard, mouse, Event, Length, Point, Rectangle};

use super::{Element, Renderer};
use crate::Theme;

pub fn key_press<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    key_code: KeyCode,
    modifiers: Modifiers,
    on_press: Message,
) -> Element<'a, Message>
where
    Message: 'a + Clone,
{
    KeyPress {
        content: base.into(),
        key_code,
        modifiers,
        on_press,
    }
    .into()
}

struct KeyPress<'a, Message> {
    content: Element<'a, Message>,
    key_code: KeyCode,
    modifiers: Modifiers,
    on_press: Message,
}

impl<'a, Message> Widget<Message, Renderer> for KeyPress<'a, Message>
where
    Message: Clone,
{
    fn width(&self) -> Length {
        self.content.as_widget().width()
    }

    fn height(&self) -> Length {
        self.content.as_widget().height()
    }

    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget().layout(renderer, limits)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            tree,
            renderer,
            theme,
            style,
            layout,
            cursor_position,
            viewport,
        )
    }

    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn operate(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<Message>,
    ) {
        self.content
            .as_widget()
            .operate(tree, layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        tree: &mut widget::Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key_code,
            modifiers,
        }) = &event
        {
            if *key_code == self.key_code && *modifiers == self.modifiers {
                shell.publish(self.on_press.clone());
                return event::Status::Captured;
            }
        }

        self.content.as_widget_mut().on_event(
            tree,
            event,
            layout,
            cursor_position,
            renderer,
            clipboard,
            shell,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            tree,
            layout,
            cursor_position,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer)
    }
}

impl<'a, Message> From<KeyPress<'a, Message>> for Element<'a, Message>
where
    Message: 'a + Clone,
{
    fn from(key_press: KeyPress<'a, Message>) -> Self {
        Element::new(key_press)
    }
}
