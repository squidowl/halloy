use std::borrow::Cow;
use std::ops::Deref;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self, Paragraph, Span, Text};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::Operation;
use iced::advanced::Widget;
use iced::widget::text::{LineHeight, Shaping};
use iced::widget::text_input::Value;
use iced::{alignment, event, Border, Shadow};
use iced::{mouse, touch};
use iced::{widget, Point};
use iced::{Color, Element, Length, Pixels, Rectangle, Size};
use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;

use super::selectable_text::{selection, Catalog, Interaction, Style, StyleFn};

/// Creates a new [`Rich`] text widget with the provided spans.
pub fn selectable_rich_text<'a, Message, Theme, Renderer>(
    spans: impl Into<Cow<'a, [CustomSpan<'a, Renderer::Font>]>>,
) -> Rich<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: text::Renderer,
{
    Rich::with_spans(spans)
}

/// A bunch of [`Rich`] text.
pub struct Rich<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    spans: Cow<'a, [Span<'a, Renderer::Font>]>,
    size: Option<Pixels>,
    line_height: LineHeight,
    width: Length,
    height: Length,
    font: Option<Renderer::Font>,
    align_x: alignment::Horizontal,
    align_y: alignment::Vertical,
    class: Theme::Class<'a>,

    value: Value,
    link_graphemes: Vec<(usize, usize)>,
    on_link_pressed: Option<Box<dyn Fn(String) -> Message + 'a>>,
}

