use std::borrow::Cow;
use std::sync::OnceLock;

use data::appearance::theme::FontStyle;
use data::{Config, config};
use iced::font;
use iced::widget::text::LineHeight;

pub static MONO: Font = Font::new(false, false);
pub static MONO_BOLD: Font = Font::new(true, false);
pub static MONO_ITALICS: Font = Font::new(false, true);
pub static MONO_BOLD_ITALICS: Font = Font::new(true, true);
pub const ICON: iced::Font = iced::Font::with_name("halloy-icons");
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
        name: String,
        weight: font::Weight,
        bold_weight: font::Weight,
    ) {
        let name = Box::leak(name.into_boxed_str());
        let weight = if self.bold { bold_weight } else { weight };
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

pub fn line_height() -> LineHeight {
    LINE_HEIGHT.get().copied().unwrap_or_default()
}

pub fn set(config: Option<&Config>) {
    let family = config
        .and_then(|config| config.font.family.clone())
        .unwrap_or_else(|| String::from("Victor Mono"));
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

    MONO.set(family.clone(), weight, bold_weight);
    MONO_BOLD.set(family.clone(), weight, bold_weight);
    MONO_ITALICS.set(family.clone(), weight, bold_weight);
    MONO_BOLD_ITALICS.set(family, weight, bold_weight);

    let lh = config
        .and_then(|c| c.font.line_height)
        .map(LineHeight::Relative)
        .unwrap_or_default();
    let _ = LINE_HEIGHT.set(lh);
}

pub fn load() -> Vec<Cow<'static, [u8]>> {
    vec![
        include_bytes!("../fonts/victor-mono/VictorMono-BoldItalic.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-Bold.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-ExtraLightItalic.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-ExtraLight.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-Italic.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-LightItalic.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-Light.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-MediumItalic.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-Medium.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-Regular.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-SemiBoldItalic.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-SemiBold.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-ThinItalic.otf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/victor-mono/VictorMono-Thin.otf")
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
        content: &"W".repeat(len),
        bounds: Size::INFINITE,
        size: config.size.map_or(theme::TEXT_SIZE, f32::from).into(),
        line_height: line_height(),
        font: MONO.clone().into(),
        align_x: text::Alignment::Right,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::default(),
        hint_factor: None,
    })
    .min_bounds()
    .expand(Size::new(1.0, 0.0))
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
        content: "\u{E81A}",
        bounds: Size::INFINITE,
        size: font_size.into(),
        line_height: line_height(),
        font: ICON,
        align_x: text::Alignment::Right,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::default(),
        hint_factor: None,
    })
    .min_bounds()
    .expand(Size::new(1.0, 0.0))
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
