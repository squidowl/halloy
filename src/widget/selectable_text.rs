use std::borrow::Cow;

use iced::advanced::renderer::Quad;
use iced::advanced::text::Paragraph;
use iced::advanced::widget::{operation, tree, Operation, Tree};
use iced::advanced::{layout, mouse, renderer, text, widget, Layout, Widget};
use iced::widget::text_input::Value;
use iced::{
    alignment, event, touch, Border, Color, Command, Element, Length, Pixels, Point, Rectangle,
    Shadow, Size,
};

use self::selection::selection;
pub use self::text::{LineHeight, Shaping};

mod selection;

pub fn selectable_text<'a, Theme, Renderer>(content: impl ToString) -> Text<'a, Theme, Renderer>
where
    Renderer: text::Renderer,
    Theme: StyleSheet,
{
    Text::new(content.to_string())
}

pub struct Text<'a, Theme, Renderer>
where
    Renderer: text::Renderer,
    Theme: StyleSheet,
{
    content: Cow<'a, str>,
    size: Option<Pixels>,
    line_height: LineHeight,
    width: Length,
    height: Length,
    horizontal_alignment: alignment::Horizontal,
    vertical_alignment: alignment::Vertical,
    font: Option<Renderer::Font>,
    shaping: Shaping,
    style: <Theme as StyleSheet>::Style,
}

