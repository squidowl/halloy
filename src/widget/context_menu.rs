use iced::advanced::widget::tree;
use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::widget::{column, container};
use iced::{event, mouse, Event, Length, Point, Rectangle, Size};

use super::{hover, Element, Renderer};
use crate::{theme, Theme};

pub fn context_menu<'a, T, Message>(
    base: impl Into<Element<'a, Message>>,
    entries: Vec<T>,
    view: impl Fn(T, bool) -> Element<'a, Message> + 'a + Clone,
) -> Element<'a, Message>
where
    Message: 'a,
    T: 'a + Copy,
{
    let menu = container(column(
        entries
            .into_iter()
            .map(|entry| {
                let view = view.clone();
                hover(move |hovered| (view)(entry, hovered))
            })
            .collect(),
    ))
    .padding(4)
    .style(theme::Container::Context);

    ContextMenu {
        base: base.into(),
        menu: menu.into(),
    }
    .into()
}

struct ContextMenu<'a, Message> {
    base: Element<'a, Message>,
    menu: Element<'a, Message>,
}

#[derive(Debug, Clone, Copy)]
enum State {
    Closed,
    Open(Point),
}

impl State {
    fn open(self) -> Option<Point> {
        match self {
            State::Closed => None,
            State::Open(point) => Some(point),
        }
    }
}

impl<'a, Message> Widget<Message, Renderer> for ContextMenu<'a, Message> {
    fn width(&self) -> Length {
        self.base.as_widget().width()
    }

    fn height(&self) -> Length {
        self.base.as_widget().height()
    }

    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.base.as_widget().layout(renderer, limits)
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
        )
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::Closed)
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.base), widget::Tree::new(&self.menu)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.base, &self.menu]);
    }

    fn operate(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<Message>,
    ) {
        self.base
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
    ) -> event::Status {
        let state = tree.state.downcast_mut::<State>();

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = &event {
            if let Some(position) = cursor.position_over(layout.bounds()) {
                *state = State::Open(position);
            }
        }

        self.base.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
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
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        let state = tree.state.downcast_mut::<State>();

        let (first, second) = tree.children.split_at_mut(1);

        let base = self
            .base
            .as_widget_mut()
            .overlay(&mut first[0], layout, renderer);

        let overlay = state.open().map(|position| {
            overlay::Element::new(
                position,
                Box::new(Overlay {
                    content: &mut self.menu,
                    tree: &mut second[0],
                    state,
                }),
            )
        });

        Some(overlay::Group::with_children(base.into_iter().chain(overlay).collect()).overlay())
    }
}

impl<'a, Message> From<ContextMenu<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(context_menu: ContextMenu<'a, Message>) -> Self {
        Element::new(context_menu)
    }
}

struct Overlay<'a, 'b, Message> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut widget::Tree,
    state: &'b mut State,
}

impl<'a, 'b, Message> overlay::Overlay<Message, Renderer> for Overlay<'a, 'b, Message> {
    fn layout(&self, renderer: &Renderer, bounds: Size, position: Point) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds)
            .width(Length::Fill)
            .height(Length::Fill);

        let mut node = self.content.as_widget().layout(renderer, &limits);
        node.move_to(position);

        node
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
        operation: &mut dyn widget::Operation<Message>,
    ) {
        self.content
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        if let Event::Mouse(mouse::Event::ButtonPressed(_)) = &event {
            if cursor.position_over(layout.bounds()).is_none() {
                *self.state = State::Closed;
            }
        }

        if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = &event {
            if cursor.position_over(layout.bounds()).is_some() {
                *self.state = State::Closed;
            }
        }

        self.content
            .as_widget_mut()
            .on_event(self.tree, event, layout, cursor, renderer, clipboard, shell)
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(self.tree, layout, cursor, viewport, renderer)
    }

    fn is_over(&self, layout: Layout<'_>, cursor_position: Point) -> bool {
        layout.bounds().contains(cursor_position)
    }
}
