use std::cell::LazyCell;

use iced::advanced::widget::{Operation, operation, tree};
use iced::advanced::{
    self, Layout, Shell, Widget, layout, overlay, renderer, widget,
};
pub use iced::widget::container::{Style, StyleFn};
use iced::widget::{column, container};
use iced::{
    Element, Event, Length, Point, Rectangle, Size, Task, Vector, mouse,
};

use super::double_pass;

#[derive(Debug, Default, Clone, Copy)]
pub enum MouseButton {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Anchor {
    #[default]
    Cursor,
    Widget,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum ToggleBehavior {
    #[default]
    KeepOpen,
    Close,
}

pub fn context_menu<'a, T, Message, Theme, Renderer>(
    activation_button: MouseButton,
    anchor: Anchor,
    toggle_behavior: ToggleBehavior,
    mouse_interaction_on_hover: Option<mouse::Interaction>,
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    entries: Vec<T>,
    entry: impl Fn(T, Length) -> Element<'a, Message, Theme, Renderer> + 'a,
) -> ContextMenu<'a, Message, Theme, Renderer>
where
    T: Copy + 'a,
    Message: 'a,
    Theme: 'a + container::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>:
        From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
{
    lazy_context_menu(
        activation_button,
        anchor,
        toggle_behavior,
        mouse_interaction_on_hover,
        base,
        move || entries,
        entry,
    )
}

pub fn lazy_context_menu<'a, T, Message, Theme, Renderer>(
    activation_button: MouseButton,
    anchor: Anchor,
    toggle_behavior: ToggleBehavior,
    mouse_interaction_on_hover: Option<mouse::Interaction>,
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    entries: impl FnOnce() -> Vec<T> + 'a,
    entry: impl Fn(T, Length) -> Element<'a, Message, Theme, Renderer> + 'a,
) -> ContextMenu<'a, Message, Theme, Renderer>
where
    T: Copy + 'a,
    Message: 'a,
    Theme: 'a + container::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>:
        From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
{
    ContextMenu {
        base: base.into(),
        menu: LazyCell::new(Box::new(move || {
            let entries = entries();

            build_menu(&entries, &entry)
        })),
        on_open: None,
        activation_button: match activation_button {
            MouseButton::Left => iced::mouse::Button::Left,
            MouseButton::Right => iced::mouse::Button::Right,
        },
        anchor,
        toggle_behavior,
        mouse_interaction_on_hover,
    }
}

type LazyElement<'a, Message, Theme, Renderer> = LazyCell<
    Element<'a, Message, Theme, Renderer>,
    Box<dyn FnOnce() -> Element<'a, Message, Theme, Renderer> + 'a>,
>;

pub struct ContextMenu<'a, Message, Theme, Renderer> {
    base: Element<'a, Message, Theme, Renderer>,
    menu: LazyElement<'a, Message, Theme, Renderer>,
    on_open: Option<Box<dyn Fn() -> Message + 'a>>,
    activation_button: iced::mouse::Button,
    anchor: Anchor,
    toggle_behavior: ToggleBehavior,
    mouse_interaction_on_hover: Option<mouse::Interaction>,
}

#[derive(Debug, Default)]
pub struct State {
    status: Status,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Status {
    #[default]
    Closed,
    Open {
        position: Point,
        // Keep context menu open if button press is inside specified bounds
        keep_open_bounds: Option<(Vector, Size)>,
        needs_relayout: bool,
    },
}

impl Status {
    pub fn position(self) -> Option<Point> {
        match self {
            Status::Closed => None,
            Status::Open { position, .. } => Some(position),
        }
    }

    pub fn keep_open_bounds(&self) -> Option<&(Vector, Size)> {
        match self {
            Status::Closed => None,
            Status::Open {
                keep_open_bounds, ..
            } => keep_open_bounds.as_ref(),
        }
    }
}

impl<'a, Message, Theme, Renderer> ContextMenu<'a, Message, Theme, Renderer> {
    pub fn mouse_interaction_on_hover(
        mut self,
        interaction: Option<mouse::Interaction>,
    ) -> Self {
        self.mouse_interaction_on_hover = interaction;
        self
    }

