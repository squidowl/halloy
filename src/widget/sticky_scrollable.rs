use iced_pure::widget::scrollable::StyleSheet;
use iced_pure::widget::tree::{self, Tree};
use iced_pure::{Element, Widget};

use iced_native::event::{self, Event};
use iced_native::layout::{self, Layout};
use iced_native::mouse;
use iced_native::renderer;
use iced_native::widget::scrollable;
use iced_native::{Clipboard, Length, Point, Rectangle, Shell};

/// Same as the scrollable from iced, but sticks to the bottom like you see in most chat apps.
pub fn scrollable<'a, Message: 'a, Renderer: iced_native::Renderer + 'a>(
    content: impl Into<Element<'a, Message, Renderer>>,
) -> Scrollable<'a, Message, Renderer> {
    Scrollable::new(content)
}

#[allow(missing_debug_implementations)]
pub struct Scrollable<'a, Message, Renderer> {
    height: Length,
    scrollbar_width: u16,
    scrollbar_margin: u16,
    scroller_width: u16,
    on_scroll: Option<Box<dyn Fn(f32) -> Ev<Message> + 'a>>,
    style_sheet: Box<dyn StyleSheet + 'a>,
    content: Element<'a, Ev<Message>, Renderer>,
}

enum Ev<Message> {
    Scroll(f32),
    Propagate(Message),
}

impl<'a, Message: 'a, Renderer: iced_native::Renderer + 'a> Scrollable<'a, Message, Renderer> {
    pub fn new(content: impl Into<Element<'a, Message, Renderer>>) -> Self {
        Scrollable {
            height: Length::Shrink,
            scrollbar_width: 10,
            scrollbar_margin: 0,
            scroller_width: 10,
            on_scroll: Some(Box::new(Ev::Scroll)),
            style_sheet: Default::default(),
            content: content.into().map(Ev::Propagate),
        }
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn scrollbar_width(mut self, scrollbar_width: u16) -> Self {
        self.scrollbar_width = scrollbar_width.max(1);
        self
    }

    pub fn scrollbar_margin(mut self, scrollbar_margin: u16) -> Self {
        self.scrollbar_margin = scrollbar_margin;
        self
    }

    pub fn scroller_width(mut self, scroller_width: u16) -> Self {
        self.scroller_width = scroller_width.max(1);
        self
    }

    pub fn style(mut self, style_sheet: impl Into<Box<dyn StyleSheet + 'a>>) -> Self {
        self.style_sheet = style_sheet.into();
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Scrollable<'a, Message, Renderer>
where
    Renderer: iced_native::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<scrollable::State>()
    }

    fn state(&self) -> tree::State {
        let mut state = scrollable::State::new();
        state.snap_to(1.0);

        tree::State::new(state)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content))
    }

    fn width(&self) -> Length {
        self.content.as_widget().width()
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        scrollable::layout(
            renderer,
            limits,
            Widget::<Message, Renderer>::width(self),
            self.height,
            |renderer, limits| self.content.as_widget().layout(renderer, limits),
        )
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<scrollable::State>();

        let mut local_messages = Vec::new();
        let mut local_shell = Shell::new(&mut local_messages);

        let status = scrollable::update(
            state,
            event,
            layout,
            cursor_position,
            clipboard,
            &mut local_shell,
            self.scrollbar_width,
            self.scrollbar_margin,
            self.scroller_width,
            &self.on_scroll,
            |event, layout, cursor_position, clipboard, shell| {
                self.content.as_widget_mut().on_event(
                    &mut tree.children[0],
                    event,
                    layout,
                    cursor_position,
                    renderer,
                    clipboard,
                    shell,
                )
            },
        );

        let mut offset = 0.0;
        for message in local_messages {
            match message {
                Ev::Scroll(f) => {
                    offset = f;
                }
                Ev::Propagate(message) => shell.publish(message),
            }
        }

        if offset == 1.0 {
            state.snap_to(1.0);
        }

        status
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        scrollable::draw(
            tree.state.downcast_ref::<scrollable::State>(),
            renderer,
            layout,
            cursor_position,
            self.scrollbar_width,
            self.scrollbar_margin,
            self.scroller_width,
            self.style_sheet.as_ref(),
            |renderer, layout, cursor_position, viewport| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    style,
                    layout,
                    cursor_position,
                    viewport,
                )
            },
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        scrollable::mouse_interaction(
            tree.state.downcast_ref::<scrollable::State>(),
            layout,
            cursor_position,
            self.scrollbar_width,
            self.scrollbar_margin,
            self.scroller_width,
            |layout, cursor_position, viewport| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    layout,
                    cursor_position,
                    viewport,
                    renderer,
                )
            },
        )
    }
}

impl<'a, Message, Renderer> From<Scrollable<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced_native::Renderer,
{
    fn from(text_input: Scrollable<'a, Message, Renderer>) -> Element<'a, Message, Renderer> {
        Element::new(text_input)
    }
}
