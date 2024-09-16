use std::slice;

use iced::advanced::widget::{operation, tree, Operation};
use iced::advanced::{self, layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::widget::{column, container};
use iced::{event, mouse, Element, Event, Length, Point, Rectangle, Size, Task, Vector};

pub use iced::widget::container::{Style, StyleFn};

use super::double_pass;

pub fn context_menu<'a, T, Message, Theme, Renderer>(
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    entries: Vec<T>,
    entry: impl Fn(T, Length) -> Element<'a, Message, Theme, Renderer> + 'a,
) -> ContextMenu<'a, T, Message, Theme, Renderer> {
    ContextMenu {
        base: base.into(),
        entries,
        entry: Box::new(entry),

        menu: None,
    }
}

pub struct ContextMenu<'a, T, Message, Theme, Renderer> {
    base: Element<'a, Message, Theme, Renderer>,
    entries: Vec<T>,
    entry: Box<dyn Fn(T, Length) -> Element<'a, Message, Theme, Renderer> + 'a>,

    // Cached, recreated during `overlay` if menu is open
    menu: Option<Element<'a, Message, Theme, Renderer>>,
}

#[derive(Debug)]
pub struct State {
    pub status: Status,
    menu_tree: widget::Tree,
}

impl State {
    pub fn new() -> Self {
        State {
            status: Status::Closed,
            menu_tree: widget::Tree::empty(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Closed,
    Open(Point),
}

impl Status {
    pub fn open(self) -> Option<Point> {
        match self {
            Status::Closed => None,
            Status::Open(position) => Some(position),
        }
    }
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ContextMenu<'a, T, Message, Theme, Renderer>
where
    T: Copy + 'a,
    Message: 'a,
    Theme: 'a + container::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
{
    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.base.as_widget().size_hint()
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.base
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
        tree::State::new(State {
            status: Status::Closed,
            menu_tree: widget::Tree::empty(),
        })
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.base)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(slice::from_ref(&self.base));
    }

    fn operate(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        let state = tree.state.downcast_mut::<State>();

        operation.custom(state, None);

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
        viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<State>();

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = &event {
            if let Some(position) = cursor.position_over(layout.bounds()) {
                state.status = Status::Open(position);
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
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        cursor
            .is_over(layout.bounds())
            .then_some(mouse::Interaction::Pointer)
            .unwrap_or_default()
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let base_state = tree.children.first_mut().unwrap();
        let base = self
            .base
            .as_widget_mut()
            .overlay(base_state, layout, renderer, translation);

        let state = tree.state.downcast_mut::<State>();

        let overlay = overlay(
            state,
            &mut self.menu,
            &self.entries,
            &self.entry,
            translation,
        );

        if base.is_none() && overlay.is_none() {
            None
        } else {
            Some(overlay::Group::with_children(base.into_iter().chain(overlay).collect()).overlay())
        }
    }
}

fn build_menu<'a, T, Message, Theme, Renderer>(
    entries: &[T],
    entry: &(dyn Fn(T, Length) -> Element<'a, Message, Theme, Renderer> + 'a),
) -> Element<'a, Message, Theme, Renderer>
where
    T: Copy + 'a,
    Message: 'a,
    Theme: 'a + container::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
{
    let build_menu =
        |length, view: &(dyn Fn(T, Length) -> Element<'a, Message, Theme, Renderer> + 'a)| {
            container(column(
                entries.iter().copied().map(|entry| view(entry, length)),
            ))
            .padding(4)
            .style(|theme| <Theme as Catalog>::style(theme, &<Theme as Catalog>::default()))
        };

    double_pass(
        build_menu(Length::Shrink, entry),
        build_menu(Length::Fill, entry),
    )
}

pub fn overlay<'a, 'b, T, Message, Theme, Renderer>(
    state: &'b mut State,
    menu: &'b mut Option<Element<'a, Message, Theme, Renderer>>,
    entries: &[T],
    entry: &(dyn Fn(T, Length) -> Element<'a, Message, Theme, Renderer> + 'a),
    translation: Vector,
) -> Option<overlay::Element<'b, Message, Theme, Renderer>>
where
    T: Copy + 'a,
    Message: 'a,
    Theme: 'a + container::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
{
    if entries.is_empty() {
        return None;
    }

    // Ensure overlay is created / diff'd
    match state.status {
        Status::Open(_) => match menu {
            Some(menu) => state.menu_tree.diff(&*menu),
            None => {
                let _menu = build_menu(entries, entry);
                state.menu_tree = widget::Tree::new(&_menu);
                *menu = Some(_menu);
            }
        },
        Status::Closed => {
            *menu = None;
        }
    }

    state
        .status
        .open()
        .zip(menu.as_mut())
        .map(|(position, menu)| {
            overlay::Element::new(Box::new(Overlay {
                menu,
                state,
                position: position + translation,
            }))
        })
}

pub fn close<Message: 'static + Send>(f: fn(bool) -> Message) -> Task<Message> {
    struct Close<T> {
        any_closed: bool,
        f: fn(bool) -> T,
    }

    impl<T> Operation<T> for Close<T> {
        fn container(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            operate_on_children: &mut dyn FnMut(&mut dyn Operation<T>),
        ) {
            operate_on_children(self)
        }

        fn custom(&mut self, state: &mut dyn std::any::Any, _id: Option<&widget::Id>) {
            if let Some(state) = state.downcast_mut::<State>() {
                if let Status::Open(_) = state.status {
                    state.status = Status::Closed;
                    self.any_closed = true;
                }
            }
        }

        fn finish(&self) -> operation::Outcome<T> {
            operation::Outcome::Some((self.f)(self.any_closed))
        }
    }

    widget::operate(Close {
        any_closed: false,
        f,
    })
}

impl<'a, T, Message, Theme, Renderer> From<ContextMenu<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Copy + 'a,
    Message: 'a,
    Theme: 'a + container::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
{
    fn from(context_menu: ContextMenu<'a, T, Message, Theme, Renderer>) -> Self {
        Element::new(context_menu)
    }
}

struct Overlay<'a, 'b, Message, Theme, Renderer> {
    menu: &'b mut Element<'a, Message, Theme, Renderer>,
    state: &'b mut State,
    position: Point,
}

impl<'a, 'b, Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'a, 'b, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds)
            .width(Length::Fill)
            .height(Length::Fill);

