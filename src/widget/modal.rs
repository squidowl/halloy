use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Widget, tree};
use iced::advanced::{self, Shell, overlay, renderer};
use iced::alignment::Alignment;
use iced::keyboard::key;
use iced::{
    Color, Element, Event, Length, Rectangle, Shadow, Size, Vector, keyboard,
    mouse, touch,
};

const BASE_WIDTH: f32 = 380.0;
const BASE_BUTTON_WIDTH: f32 = 250.0;
const BASE_PADDING: f32 = 25.0;

pub fn width(font: &data::config::Font) -> Length {
    Length::Fixed(BASE_WIDTH * crate::theme::scale_for_font_size(font))
}

pub fn button_width(font: &data::config::Font) -> Length {
    Length::Fixed(BASE_BUTTON_WIDTH * crate::theme::scale_for_font_size(font))
}

pub fn container<'a, Message: 'a>(
    content: impl Into<crate::widget::Element<'a, Message>>,
    font: &data::config::Font,
) -> crate::widget::Container<'a, Message> {
    let scale = crate::theme::scale_for_font_size(font);

    iced::widget::container(
        iced::widget::scrollable(content).height(Length::Shrink),
    )
    .width(width(font))
    .align_x(iced::Alignment::Center)
    .padding(BASE_PADDING * scale)
}

pub fn modal<'a, Message, Theme, Renderer>(
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    modal: impl Into<Element<'a, Message, Theme, Renderer>>,
    on_blur: impl Fn() -> Message + 'a,
    backdrop_alpha: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: 'a + advanced::Renderer,
    Message: 'a,
{
    Modal::new(base, modal, on_blur, backdrop_alpha).into()
}

/// A widget that centers a modal element over some base element
pub struct Modal<'a, Message, Theme, Renderer> {
    base: Element<'a, Message, Theme, Renderer>,
    modal: Element<'a, Message, Theme, Renderer>,
    on_blur: Box<dyn Fn() -> Message + 'a>,
    backdrop: Color,
    shadow: Shadow,
}

#[derive(Debug, Default)]
struct State {
    layout: Option<layout::Node>,
}

impl<'a, Message, Theme, Renderer> Modal<'a, Message, Theme, Renderer> {
    /// Returns a new [`Modal`]
    pub fn new(
        base: impl Into<Element<'a, Message, Theme, Renderer>>,
        modal: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_blur: impl Fn() -> Message + 'a,
        backdrop_alpha: f32,
    ) -> Self {
        Self {
            base: base.into(),
            modal: modal.into(),
            on_blur: Box::new(on_blur),
            backdrop: Color {
                a: backdrop_alpha.clamp(0.0, 1.0),
                ..Color::BLACK
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                offset: Vector::new(0.0, 10.0),
                blur_radius: 24.0,
            },
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Modal<'_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn diff(&mut self, tree: &mut widget::Tree) {
        tree.state.downcast_mut::<State>().layout = None;
        tree.diff_children(&mut [&mut self.base, &mut self.modal]);
    }

    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.base.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        )
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if matches!(
            event,
            Event::Mouse(_)
                | Event::Keyboard(_)
                | Event::Touch(touch::Event::FingerPressed { .. })
                | Event::Touch(touch::Event::FingerMoved { .. })
                | Event::Touch(touch::Event::FingerLifted { .. })
                | Event::Touch(touch::Event::FingerLost { .. })
        ) {
            return;
        }

        self.base.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            mouse::Cursor::Unavailable,
            renderer,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            mouse::Cursor::Unavailable,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
        window: Size,
    ) -> Vec<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();

        if state.layout.is_none() {
            let size = layout.bounds().size();
            let limits = layout::Limits::new(Size::ZERO, size)
                .width(Length::Fill)
                .height(Length::Fill);

            state.layout = Some(
                self.modal
                    .as_widget_mut()
                    .layout(&mut tree.children[1], renderer, &limits)
                    .align(Alignment::Center, Alignment::Center, limits.max),
            );
        }

        let node = state
            .layout
            .as_ref()
            .expect("the modal's node was computed above");

        vec![overlay::Element::new(Box::new(Overlay {
            content: &mut self.modal,
            tree: &mut tree.children[1],
            layout: Layout::new(node).move_to(layout.position() + translation),
            on_blur: &self.on_blur,
            backdrop: self.backdrop,
            shadow: self.shadow,
            window,
        }))]
    }

    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        self.base.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            viewport,
            renderer,
            operation,
        );
    }
}

struct Overlay<'a, 'b, Message, Theme, Renderer> {
    content: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut widget::Tree,
    layout: Layout<'b>,
    on_blur: &'b dyn Fn() -> Message,
    backdrop: Color,
    shadow: Shadow,
    window: Size,
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'_, '_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn update(
        &mut self,
        event: &Event,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) => {
                shell.publish((self.on_blur)());
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::ButtonPressed {
                button: mouse::Button::Left,
                ..
            }) => {
                let bounds = self.layout.bounds();

                if !cursor.is_over(bounds) {
                    shell.publish((self.on_blur)());
                    shell.capture_event();
                    return;
                }
            }
            _ => {}
        }

        self.content.as_widget_mut().update(
            self.tree,
            event,
            self.layout,
            cursor,
            renderer,
            shell,
            &self.layout.bounds(),
        );
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        cursor: mouse::Cursor,
    ) {
        renderer.with_layer(self.layout.bounds(), |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: self.layout.bounds(),
                    ..renderer::Quad::default()
                },
                self.backdrop,
            );

            let bounds = self.layout.bounds();

            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..iced::Border::default()
                    },
                    shadow: self.shadow,
                    ..renderer::Quad::default()
                },
                Color::TRANSPARENT,
            );

            self.content.as_widget().draw(
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
        self.content.as_widget_mut().operate(
            self.tree,
            self.layout,
            &self.layout.bounds(),
            renderer,
            operation,
        );
    }

    fn mouse_interaction(
        &self,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            self.layout,
            cursor,
            &self.layout.bounds(),
            renderer,
        )
    }

    fn overlay<'c>(
        &'c mut self,
        renderer: &Renderer,
    ) -> Vec<overlay::Element<'c, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            self.tree,
            self.layout,
            renderer,
            &self.layout.bounds(),
            Vector::ZERO,
            self.window,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Modal<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: 'a + advanced::Renderer,
    Message: 'a,
{
    fn from(modal: Modal<'a, Message, Theme, Renderer>) -> Self {
        Element::new(modal)
    }
}
