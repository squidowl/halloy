use std::marker::PhantomData;
use std::slice;

use iced::advanced::{self, Widget, layout};
use iced::{Element, Rectangle};

pub fn decorate<'a, Message, Theme, Renderer>(
    element: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Decorate<'a, Message, Theme, Renderer> {
    Decorate::new(element)
}

pub struct Decorate<
    'a,
    Message,
    Theme,
    Renderer,
    Layout = (),
    Update = (),
    Draw = (),
    MouseInteraction = (),
    Operate = (),
    Overlay = (),
    State = (),
> {
    inner: Element<'a, Message, Theme, Renderer>,
    layout: Layout,
    update: Update,
    draw: Draw,
    mouse_interaction: MouseInteraction,
    operate: Operate,
    overlay: Overlay,
    state: PhantomData<State>,
}

impl<'a, Message, Theme, Renderer> Decorate<'a, Message, Theme, Renderer> {
    pub fn new(
        inner: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            inner: inner.into(),
            update: (),
            layout: (),
            draw: (),
            mouse_interaction: (),
            operate: (),
            overlay: (),
            state: PhantomData,
        }
    }
}

impl<
    'a,
    Message,
    Theme,
    Renderer,
    Layout,
    Update,
    Draw,
    MouseInteraction,
    Operate,
    Overlay,
    State,
>
    Decorate<
        'a,
        Message,
        Theme,
        Renderer,
        Layout,
        Update,
        Draw,
        MouseInteraction,
        Operate,
        Overlay,
        State,
    >
{
    pub fn layout<T, U>(
        self,
        layout: T,
    ) -> Decorate<
        'a,
        Message,
        Theme,
        Renderer,
        T,
        Update,
        Draw,
        MouseInteraction,
        Operate,
        Overlay,
        U,
    >
    where
        T: self::Layout<'a, Message, Theme, Renderer, U>,
    {
        Decorate {
            inner: self.inner,
            update: self.update,
            draw: self.draw,
            mouse_interaction: self.mouse_interaction,
            operate: self.operate,
            overlay: self.overlay,
            layout,
            state: PhantomData,
        }
    }

    pub fn update<T, U>(
        self,
        update: T,
    ) -> Decorate<
        'a,
        Message,
        Theme,
        Renderer,
        Layout,
        T,
        Draw,
        MouseInteraction,
        Operate,
        Overlay,
        U,
    >
    where
        T: self::Update<'a, Message, Theme, Renderer, U>,
    {
        Decorate {
            inner: self.inner,
            layout: self.layout,
            draw: self.draw,
            mouse_interaction: self.mouse_interaction,
            operate: self.operate,
            overlay: self.overlay,
            update,
            state: PhantomData,
        }
    }

    pub fn draw<T, U>(
        self,
        draw: T,
    ) -> Decorate<
        'a,
        Message,
        Theme,
        Renderer,
        Layout,
        Update,
        T,
        MouseInteraction,
        Operate,
        Overlay,
        U,
    >
    where
        T: self::Draw<'a, Message, Theme, Renderer, U>,
    {
        Decorate {
            inner: self.inner,
            update: self.update,
            layout: self.layout,
            mouse_interaction: self.mouse_interaction,
            operate: self.operate,
            overlay: self.overlay,
            draw,
            state: PhantomData,
        }
    }

    pub fn mouse_interaction<T, U>(
        self,
        mouse_interaction: T,
    ) -> Decorate<
        'a,
        Message,
        Theme,
        Renderer,
        Layout,
        Update,
        Draw,
        T,
        Operate,
        Overlay,
        U,
    >
    where
        T: self::MouseInteraction<'a, Message, Theme, Renderer, U>,
    {
        Decorate {
            inner: self.inner,
            layout: self.layout,
            update: self.update,
            draw: self.draw,
            operate: self.operate,
            overlay: self.overlay,
            mouse_interaction,
            state: PhantomData,
        }
    }

    pub fn operate<T, U>(
        self,
        operate: T,
    ) -> Decorate<
        'a,
        Message,
        Theme,
        Renderer,
        Layout,
        Update,
        Draw,
        MouseInteraction,
        T,
        Overlay,
        U,
    >
    where
        T: self::Operate<'a, Message, Theme, Renderer, U>,
    {
        Decorate {
            inner: self.inner,
            layout: self.layout,
            update: self.update,
            draw: self.draw,
            mouse_interaction: self.mouse_interaction,
            overlay: self.overlay,
            operate,
            state: PhantomData,
        }
    }

    pub fn overlay<T, U>(
        self,
        overlay: T,
    ) -> Decorate<
        'a,
        Message,
        Theme,
        Renderer,
        Layout,
        Update,
        Draw,
        MouseInteraction,
        Operate,
        T,
        U,
    >
    where
        T: self::Operate<'a, Message, Theme, Renderer, U>,
    {
        Decorate {
            inner: self.inner,
            layout: self.layout,
            update: self.update,
            draw: self.draw,
            mouse_interaction: self.mouse_interaction,
            operate: self.operate,
            overlay,
            state: PhantomData,
        }
    }
}

