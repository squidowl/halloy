use data::{appearance, Theme};
use futures::{stream::BoxStream, StreamExt};
use iced::advanced::subscription::Hasher;
use iced::futures;
use iced::{advanced::graphics::futures::subscription, Subscription};

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Dark,
    Light,
}

impl TryFrom<dark_light::Mode> for Mode {
    type Error = ();

    fn try_from(mode: dark_light::Mode) -> Result<Self, Self::Error> {
        match mode {
            dark_light::Mode::Dark => Ok(Mode::Dark),
            dark_light::Mode::Light => Ok(Mode::Light),
            // We ignore `Default` as it is defined as `Unspecified``.
            dark_light::Mode::Default => Err(()),
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

    fn stream(self: Box<Self>, _input: subscription::EventStream) -> BoxStream<'static, Mode> {
        let stream_future = async {
            match dark_light::subscribe().await {
                Ok(stream) => stream
                    .filter_map(|m| async move {
                        match Mode::try_from(m) {
                            Ok(mode) => Some(mode),
                            Err(_) => None,
                        }
                    })
                    .boxed(),
                Err(err) => {
                    println!("error: {:?}", err);
                    futures::stream::empty().boxed()
                }
            }
        };

        futures::stream::once(stream_future).flatten().boxed()
    }
}

pub fn subscription() -> Subscription<Mode> {
    subscription::from_recipe(Appearance)
}

pub fn detect() -> Option<Mode> {
    Mode::try_from(dark_light::detect()).ok()
}

pub fn theme(selected: &data::appearance::Selected) -> Theme {
    match &selected {
        appearance::Selected::Static(theme) => theme.clone(),
        appearance::Selected::Dynamic { light, dark } => match detect() {
            Some(mode) => match mode {
                Mode::Dark => dark.clone(),
                Mode::Light => light.clone(),
            },
            None => {
                log::warn!(
                    "[theme] couldn't determine the OS appearance, using the default theme."
                );
                Theme::default()
            }
        },
    }
}
