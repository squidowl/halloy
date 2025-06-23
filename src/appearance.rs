use data::appearance;
use futures::stream::BoxStream;
use futures::StreamExt;
use iced::advanced::graphics::futures::subscription;
use iced::advanced::subscription::Hasher;
use iced::{Subscription, futures};
use mundy::{ColorScheme, Interest, Preferences};
pub use theme::Theme;

pub mod theme;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Mode {
    Dark,
    Light,
    Unspecified,
}

impl From<ColorScheme> for Mode {
    fn from(mode: ColorScheme) -> Self {
        match mode {
            ColorScheme::Dark => Mode::Dark,
            ColorScheme::Light => Mode::Light,
            ColorScheme::NoPreference => Mode::Unspecified,
        }
    }
}

impl Mode {
    pub fn theme(&self, selected: &data::appearance::Selected) -> data::appearance::Theme {
        match &selected {
            appearance::Selected::Static(theme) => theme.clone(),
            appearance::Selected::Dynamic { light, dark } => match self {
                Self::Dark => dark.clone(),
                Self::Light => light.clone(),
                // We map `Unspecified` to `Light`.
                // This is because Gnome never specifies `Light` and only sends `Unspecified`.
                Self::Unspecified => light.clone()
            },
        }
    }
}

struct Appearance;

impl subscription::Recipe for Appearance {
    type Output = Mode;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream,
    ) -> BoxStream<'static, Mode> {

        Preferences::stream(Interest::ColorScheme)
            .map(|preference| Mode::from(preference.color_scheme))
            .boxed()
    }
}

pub fn subscription() -> Subscription<Mode> {
    subscription::from_recipe(Appearance)
}
