use std::borrow::Cow;
use std::sync::OnceLock;

use data::Config;
use iced::font;

pub static MONO: Font = Font::new(false);
pub static MONO_BOLD: Font = Font::new(true);
pub const ICON: iced::Font = iced::Font::with_name("bootstrap-icons");

#[derive(Debug, Clone)]
pub struct Font {
    bold: bool,
    inner: OnceLock<iced::Font>,
}

impl Font {
    const fn new(bold: bool) -> Self {
        Self {
            bold,
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

        let _ = self.inner.set(iced::Font {
            weight,
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
    MONO_BOLD.set(family);
}

pub fn load() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../fonts/iosevka-term-regular.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/iosevka-term-bold.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/iosevka-term-italic.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/icons.ttf")),
    ]
}
