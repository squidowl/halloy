use std::borrow::Cow;
use std::sync::Arc;

use iced::advanced::graphics::core::touch;
use iced::advanced::renderer::Quad;
use iced::advanced::text::{self, Highlight, Paragraph, Span, Text};
use iced::advanced::widget::Operation;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::widget::container;
use iced::widget::text::{LineHeight, Shaping};
use iced::widget::text_input::Value;
use iced::{
    self, Background, Border, Color, Element, Event, Length, Pixels, Point,
    Rectangle, Shadow, Size, Vector, alignment, mouse, widget,
};
use itertools::Itertools;

use super::context_menu;
use super::selectable_text::{Catalog, Interaction, Style, StyleFn, selection};

/// Creates a new [`Rich`] text widget with the provided spans.
pub fn selectable_rich_text<'a, Message, Link, Entry, Theme, Renderer>(
    spans: impl Into<Cow<'a, [Span<'a, Link, Renderer::Font>]>>,
) -> Rich<'a, Message, Link, Entry, Theme, Renderer>
where
    Link: self::Link + 'static,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    Rich::with_spans(spans)
}

/// A bunch of [`Rich`] text.
#[allow(missing_debug_implementations)]
pub struct Rich<
    'a,
    Message,
    Link = (),
    Entry = (),
    Theme = iced::Theme,
    Renderer = iced::Renderer,
> where
    Link: self::Link + 'static,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    spans: Cow<'a, [Span<'a, Link, Renderer::Font>]>,
    size: Option<Pixels>,
    line_height: LineHeight,
    width: Length,
    height: Length,
    font: Option<Renderer::Font>,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
    class: Theme::Class<'a>,
    on_link: Option<Box<dyn Fn(Link) -> Message + 'a>>,

    #[allow(clippy::type_complexity)]
    context_menu: Option<(
        Box<dyn Fn(&Link) -> Vec<Entry> + 'a>,
        Arc<
            dyn Fn(
                    &Link,
                    Entry,
                    Length,
                ) -> Element<'a, Message, Theme, Renderer>
                + 'a,
        >,
    )>,
    cached_entries: Vec<Entry>,
    cached_menu: Option<Element<'a, Message, Theme, Renderer>>,
}

impl<'a, Message, Link, Entry, Theme, Renderer>
    Rich<'a, Message, Link, Entry, Theme, Renderer>
where
    Link: self::Link + 'static,
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
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            class: Theme::default(),
            on_link: None,

            context_menu: None,
            cached_entries: vec![],
            cached_menu: None,
        }
    }

    /// Creates a new [`Rich`] text with the given text spans.
    pub fn with_spans(
        spans: impl Into<Cow<'a, [Span<'a, Link, Renderer::Font>]>>,
    ) -> Self {
        Self {
            spans: spans.into(),
            ..Self::new()
        }
    }

    /// Sets the default size of the [`Rich`] text.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Sets the default [`LineHeight`] of the [`Rich`] text.
    pub fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the default font of the [`Rich`] text.
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the default font of the [`Rich`] text, if `Some`.
    pub fn font_maybe(
        mut self,
        font: Option<impl Into<Renderer::Font>>,
    ) -> Self {
        self.font = font.map(Into::into);
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
    pub fn align_x(mut self, alignment: impl Into<text::Alignment>) -> Self {
        self.align_x = alignment.into();
        self
    }

    /// Sets the [`alignment::Vertical`] of the [`Rich`] text.
    pub fn align_y(
        mut self,
        alignment: impl Into<alignment::Vertical>,
    ) -> Self {
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
            ..Style::default()
        })
    }

    /// Sets the message handler for link clicks on the [`Rich`] text.
    pub fn on_link(mut self, on_link: impl Fn(Link) -> Message + 'a) -> Self {
        self.on_link = Some(Box::new(on_link));
        self
    }

    pub fn context_menu(
        self,
        link_entries: impl Fn(&Link) -> Vec<Entry> + 'a,
        view: impl Fn(&Link, Entry, Length) -> Element<'a, Message, Theme, Renderer>
        + 'a,
    ) -> Self {
        Self {
            context_menu: Some((Box::new(link_entries), Arc::new(view))),
            ..self
        }
    }
}