pub trait Layout<'a, Message, Theme, Renderer, State> {
    fn layout(
        &self,
        state: &mut State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> layout::Node;
}

impl<'a, Message, Theme, Renderer, State>
    Layout<'a, Message, Theme, Renderer, State> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn layout(
        &self,
        _state: &mut State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> layout::Node {
        inner.as_widget().layout(tree, renderer, limits)
    }
}

impl<'a, T, Message, Theme, Renderer, State>
    Layout<'a, Message, Theme, Renderer, State> for T
where
    T: Fn(
            &mut State,
            &Element<'a, Message, Theme, Renderer>,
            &mut iced::advanced::widget::Tree,
            &Renderer,
            &iced::advanced::layout::Limits,
        ) -> layout::Node
        + 'a,
{
    fn layout(
        &self,
        state: &mut State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> layout::Node {
        self(state, inner, tree, renderer, limits)
    }
}

pub trait Update<'a, Message, Theme, Renderer, State> {
    fn update(
        &mut self,
        state: &mut State,
        inner: &mut Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        event: &iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    );
}

impl<'a, Message, Theme, Renderer, State>
    Update<'a, Message, Theme, Renderer, State> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn update(
        &mut self,
        _state: &mut State,
        inner: &mut Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        event: &iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        inner.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }
}

impl<'a, T, Message, Theme, Renderer, State>
    Update<'a, Message, Theme, Renderer, State> for T
where
    T: Fn(
            &mut State,
            &mut Element<'a, Message, Theme, Renderer>,
            &mut advanced::widget::Tree,
            &iced::Event,
            advanced::Layout<'_>,
            advanced::mouse::Cursor,
            &Renderer,
            &mut dyn advanced::Clipboard,
            &mut advanced::Shell<'_, Message>,
            &iced::Rectangle,
        ) + 'a,
{
    fn update(
        &mut self,
        state: &mut State,
        inner: &mut Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        event: &iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        self(
            state, inner, tree, event, layout, cursor, renderer, clipboard,
            shell, viewport,
        );
    }
}

pub trait Draw<'a, Message, Theme, Renderer, State> {
    fn draw(
        &self,
        state: &State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    );
}

impl<'a, Message, Theme, Renderer, State>
    Draw<'a, Message, Theme, Renderer, State> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn draw(
        &self,
        _state: &State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        inner
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
}

impl<'a, T, Message, Theme, Renderer, State>
    Draw<'a, Message, Theme, Renderer, State> for T
where
    T: Fn(
            &State,
            &Element<'a, Message, Theme, Renderer>,
            &iced::advanced::widget::Tree,
            &mut Renderer,
            &Theme,
            &iced::advanced::renderer::Style,
            iced::advanced::Layout<'_>,
            iced::advanced::mouse::Cursor,
            &iced::Rectangle,
        ) + 'a,
{
    fn draw(
        &self,
        state: &State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self(
            state, inner, tree, renderer, theme, style, layout, cursor,
            viewport,
        );
    }
}

pub trait MouseInteraction<'a, Message, Theme, Renderer, State> {
    fn mouse_interaction(
        &self,
        state: &State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> advanced::mouse::Interaction;
}

impl<'a, Message, Theme, Renderer, State>
    MouseInteraction<'a, Message, Theme, Renderer, State> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn mouse_interaction(
        &self,
        _state: &State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> advanced::mouse::Interaction {
        inner
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }
}

impl<'a, T, Message, Theme, Renderer, State>
    MouseInteraction<'a, Message, Theme, Renderer, State> for T
where
    T: Fn(
            &State,
            &Element<'a, Message, Theme, Renderer>,
            &advanced::widget::Tree,
            advanced::Layout<'_>,
            advanced::mouse::Cursor,
            &iced::Rectangle,
            &Renderer,
        ) -> advanced::mouse::Interaction
        + 'a,
{
    fn mouse_interaction(
        &self,
        state: &State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> advanced::mouse::Interaction {
        self(state, inner, tree, layout, cursor, viewport, renderer)
    }
}

pub trait Operate<'a, Message, Theme, Renderer, State> {
    fn operate(
        &self,
        state: &mut State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation<()>,
    );
}

impl<'a, Message, Theme, Renderer, State>
    Operate<'a, Message, Theme, Renderer, State> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn operate(
        &self,
        _state: &mut State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation<()>,
    ) {
        inner.as_widget().operate(tree, layout, renderer, operation);
    }
}

impl<'a, T, Message, Theme, Renderer, State>
    Operate<'a, Message, Theme, Renderer, State> for T
where
    T: Fn(
            &mut State,
            &Element<'a, Message, Theme, Renderer>,
            &mut advanced::widget::Tree,
            advanced::Layout<'_>,
            &Renderer,
            &mut dyn advanced::widget::Operation<()>,
        ) + 'a,
{
    fn operate(
        &self,
        state: &mut State,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation<()>,
    ) {
        self(state, inner, tree, layout, renderer, operation);
    }
}