impl<'a, Theme, Renderer> Text<'a, Theme, Renderer>
where
    Renderer: text::Renderer,
    Theme: StyleSheet,
{
    pub fn new(content: impl Into<Cow<'a, str>>) -> Self {
        Text {
            content: content.into(),
            size: None,
            line_height: LineHeight::default(),
            font: None,
            width: Length::Shrink,
            height: Length::Shrink,
            horizontal_alignment: alignment::Horizontal::Left,
            vertical_alignment: alignment::Vertical::Top,
            #[cfg(debug_assertions)]
            shaping: Shaping::Basic,
            #[cfg(not(debug_assertions))]
            shaping: Shaping::Advanced,
            style: Default::default(),
        }
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    pub fn style(mut self, style: impl Into<<Theme as StyleSheet>::Style>) -> Self {
        self.style = style.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn horizontal_alignment(mut self, alignment: alignment::Horizontal) -> Self {
        self.horizontal_alignment = alignment;
        self
    }

    pub fn vertical_alignment(mut self, alignment: alignment::Vertical) -> Self {
        self.vertical_alignment = alignment;
        self
    }

    pub fn shaping(mut self, shaping: Shaping) -> Self {
        self.shaping = shaping;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Text<'a, Theme, Renderer>
where
    Renderer: text::Renderer,
    Theme: StyleSheet,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::default())
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        layout::sized(&limits, self.width, self.height, |limits| {
            let bounds = limits.max();

            let size = self
                .size
                .map(Pixels::from)
                .unwrap_or_else(|| renderer.default_size());
            let font = self.font.unwrap_or_else(|| renderer.default_font());

            let paragraph = state.paragraph.update(text::Text {
                content: &self.content,
                size,
                line_height: self.line_height,
                bounds,
                font,
                horizontal_alignment: self.horizontal_alignment,
                vertical_alignment: self.vertical_alignment,
                shaping: self.shaping,
            });

            paragraph.min_bounds()
        })
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: iced::Event,
        _layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        _shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | iced::Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(cursor) = cursor.position() {
                    state.interaction = Interaction::Selecting(selection::Raw {
                        start: cursor,
                        end: cursor,
                    });
                } else {
                    state.interaction = Interaction::Idle;
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | iced::Event::Touch(touch::Event::FingerLifted { .. })
            | iced::Event::Touch(touch::Event::FingerLost { .. }) => {
                if let Interaction::Selecting(raw) = state.interaction {
                    state.interaction = Interaction::Selected(raw);
                } else {
                    state.interaction = Interaction::Idle;
                }
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. })
            | iced::Event::Touch(touch::Event::FingerMoved { .. }) => {
                if let Some(cursor) = cursor.position() {
                    if let Interaction::Selecting(raw) = &mut state.interaction {
                        raw.end = cursor;
                    }
                }
            }
            _ => {}
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor_position: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if viewport.intersection(&bounds).is_none() {
            return;
        }

        let appearance = theme.appearance(&self.style);

        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        if let Some(selection) = state
            .interaction
            .selection()
            .and_then(|raw| raw.resolve(bounds))
        {
            let line_height = f32::from(
                self.line_height.to_absolute(
                    self.size
                        .map(Pixels::from)
                        .unwrap_or_else(|| renderer.default_size()),
                ),
            );

            let baseline_y =
                bounds.y + ((selection.start.y - bounds.y) / line_height).floor() * line_height;

            let height = selection.end.y - baseline_y - 0.5;
            let rows = (height / line_height).ceil() as usize;

            for row in 0..rows {
                let (x, width) = if row == 0 {
                    (
                        selection.start.x,
                        if rows == 1 {
                            f32::min(selection.end.x, bounds.x + bounds.width) - selection.start.x
                        } else {
                            bounds.x + bounds.width - selection.start.x
                        },
                    )
                } else if row == rows - 1 {
                    (bounds.x, selection.end.x - bounds.x)
                } else {
                    (bounds.x, bounds.width)
                };
                let y = baseline_y + row as f32 * line_height;

                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(Point::new(x, y), Size::new(width, line_height)),
                        border: Border {
                            radius: 0.0.into(),
                            width: 0.0,
                            color: Color::TRANSPARENT,
                        },
                        shadow: Shadow::default(),
                    },
                    appearance.selection_color,
                );
            }
        }

        // TODO: This method is better for ensuring whole letters are visually selected,
        // but breaks down once wrapping comes to play.
        // if let Some(Selection { start, end }) = state.selection().and_then(|raw| {
        //     selection(
        //         raw,
        //         renderer,
        //         self.font,
        //         self.size,
        //         self.line_height,
        //         layout.bounds(),
        //         &value,
        //     )
        // }) {
        //     let pre_value = (start > 0).then(|| value.select(0, start));
        //     let value = value.select(start, end);

        //     let pre_width = pre_value
        //         .as_ref()
        //         .map(|value| measure(renderer, value, self.size, self.font));
        //     let selected_width = measure(renderer, &value, self.size, self.font);

        //     let line_height = f32::from(
        //         self.line_height
        //             .to_absolute(self.size.unwrap_or_else(|| renderer.default_size()).into()),
        //     );

        //     let bounds = layout.bounds();

        //     let mut position = bounds.position();
        //     let mut remaining = pre_width.unwrap_or_default();

        //     while remaining > 0.0 {
        //         let max_width = bounds.width - (position.x - bounds.x);
        //         let width = remaining.min(max_width);

        //         position = if width == max_width {
        //             Point::new(bounds.x, position.y + line_height)
        //         } else {
        //             Point::new(position.x + width, position.y)
        //         };
        //         remaining -= width;
        //     }

        //     let mut remaining = selected_width;

        //     while remaining > 0.0 {
        //         let max_width = bounds.width - (position.x - bounds.x);
        //         let width = remaining.min(max_width);

        //         renderer.fill_quad(
        //             Quad {
        //                 bounds: Rectangle::new(position, Size::new(width, line_height)),
        //                 border_radius: 0.0.into(),
        //                 border_width: 0.0,
        //                 border_color: Color::TRANSPARENT,
        //             },
        //             theme.selection_color(&self.style),
        //         );

        //         position = if width == max_width {
        //             Point::new(bounds.x, position.y + line_height)
        //         } else {
        //             Point::new(position.x + width, position.y)
        //         };
        //         remaining -= width;
        //     }
        // }

        draw(renderer, style, layout, state, appearance, viewport);
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.position_over(layout.bounds()).is_some() {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        let bounds = layout.bounds();
        let value = Value::new(&self.content);
        if let Some(selection) = state.interaction.selection().and_then(|raw| {
            selection(
                raw,
                renderer,
                self.font,
                self.size,
                self.line_height,
                bounds,
                &value,
                &state.paragraph,
            )
        }) {
            let content = value.select(selection.start, selection.end).to_string();
            operation.custom(&mut (bounds.y, content), None);
        }
    }
}

