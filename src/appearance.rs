use std::time::Duration;

use data::appearance;
use futures::stream::BoxStream;
use futures::{StreamExt, stream};
use iced::advanced::graphics::futures::subscription;
use iced::advanced::subscription::Hasher;
use iced::{Subscription, futures};
pub use theme::Theme;
use tokio::time;

pub mod theme;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Mode {
    Dark,
    Light,
}

impl From<dark_light::Mode> for Mode {
    fn from(mode: dark_light::Mode) -> Self {
        match mode {
            dark_light::Mode::Dark => Mode::Dark,
            dark_light::Mode::Light => Mode::Light,
            // We map `Unspecified` to `Light`.
            dark_light::Mode::Unspecified => Mode::Light,
        }
    }
}

pub fn detect() -> Option<Mode> {
    let Ok(mode) = dark_light::detect() else {
        return None;
    };

    Some(Mode::from(mode))
}

pub fn theme(selected: &data::appearance::Selected) -> data::appearance::Theme {
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
                appearance::Theme::default()
            }
        },
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
        let interval = time::interval(Duration::from_secs(5));

        stream::unfold(
            (interval, detect().unwrap_or(Mode::Light)),
            move |(mut interval, old_mode)| async move {
                loop {
                    interval.tick().await;
                    let new_mode = detect().unwrap_or(Mode::Light);

                    if new_mode != old_mode {
                        return Some((new_mode, (interval, new_mode)));
                    }
                }
            },
        )
        .boxed()
    }
}

pub fn subscription() -> Subscription<Mode> {
    subscription::from_recipe(Appearance)
}
