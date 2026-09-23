use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{self, Shell, overlay, renderer};
use iced::keyboard::key;
use iced::{
    Alignment, Color, Element, Event, Length, Rectangle, Shadow, Size, Vector,
    keyboard, mouse, touch,
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

#[derive(Clone, Copy)]
enum Position {
    Center,
    Top,
}

pub fn modal<'a, Message, Theme, Renderer>(
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    modal: Option<Element<'a, Message, Theme, Renderer>>,
    on_blur: impl Fn() -> Message + 'a,
    backdrop_alpha: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: 'a + advanced::Renderer,
    Message: 'a,
{
    let shadow = Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
        offset: Vector::new(0.0, 10.0),
        blur_radius: 24.0,
    };

    Modal::new(
        base,
        modal,
        on_blur,
        backdrop_alpha,
        Position::Center,
        shadow,
    )
    .into()
}

pub fn top<'a, Message, Theme, Renderer>(
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    modal: Option<Element<'a, Message, Theme, Renderer>>,
    on_blur: impl Fn() -> Message + 'a,
    backdrop_alpha: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: 'a + advanced::Renderer,
    Message: 'a,
{
    Modal::new(
        base,
        modal,
        on_blur,
        backdrop_alpha,
        Position::Top,
        Shadow::default(),
    )
    .into()
}

/// A widget that displays optional modal content over a base element.
pub struct Modal<'a, Message, Theme, Renderer> {
    base: Element<'a, Message, Theme, Renderer>,
    modal: Option<Element<'a, Message, Theme, Renderer>>,
    on_blur: Box<dyn Fn() -> Message + 'a>,
    backdrop: Color,
    shadow: Shadow,
    position: Position,
}

impl<'a, Message, Theme, Renderer> Modal<'a, Message, Theme, Renderer> {
    /// Returns a new [`Modal`]
    fn new(
        base: impl Into<Element<'a, Message, Theme, Renderer>>,
        modal: Option<Element<'a, Message, Theme, Renderer>>,
        on_blur: impl Fn() -> Message + 'a,
        backdrop_alpha: f32,
        position: Position,
        shadow: Shadow,
    ) -> Self {
        Self {
            base: base.into(),
            modal,
            on_blur: Box::new(on_blur),
            backdrop: Color {
                a: backdrop_alpha.clamp(0.0, 1.0),
                ..Color::BLACK
            },
            shadow,
            position,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Modal<'_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn diff(&mut self, tree: &mut widget::Tree) {
        if let Some(modal) = &mut self.modal {
            tree.diff_children(&mut [&mut self.base, modal]);
        } else {
            tree.diff_children(std::slice::from_mut(&mut self.base));
        }
    }

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
        if self.modal.is_some()
            && matches!(
                event,
                Event::Mouse(_)
                    | Event::Keyboard(_)
                    | Event::Touch(touch::Event::FingerPressed { .. })
                    | Event::Touch(touch::Event::FingerMoved { .. })
                    | Event::Touch(touch::Event::FingerLifted { .. })
                    | Event::Touch(touch::Event::FingerLost { .. })
            )
        {
            return;
        }

        self.base.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            if self.modal.is_some() {
                mouse::Cursor::Unavailable
            } else {
                cursor
            },
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
            if self.modal.is_some() {
                mouse::Cursor::Unavailable
            } else {
                cursor
            },
            viewport,
        );
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
        let Some(modal) = &mut self.modal else {
            return self.base.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                translation,
                window,
            );
        };

        let size = layout.bounds().size();
        let limits = layout::Limits::new(Size::ZERO, size)
            .width(Length::Fill)
            .height(Length::Fill);

        modal
            .as_widget_mut()
            .layout(&mut tree.children[1], renderer, &limits);

        let size = tree.children[1].size;
        let offset = size.align(
            limits.max,
            Alignment::Center,
            match self.position {
                Position::Center => Alignment::Center,
                Position::Top => Alignment::Start,
            },
        );
        let layout =
            Layout::new(layout.size()).move_to(layout.position() + translation);
        let content_layout =
            Layout::new(size).move_to(layout.position() + offset);

        vec![overlay::Element::new(Box::new(Overlay {
            content: modal,
            tree: &mut tree.children[1],
            layout,
            content_layout,
            on_blur: &self.on_blur,
            backdrop: self.backdrop,
            shadow: self.shadow,
            window,
        }))]
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.modal.is_some() {
            mouse::Interaction::default()
        } else {
            self.base.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        }
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout,
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
    layout: Layout,
    content_layout: Layout,
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
                let bounds = self.content_layout.bounds();

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
            self.content_layout,
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

            let bounds = self.content_layout.bounds();

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
                self.content_layout,
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
            self.content_layout,
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
            self.content_layout,
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
            self.content_layout,
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