pub trait Overlay<'a, Message, Theme, Renderer, State> {
    fn overlay<'b>(
        &'b mut self,
        state: &'b mut State,
        inner: &'b mut Element<'a, Message, Theme, Renderer>,
        tree: &'b mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<advanced::overlay::Element<'b, Message, Theme, Renderer>>;
}

impl<'a, Message, Theme, Renderer, State>
    Overlay<'a, Message, Theme, Renderer, State> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn overlay<'b>(
        &'b mut self,
        _state: &'b mut State,
        inner: &'b mut Element<'a, Message, Theme, Renderer>,
        tree: &'b mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        inner.as_widget_mut().overlay(
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, T, Message, Theme, Renderer, State>
    Overlay<'a, Message, Theme, Renderer, State> for T
where
    T: for<'b> Fn(
            &'b mut State,
            &'b mut Element<'a, Message, Theme, Renderer>,
            &'b mut advanced::widget::Tree,
            advanced::Layout<'_>,
            &Renderer,
            iced::Vector,
        ) -> Option<
            advanced::overlay::Element<'b, Message, Theme, Renderer>,
        > + 'a,
{
    fn overlay<'b>(
        &'b mut self,
        state: &'b mut State,
        inner: &'b mut Element<'a, Message, Theme, Renderer>,
        tree: &'b mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        _viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self(state, inner, tree, layout, renderer, translation)
    }
}

impl<
    'a,
    Message,
    Theme,
    Renderer,
    Layout,
    Update,
    Draw,
    MouseInteraction,
    Operate,
    Overlay,
    State,
> Widget<Message, Theme, Renderer>
    for Decorate<
        'a,
        Message,
        Theme,
        Renderer,
        Layout,
        Update,
        Draw,
        MouseInteraction,
        Operate,
        Overlay,
        State,
    >
where
    Renderer: advanced::Renderer,
    Layout: self::Layout<'a, Message, Theme, Renderer, State>,
    Update: self::Update<'a, Message, Theme, Renderer, State>,
    Draw: self::Draw<'a, Message, Theme, Renderer, State>,
    MouseInteraction:
        self::MouseInteraction<'a, Message, Theme, Renderer, State> + 'a,
    Operate: self::Operate<'a, Message, Theme, Renderer, State> + 'a,
    Overlay: self::Overlay<'a, Message, Theme, Renderer, State> + 'a,
    State: Default + 'static,
{
    fn size(&self) -> iced::Size<iced::Length> {
        self.inner.as_widget().size()
    }

    fn size_hint(&self) -> iced::Size<iced::Length> {
        self.inner.as_widget().size_hint()
    }

    fn tag(&self) -> advanced::widget::tree::Tag {
        struct Marker<State>(State);
        advanced::widget::tree::Tag::of::<Marker<State>>()
    }

    fn state(&self) -> advanced::widget::tree::State {
        advanced::widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<advanced::widget::Tree> {
        vec![advanced::widget::Tree::new(&self.inner)]
    }

    fn diff(&self, tree: &mut advanced::widget::Tree) {
        tree.diff_children(slice::from_ref(&self.inner));
    }

    fn layout(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.layout.layout(
            tree.state.downcast_mut(),
            &self.inner,
            &mut tree.children[0],
            renderer,
            limits,
        )
    }

    fn update(
        &mut self,
        tree: &mut advanced::widget::Tree,
        event: &iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        self.update.update(
            tree.state.downcast_mut(),
            &mut self.inner,
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

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.draw.draw(
            tree.state.downcast_ref(),
            &self.inner,
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> advanced::mouse::Interaction {
        self.mouse_interaction.mouse_interaction(
            tree.state.downcast_ref(),
            &self.inner,
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &self,
        tree: &mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation<()>,
    ) {
        self.operate.operate(
            tree.state.downcast_mut(),
            &self.inner,
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.overlay.overlay(
            tree.state.downcast_mut(),
            &mut self.inner,
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<
    'a,
    Message,
    Theme,
    Renderer,
    Layout,
    Update,
    Draw,
    MouseInteraction,
    Operate,
    Overlay,
    State,
>
    From<
        Decorate<
            'a,
            Message,
            Theme,
            Renderer,
            Layout,
            Update,
            Draw,
            MouseInteraction,
            Operate,
            Overlay,
            State,
        >,
    > for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: advanced::Renderer + 'a,
    Layout: self::Layout<'a, Message, Theme, Renderer, State> + 'a,
    Update: self::Update<'a, Message, Theme, Renderer, State> + 'a,
    Draw: self::Draw<'a, Message, Theme, Renderer, State> + 'a,
    MouseInteraction:
        self::MouseInteraction<'a, Message, Theme, Renderer, State> + 'a,
    Operate: self::Operate<'a, Message, Theme, Renderer, State> + 'a,
    Overlay: self::Overlay<'a, Message, Theme, Renderer, State> + 'a,
    State: Default + 'static,
{
    fn from(
        wrap: Decorate<
            'a,
            Message,
            Theme,
            Renderer,
            Layout,
            Update,
            Draw,
            MouseInteraction,
            Operate,
            Overlay,
            State,
        >,
    ) -> Self {
        Element::new(wrap)
    }
}
