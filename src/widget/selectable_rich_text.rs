use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use iced::advanced::graphics::core::touch;
use iced::advanced::renderer::Quad;
use iced::advanced::text::{self, Highlight, Paragraph, Span, Text};
use iced::advanced::widget::Operation;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Layout, Shell, Widget, layout, renderer};
use iced::widget::container;
use iced::widget::text::{IntoFragment as _, LineHeight, Shaping};
use iced::{
    self, Background, Border, Color, Element, Event, Font, Length, Pixels,
    Point, Rectangle, Shadow, Size, Vector, alignment, mouse, widget,
};
use itertools::Itertools;

use super::context_menu;
use super::selectable_text::{Catalog, Interaction, Style, StyleFn, selection};

/// Creates a new [`Rich`] text widget with the provided spans.
pub fn selectable_rich_text<'a, Message, Link, Entry, Theme, Renderer>(
    spans: impl Into<Cow<'a, [Span<'a, Link>]>>,
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
    spans: Cow<'a, [Span<'a, Link>]>,
    size: Option<Pixels>,
    line_height: Option<LineHeight>,
    width: Length,
    height: Length,
    font: Option<Font>,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
    class: Theme::Class<'a>,
    on_link: Option<Box<dyn Fn(Link) -> Message + 'a>>,

    context_menus: Vec<Element<'a, Message, Theme, Renderer>>,
    marker: PhantomData<Entry>,
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
            line_height: None,
            width: Length::Shrink,
            height: Length::Shrink,
            font: None,
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            class: Theme::default(),
            on_link: None,

            context_menus: vec![],
            marker: PhantomData,
        }
    }

    /// Creates a new [`Rich`] text with the given text spans.
    pub fn with_spans(spans: impl Into<Cow<'a, [Span<'a, Link>]>>) -> Self {
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
        self.line_height = Some(line_height.into());
        self
    }

    /// Sets the default font of the [`Rich`] text.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the default font of the [`Rich`] text, if `Some`.
    pub fn font_maybe(mut self, font: Option<impl Into<Font>>) -> Self {
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
        mut self,
        link_entries: impl Fn(&Link) -> Vec<Entry> + 'a,
        view: impl Fn(&Link, Entry, Length) -> Element<'a, Message, Theme, Renderer>
        + 'a,
    ) -> Self
    where
        Entry: Copy + 'a,
        Message: 'a,
        Theme: 'a + container::Catalog + context_menu::Catalog,
        <Theme as container::Catalog>::Class<'a>:
            From<container::StyleFn<'a, Theme>>,
        Renderer: 'a,
    {
        let link_entries = Arc::new(link_entries);
        let view = Arc::new(view);

        self.context_menus = self
            .spans
            .iter()
            .map(|span| {
                if let Some(link) = span.link.as_ref() {
                    let link_entries = Arc::clone(&link_entries);
                    let view = Arc::clone(&view);

                    let entries_link = link.clone();
                    let link = link.clone();

                    return context_menu::lazy_context_menu(
                        context_menu::MouseButton::Right,
                        context_menu::Anchor::Cursor,
                        context_menu::ToggleBehavior::KeepOpen,
                        None,
                        widget::Space::new(),
                        move || link_entries(&entries_link),
                        move |entry, length| view(&link, entry, length),
                    )
                    .into();
                }

                widget::Space::new().into()
            })
            .collect();

        self
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
    spans: Vec<Span<'static, Link>>,
    span_pressed: Option<usize>,
    paragraph: P,
    hovered: bool,
    link_hovered: bool,
    spoiler_hovered: bool,
    interaction: Interaction,
    shown_spoilers: HashMap<usize, (Color, Highlight)>,
}

struct Snapshot {
    hovered: bool,
    link_hovered: bool,
    spoiler_hovered: bool,
    span_pressed: Option<usize>,
    interaction: Interaction,
    shown_spoilers: HashMap<usize, (Color, Highlight)>,
}

impl<Link, P: Paragraph> From<&State<Link, P>> for Snapshot {
    fn from(value: &State<Link, P>) -> Self {
        Snapshot {
            hovered: value.hovered,
            link_hovered: value.link_hovered,
            spoiler_hovered: value.spoiler_hovered,
            span_pressed: value.span_pressed,
            interaction: value.interaction,
            shown_spoilers: value.shown_spoilers.clone(),
        }
    }
}

