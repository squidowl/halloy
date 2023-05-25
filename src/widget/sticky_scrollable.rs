use iced::widget::runtime::core::widget::{tree, Tree};
use iced::widget::runtime::core::{layout, renderer, Clipboard, Layout, Shell, Widget};
use iced::widget::scrollable::{self};

use iced::event::{self, Event};
use iced::{mouse, Element};
use iced::{Length, Point, Rectangle};

use super::Renderer;

/// Same as the scrollable from iced, but sticks to the bottom like you see in most chat apps.
pub fn scrollable<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Scrollable<'a, Message> {
    Scrollable::new(content)
}

#[allow(missing_debug_implementations)]
pub struct Scrollable<'a, Message> {
    height: Length,
    on_scroll: Option<Box<dyn Fn(f32) -> Ev<Message> + 'a>>,
    content: Element<'a, Ev<Message>>,
}

enum Ev<Message> {
    Scroll(f32),
    Propagate(Message),
}

impl<'a, Message: 'a> Scrollable<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Scrollable {
            height: Length::Shrink,
            on_scroll: Some(Box::new(Ev::Scroll)),
            content: content.into().map(Ev::Propagate),
        }
    }
}

impl<'a, Message> Widget<Message, Renderer> for Scrollable<'a, Message>
where
    Renderer: renderer::Renderer,
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
            false,
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
            Default::default(),
            Default::default(),
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
        theme: &<Renderer as renderer::Renderer>::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        scrollable::draw(
            tree.state.downcast_ref::<scrollable::State>(),
            renderer,
            theme,
            layout,
            cursor_position,
            Default::default(),
            Default::default(),
            self.style_sheet.as_ref(),
            |renderer, layout, cursor_position, viewport| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
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
            Default::default(),
            Default::default(),
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

impl<'a, Message, Renderer> From<Scrollable<'a, Message>> for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + renderer::Renderer,
{
    fn from(text_input: Scrollable<'a, Message>) -> Element<'a, Message, Renderer> {
        Element::new(text_input)
    }
}
