use iced::advanced::text::{self, Paragraph};
use iced::{Pixels, Point, Rectangle, Vector};

use super::Value;

#[derive(Debug, Clone, Copy)]
pub struct Raw {
    pub start: Point,
    pub end: Point,
}

impl Raw {
    pub fn resolve(&self, bounds: Rectangle) -> Option<Resolved> {
        if f32::max(f32::min(self.start.y, self.end.y), bounds.y)
            <= f32::min(f32::max(self.start.y, self.end.y), bounds.y + bounds.height)
        {
            let (mut start, mut end) = if self.start.y < self.end.y
                || self.start.y == self.end.y && self.start.x < self.end.x
            {
                (self.start, self.end)
            } else {
                (self.end, self.start)
            };

            let clip = |p: Point| Point {
                x: p.x.max(bounds.x).min(bounds.x + bounds.width),
                y: p.y.max(bounds.y).min(bounds.y + bounds.height),
            };

            if start.y < bounds.y {
                start = bounds.position();
            } else {
                start = clip(start);
            }

            if end.y > bounds.y + bounds.height {
                end = bounds.position() + Vector::from(bounds.size());
            } else {
                end = clip(end);
            }

            ((start.x - end.x).abs() > 1.0).then_some(Resolved { start, end })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Resolved {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

pub fn selection<Renderer>(
    raw: Raw,
    renderer: &Renderer,
    font: Option<Renderer::Font>,
    size: Option<Pixels>,
    line_height: text::LineHeight,
    bounds: Rectangle,
    value: &Value,
    paragraph: &Renderer::Paragraph,
) -> Option<Selection>
where
    Renderer: text::Renderer,
{
    let resolved = raw.resolve(bounds)?;

    let start_pos = relative(resolved.start, bounds);
    let end_pos = relative(resolved.end, bounds);

    let start = find_cursor_position(
        renderer,
        font,
        size,
        line_height,
        bounds,
        value,
        start_pos,
        paragraph,
    )?;
    let end = find_cursor_position(
        renderer,
        font,
        size,
        line_height,
        bounds,
        value,
        end_pos,
        paragraph,
    )?;

    (start != end).then(|| Selection {
        start: start.min(end),
        end: start.max(end),
    })
}

fn find_cursor_position<Renderer>(
    renderer: &Renderer,
    font: Option<Renderer::Font>,
    size: Option<Pixels>,
    line_height: text::LineHeight,
    bounds: Rectangle,
    value: &Value,
    cursor_position: Point,
    paragraph: &Renderer::Paragraph,
) -> Option<usize>
where
    Renderer: text::Renderer,
{
    let value = value.to_string();

    let char_offset = paragraph.hit_test(cursor_position).map(text::Hit::cursor)?;

    Some(unicode_segmentation::UnicodeSegmentation::graphemes(&value[..char_offset], true).count())
}

fn relative(point: Point, bounds: Rectangle) -> Point {
    point - Vector::new(bounds.x, bounds.y)
}
