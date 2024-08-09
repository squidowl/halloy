use std::{marker::PhantomData, slice};

use iced::{
    advanced::{self, layout, Widget},
    event, Element,
};

pub fn wrap<'a, Message, Theme, Renderer>(
    element: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Wrap<'a, Message, Theme, Renderer> {
    Wrap::new(element)
}

pub struct Wrap<'a, Message, Theme, Renderer, OnEvent = (), Layout = (), State = ()> {
    inner: Element<'a, Message, Theme, Renderer>,
    on_event: OnEvent,
    layout: Layout,
    state: PhantomData<State>,
}

impl<'a, Message, Theme, Renderer> Wrap<'a, Message, Theme, Renderer> {
    pub fn new(inner: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            inner: inner.into(),
            on_event: (),
            layout: (),
            state: PhantomData,
        }
    }
}

impl<'a, Message, Theme, Renderer, OnEvent, Layout, State>
    Wrap<'a, Message, Theme, Renderer, OnEvent, Layout, State>
{
    pub fn on_event<T>(self, on_event: T) -> Wrap<'a, Message, Theme, Renderer, T, Layout, State> {
        Wrap {
            inner: self.inner,
            layout: self.layout,
            state: self.state,
            on_event,
        }
    }

    pub fn layout<T>(self, layout: T) -> Wrap<'a, Message, Theme, Renderer, OnEvent, T, State> {
        Wrap {
            inner: self.inner,
            on_event: self.on_event,
            state: self.state,
            layout,
        }
    }

    pub fn state<T>(self) -> Wrap<'a, Message, Theme, Renderer, OnEvent, Layout, T> {
        Wrap {
            inner: self.inner,
            on_event: self.on_event,
            layout: self.layout,
            state: PhantomData,
        }
    }
}

pub trait OnEvent<'a, Message, Theme, Renderer, State> {
    fn on_event(
        &mut self,
        state: &mut State,
        inner: &mut Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        event: iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> event::Status;
}

impl<'a, Message, Theme, Renderer, State> OnEvent<'a, Message, Theme, Renderer, State> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn on_event(
        &mut self,
        _state: &mut State,
        inner: &mut Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        event: iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> event::Status {
        inner.as_widget_mut().on_event(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
    }
}

impl<'a, T, Message, Theme, Renderer, State> OnEvent<'a, Message, Theme, Renderer, State> for T
where
    T: Fn(
            &mut State,
            &mut Element<'a, Message, Theme, Renderer>,
            &mut advanced::widget::Tree,
            iced::Event,
            advanced::Layout<'_>,
            advanced::mouse::Cursor,
            &Renderer,
            &mut dyn advanced::Clipboard,
            &mut advanced::Shell<'_, Message>,
            &iced::Rectangle,
        ) -> event::Status
        + 'a,
{
    fn on_event(
        &mut self,
        state: &mut State,
        inner: &mut Element<'a, Message, Theme, Renderer>,
        tree: &mut advanced::widget::Tree,
        event: iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> event::Status {
        self(
            state, inner, tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
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

impl<'a, Message, Theme, Renderer, State> Layout<'a, Message, Theme, Renderer, State> for ()
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

impl<'a, T, Message, Theme, Renderer, State> Layout<'a, Message, Theme, Renderer, State> for T
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

impl<'a, Message, Theme, Renderer, OnEvent, Layout, State> Widget<Message, Theme, Renderer>
    for Wrap<'a, Message, Theme, Renderer, OnEvent, Layout, State>
where
    Renderer: advanced::Renderer,
    OnEvent: self::OnEvent<'a, Message, Theme, Renderer, State>,
    Layout: self::Layout<'a, Message, Theme, Renderer, State>,
    State: Default + 'static,
{
    fn size(&self) -> iced::Size<iced::Length> {
        self.inner.as_widget().size()
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
        self.inner.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        )
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

    fn operate(
        &self,
        state: &mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn advanced::widget::Operation<()>,
    ) {
        self.inner
            .as_widget()
            .operate(&mut state.children[0], layout, renderer, operation)
    }

    fn on_event(
        &mut self,
        tree: &mut advanced::widget::Tree,
        event: iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> advanced::graphics::core::event::Status {
        self.on_event.on_event(
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
        )
    }

    fn mouse_interaction(
        &self,
        state: &advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> advanced::mouse::Interaction {
        self.inner.as_widget().mouse_interaction(
            &state.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut advanced::widget::Tree,
        layout: advanced::Layout<'_>,
        renderer: &Renderer,
        translation: iced::Vector,
    ) -> Option<advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.inner
            .as_widget_mut()
            .overlay(&mut state.children[0], layout, renderer, translation)
    }
}

impl<'a, Message, Theme, Renderer, OnEvent, Layout, State>
    From<Wrap<'a, Message, Theme, Renderer, OnEvent, Layout, State>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: advanced::Renderer + 'a,
    OnEvent: self::OnEvent<'a, Message, Theme, Renderer, State> + 'a,
    Layout: self::Layout<'a, Message, Theme, Renderer, State> + 'a,
    State: Default + 'static,
{
    fn from(wrap: Wrap<'a, Message, Theme, Renderer, OnEvent, Layout, State>) -> Self {
        Element::new(wrap)
    }
}
