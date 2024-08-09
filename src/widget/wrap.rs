use iced::{
    advanced::{self, layout, Widget},
    event, Element,
};

pub fn wrap<'a, Message, Theme, Renderer>(
    element: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Wrap<'a, Message, Theme, Renderer> {
    Wrap::new(element)
}

pub struct Wrap<'a, Message, Theme, Renderer, OnEvent = (), Layout = ()> {
    inner: Element<'a, Message, Theme, Renderer>,
    on_event: OnEvent,
    layout: Layout,
}

impl<'a, Message, Theme, Renderer> Wrap<'a, Message, Theme, Renderer> {
    pub fn new(inner: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            inner: inner.into(),
            on_event: (),
            layout: (),
        }
    }
}

impl<'a, Message, Theme, Renderer, OnEvent, Layout>
    Wrap<'a, Message, Theme, Renderer, OnEvent, Layout>
{
    pub fn on_event<T>(self, on_event: T) -> Wrap<'a, Message, Theme, Renderer, T, Layout>
    where
        T: self::OnEvent<'a, Message, Theme, Renderer> + 'a,
    {
        Wrap {
            inner: self.inner,
            layout: self.layout,
            on_event,
        }
    }

    pub fn layout<T>(self, layout: T) -> Wrap<'a, Message, Theme, Renderer, OnEvent, T> {
        Wrap {
            inner: self.inner,
            on_event: self.on_event,
            layout,
        }
    }
}

pub trait OnEvent<'a, Message, Theme, Renderer> {
    fn on_event(
        &mut self,
        inner: &mut Element<'a, Message, Theme, Renderer>,
        state: &mut advanced::widget::Tree,
        event: iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> event::Status;
}

impl<'a, Message, Theme, Renderer> OnEvent<'a, Message, Theme, Renderer> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn on_event(
        &mut self,
        inner: &mut Element<'a, Message, Theme, Renderer>,
        state: &mut advanced::widget::Tree,
        event: iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> event::Status {
        inner.as_widget_mut().on_event(
            state, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
    }
}

impl<'a, T, Message, Theme, Renderer> OnEvent<'a, Message, Theme, Renderer> for T
where
    T: Fn(
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
        inner: &mut Element<'a, Message, Theme, Renderer>,
        state: &mut advanced::widget::Tree,
        event: iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> event::Status {
        self(
            inner, state, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
    }
}

pub trait Layout<'a, Message, Theme, Renderer> {
    fn layout(
        &self,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> layout::Node;
}

impl<'a, Message, Theme, Renderer> Layout<'a, Message, Theme, Renderer> for ()
where
    Renderer: advanced::Renderer + 'a,
{
    fn layout(
        &self,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> layout::Node {
        inner.as_widget().layout(tree, renderer, limits)
    }
}

impl<'a, T, Message, Theme, Renderer> Layout<'a, Message, Theme, Renderer> for T
where
    T: Fn(
            &Element<'a, Message, Theme, Renderer>,
            &mut iced::advanced::widget::Tree,
            &Renderer,
            &iced::advanced::layout::Limits,
        ) -> layout::Node
        + 'a,
{
    fn layout(
        &self,
        inner: &Element<'a, Message, Theme, Renderer>,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> layout::Node {
        self(inner, tree, renderer, limits)
    }
}

impl<'a, Message, Theme, Renderer, OnEvent, Layout> Widget<Message, Theme, Renderer>
    for Wrap<'a, Message, Theme, Renderer, OnEvent, Layout>
where
    Renderer: advanced::Renderer,
    OnEvent: self::OnEvent<'a, Message, Theme, Renderer>,
    Layout: self::Layout<'a, Message, Theme, Renderer>,
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
        self.layout.layout(&self.inner, tree, renderer, limits)
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
        self.inner
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport)
    }

    fn size_hint(&self) -> iced::Size<iced::Length> {
        self.inner.as_widget().size_hint()
    }

    fn tag(&self) -> advanced::widget::tree::Tag {
        self.inner.as_widget().tag()
    }

    fn state(&self) -> advanced::widget::tree::State {
        self.inner.as_widget().state()
    }

    fn children(&self) -> Vec<advanced::widget::Tree> {
        self.inner.as_widget().children()
    }

    fn diff(&self, tree: &mut advanced::widget::Tree) {
        self.inner.as_widget().diff(tree)
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
            .operate(state, layout, renderer, operation)
    }

    fn on_event(
        &mut self,
        state: &mut advanced::widget::Tree,
        event: iced::Event,
        layout: advanced::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> advanced::graphics::core::event::Status {
        self.on_event.on_event(
            &mut self.inner,
            state,
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
        self.inner
            .as_widget()
            .mouse_interaction(state, layout, cursor, viewport, renderer)
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
            .overlay(state, layout, renderer, translation)
    }
}

impl<'a, Message, Theme, Renderer, OnEvent, Layout>
    From<Wrap<'a, Message, Theme, Renderer, OnEvent, Layout>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: advanced::Renderer + 'a,
    OnEvent: self::OnEvent<'a, Message, Theme, Renderer> + 'a,
    Layout: self::Layout<'a, Message, Theme, Renderer> + 'a,
{
    fn from(wrap: Wrap<'a, Message, Theme, Renderer, OnEvent, Layout>) -> Self {
        Element::new(wrap)
    }
}
