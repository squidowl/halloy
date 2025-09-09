use data::appearance;
pub use theme::Theme;

pub mod theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mode(iced::theme::Mode);

impl From<iced::theme::Mode> for Mode {
    fn from(mode: iced::theme::Mode) -> Self {
        Self(mode)
    }
}

impl Mode {
    pub fn theme(&self, selected: &data::appearance::Selected) -> data::appearance::Theme {
        match &selected {
            appearance::Selected::Static(theme) => theme.clone(),
            appearance::Selected::Dynamic { light, dark } => match self.0 {
                iced::theme::Mode::Dark => dark.clone(),
                iced::theme::Mode::Light => light.clone(),
                // We map `None` to `Light`.
                iced::theme::Mode::None => light.clone()
            },
        }
    }
}