use data::palette::{self, Palette};
use iced::widget::{button, container, pane_grid, scrollable, text, text_input};
use iced::{application, Background, Color};

pub const TEXT_SIZE: f32 = 13.0;

pub struct Theme {
    pub palette: Palette,
    pub colors: Colors,
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: self.colors.background.base,
            text_color: self.colors.text.base,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        let palette = Palette::default();

        Theme {
            palette,
            colors: Colors::colors_from_palette(&palette),
        }
    }
}

impl Theme {
    pub fn new_from_palette(palette: data::palette::Palette) -> Self {
        Theme {
            palette,
            colors: Colors {
                background: Subpalette::from_color(palette.background),
                text: Subpalette::from_color(palette.text),
                action: Subpalette::from_color(palette.action),
                accent: Subpalette::from_color(palette.accent),
                alert: Subpalette::from_color(palette.alert),
                error: Subpalette::from_color(palette.error),
                info: Subpalette::from_color(palette.info),
                success: Subpalette::from_color(palette.success),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Colors {
    pub background: Subpalette,
    pub text: Subpalette,
    pub action: Subpalette,
    pub accent: Subpalette,
    pub alert: Subpalette,
    pub error: Subpalette,
    pub info: Subpalette,
    pub success: Subpalette,
}

impl Colors {
    pub fn colors_from_palette(palette: &Palette) -> Self {
        Colors {
            background: Subpalette::from_color(palette.background),
            text: Subpalette::from_color(palette.text),
            action: Subpalette::from_color(palette.action),
            accent: Subpalette::from_color(palette.accent),
            alert: Subpalette::from_color(palette.alert),
            error: Subpalette::from_color(palette.error),
            info: Subpalette::from_color(palette.info),
            success: Subpalette::from_color(palette.success),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Subpalette {
    pub base: Color,
    pub lighten_03: Color,
    pub lighten_06: Color,
    pub lighten_12: Color,
    pub darken_03: Color,
    pub darken_06: Color,
    pub darken_12: Color,
    pub mute_03: Color,
    pub mute_06: Color,
    pub mute_12: Color,
}

impl Subpalette {
    pub fn from_color(color: Color) -> Subpalette {
        Subpalette {
            base: color,
            lighten_03: palette::lighten(color, 0.03),
            lighten_06: palette::lighten(color, 0.06),
            lighten_12: palette::lighten(color, 0.12),
            darken_03: palette::darken(color, 0.03),
            darken_06: palette::darken(color, 0.06),
            darken_12: palette::darken(color, 0.12),
            mute_03: palette::mute(color, 0.03),
            mute_06: palette::mute(color, 0.06),
            mute_12: palette::mute(color, 0.12),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Text {
    #[default]
    Default,
}

impl text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: Self::Style) -> text::Appearance {
        match style {
            Text::Default => text::Appearance {
                color: Some(self.colors.text.base),
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Container {
    #[default]
    Default,
    Primary,
    Pane {
        selected: bool,
    },
    Header,
}

impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        match style {
            Container::Default => container::Appearance {
                ..Default::default()
            },
            Container::Primary => container::Appearance {
                background: Some(Background::Color(self.colors.background.base)),
                text_color: Some(self.colors.text.base),
                ..Default::default()
            },
            Container::Pane { selected: _ } => container::Appearance {
                background: Some(Background::Color(self.colors.background.mute_03)),
                border_radius: [0.0, 0.0, 4.0, 4.0].into(),
                ..Default::default()
            },
            Container::Header => container::Appearance {
                background: Some(Background::Color(self.colors.background.mute_03)),
                border_radius: [4.0, 4.0, 0.0, 0.0].into(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Button {
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Selectable {
        selected: bool,
    },
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_06)),
                border_radius: 4.0.into(),
                ..Default::default()
            },
            Button::Secondary => button::Appearance {
                text_color: self.colors.text.base,
                border_width: 1.0,
                border_color: self.colors.action.base,
                ..Default::default()
            },
            Button::Tertiary => button::Appearance {
                ..Default::default()
            },
            Button::Selectable { selected: _ } => button::Appearance {
                text_color: self.colors.text.base,
                border_width: 1.0,
                border_color: self.colors.action.base,
                ..Default::default()
            },
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        match style {
            Button::Primary => button::Appearance { ..active },
            Button::Secondary => button::Appearance { ..active },
            Button::Tertiary => button::Appearance { ..active },
            Button::Selectable { selected: _ } => button::Appearance { ..active },
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Primary => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_12)),
                border_radius: 4.0.into(),
                ..Default::default()
            },
            Button::Secondary => button::Appearance {
                text_color: self.colors.action.base,
                background: Some(Background::Color(self.colors.action.base)),
                border_width: 1.0,
                border_color: self.colors.action.base,
                ..Default::default()
            },
            Button::Tertiary => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_03)),
                border_radius: 4.0.into(),
                ..Default::default()
            },
            Button::Selectable { selected: _ } => button::Appearance {
                text_color: self.colors.action.base,
                background: Some(Background::Color(self.colors.action.base)),
                border_width: 1.0,
                border_color: self.colors.action.base,
                ..Default::default()
            },
        }
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);

        button::Appearance {
            text_color: Color {
                a: 0.2,
                ..active.text_color
            },
            border_color: Color {
                a: 0.2,
                ..active.border_color
            },
            ..active
        }
    }
}

#[derive(Default)]
pub enum Scrollable {
    #[default]
    Default,
}

impl scrollable::StyleSheet for Theme {
    type Style = Scrollable;

    fn active(&self, style: &Self::Style) -> scrollable::Scrollbar {
        match style {
            Scrollable::Default => scrollable::Scrollbar {
                background: Some(Background::Color(self.colors.alert.base)),
                border_radius: 4.0.into(),
                border_width: 2.0,
                border_color: self.colors.info.base,
                scroller: scrollable::Scroller {
                    color: self.colors.action.darken_12,
                    border_radius: 4.0.into(),
                    border_width: 1.0,
                    border_color: self.colors.accent.base,
                },
            },
        }
    }

    fn hovered(
        &self,
        style: &Self::Style,
        _is_mouse_over_scrollbar: bool,
    ) -> scrollable::Scrollbar {
        match style {
            Scrollable::Default => scrollable::Scrollbar {
                background: Some(Background::Color(self.colors.alert.base)),
                border_radius: 4.0.into(),
                border_width: 2.0,
                border_color: self.colors.info.base,
                scroller: scrollable::Scroller {
                    color: self.colors.action.darken_12,
                    border_radius: 4.0.into(),
                    border_width: 1.0,
                    border_color: self.colors.accent.base,
                },
            },
        }
    }
}

#[derive(Default)]
pub enum PaneGrid {
    #[default]
    Default,
}

impl pane_grid::StyleSheet for Theme {
    type Style = PaneGrid;

    fn hovered_region(&self, style: &Self::Style) -> pane_grid::Appearance {
        match style {
            PaneGrid::Default => pane_grid::Appearance {
                background: Background::Color(self.colors.action.lighten_06),
                border_width: 1.0,
                border_color: self.colors.action.lighten_06,
                border_radius: 4.0.into(),
            },
        }
    }

    fn picked_split(&self, style: &Self::Style) -> Option<pane_grid::Line> {
        match style {
            PaneGrid::Default => Some(pane_grid::Line {
                color: self.colors.accent.base,
                width: 2.0,
            }),
        }
    }

    fn hovered_split(&self, style: &Self::Style) -> Option<pane_grid::Line> {
        match style {
            PaneGrid::Default => Some(pane_grid::Line {
                color: self.colors.accent.base,
                width: 2.0,
            }),
        }
    }
}

#[derive(Default)]
pub enum TextInput {
    #[default]
    Default,
}

impl text_input::StyleSheet for Theme {
    type Style = TextInput;

    fn active(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default => text_input::Appearance {
                background: Background::Color(self.colors.background.lighten_03),
                border_radius: 0.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                icon_color: self.colors.action.mute_06,
            },
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default => text_input::Appearance {
                background: Background::Color(self.colors.background.lighten_03),
                border_radius: 0.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                icon_color: self.colors.action.mute_06,
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default => text_input::Appearance {
                border_color: self.colors.action.base,
                ..self.active(style)
            },
        }
    }

    fn selection_color(&self, style: &Self::Style) -> Color {
        match style {
            TextInput::Default => self.colors.action.base,
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> Color {
        match style {
            TextInput::Default => self.colors.text.darken_06,
        }
    }

    fn value_color(&self, style: &Self::Style) -> Color {
        match style {
            TextInput::Default => self.colors.text.base,
        }
    }

    fn disabled_color(&self, style: &Self::Style) -> Color {
        match style {
            TextInput::Default => self.colors.text.base,
        }
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default => text_input::Appearance {
                background: Background::Color(self.colors.background.lighten_03),
                border_radius: 0.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                icon_color: self.colors.action.mute_06,
            },
        }
    }
}