    pub fn on_open(mut self, message: impl Fn() -> Message + 'a) -> Self {
        self.on_open = Some(Box::new(message));
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ContextMenu<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a + container::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>:
        From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
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

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn diff(&mut self, tree: &mut widget::Tree) {
        let state = tree.state.downcast_mut::<State>();

        if let Status::Open { needs_relayout, .. } = &mut state.status {
            *needs_relayout = true;
            tree.diff_children(&mut [&mut self.base, &mut self.menu]);
        } else {
            tree.diff_children(&mut [&mut self.base]);
        }
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        let state = tree.state.downcast_mut::<State>();

        operation.custom(None, layout.bounds(), state);

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

        if shell.is_event_captured() {
            return;
        }

        // is this a mouse event we are waiting for?
        let is_mouse_event =
            matches!(event, Event::Mouse(mouse::Event::ButtonPressed { .. }));

        if is_mouse_event {
            let state = tree.state.downcast_mut::<State>();
            let prev_status = state.status;

            // is this a mouse event for that we should do something?
            let is_activation_mouse_event =
                if let Event::Mouse(mouse::Event::ButtonPressed {
                    button, ..
                }) = event
                    && *button == self.activation_button
                {
                    true
                } else {
                    false
                };

            let position = if is_activation_mouse_event {
                match self.anchor {
                    Anchor::Widget => {
                        cursor.is_over(layout.bounds()).then_some({
                            let widget = layout.bounds();
                            Point::new(
                                widget.x + widget.width,
                                widget.y + widget.height,
                            )
                        })
                    }
                    Anchor::Cursor => {
                        cursor.position_over(layout.bounds()).map(|cursor| {
                            Point::new(cursor.x + 5.0, cursor.y + 5.0)
                        })
                    }
                }
            } else {
                None
            };

            // determinate next status
            let next_status = match (
                is_activation_mouse_event,
                prev_status,
                self.toggle_behavior,
                position,
            ) {
                (true, _, ToggleBehavior::KeepOpen, Some(position)) => {
                    Status::Open {
                        position,
                        keep_open_bounds: None,
                        needs_relayout: true,
                    }
                }
                (
                    true,
                    Status::Closed,
                    ToggleBehavior::Close,
                    Some(position),
                ) => {
                    let layout_bounds = layout.bounds();

                    // Position may be relative to containing scrollable, so
                    // store bounds as offset vector and size
                    Status::Open {
                        position,
                        keep_open_bounds: Some((
                            layout_bounds.position() - position,
                            layout_bounds.size(),
                        )),
                        needs_relayout: true,
                    }
                }
                (_, Status::Open { .. }, _, None)
                | (true, Status::Open { .. }, ToggleBehavior::Close, Some(_)) => {
                    Status::Closed
                }
                _ => prev_status, // keep status
            };

            if next_status != prev_status {
                state.status = next_status;

                if matches!(next_status, Status::Open { .. })
                    != matches!(prev_status, Status::Open { .. })
                {
                    shell.invalidate_overlay();

                    if tree.children.len() == 1 {
                        tree.children.push(widget::Tree::new(&*self.menu));
                    }
                    self.menu.as_widget_mut().diff(&mut tree.children[1]);
                } else {
                    shell.request_redraw();
                }

                if matches!(prev_status, Status::Closed)
                    && matches!(next_status, Status::Open { .. })
                    && let Some(message) = self.on_open.as_ref().map(|f| f())
                {
                    shell.publish(message);
                }

                if is_activation_mouse_event {
                    shell.capture_event();
                }
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            self.mouse_interaction_on_hover.unwrap_or({
                self.base.as_widget().mouse_interaction(
                    &tree.children[0],
                    layout,
                    cursor,
                    viewport,
                    renderer,
                )
            })
        } else {
            mouse::Interaction::default()
        }
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
        let state = tree.state.downcast_mut::<State>();

        let Some(position) = state.status.position() else {
            return self.base.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                translation,
                window,
            );
        };

        let (first, second) = tree.children.split_at_mut(1);

        let base = self.base.as_widget_mut().overlay(
            &mut first[0],
            layout,
            renderer,
            viewport,
            translation,
            window,
        );

        if let Status::Open { needs_relayout, .. } = &mut state.status
            && *needs_relayout
        {
            let limits = layout::Limits::new(Size::ZERO, window)
                .width(Length::Fill)
                .height(Length::Fill);

            self.menu
                .as_widget_mut()
                .layout(&mut second[0], renderer, &limits);

            *needs_relayout = false;
        }

        // Small padding to ensure that we don't spawn context menu at the very edge of the viewport.
        let padding = 5.0;
        let viewport = Rectangle::new(
            Point::new(padding, padding),
            Size::new(
                window.width - 2.0 * padding,
                window.height - 2.0 * padding,
            ),
        );
        let size = second[0].size;
        let mut bounds = Rectangle::new(position + translation, size);

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

        let overlay = overlay::Element::new(Box::new(Overlay {
            menu: &mut self.menu,
            tree: &mut second[0],
            status: &mut state.status,
            position: position + translation,
            layout: Layout::new(size).move_to(bounds.position()),
        }));

        base.into_iter().chain(std::iter::once(overlay)).collect()
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
    <Theme as container::Catalog>::Class<'a>:
        From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
{
    let build_menu =
        |length,
         view: &(
              dyn Fn(T, Length) -> Element<'a, Message, Theme, Renderer> + 'a
          )| {
            container(column(
                entries.iter().copied().map(|entry| view(entry, length)),
            ))
            .padding(4)
            .style(|theme| {
                <Theme as Catalog>::style(theme, &<Theme as Catalog>::default())
            })
        };

    double_pass(
        build_menu(Length::Shrink, entry),
        build_menu(Length::Fill, entry),
    )
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
            _viewport: &Rectangle,
        ) {
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }

        fn custom(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn std::any::Any,
        ) {
            if let Some(state) = state.downcast_mut::<State>()
                && let Status::Open { .. } = state.status
            {
                state.status = Status::Closed;
                self.any_closed = true;
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

impl<'a, Message, Theme, Renderer>
    From<ContextMenu<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a + container::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>:
        From<container::StyleFn<'a, Theme>>,
    Renderer: advanced::Renderer + 'a,
{
    fn from(context_menu: ContextMenu<'a, Message, Theme, Renderer>) -> Self {
        Element::new(context_menu)
    }
}

struct Overlay<'a, 'b, Message, Theme, Renderer> {
    menu: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut widget::Tree,
    status: &'b mut Status,
    position: Point,
    layout: Layout,
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'_, '_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        cursor: mouse::Cursor,
    ) {
        renderer.with_layer(self.layout.bounds(), |renderer| {
            self.menu.as_widget().draw(
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
        self.menu.as_widget_mut().operate(
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
        if let Event::Mouse(mouse::Event::ButtonPressed { .. }) = &event
            && cursor.position_over(self.layout.bounds()).is_none()
            && self.status.keep_open_bounds().is_none_or(
                |(keep_open_vector, keep_open_size)| {
                    let keep_open_bounds = Rectangle::new(
                        self.position + *keep_open_vector,
                        *keep_open_size,
                    );

                    cursor.position_over(keep_open_bounds).is_none()
                },
            )
        {
            *self.status = Status::Closed;
            shell.invalidate_overlay();
        }

        self.menu.as_widget_mut().update(
            self.tree,
            event,
            self.layout,
            cursor,
            renderer,
            shell,
            &self.layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        let interaction = self.menu.as_widget().mouse_interaction(
            self.tree,
            self.layout,
            cursor,
            &self.layout.bounds(),
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