impl<Message, Link, Entry, Theme, Renderer> Default
    for Rich<'_, Message, Link, Entry, Theme, Renderer>
where
    Link: self::Link + 'static,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn default() -> Self {
        Self::new()
    }
}

pub trait Link: Clone {
    fn underline(&self) -> bool {
        true
    }
}

impl Link for () {}

struct State<Link, P: Paragraph> {
    spans: Vec<Span<'static, Link, P::Font>>,
    span_pressed: Option<usize>,
    paragraph: P,
    hovered: bool,
    link_hovered: bool,
    interaction: Interaction,
    shown_spoiler: Option<(usize, Color, Highlight)>,

    context_menu_link: Option<Link>,
    context_menu: context_menu::State,
}

struct Snapshot {
    hovered: bool,
    link_hovered: bool,
    span_pressed: Option<usize>,
    interaction: Interaction,
    context_menu_status: context_menu::Status,
    shown_spoiler: Option<(usize, Color, Highlight)>,
}

impl<Link, P: Paragraph> From<&State<Link, P>> for Snapshot {
    fn from(value: &State<Link, P>) -> Self {
        Snapshot {
            hovered: value.hovered,
            link_hovered: value.link_hovered,
            span_pressed: value.span_pressed,
            interaction: value.interaction,
            context_menu_status: value.context_menu.status,
            shown_spoiler: value.shown_spoiler,
        }
    }
}

impl Snapshot {
    fn is_changed(&self, other: &Self) -> bool {
        self.hovered != other.hovered
            || self.link_hovered != other.link_hovered
            || self.span_pressed != other.span_pressed
            || self.interaction != other.interaction
            || self.context_menu_status != other.context_menu_status
            || self.shown_spoiler != other.shown_spoiler
    }
}

