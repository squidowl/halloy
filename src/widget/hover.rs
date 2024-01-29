use super::Element;

pub fn hover<'a, Message: 'a>(
    f: impl Fn(bool) -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    component::Hover::new(f).into()
}

mod component {
    use iced::widget::{component, Component};

    use crate::{
        theme::Theme,
        widget::{Element, Renderer},
    };

    pub enum Event<M> {
        Change(super::widget::Cursor),
        Message(M),
    }

    pub struct Hover<'a, Message> {
        view: Box<dyn Fn(bool) -> Element<'a, Message> + 'a>,
    }

    impl<'a, Message> Hover<'a, Message> {
        pub fn new(view: impl Fn(bool) -> Element<'a, Message> + 'a) -> Self {
            Self {
                view: Box::new(view),
            }
        }
    }

    impl<'a, Message> Component<Message, Theme, Renderer> for Hover<'a, Message> {
        type State = bool;
        type Event = Event<Message>;

        fn update(&mut self, hovered: &mut Self::State, event: Self::Event) -> Option<Message> {
            match event {
                Event::Change(change) => {
                    match change {
                        super::widget::Cursor::Entered => *hovered = true,
                        super::widget::Cursor::Left => *hovered = false,
                    }
                    None
                }
                Event::Message(message) => Some(message),
            }
        }

        fn view(&self, hovered: &Self::State) -> Element<'_, Self::Event> {
            super::widget::Hover::new((self.view)(*hovered).map(Event::Message), Event::Change)
                .into()
        }
    }

    impl<'a, Message> From<Hover<'a, Message>> for Element<'a, Message>
    where
        Message: 'a,
    {
        fn from(hover: Hover<'a, Message>) -> Self {
            component(hover)
        }
    }
}

mod widget {
    use iced::advanced::widget::{self, tree};
    use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
    use iced::{Length, Size};

    use crate::widget::{Element, Renderer};
    use crate::Theme;

    pub enum Cursor {
        Entered,
        Left,
    }

    pub struct Hover<'a, Message> {
        content: Element<'a, Message>,
        on_change: Box<dyn Fn(Cursor) -> Message + 'a>,
    }

    impl<'a, Message> Hover<'a, Message> {
        pub fn new(
            content: impl Into<Element<'a, Message>>,
            on_change: impl Fn(Cursor) -> Message + 'a,
        ) -> Self {
            Self {
                content: content.into(),
                on_change: Box::new(on_change),
            }
        }
    }

    impl<'a, Message> Widget<Message, Theme, Renderer> for Hover<'a, Message> {
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
            self.content.as_widget().layout(tree, renderer, limits)
        }

        fn tag(&self) -> widget::tree::Tag {
            struct Marker;
            tree::Tag::of::<Marker>()
        }

        fn state(&self) -> widget::tree::State {
            tree::State::new(false)
        }

        fn children(&self) -> Vec<widget::Tree> {
            vec![widget::Tree::new(&self.content)]
        }

        fn diff(&self, tree: &mut widget::Tree) {
            tree.diff_children(&[&self.content]);
        }

        fn draw(
            &self,
            tree: &widget::Tree,
            renderer: &mut Renderer,
            theme: &Theme,
            style: &renderer::Style,
            layout: Layout<'_>,
            cursor: mouse::Cursor,
            viewport: &iced::Rectangle,
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

        fn on_event(
            &mut self,
            tree: &mut widget::Tree,
            event: iced::Event,
            layout: Layout<'_>,
            cursor: mouse::Cursor,
            renderer: &Renderer,
            clipboard: &mut dyn Clipboard,
            shell: &mut Shell<'_, Message>,
            viewport: &iced::Rectangle,
        ) -> iced::event::Status {
            let hovered = tree.state.downcast_mut::<bool>();
            let prev_hovered = *hovered;
            *hovered = cursor.position_over(layout.bounds()).is_some();

            match (prev_hovered, *hovered) {
                (true, false) => {
                    shell.publish((self.on_change)(Cursor::Left));
                }
                (false, true) => {
                    shell.publish((self.on_change)(Cursor::Entered));
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
            viewport: &iced::Rectangle,
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

        fn operate(
            &self,
            tree: &mut widget::Tree,
            layout: Layout<'_>,
            renderer: &Renderer,
            operation: &mut dyn widget::Operation<Message>,
        ) {
            self.content
                .as_widget()
                .operate(&mut tree.children[0], layout, renderer, operation)
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

    impl<'a, Message> From<Hover<'a, Message>> for Element<'a, Message>
    where
        Message: 'a,
    {
        fn from(hover: Hover<'a, Message>) -> Self {
            Element::new(hover)
        }
    }
}
