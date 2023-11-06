use iced::{
    color,
    widget::{container, row},
};

use crate::theme;
use crate::widget::{selectable_text, Element};

#[derive(Debug)]
enum Style {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Monospace,
    Color(IrcColor, Option<IrcColor>),
    Reset,
}

#[derive(Debug)]
enum IrcColor {
    White,
    Black,
    Blue,
    Green,
    Red,
    Brown,
    Magenta,
    Orange,
    Yellow,
    LightGreen,
    Cyan,
    LightCyan,
    LightBlue,
    Pink,
    Grey,
    LightGrey,
    Extended(u8),
}

impl IrcColor {
    fn from_code(c: u8) -> Self {
        match c {
            0 => Self::White,
            1 => Self::Black,
            2 => Self::Blue,
            3 => Self::Green,
            4 => Self::Red,
            5 => Self::Brown,
            6 => Self::Magenta,
            7 => Self::Orange,
            8 => Self::Yellow,
            9 => Self::LightGreen,
            10 => Self::Cyan,
            11 => Self::LightCyan,
            12 => Self::LightBlue,
            13 => Self::Pink,
            14 => Self::Grey,
            15 => Self::LightGrey,
            c => Self::Extended(c),
        }
    }
}

impl From<u8> for IrcColor {
    fn from(value: u8) -> Self {
        Self::from_code(value)
    }
}

impl From<IrcColor> for iced::Color {
    fn from(value: IrcColor) -> Self {
        // TODO: don't hardcode, move into theme
        match value {
            IrcColor::White => color!(255, 255, 255),
            IrcColor::Black => color!(0, 0, 0),
            IrcColor::Blue => color!(0, 0, 127),
            IrcColor::Green => color!(0, 147, 0),
            IrcColor::Red => color!(255, 0, 0),
            IrcColor::Brown => color!(127, 0, 0),
            IrcColor::Magenta => color!(156, 0, 156),
            IrcColor::Orange => color!(252, 127, 0),
            IrcColor::Yellow => color!(255, 255, 0),
            IrcColor::LightGreen => color!(0, 252, 0),
            IrcColor::Cyan => color!(0, 147, 147),
            IrcColor::LightCyan => color!(0, 255, 255),
            IrcColor::LightBlue => color!(0, 0, 252),
            IrcColor::Pink => color!(255, 0, 255),
            IrcColor::Grey => color!(127, 127, 127),
            IrcColor::LightGrey => color!(210, 210, 210),
            IrcColor::Extended(_) => todo!(),
        }
    }
}

fn style_part<'a, Message: 'a>(
    style: Style,
    text: String,
    default: &theme::Text,
) -> Element<'a, Message> {
    let text = selectable_text(text);
    match style {
        Style::Color(fg, bg) => {
            let text = text.style(theme::Text::Custom(Some(fg.into())));
            if let Some(bg) = bg {
                container(text)
                    .style(theme::Container::Custom {
                        background: Some(bg.into()),
                    })
                    .into()
            } else {
                text.into()
            }
        }
        _ => text.style(default.clone()).into(),
    }
}
pub fn format_message<'a, Message: 'a>(
    text: &'a str,
    default: theme::Text,
) -> Element<'a, Message> {
    let mut parts = row![];
    let mut current = String::new();
    let mut style = Style::Reset;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        let new_style = match c {
            '\x0f' => Style::Reset,
            '\x02' => Style::Bold,
            '\x1d' => Style::Italic,
            '\x1f' => Style::Underline,
            '\x1e' => Style::Strikethrough,
            '\x11' => Style::Monospace,
            '\x03' => {
                let mut fg = String::new();
                while let Some(&c @ '0'..='9') = chars.peek() {
                    chars.next();
                    fg.push(c);
                    if fg.len() >= 2 {
                        break;
                    }
                }
                let mut bg = String::new();
                if chars.next_if(|&c| c == ',').is_some() {
                    while let Some(&c @ '0'..='9') = chars.peek() {
                        chars.next();
                        bg.push(c);
                        if bg.len() >= 2 {
                            break;
                        }
                    }
                }
                let Some(fg) = fg.parse().ok().map(IrcColor::from_code) else {
                    continue;
                };
                Style::Color(fg, bg.parse().ok().map(IrcColor::from_code))
            }
            c => {
                current.push(c);
                continue;
            }
        };

        parts = parts.push(style_part(
            std::mem::replace(&mut style, new_style),
            std::mem::take(&mut current),
            &default,
        ));
    }

    parts = parts.push(style_part(style, current, &default));

    parts.into()
}
