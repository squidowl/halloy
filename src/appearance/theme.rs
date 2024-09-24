use iced::application;

use crate::widget::combo_box;

pub use data::appearance::theme::{
    color_to_hex, hex_to_color, Buffer, Button, Buttons, Colors, General, ServerMessages, Text,
};

pub mod button;
pub mod checkbox;
pub mod container;
pub mod context_menu;
pub mod menu;
pub mod pane_grid;
pub mod progress_bar;
pub mod rule;
pub mod scrollable;
pub mod selectable_text;
pub mod text;
pub mod text_input;

// TODO: If we use non-standard font sizes, we should consider
// Config.font.size since it's user configurable
pub const TEXT_SIZE: f32 = 13.0;
pub const ICON_SIZE: f32 = 12.0;

#[derive(Debug, Clone)]
pub enum Theme {
    Selected(data::Theme),
    Preview {
        selected: data::Theme,
        preview: data::Theme,
    },
}

impl Theme {
    pub fn preview(&self, theme: data::Theme) -> Self {
        match self {
            Theme::Selected(selected) | Theme::Preview { selected, .. } => Self::Preview {
                selected: selected.clone(),
                preview: theme,
            },
        }
    }

    pub fn selected(&self) -> Self {
        match self {
            Theme::Selected(selected) | Theme::Preview { selected, .. } => {
                Self::Selected(selected.clone())
            }
        }
    }

    pub fn colors(&self) -> &Colors {
        match self {
            Theme::Selected(selected) => &selected.colors,
            Theme::Preview { preview, .. } => &preview.colors,
        }
    }
}

impl From<data::Theme> for Theme {
    fn from(theme: data::Theme) -> Self {
        Theme::Selected(theme)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from(data::Theme::default())
    }
}

impl application::DefaultStyle for Theme {
    fn default_style(&self) -> application::Appearance {
        application::Appearance {
            background_color: self.colors().general.background,
            text_color: self.colors().text.primary,
        }
    }
}

impl combo_box::Catalog for Theme {}