        let node = self
            .menu
            .as_widget()
            .layout(&mut self.state.menu_tree, renderer, &limits);

        let viewport = Rectangle::new(Point::ORIGIN, bounds);
        let mut bounds = Rectangle::new(self.position, node.size());

        if bounds.x < viewport.x {
            bounds.x = viewport.x;
        } else if viewport.x + viewport.width < bounds.x + bounds.width {
            bounds.x = viewport.x + viewport.width - bounds.width;
        }

        if bounds.y < viewport.y {
            bounds.y = viewport.y;
        } else if viewport.y + viewport.height < bounds.y + bounds.height {
            bounds.y = viewport.y + viewport.height - bounds.height;
        }

        node.move_to(bounds.position())
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.menu.as_widget().draw(
            &self.state.menu_tree,
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
        self.menu
            .as_widget_mut()
            .operate(&mut self.state.menu_tree, layout, renderer, operation);
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
                self.state.status = Status::Closed;
            }
        }

        if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = &event {
            if cursor.position_over(layout.bounds()).is_some() {
                self.state.status = Status::Closed;
            }
        }

        self.menu.as_widget_mut().on_event(
            &mut self.state.menu_tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.menu.as_widget().mouse_interaction(
            &self.state.menu_tree,
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn is_over(&self, layout: Layout<'_>, _renderer: &Renderer, cursor_position: Point) -> bool {
        layout.bounds().contains(cursor_position)
    }
}

/// The theme catalog of a [`Catalog`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>) -> container::Style;
}
