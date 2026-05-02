use std::borrow::Cow;
use std::sync::{LazyLock, OnceLock};

use data::appearance::theme::FontStyle;
use data::{Config, config};
use iced::font;
use iced::widget::text::LineHeight;

pub static MONO: Font = Font::new(false, false);
pub static MONO_BOLD: Font = Font::new(true, false);
pub static MONO_ITALICS: Font = Font::new(false, true);
pub static MONO_BOLD_ITALICS: Font = Font::new(true, true);
pub static ICON: LazyLock<iced::Font> =
    LazyLock::new(|| iced::Font::with_family("halloy-icons"));
pub const MESSAGE_MARKER_FONT_SCALE: f32 = 1.33;

static LINE_HEIGHT: OnceLock<LineHeight> = OnceLock::new();

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

    fn set(
        &self,
        font: iced::Font,
        stretch: font::Stretch,
        weight: font::Weight,
        bold_weight: font::Weight,
    ) {
        let weight = if self.bold { bold_weight } else { weight };
        let style = if self.italics {
            font::Style::Italic
        } else {
            font::Style::Normal
        };

        let _ = self.inner.set(iced::Font {
            stretch,
            weight,
            style,
            ..font
        });
    }
}

impl From<Font> for iced::Font {
    fn from(value: Font) -> Self {
        value.inner.get().copied().expect("font is set on startup")
    }
}

pub fn line_height() -> LineHeight {
    LINE_HEIGHT.get().copied().unwrap_or_default()
}

fn default_font() -> iced::Font {
    #[cfg(feature = "iosevka-font")]
    {
        iced::Font::with_family("Iosevka Term")
    }

    #[cfg(not(feature = "iosevka-font"))]
    {
        iced::Font::MONOSPACE
    }
}

pub fn set(config: Option<&Config>) {
    let font = config
        .and_then(|config| config.font.family.clone())
        .map_or_else(default_font, |family| {
            iced::Font::with_family(family.as_str())
        });
    let stretch =
        config.map_or(font::Stretch::Normal, |config| config.font.stretch);
    let weight =
        config.map_or(font::Weight::Normal, |config| config.font.weight);
    let bold_weight = config
        .and_then(|config| config.font.bold_weight)
        .unwrap_or(match weight {
            font::Weight::Thin => font::Weight::Normal,
            font::Weight::ExtraLight => font::Weight::Medium,
            font::Weight::Light => font::Weight::Semibold,
            font::Weight::Normal => font::Weight::Bold,
            font::Weight::Medium => font::Weight::ExtraBold,
            font::Weight::Semibold
            | font::Weight::Bold
            | font::Weight::ExtraBold
            | font::Weight::Black => font::Weight::Black,
        });

    MONO.set(font, stretch, weight, bold_weight);
    MONO_BOLD.set(font, stretch, weight, bold_weight);
    MONO_ITALICS.set(font, stretch, weight, bold_weight);
    MONO_BOLD_ITALICS.set(font, stretch, weight, bold_weight);

    let lh = config
        .and_then(|c| c.font.line_height)
        .map(LineHeight::Relative)
        .unwrap_or_default();
    let _ = LINE_HEIGHT.set(lh);
}

pub fn load() -> Vec<Cow<'static, [u8]>> {
    vec![
        #[cfg(feature = "iosevka-font")]
        include_bytes!("../fonts/iosevka-term-regular.ttf")
            .as_slice()
            .into(),
        #[cfg(feature = "iosevka-font")]
        include_bytes!("../fonts/iosevka-term-bold.ttf")
            .as_slice()
            .into(),
        #[cfg(feature = "iosevka-font")]
        include_bytes!("../fonts/iosevka-term-italic.ttf")
            .as_slice()
            .into(),
        #[cfg(feature = "iosevka-font")]
        include_bytes!("../fonts/iosevka-term-light.ttf")
            .as_slice()
            .into(),
        #[cfg(feature = "iosevka-font")]
        include_bytes!("../fonts/iosevka-term-semibold.ttf")
            .as_slice()
            .into(),
        #[cfg(feature = "iosevka-font")]
        include_bytes!("../fonts/iosevka-term-lightitalic.ttf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/halloy-icons.ttf")
            .as_slice()
            .into(),
    ]
}

pub fn width_from_str(text: &str, config: &config::Font) -> f32 {
    use iced::advanced::graphics::text::Paragraph;
    use iced::advanced::text::{self, Paragraph as _, Text};
    use iced::{Size, alignment};

    use crate::theme;

    Paragraph::with_text(Text {
        content: text,
        bounds: Size::INFINITE,
        size: config.size.map_or(theme::TEXT_SIZE, f32::from).into(),
        line_height: line_height(),
        font: MONO.clone().into(),
        align_x: text::Alignment::Right,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::default(),
        ellipsis: text::Ellipsis::default(),
        hint_factor: None,
    })
    .min_bounds()
    .width
}

pub fn width_of_message_marker(config: &config::Font) -> f32 {
    use iced::advanced::graphics::text::Paragraph;
    use iced::advanced::text::{self, Paragraph as _, Text};
    use iced::{Size, alignment};

    use crate::theme;

    let font_size = config.size.map_or(theme::TEXT_SIZE, f32::from)
        * MESSAGE_MARKER_FONT_SCALE;

    Paragraph::with_text(Text {
        content: "\u{2022}",
        bounds: Size::INFINITE,
        size: font_size.into(),
        line_height: line_height(),
        font: *ICON,
        align_x: text::Alignment::Right,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::default(),
        ellipsis: text::Ellipsis::default(),
        hint_factor: None,
    })
    .min_bounds()
    .width
}

pub fn get(font_style: FontStyle) -> Font {
    match font_style {
        FontStyle::Normal => MONO.clone(),
        FontStyle::Bold => MONO_BOLD.clone(),
        FontStyle::Italic => MONO_ITALICS.clone(),
        FontStyle::ItalicBold => MONO_BOLD_ITALICS.clone(),
    }
}