impl Snapshot {
    fn is_changed(&self, other: &Self) -> bool {
        self.hovered != other.hovered
            || self.link_hovered != other.link_hovered
            || self.spoiler_hovered != other.spoiler_hovered
            || self.span_pressed != other.span_pressed
            || self.interaction != other.interaction
            || self.shown_spoilers != other.shown_spoilers
    }
}

impl<'a, Message, Link, Entry, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Rich<'a, Message, Link, Entry, Theme, Renderer>
where
    Message: 'a,
    Link: self::Link + 'static,
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
            shown_spoilers: HashMap::new(),
            hovered: false,
            link_hovered: false,
            spoiler_hovered: false,
        })
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.context_menus);
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
    ) {
        tree.size = layout(
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
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        renderer: &Renderer,
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
        state.spoiler_hovered = false;

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

            // Check if cursor is over a spoiler span (hidden or revealed)
            for (index, span) in state.spans.iter().enumerate() {
                let is_hidden_spoiler = span
                    .color
                    .zip(span.highlight)
                    .is_some_and(|(fg, highlight)| {
                        highlight.background == Background::Color(fg)
                    });

                let is_shown_spoiler =
                    state.shown_spoilers.contains_key(&index);

                if (is_hidden_spoiler || is_shown_spoiler)
                    && state
                        .paragraph
                        .span_bounds(index)
                        .into_iter()
                        .any(|bounds| bounds.contains(position))
                {
                    state.spoiler_hovered = true;
                    break;
                }
            }
        }

        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed {
                button: mouse::Button::Left,
                ..
            })
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

                // Toggle spoiler on click
                if let Some(position) = cursor.position_in(bounds) {
                    let font = self.font.unwrap_or_else(|| renderer.font());
                    let size =
                        self.size.unwrap_or_else(|| renderer.text_size());
                    let line_height = self
                        .line_height
                        .unwrap_or_else(|| renderer.line_height());

                    let text_with_spans = |spans| Text {
                        content: spans,
                        bounds: bounds.size(),
                        size,
                        line_height,
                        font,
                        align_x: self.align_x,
                        align_y: self.align_y,
                        shaping: Shaping::Advanced,
                        wrapping: text::Wrapping::WordOrGlyph,
                        ellipsis: text::Ellipsis::default(),
                        hint_factor: renderer.hint_factor(),
                    };

                    // If a spoiler is currently shown and we clicked on it, hide it
                    if let Some((index, fg, highlight)) = state
                        .shown_spoilers
                        .iter()
                        .find_map(|(index, (fg, highlight))| {
                            state
                                .paragraph
                                .span_bounds(*index)
                                .into_iter()
                                .any(|bounds| bounds.contains(position))
                                .then_some((*index, *fg, *highlight))
                        })
                    {
                        state.shown_spoilers.remove(&index);
                        if let Some(span) = state.spans.get_mut(index) {
                            span.color = Some(fg);
                            span.highlight = Some(highlight);
                        }
                        state.paragraph = Renderer::Paragraph::with_spans(
                            text_with_spans(state.spans.as_ref()),
                        );
                    } else {
                        // Check if we clicked on a hidden spoiler to reveal it
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
                                        .any(|bounds| bounds.contains(position))
                                {
                                    state
                                        .shown_spoilers
                                        .insert(index, (fg, highlight));
                                    let span = &mut state.spans[index];
                                    span.color = None;
                                    span.highlight = None;
                                    state.paragraph =
                                        Renderer::Paragraph::with_spans(
                                            text_with_spans(
                                                state.spans.as_ref(),
                                            ),
                                        );
                                    break;
                                }
                            }
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
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed {
                button: mouse::Button::Right,
                ..
            }) => {
                if let Some(index) = cursor
                    .position_in(bounds)
                    .and_then(|position| state.paragraph.hit_span(position))
                {
                    self.context_menus[index].as_widget_mut().update(
                        &mut tree.children[index],
                        event,
                        layout,
                        cursor,
                        renderer,
                        shell,
                        viewport,
                    );
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
        layout: Layout,
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
                                    span.padding.left + span.padding.right,
                                    span.padding.top + span.padding.bottom,
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
                        .unwrap_or_else(|| renderer.text_size());
                    let line_height = span
                        .line_height
                        .or(self.line_height)
                        .unwrap_or_else(|| renderer.line_height())
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
            let size = self.size.unwrap_or_else(|| renderer.text_size());
            let line_height_rel =
                self.line_height.unwrap_or_else(|| renderer.line_height());

            let line_height = f32::from(line_height_rel.to_absolute(size));

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
        _layout: Layout,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree
            .state
            .downcast_ref::<State<Link, Renderer::Paragraph>>();

        if state.hovered {
            if state.link_hovered || state.spoiler_hovered {
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
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn Operation<()>,
    ) {
        let state = tree
            .state
            .downcast_mut::<State<Link, Renderer::Paragraph>>();

        let bounds = layout.bounds();
        let value = &self
            .spans
            .iter()
            .map(|s| s.text.as_ref())
            .join("")
            .into_fragment();
        if let Some(selection) = state
            .interaction
            .selection()
            .and_then(|raw| selection(raw, bounds, &state.paragraph))
        {
            let mut content = value[selection.start..selection.end].to_string();
            operation.custom(None, bounds, &mut content);
        }

        for (menu, child) in
            self.context_menus.iter_mut().zip(&mut tree.children)
        {
            menu.as_widget_mut()
                .operate(child, layout, viewport, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
        window: Size,
    ) -> Vec<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>>
    {
        self.context_menus
            .iter_mut()
            .zip(&mut tree.children)
            .flat_map(|(menu, child)| {
                menu.as_widget_mut().overlay(
                    child,
                    layout,
                    renderer,
                    viewport,
                    translation,
                    window,
                )
            })
            .collect()
    }
}

fn layout<Link, Renderer>(
    state: &mut State<Link, Renderer::Paragraph>,
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    height: Length,
    spans: &[Span<'_, Link>],
    line_height: Option<LineHeight>,
    size: Option<Pixels>,
    font: Option<Font>,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
) -> Size
where
    Link: Clone,
    Renderer: text::Renderer,
{
    layout::sized(limits, width, height, |limits| {
        let bounds = limits.bounds();

        let font = font.unwrap_or_else(|| renderer.font());
        let size = size.unwrap_or_else(|| renderer.text_size());
        let line_height = line_height.unwrap_or_else(|| renderer.line_height());

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
            ellipsis: text::Ellipsis::default(),
            hint_factor: renderer.hint_factor(),
        };

        // WTF? `Span`'s `PartialEq` ignores decorations,
        // a decoration-only change would otherwise go
        // undetected and only render once another change
        // forces reshape
        let decorations_changed = state.spans.len() != spans.len()
            || state.spans.iter().zip(spans.iter()).any(|(a, b)| {
                a.highlight != b.highlight
                    || a.underline != b.underline
                    || a.strikethrough != b.strikethrough
            });

        if state.spans != spans || decorations_changed {
            state.spans = spans.iter().cloned().map(Span::to_static).collect();

            // Apply shown spoiler
            for index in state.shown_spoilers.keys() {
                if let Some(span) = state.spans.get_mut(*index) {
                    span.color = None;
                    span.highlight = None;
                }
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
                ellipsis: text::Ellipsis::default(),
                hint_factor: renderer.hint_factor(),
            }) {
                text::Difference::None => {}
                text::Difference::Bounds => {
                    state.paragraph.resize(bounds);
                }
                text::Difference::Shape => {
                    state.spans =
                        spans.iter().cloned().map(Span::to_static).collect();

                    // Apply shown spoiler
                    for index in state.shown_spoilers.keys() {
                        if let Some(span) = state.spans.get_mut(*index) {
                            span.color = None;
                            span.highlight = None;
                        }
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

impl<'a, Message, Link, Entry, Theme, Renderer> FromIterator<Span<'a, Link>>
    for Rich<'a, Message, Link, Entry, Theme, Renderer>
where
    Link: self::Link + 'static,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn from_iter<T: IntoIterator<Item = Span<'a, Link>>>(spans: T) -> Self {
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
    Entry: 'a,
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
