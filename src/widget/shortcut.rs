use data::shortcut;
pub use data::shortcut::Command;
use iced::advanced::widget::tree;
use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::{event, keyboard, mouse, Event, Length, Rectangle, Size};

use super::{Element, Renderer};
use crate::Theme;

pub fn shortcut<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    shortcuts: Vec<data::Shortcut>,
    on_press: impl Fn(Command) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: 'a,
{
    Shortcut {
        content: base.into(),
        shortcuts,
        on_press: Box::new(on_press),
    }
    .into()
}

struct Shortcut<'a, Message> {
    content: Element<'a, Message>,
    shortcuts: Vec<data::Shortcut>,
    on_press: Box<dyn Fn(Command) -> Message + 'a>,
}

impl<'a, Message> Widget<Message, Theme, Renderer> for Shortcut<'a, Message> {
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
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
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

    fn tag(&self) -> tree::Tag {
        struct Marker;
        tree::Tag::of::<Marker>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(shortcut::Modifiers::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.content])
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
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let modifiers = tree.state.downcast_mut::<shortcut::Modifiers>();

        match &event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                let key_bind = shortcut::KeyBind::from((key.clone(), *modifiers));

                if let Some(command) = self
                    .shortcuts
                    .iter()
                    .find_map(|shortcut| shortcut.execute(&key_bind))
                {
                    shell.publish((self.on_press)(command));
                    return event::Status::Captured;
                }
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(new_modifiers)) => {
                *modifiers = (*new_modifiers).into();
            }
            _ => {}
        }

        self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
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
        self.content.as_widget().mouse_interaction(
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
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer)
    }
}

impl<'a, Message> From<Shortcut<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(shortcut: Shortcut<'a, Message>) -> Self {
        Element::new(shortcut)
    }
}
