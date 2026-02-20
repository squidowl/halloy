pub use data::appearance::theme::{
    Buffer, Button, Buttons, Formatting, General, ServerMessages, Styles, Text,
    color_to_hex, hex_to_color,
};
use data::config;
use iced::widget::text::LineHeight;

use crate::widget::combo_box;

pub mod button;
pub mod checkbox;
pub mod container;
pub mod context_menu;
pub mod font_style;
pub mod menu;
pub mod pane_grid;
pub mod progress_bar;
pub mod rule;
pub mod scrollable;
pub mod selectable_text;
pub mod svg;
pub mod text;
pub mod text_editor;
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
            Theme::Selected(selected) | Theme::Preview { selected, .. } => {
                Self::Preview {
                    selected: selected.clone(),
                    preview: theme,
                }
            }
        }
    }

    pub fn selected(&self) -> Self {
        match self {
            Theme::Selected(selected) | Theme::Preview { selected, .. } => {
                Self::Selected(selected.clone())
            }
        }
    }

    pub fn styles(&self) -> &Styles {
        match self {
            Theme::Selected(selected) => &selected.styles,
            Theme::Preview { preview, .. } => &preview.styles,
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

impl iced::theme::Base for Theme {
    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: self.styles().general.background,
            text_color: self.styles().text.primary.color,
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        None
    }

    fn default(_preference: iced::theme::Mode) -> Self {
        Self::from(data::Theme::default())
    }

    fn mode(&self) -> iced::theme::Mode {
        iced::theme::Mode::Dark
    }
    fn name(&self) -> &str {
        match self {
            Theme::Selected(selected) => selected.name.as_str(),
            Theme::Preview { preview, .. } => preview.name.as_str(),
        }
    }
}

impl combo_box::Catalog for Theme {}

pub fn line_height(config: &config::Font) -> LineHeight {
    config
        .line_height
        .map(LineHeight::Relative)
        .unwrap_or_default()
}

pub fn resolve_line_height(config: &config::Font, size: Option<f32>) -> f32 {
    line_height(config)
        .to_absolute(size.unwrap_or(TEXT_SIZE).into())
        .0
}
