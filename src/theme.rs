use data::palette::{self, Palette};
use iced::widget::{button, container, pane_grid, rule, scrollable, text, text_input};
use iced::{application, Background, Color};

pub const TEXT_SIZE: f32 = 13.0;
pub const ICON_SIZE: f32 = 11.0;

#[derive(Clone)]
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
    pub base_50: Color,
    pub base_100: Color,
    pub base_200: Color,
    pub base_300: Color,
    pub base_400: Color,
    pub base_500: Color,
    pub base_600: Color,
    pub base_700: Color,
    pub base_800: Color,
    pub base_900: Color,
    pub base_950: Color,
    pub mute_03: Color,
    pub mute_06: Color,
    pub mute_09: Color,
    pub mute_12: Color,
    pub mute_15: Color,
    pub mute_18: Color,
    pub intensify_03: Color,
    pub intensify_06: Color,
    pub intensify_09: Color,
    pub intensify_12: Color,
    pub intensify_15: Color,
    pub intensify_18: Color,
    pub lighten_03: Color,
    pub lighten_06: Color,
    pub lighten_09: Color,
    pub lighten_12: Color,
    pub lighten_15: Color,
    pub lighten_18: Color,
    pub darken_03: Color,
    pub darken_06: Color,
    pub darken_09: Color,
    pub darken_12: Color,
    pub darken_15: Color,
    pub darken_18: Color,
}

impl Subpalette {
    pub fn from_color(color: Color) -> Subpalette {
        Subpalette {
            base: color,
            base_50: palette::lighten(color, 0.95),
            base_100: palette::lighten(color, 0.90),
            base_200: palette::lighten(color, 0.80),
            base_300: palette::lighten(color, 0.70),
            base_400: palette::lighten(color, 0.60),
            base_500: palette::lighten(color, 0.50),
            base_600: palette::lighten(color, 0.40),
            base_700: palette::lighten(color, 0.30),
            base_800: palette::lighten(color, 0.20),
            base_900: palette::lighten(color, 0.10),
            base_950: palette::lighten(color, 0.05),
            mute_03: palette::mute(color, 0.03),
            mute_06: palette::mute(color, 0.06),
            mute_09: palette::mute(color, 0.09),
            mute_12: palette::mute(color, 0.12),
            mute_15: palette::mute(color, 0.15),
            mute_18: palette::mute(color, 0.18),
            intensify_03: palette::intensify(color, 0.03),
            intensify_06: palette::intensify(color, 0.06),
            intensify_09: palette::intensify(color, 0.09),
            intensify_12: palette::intensify(color, 0.12),
            intensify_15: palette::intensify(color, 0.15),
            intensify_18: palette::intensify(color, 0.18),
            lighten_03: palette::lighten(color, 0.03),
            lighten_06: palette::lighten(color, 0.06),
            lighten_09: palette::lighten(color, 0.09),
            lighten_12: palette::lighten(color, 0.12),
            lighten_15: palette::lighten(color, 0.15),
            lighten_18: palette::lighten(color, 0.1),
            darken_03: palette::darken(color, 0.03),
            darken_06: palette::darken(color, 0.06),
            darken_09: palette::darken(color, 0.09),
            darken_12: palette::darken(color, 0.12),
            darken_15: palette::darken(color, 0.15),
            darken_18: palette::darken(color, 0.18),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Rule {
    #[default]
    Default,
}

impl rule::StyleSheet for Theme {
    type Style = Rule;

    fn appearance(&self, style: &Self::Style) -> rule::Appearance {
        match style {
            Rule::Default => rule::Appearance {
                color: self.colors.background.lighten_03,
                width: 1,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Text {
    #[default]
    Default,
    Accent,
}

impl text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: Self::Style) -> text::Appearance {
        match style {
            Text::Default => text::Appearance {
                color: Some(self.colors.text.base),
            },
            Text::Accent => text::Appearance {
                color: Some(self.colors.accent.base),
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Container {
    #[default]
    Default,
    Primary,
    PaneBody {
        selected: bool,
    },
    PaneHeader,
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
            Container::PaneBody { selected } => container::Appearance {
                background: Some(Background::Color(self.colors.background.darken_03)),
                border_radius: 4.0.into(),
                border_width: 1.0,
                border_color: if *selected {
                    self.colors.action.mute_15
                } else {
                    Color::TRANSPARENT
                },
                ..Default::default()
            },
            Container::PaneHeader => container::Appearance {
                background: Some(Background::Color(self.colors.background.darken_06)),
                border_radius: [4.0, 4.0, 0.0, 0.0].into(),
                border_width: 1.0,
                border_color: Color::TRANSPARENT,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Button {
    #[default]
    Default,
    SideMenu {
        selected: bool,
    },
    Pane {
        selected: bool,
    },
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Default => button::Appearance {
                background: Some(Background::Color(self.colors.background.darken_09)),
                border_color: self.colors.background.mute_03,
                border_width: 1.0,
                border_radius: 3.0.into(),
                ..Default::default()
            },
            Button::SideMenu { selected } if *selected => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_06)),
                border_radius: 3.0.into(),
                ..Default::default()
            },
            Button::SideMenu { .. } => button::Appearance {
                background: None,
                ..Default::default()
            },
            Button::Pane { selected } if *selected => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_03)),
                border_color: self.colors.action.mute_06,
                border_width: 1.0,
                border_radius: 3.0.into(),
                ..Default::default()
            },
            Button::Pane { .. } => button::Appearance {
                background: Some(Background::Color(self.colors.background.darken_09)),
                border_color: self.colors.background.mute_03,
                border_width: 1.0,
                border_radius: 3.0.into(),
                ..Default::default()
            },
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        match style {
            Button::Default => button::Appearance { ..active },
            Button::SideMenu { selected: _ } => button::Appearance { ..active },
            Button::Pane { selected: _ } => button::Appearance { ..active },
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);

        match style {
            Button::Default => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_06)),
                border_radius: 4.0.into(),
                ..Default::default()
            },
            Button::SideMenu { selected } if *selected => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_12)),
                ..active
            },
            Button::SideMenu { .. } => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_06)),
                border_radius: 3.0.into(),
                ..active
            },
            Button::Pane { selected } if *selected => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_12)),
                ..active
            },
            Button::Pane { .. } => button::Appearance {
                background: Some(Background::Color(self.colors.background.mute_06)),
                border_color: self.colors.background.mute_06,
                ..active
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
    Hidden,
}