impl<'a, Message, Theme, Renderer> Rich<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// Creates a new empty [`Rich`] text.
    pub fn new() -> Self {
        Self {
            spans: Cow::default(),
            size: None,
            line_height: LineHeight::default(),
            width: Length::Shrink,
            height: Length::Shrink,
            font: None,
            align_x: alignment::Horizontal::Left,
            align_y: alignment::Vertical::Top,
            class: Theme::default(),

            value: Value::new(""),
            link_graphemes: vec![],
            on_link_pressed: None,
        }
    }

    /// Creates a new [`Rich`] text with the given text spans.
    pub fn with_spans(spans: impl Into<Cow<'a, [CustomSpan<'a, Renderer::Font>]>>) -> Self {
        let custom_spans = spans.into();

        let mut i = 0;
        let link_graphemes = custom_spans.iter().fold(vec![], |mut acc, span| {
            let count = UnicodeSegmentation::graphemes(span.text.as_ref(), true).count();

            if span.is_link() {
                acc.push((i, i + count));
            }

            i += count;

            acc
        });

        let spans = custom_spans
            .iter()
            .cloned()
            .map(CustomSpan::into_span)
            .collect::<Vec<_>>();
        let value = Value::new(&spans.iter().map(|s| s.text.as_ref()).join(""));

        Self {
            spans: spans.into(),
            value,
            link_graphemes,
            ..Self::new()
        }
    }

    /// Sets the default size of the [`Rich`] text.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Sets the defualt [`LineHeight`] of the [`Rich`] text.
    pub fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the default font of the [`Rich`] text.
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the width of the [`Rich`] text boundaries.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Rich`] text boundaries.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Centers the [`Rich`] text, both horizontally and vertically.
    pub fn center(self) -> Self {
        self.align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
    }

    /// Sets the [`alignment::Horizontal`] of the [`Rich`] text.
    pub fn align_x(mut self, alignment: impl Into<alignment::Horizontal>) -> Self {
        self.align_x = alignment.into();
        self
    }

    /// Sets the [`alignment::Vertical`] of the [`Rich`] text.
    pub fn align_y(mut self, alignment: impl Into<alignment::Vertical>) -> Self {
        self.align_y = alignment.into();
        self
    }

    /// Sets the default style of the [`Rich`] text.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the default [`Color`] of the [`Rich`] text.
    pub fn color(self, color: impl Into<Color>) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.color_maybe(Some(color))
    }

    /// Sets the default [`Color`] of the [`Rich`] text, if `Some`.
    pub fn color_maybe(self, color: Option<impl Into<Color>>) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        let color = color.map(Into::into);

        self.style(move |_theme| Style {
            color,
            selection_color: Color::WHITE,
        })
    }

    /// Sets the default style class of the [`Rich`] text.
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    pub fn on_link_pressed(self, f: impl Fn(String) -> Message + 'a) -> Self {
        Self {
            on_link_pressed: Some(Box::new(f)),
            ..self
        }
    }
}

impl<'a, Message, Theme, Renderer> Default for Rich<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CustomSpan<'a, Font = iced::Font> {
    Span(Span<'a, Font>),
    Link(Span<'a, Font>),
}

impl<'a, Font> CustomSpan<'a, Font> {
    fn is_link(&self) -> bool {
        matches!(self, Self::Link(_))
    }

    fn into_span(self) -> Span<'a, Font> {
        match self {
            CustomSpan::Span(s) => s,
            CustomSpan::Link(s) => s,
        }
    }
}

impl<'a, Font> Deref for CustomSpan<'a, Font> {
    type Target = Span<'a, Font>;

    fn deref(&self) -> &Self::Target {
        match self {
            CustomSpan::Span(s) => s,
            CustomSpan::Link(s) => s,
        }
    }
}

struct State<P: Paragraph> {
    spans: Vec<Span<'static, P::Font>>,
    paragraph: P,
    interaction: Interaction,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Rich<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            spans: Vec::new(),
            paragraph: Renderer::Paragraph::default(),
            interaction: Interaction::default(),
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout(
            tree.state.downcast_mut::<State<Renderer::Paragraph>>(),
            renderer,
            limits,
            self.width,
            self.height,
            self.spans.as_ref(),
            self.line_height,
            self.size,
            self.font,
            self.align_x,
            self.align_y,
        )
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
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

                if let Some(cursor) = cursor.position_in(layout.bounds()) {
                    if let Some(link) =
                        is_over_link(cursor, &state.paragraph, &self.value, &self.link_graphemes)
                    {
                        if let Some(f) = self.on_link_pressed.as_ref() {
                            shell.publish((f)(link));

                            return event::Status::Captured;
                        }
                    }
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
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor_position: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if viewport.intersection(&bounds).is_none() {
            return;
        }

        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let style = theme.style(&self.class);

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
                    style.selection_color,
                );
            }
        }

        widget::text::draw(
            renderer,
            defaults,
            layout,
            &state.paragraph,
            widget::text::Style { color: style.color },
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        if let Some(cursor) = cursor.position_in(layout.bounds()) {
            if is_over_link(cursor, &state.paragraph, &self.value, &self.link_graphemes).is_some() {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::Text
            }
        } else {
            mouse::Interaction::default()
        }
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation<()>,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        let bounds = layout.bounds();
        if let Some(selection) = state
            .interaction
            .selection()
            .and_then(|raw| selection(raw, bounds, &state.paragraph, &self.value))
        {
            let content = self
                .value
                .select(selection.start, selection.end)
                .to_string();
            operation.custom(&mut (bounds.y, content), None);
        }
    }
}

fn is_over_link<P: Paragraph>(
    cursor: Point,
    paragraph: &P,
    value: &Value,
    link_graphemes: &[(usize, usize)],
) -> Option<String> {
    if let Some(pos) = selection::find_cursor_position(paragraph, value, cursor) {
        link_graphemes.iter().find_map(|(start, end)| {
            (*start..*end)
                .contains(&pos)
                .then(|| value.select(*start, *end).to_string())
        })
    } else {
        None
    }
}

fn layout<Renderer>(
    state: &mut State<Renderer::Paragraph>,
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    height: Length,
    spans: &[Span<'_, Renderer::Font>],
    line_height: LineHeight,
    size: Option<Pixels>,
    font: Option<Renderer::Font>,
    horizontal_alignment: alignment::Horizontal,
    vertical_alignment: alignment::Vertical,
) -> layout::Node
where
    Renderer: text::Renderer,
{
    layout::sized(limits, width, height, |limits| {
        let bounds = limits.max();

        let size = size.unwrap_or_else(|| renderer.default_size());
        let font = font.unwrap_or_else(|| renderer.default_font());

        let text_with_spans = || Text {
            content: spans,
            bounds,
            size,
            line_height,
            font,
            horizontal_alignment,
            vertical_alignment,
            shaping: Shaping::Advanced,
        };

        if state.spans != spans {
            state.paragraph = Renderer::Paragraph::with_spans(text_with_spans());
            state.spans = spans.iter().cloned().map(Span::to_static).collect();
        } else {
            match state.paragraph.compare(Text {
                content: (),
                bounds,
                size,
                line_height,
                font,
                horizontal_alignment,
                vertical_alignment,
                shaping: Shaping::Advanced,
            }) {
                text::Difference::None => {}
                text::Difference::Bounds => {
                    state.paragraph.resize(bounds);
                }
                text::Difference::Shape => {
                    state.paragraph = Renderer::Paragraph::with_spans(text_with_spans());
                }
            }
        }

        state.paragraph.min_bounds()
    })
}

impl<'a, Message, Theme, Renderer> FromIterator<Span<'a, Renderer::Font>>
    for Rich<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn from_iter<T: IntoIterator<Item = Span<'a, Renderer::Font>>>(spans: T) -> Self {
        Self {
            spans: spans.into_iter().collect(),
            ..Self::new()
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Rich<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
    Message: 'a,
{
    fn from(text: Rich<'a, Message, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}
