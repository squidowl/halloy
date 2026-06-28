use data::command::search::DisplayMatch;
use iced::widget::span;
use iced::widget::{Scrollable, column, container, row};
use iced::{Background, Length, padding};

use crate::widget::{Element, selectable_rich_text, selectable_text};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub title: String,
    pub summary: String,
    pub lines: Vec<DisplayMatch>,
}

impl SearchResults {
    pub fn new(
        title: String,
        summary: String,
        lines: Vec<DisplayMatch>,
    ) -> Self {
        Self {
            title,
            summary,
            lines,
        }
    }
}

pub fn view<'a>(
    state: &'a SearchResults,
    theme: &'a Theme,
) -> Element<'a, super::Message> {
    // Search results are an ephemeral, local-only surface. They intentionally
    // render owned strings instead of borrowing from history so closing or
    // reloading buffers cannot invalidate the result view.
    let mut rows = column![
        selectable_text(&state.summary)
            .font_maybe(theme::font_style::primary(theme).map(font::get))
            .style(theme::selectable_text::default)
    ]
    .spacing(6);

    if state.lines.is_empty() {
        rows = rows.push(
            selectable_text("No matching loaded messages")
                .font_maybe(theme::font_style::primary(theme).map(font::get))
                .style(theme::selectable_text::default),
        );
    } else {
        for line in &state.lines {
            rows = rows.push(
                container(row![highlighted_line(line, theme)])
                    .width(Length::Fill)
                    .padding(padding::top(2)),
            );
        }
    }

    container(Scrollable::new(
        container(rows).width(Length::Fill).padding(8),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn highlighted_line<'a>(
    line: &'a DisplayMatch,
    theme: &'a Theme,
) -> Element<'a, super::Message> {
    let mut spans = vec![span(line.prefix.as_str())];
    let mut cursor = 0;

    for range in &line.text_highlights {
        if cursor < range.start {
            spans.push(span(&line.text[cursor..range.start]));
        }

        spans
            .push(span(&line.text[range.clone()]).background(
                Background::Color(theme.styles().buffer.highlight),
            ));

        cursor = range.end;
    }

    if cursor < line.text.len() {
        spans.push(span(&line.text[cursor..]));
    }

    selectable_rich_text::<_, (), (), _, _>(spans)
        .width(Length::Fill)
        .font_maybe(theme::font_style::primary(theme).map(font::get))
        .style(theme::selectable_text::default)
        .into()
}
