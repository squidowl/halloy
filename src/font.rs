use std::borrow::Cow;
use std::sync::OnceLock;

use data::{Config, config};
use iced::font;

pub static MONO: Font = Font::new(false, false);
pub static MONO_BOLD: Font = Font::new(true, false);
pub static MONO_ITALICS: Font = Font::new(false, true);
pub static MONO_BOLD_ITALICS: Font = Font::new(true, true);
pub const ICON: iced::Font = iced::Font::with_name("halloy-icons");

#[derive(Debug, Clone)]
pub struct Font {
    bold: bool,
    italics: bool,
    inner: OnceLock<iced::Font>,
}

impl Font {
    const fn new(bold: bool, italics: bool) -> Self {
        Self {
            bold,
            italics,
            inner: OnceLock::new(),
        }
    }

    fn set(&self, name: String) {
        let name = Box::leak(name.into_boxed_str());
        let weight = if self.bold {
            font::Weight::Bold
        } else {
            font::Weight::Normal
        };
        let style = if self.italics {
            font::Style::Italic
        } else {
            font::Style::Normal
        };

        let _ = self.inner.set(iced::Font {
            weight,
            style,
            ..iced::Font::with_name(name)
        });
    }
}

impl From<Font> for iced::Font {
    fn from(value: Font) -> Self {
        value.inner.get().copied().expect("font is set on startup")
    }
}

pub fn set(config: Option<&Config>) {
    let family = config
        .and_then(|config| config.font.family.clone())
        .unwrap_or_else(|| String::from("Iosevka Term"));

    MONO.set(family.clone());
    MONO_BOLD.set(family.clone());
    MONO_ITALICS.set(family.clone());
    MONO_BOLD_ITALICS.set(family);
}

pub fn load() -> Vec<Cow<'static, [u8]>> {
    vec![
        include_bytes!("../fonts/iosevka-term-regular.ttf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/iosevka-term-bold.ttf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/iosevka-term-italic.ttf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/halloy-icons.ttf")
            .as_slice()
            .into(),
    ]
}

pub fn width_from_chars(len: usize, config: &config::Font) -> f32 {
    use iced::advanced::graphics::text::Paragraph;
    use iced::advanced::text::{self, Paragraph as _, Text};
    use iced::{Size, alignment};

    use crate::theme;

    Paragraph::with_text(Text {
        content: &" ".repeat(len),
        bounds: Size::INFINITY,
        size: config.size.map_or(theme::TEXT_SIZE, f32::from).into(),
        line_height: text::LineHeight::default(),
        font: MONO.clone().into(),
        align_x: text::Alignment::Right,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::default(),
    })
    .min_bounds()
    .expand(Size::new(1.0, 0.0))
    .width
}
