use data::appearance;

pub use theme::Theme;

pub mod theme;

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
            // We ignore `Unspecified`.
            dark_light::Mode::Unspecified => Err(()),
        }
    }
}

pub fn detect() -> Option<Mode> {
    let Ok(mode) = dark_light::detect() else {
        return None;
    };

    Mode::try_from(mode).ok()
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
                Default::default()
            }
        },
    }
}