fn draw<Renderer>(
    renderer: &mut Renderer,
    style: &renderer::Style,
    layout: Layout<'_>,
    state: &State<Renderer::Paragraph>,
    appearance: Appearance,
    viewport: &Rectangle,
) where
    Renderer: text::Renderer,
{
    let State { paragraph, .. } = &state;
    let bounds = layout.bounds();

    let x = match paragraph.horizontal_alignment() {
        alignment::Horizontal::Left => bounds.x,
        alignment::Horizontal::Center => bounds.center_x(),
        alignment::Horizontal::Right => bounds.x + bounds.width,
    };

    let y = match paragraph.vertical_alignment() {
        alignment::Vertical::Top => bounds.y,
        alignment::Vertical::Center => bounds.center_y(),
        alignment::Vertical::Bottom => bounds.y + bounds.height,
    };

    renderer.fill_paragraph(
        paragraph,
        Point::new(x, y),
        appearance.color.unwrap_or(style.text_color),
        *viewport,
    );
}

impl<'a, Message, Theme, Renderer> From<Text<'a, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Renderer: text::Renderer + 'a,
    Theme: StyleSheet,
{
    fn from(text: Text<'a, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}

#[derive(Debug, Default)]
pub struct State<P: Paragraph> {
    paragraph: P,
    interaction: Interaction,
}

#[derive(Debug, Clone, Copy, Default)]
enum Interaction {
    #[default]
    Idle,
    Selecting(selection::Raw),
    Selected(selection::Raw),
}

impl Interaction {
    fn selection(self) -> Option<selection::Raw> {
        match &self {
            Interaction::Idle => None,
            Interaction::Selecting(raw) | Interaction::Selected(raw) => Some(*raw),
        }
    }
}

// fn measure<Renderer>(
//     renderer: &Renderer,
//     value: &Value,
//     size: Option<f32>,
//     font: Option<Renderer::Font>,
// ) -> f32
// where
//     Renderer: text::Renderer,
// {
//     let size = size.unwrap_or_else(|| renderer.default_size());
//     let font = font.unwrap_or_else(|| renderer.default_font());

//     renderer.measure_width(&value.to_string(), size, font, text::Shaping::Advanced)
// }

pub fn selected<Message: 'static>(f: fn(Vec<(f32, String)>) -> Message) -> Command<Message> {
    struct Selected<T> {
        contents: Vec<(f32, String)>,
        f: fn(Vec<(f32, String)>) -> T,
    }

    impl<T> Operation<T> for Selected<T> {
        fn container(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            operate_on_children: &mut dyn FnMut(&mut dyn Operation<T>),
        ) {
            operate_on_children(self)
        }

        fn custom(&mut self, state: &mut dyn std::any::Any, _id: Option<&widget::Id>) {
            if let Some(content) = state.downcast_ref::<(f32, String)>() {
                self.contents.push(content.clone());
            }
        }

        fn finish(&self) -> operation::Outcome<T> {
            operation::Outcome::Some((self.f)(self.contents.clone()))
        }
    }

    Command::widget(Selected {
        contents: vec![],
        f,
    })
}

pub trait StyleSheet {
    type Style: Default;

    fn appearance(&self, style: &Self::Style) -> Appearance;
}

pub struct Appearance {
    pub color: Option<Color>,
    pub selection_color: Color,
}