impl<'a, Message, Link, Entry, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Rich<'a, Message, Link, Entry, Theme, Renderer>
where
    Message: 'a,
    Link: self::Link + 'static,
    Entry: Copy + 'a,
    Theme: 'a + container::Catalog + context_menu::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>:
        From<container::StyleFn<'a, Theme>>,
    Renderer: text::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Link, Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Link, _> {
            spans: Vec::new(),
            span_pressed: None,
            paragraph: Renderer::Paragraph::default(),
            interaction: Interaction::default(),
            shown_spoiler: None,
            context_menu_link: None,
            context_menu: context_menu::State::new(),
            hovered: false,
            link_hovered: false,
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout(
            tree.state
                .downcast_mut::<State<Link, Renderer::Paragraph>>(),
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

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree
            .state
            .downcast_mut::<State<Link, Renderer::Paragraph>>();
        let prev_snapshot = Snapshot::from(&*state);

        let bounds = layout.bounds();

        if viewport.intersection(&bounds).is_none()
            && matches!(state.interaction, Interaction::Idle)
        {
            return;
        }

        state.hovered = false;
        state.link_hovered = false;

        if let Some(position) = cursor.position_in(layout.bounds()) {
            state.hovered = true;

            if self.on_link.is_some()
                && let Some(span) = state
                    .paragraph
                    .hit_span(position)
                    .and_then(|span| self.spans.get(span))
                && span.link.is_some()
            {
                state.link_hovered = true;
            }
        }

        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Left,
            ))
            | iced::Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(position) = cursor.position_in(bounds)
                    && let Some(span) = state.paragraph.hit_span(position)
                {
                    state.span_pressed = Some(span);
                    shell.capture_event();
                }

                if let Some(cursor) = cursor.position() {
                    state.interaction =
                        Interaction::Selecting(selection::Raw {
                            start: cursor,
                            end: cursor,
                        });
                } else {
                    state.interaction = Interaction::Idle;
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(
                mouse::Button::Left,
            ))
            | iced::Event::Touch(touch::Event::FingerLifted { .. })
            | iced::Event::Touch(touch::Event::FingerLost { .. }) => {
                if let Some(on_link_click) = self.on_link.as_ref()
                    && let Some(span_pressed) = state.span_pressed
                {
                    state.span_pressed = None;

                    if let Some(position) = cursor.position_in(bounds) {
                        match state.paragraph.hit_span(position) {
                            Some(span) if span == span_pressed => {
                                if let Some(link) = self
                                    .spans
                                    .get(span)
                                    .and_then(|span| span.link.clone())
                                {
                                    shell.publish(on_link_click(link));
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if let Interaction::Selecting(raw) = state.interaction {
                    state.interaction = Interaction::Selected(raw);
                } else {
                    state.interaction = Interaction::Idle;
                }
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. })
            | iced::Event::Touch(touch::Event::FingerMoved { .. }) => {
                if let Some(cursor) = cursor.position()
                    && let Interaction::Selecting(raw) = &mut state.interaction
                {
                    raw.end = cursor;
                }

                let size = self.size.unwrap_or_else(|| renderer.default_size());
                let font = self.font.unwrap_or_else(|| renderer.default_font());

                let text_with_spans = |spans| Text {
                    content: spans,
                    bounds: bounds.size(),
                    size,
                    line_height: self.line_height,
                    font,
                    align_x: self.align_x,
                    align_y: self.align_y,
                    shaping: Shaping::Advanced,
                    wrapping: text::Wrapping::WordOrGlyph,
                };

                // Check spoiler
                if let Some(cursor) = cursor.position_in(bounds) {
                    if state.shown_spoiler.is_none() {
                        // Find if spoiler is hovered
                        for (index, span) in state.spans.iter().enumerate() {
                            if let Some((fg, highlight)) =
                                span.color.zip(span.highlight)
                            {
                                let is_spoiler = highlight.background
                                    == Background::Color(fg);

                                if is_spoiler
                                    && state
                                        .paragraph
                                        .span_bounds(index)
                                        .into_iter()
                                        .any(|bounds| bounds.contains(cursor))
                                {
                                    state.shown_spoiler =
                                        Some((index, fg, highlight));
                                    break;
                                }
                            }
                        }

                        // Show spoiler
                        if let Some((index, _, _)) = state.shown_spoiler {
                            // Safe we just got this index
                            let span = &mut state.spans[index];
                            span.color = None;
                            span.highlight = None;
                            state.paragraph = Renderer::Paragraph::with_spans(
                                text_with_spans(state.spans.as_ref()),
                            );
                        }
                    }
                }
                // Hide spoiler
                else if let Some((index, fg, highlight)) =
                    state.shown_spoiler.take()
                {
                    if let Some(span) = state.spans.get_mut(index) {
                        span.color = Some(fg);
                        span.highlight = Some(highlight);
                    }
                    state.paragraph = Renderer::Paragraph::with_spans(
                        text_with_spans(state.spans.as_ref()),
                    );
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Right,
            )) => {
                if let Some(position) = cursor.position_in(bounds)
                    && let Some((link_entries, _)) = &self.context_menu
                    && let Some((link, entries)) =
                        state.spans.iter().enumerate().find_map(|(i, span)| {
                            if span.link.is_some()
                                && state
                                    .paragraph
                                    .span_bounds(i)
                                    .into_iter()
                                    .any(|bounds| bounds.contains(position))
                            {
                                let link = span.link.clone().unwrap();
                                let entries = (link_entries)(&link);

                                if !entries.is_empty() {
                                    return Some((link, entries));
                                }
                            }

                            None
                        })
                {
                    state.context_menu.status = context_menu::Status::Open(
                        // Need absolute position. Infallible since we're within position_in
                        cursor.position_over(bounds).unwrap(),
                    );
                    state.context_menu_link = Some(link);
                    self.cached_entries = entries;
                }
            }
            _ => {}
        }

        if prev_snapshot.is_changed(&Snapshot::from(&*state)) {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if viewport.intersection(&bounds).is_none() {
            return;
        }

        let state = tree
            .state
            .downcast_ref::<State<Link, Renderer::Paragraph>>();

        let style = <Theme as Catalog>::style(theme, &self.class);

        let hovered_span = cursor
            .position_in(bounds)
            .and_then(|position| state.paragraph.hit_span(position));

        for (index, span) in state.spans.iter().enumerate() {
            let is_hovered_link =
                span.link.is_some() && Some(index) == hovered_span;

            if span.highlight.is_some()
                || span.underline
                || span.strikethrough
                || is_hovered_link
            {
                let translation = layout.position() - Point::ORIGIN;
                let regions = state.paragraph.span_bounds(index);

                if let Some(highlight) = span.highlight {
                    for bounds in &regions {
                        let bounds = Rectangle::new(
                            bounds.position()
                                - Vector::new(
                                    span.padding.left,
                                    span.padding.top,
                                ),
                            bounds.size()
                                + Size::new(
                                    span.padding.horizontal(),
                                    span.padding.vertical(),
                                ),
                        );

                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: bounds + translation,
                                border: highlight.border,
                                ..Default::default()
                            },
                            highlight.background,
                        );
                    }
                }

                if span.underline || span.strikethrough || is_hovered_link {
                    let size = span
                        .size
                        .or(self.size)
                        .unwrap_or(renderer.default_size());

                    let line_height = span
                        .line_height
                        .unwrap_or(self.line_height)
                        .to_absolute(size);

                    let color = span
                        .color
                        .or(style.color)
                        .unwrap_or(defaults.text_color);

                    let baseline = translation
                        + Vector::new(
                            0.0,
                            size.0 + (line_height.0 - size.0) / 2.0,
                        );

                    if span.underline
                        || (is_hovered_link
                            && span.link.as_ref().unwrap().underline())
                    {
                        for bounds in &regions {
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle::new(
                                        bounds.position() + baseline
                                            - Vector::new(0.0, size.0 * 0.08),
                                        Size::new(bounds.width, 1.0),
                                    ),
                                    ..Default::default()
                                },
                                color,
                            );
                        }
                    }

                    if span.strikethrough {
                        for bounds in &regions {
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle::new(
                                        bounds.position() + baseline
                                            - Vector::new(0.0, size.0 / 2.0),
                                        Size::new(bounds.width, 1.0),
                                    ),
                                    ..Default::default()
                                },
                                color,
                            );
                        }
                    }
                }
            }
        }

        if let Some(selection) = state
            .interaction
            .selection()
            .and_then(|raw| raw.resolve(bounds))
        {
            let line_height = f32::from(self.line_height.to_absolute(
                self.size.unwrap_or_else(|| renderer.default_size()),
            ));

            let baseline_y = bounds.y
                + ((selection.start.y - bounds.y) / line_height).floor()
                    * line_height;

            let height = selection.end.y - baseline_y - 0.5;
            let rows = (height / line_height).ceil() as usize;

            for row in 0..rows {
                let (x, width) = if row == 0 {
                    (
                        selection.start.x,
                        if rows == 1 {
                            f32::min(selection.end.x, bounds.x + bounds.width)
                                - selection.start.x
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
                        bounds: Rectangle::new(
                            Point::new(x, y),
                            Size::new(width, line_height),
                        ),
                        border: Border {
                            radius: 0.0.into(),
                            width: 0.0,
                            color: Color::TRANSPARENT,
                        },
                        shadow: Shadow::default(),
                        snap: true,
                    },
                    style.selection_color,
                );
            }
        }

        widget::text::draw(
            renderer,
            defaults,
            bounds,
            &state.paragraph,
            widget::text::Style { color: style.color },
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree
            .state
            .downcast_ref::<State<Link, Renderer::Paragraph>>();

        if state.hovered {
            if state.link_hovered {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::Text
            }
        } else {
            mouse::Interaction::None
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation<()>,
    ) {
        let state = tree
            .state
            .downcast_mut::<State<Link, Renderer::Paragraph>>();

        let bounds = layout.bounds();
        let value =
            Value::new(&self.spans.iter().map(|s| s.text.as_ref()).join(""));
        if let Some(selection) = state
            .interaction
            .selection()
            .and_then(|raw| selection(raw, bounds, &state.paragraph, &value))
        {
            let mut content =
                value.select(selection.start, selection.end).to_string();
            operation.custom(None, bounds, &mut content);
        }

        // Context menu
        operation.custom(None, bounds, &mut state.context_menu);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        _layout: Layout<'_>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>>
    {
        let state = tree
            .state
            .downcast_mut::<State<Link, Renderer::Paragraph>>();

        // Sync local state w/ context menu change
        if state.context_menu.status.open().is_none() {
            state.context_menu_link = None;
        }

        if let Some((link, (link_entries, view))) = state
            .context_menu_link
            .clone()
            .zip(self.context_menu.as_ref())
        {
            let view = view.clone();

            // Rebuild if not cached (view recreated)
            if self.cached_entries.is_empty() {
                self.cached_entries = link_entries(&link);
            }

            context_menu::overlay(
                &mut state.context_menu,
                &mut self.cached_menu,
                &self.cached_entries,
                &move |entry, length| view(&link, entry, length),
                translation,
            )
        } else {
            None
        }
    }
}

fn layout<Link, Renderer>(
    state: &mut State<Link, Renderer::Paragraph>,
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    height: Length,
    spans: &[Span<'_, Link, Renderer::Font>],
    line_height: LineHeight,
    size: Option<Pixels>,
    font: Option<Renderer::Font>,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
) -> layout::Node
where
    Link: Clone,
    Renderer: text::Renderer,
{
    layout::sized(limits, width, height, |limits| {
        let bounds = limits.max();

        let size = size.unwrap_or_else(|| renderer.default_size());
        let font = font.unwrap_or_else(|| renderer.default_font());

        let text_with_spans = |spans| Text {
            content: spans,
            bounds,
            size,
            line_height,
            font,
            align_x,
            align_y,
            shaping: Shaping::Advanced,
            wrapping: text::Wrapping::WordOrGlyph,
        };

        if state.spans != spans {
            state.spans = spans.iter().cloned().map(Span::to_static).collect();

            // Apply shown spoiler
            if let Some((index, _, _)) = state.shown_spoiler
                && let Some(span) = state.spans.get_mut(index)
            {
                span.color = None;
                span.highlight = None;
            }

            state.paragraph = Renderer::Paragraph::with_spans(text_with_spans(
                state.spans.as_slice(),
            ));
        } else {
            match state.paragraph.compare(Text {
                content: (),
                bounds,
                size,
                line_height,
                font,
                align_x,
                align_y,
                shaping: Shaping::Advanced,
                wrapping: text::Wrapping::WordOrGlyph,
            }) {
                text::Difference::None => {}
                text::Difference::Bounds => {
                    state.paragraph.resize(bounds);
                }
                text::Difference::Shape => {
                    state.spans =
                        spans.iter().cloned().map(Span::to_static).collect();

                    // Apply shown spoiler
                    if let Some((index, _, _)) = state.shown_spoiler
                        && let Some(span) = state.spans.get_mut(index)
                    {
                        span.color = None;
                        span.highlight = None;
                    }

                    state.paragraph = Renderer::Paragraph::with_spans(
                        text_with_spans(state.spans.as_slice()),
                    );
                }
            }
        }

        state.paragraph.min_bounds()
    })
}

impl<'a, Message, Link, Entry, Theme, Renderer>
    FromIterator<Span<'a, Link, Renderer::Font>>
    for Rich<'a, Message, Link, Entry, Theme, Renderer>
where
    Link: self::Link + 'static,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn from_iter<T: IntoIterator<Item = Span<'a, Link, Renderer::Font>>>(
        spans: T,
    ) -> Self {
        Self {
            spans: spans.into_iter().collect(),
            ..Self::new()
        }
    }
}

impl<'a, Message, Link, Entry, Theme, Renderer>
    From<Rich<'a, Message, Link, Entry, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Link: self::Link + 'static,
    Entry: Copy + 'a,
    Theme: 'a + container::Catalog + context_menu::Catalog + Catalog,
    <Theme as container::Catalog>::Class<'a>:
        From<container::StyleFn<'a, Theme>>,
    Renderer: text::Renderer + 'a,
{
    fn from(
        text: Rich<'a, Message, Link, Entry, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}