impl scrollable::StyleSheet for Theme {
    type Style = Scrollable;

    fn active(&self, style: &Self::Style) -> scrollable::Scrollbar {
        match style {
            Scrollable::Default => scrollable::Scrollbar {
                background: Some(Background::Color(self.colors.background.mute_06)),
                border_radius: 8.0.into(),
                border_width: 1.0,
                border_color: Color::TRANSPARENT,
                scroller: scrollable::Scroller {
                    color: self.colors.background.lighten_03,
                    border_radius: 8.0.into(),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            },
            Scrollable::Hidden => scrollable::Scrollbar {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border_radius: 8.0.into(),
                border_width: 1.0,
                border_color: Color::TRANSPARENT,
                scroller: scrollable::Scroller {
                    color: Color::TRANSPARENT,
                    border_radius: 8.0.into(),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            },
        }
    }

    fn hovered(
        &self,
        style: &Self::Style,
        _is_mouse_over_scrollbar: bool,
    ) -> scrollable::Scrollbar {
        let active = self.active(style);
        match style {
            Scrollable::Default => scrollable::Scrollbar { ..active },
            Scrollable::Hidden => scrollable::Scrollbar { ..active },
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
                background: Background::Color(self.colors.background.mute_03),
                border_width: 1.0,
                border_color: self.colors.accent.base,
                border_radius: 4.0.into(),
            },
        }
    }

    fn picked_split(&self, style: &Self::Style) -> Option<pane_grid::Line> {
        match style {
            PaneGrid::Default => Some(pane_grid::Line {
                color: self.colors.background.mute_03,
                width: 2.0,
            }),
        }
    }

    fn hovered_split(&self, style: &Self::Style) -> Option<pane_grid::Line> {
        match style {
            PaneGrid::Default => Some(pane_grid::Line {
                color: self.colors.background.mute_03,
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
                border_radius: 4.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                icon_color: self.colors.action.mute_03,
            },
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default => text_input::Appearance {
                ..self.active(style)
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default => text_input::Appearance {
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
            TextInput::Default => Color {
                a: 0.4,
                ..self.colors.text.base
            },
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
                icon_color: self.colors.action.mute_03,
            },
        }
    }
}
