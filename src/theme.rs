use data::message;
use data::theme::{alpha, randomize_color, Colors};
use iced::widget::{button, container, pane_grid, rule, scrollable, text, text_input};
use iced::{application, overlay, Background, Border, Color};

use crate::widget::selectable_text;

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

    fn colors(&self) -> &Colors {
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

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: self.colors().background.base,
            text_color: self.colors().text.base,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Rule {
    #[default]
    Default,
    Unread,
}

impl rule::StyleSheet for Theme {
    type Style = Rule;

    fn appearance(&self, style: &Self::Style) -> rule::Appearance {
        match style {
            Rule::Default => rule::Appearance {
                color: self.colors().background.light,
                width: 1,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
            },
            Rule::Unread => rule::Appearance {
                color: self.colors().accent.base,
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
    Primary,
    Action,
    Accent,
    Info,
    Server,
    Error,
    Transparent,
    Status(message::source::Status),
    Nickname(Option<String>, bool),
}

impl text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: Self::Style) -> text::Appearance {
        match style {
            Text::Default => text::Appearance { color: None },
            Text::Primary => text::Appearance {
                color: Some(self.colors().text.base),
            },
            Text::Action => text::Appearance {
                color: Some(self.colors().action.base),
            },
            Text::Accent => text::Appearance {
                color: Some(self.colors().accent.base),
            },
            Text::Info => text::Appearance {
                color: Some(self.colors().info.base),
            },
            Text::Error => text::Appearance {
                color: Some(self.colors().error.base),
            },
            Text::Nickname(seed, transparent) => {
                let original_color = self.colors().action.base;
                let randomized_color = seed
                    .map(|seed| randomize_color(original_color, seed.as_str()))
                    .unwrap_or_else(|| original_color);

                let color = if transparent {
                    let dark_theme = self.colors().is_dark_theme();
                    alpha(randomized_color, if dark_theme { 0.2 } else { 0.4 })
                } else {
                    randomized_color
                };

                text::Appearance { color: Some(color) }
            }
            Text::Server => text::Appearance {
                color: Some(self.colors().info.base),
            },
            Text::Status(status) => text::Appearance {
                color: Some(match status {
                    message::source::Status::Success => self.colors().success.base,
                    message::source::Status::Error => self.colors().error.base,
                }),
            },
            Text::Transparent => text::Appearance {
                color: Some(self.colors().text.low_alpha),
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
    Command {
        selected: bool,
    },
    Context,
    Highlight,
    SemiTransparent,
}

impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        match style {
            Container::Default => container::Appearance {
                ..Default::default()
            },
            Container::Primary => container::Appearance {
                background: Some(Background::Color(self.colors().background.base)),
                text_color: Some(self.colors().text.base),
                ..Default::default()
            },
            Container::PaneBody { selected } => container::Appearance {
                background: Some(Background::Color(self.colors().background.dark)),
                border: Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: if *selected {
                        self.colors().action.base
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            },
            Container::PaneHeader => container::Appearance {
                background: Some(Background::Color(self.colors().background.darker)),
                border: Border {
                    radius: [4.0, 4.0, 0.0, 0.0].into(),
                    width: 1.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            },
            Container::Command { selected } if *selected => container::Appearance {
                background: Some(Background::Color(self.colors().background.darker)),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Container::Command { .. } => container::Appearance {
                background: None,
                ..Default::default()
            },
            Container::Context => container::Appearance {
                //TODO: Blur background when possible?
                background: Some(Background::Color(self.colors().background.base)),
                border: Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: if self.colors().is_dark_theme() {
                        self.colors().background.lighter
                    } else {
                        self.colors().background.darker
                    },
                },
                ..Default::default()
            },
            Container::Highlight => container::Appearance {
                background: Some(Background::Color(self.colors().info.high_alpha)),
                border: Border {
                    radius: 0.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Container::SemiTransparent => container::Appearance {
                background: Some(
                    Color {
                        a: 0.80,
                        ..self.colors().background.base
                    }
                    .into(),
                ),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Button {
    #[default]
    Default,
    Secondary,
    SideMenu {
        selected: bool,
    },
    Pane {
        selected: bool,
    },
    Context,
    Bare,
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        match style {
            Button::Default => button::Appearance {
                background: Some(Background::Color(self.colors().action.high_alpha)),
                text_color: self.colors().action.base,
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Button::Secondary => button::Appearance {
                background: Some(Background::Color(self.colors().text.high_alpha)),
                text_color: self.colors().text.base,
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Button::SideMenu { selected } if *selected => button::Appearance {
                background: Some(Background::Color(self.colors().background.darker)),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Button::SideMenu { .. } => button::Appearance {
                background: None,
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Button::Pane { selected } if *selected => button::Appearance {
                background: Some(Background::Color(self.colors().action.med_alpha)),
                border: Border {
                    color: self.colors().action.low_alpha,
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..Default::default()
            },
            Button::Pane { .. } => button::Appearance {
                background: Some(Background::Color(self.colors().background.dark)),
                border: Border {
                    color: if self.colors().is_dark_theme() {
                        self.colors().background.lightest
                    } else {
                        self.colors().background.darkest
                    },
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..Default::default()
            },
            Button::Context => button::Appearance {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Button::Bare => button::Appearance {
                background: Some(Background::Color(Color::TRANSPARENT)),
                ..Default::default()
            },
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        match style {
            Button::Default => button::Appearance { ..active },
            Button::Secondary => button::Appearance { ..active },
            Button::SideMenu { selected: _ } => button::Appearance { ..active },
            Button::Pane { selected: _ } => button::Appearance { ..active },
            Button::Context => button::Appearance { ..active },
            Button::Bare => button::Appearance { ..active },
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);

        match style {
            Button::Default => button::Appearance {
                background: Some(Background::Color(self.colors().action.med_alpha)),
                text_color: self.colors().action.base,
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Button::Secondary => button::Appearance {
                background: Some(Background::Color(self.colors().text.med_alpha)),
                text_color: self.colors().text.base,
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            Button::SideMenu { selected } if *selected => button::Appearance {
                background: Some(Background::Color(self.colors().background.darkest)),
                ..active
            },
            Button::SideMenu { .. } => button::Appearance {
                background: Some(Background::Color(self.colors().background.dark)),
                ..active
            },
            Button::Pane { selected } if *selected => button::Appearance {
                background: Some(Background::Color(self.colors().action.high_alpha)),
                ..active
            },
            Button::Pane { .. } => button::Appearance {
                background: Some(Background::Color(if self.colors().is_dark_theme() {
                    self.colors().background.light
                } else {
                    self.colors().background.darker
                })),
                ..active
            },
            Button::Context => button::Appearance {
                background: Some(Background::Color(self.colors().background.darker)),
                ..active
            },
            Button::Bare => button::Appearance { ..active },
        }
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);

        button::Appearance {
            text_color: Color {
                a: 0.2,
                ..active.text_color
            },
            border: Border {
                color: Color {
                    a: 0.2,
                    ..active.border.color
                },
                ..Default::default()
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
                background: Some(Background::Color(self.colors().background.darker)),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: Color::TRANSPARENT,
                },
                scroller: scrollable::Scroller {
                    color: self.colors().background.darkest,
                    border: Border {
                        radius: 8.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
            },
            Scrollable::Hidden => scrollable::Scrollbar {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: Color::TRANSPARENT,
                },
                scroller: scrollable::Scroller {
                    color: Color::TRANSPARENT,
                    border: Border {
                        radius: 8.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
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
                background: Background::Color(self.colors().action.high_alpha),
                border: Border {
                    width: 1.0,
                    color: self.colors().action.base,
                    radius: 4.0.into(),
                },
            },
        }
    }

    fn picked_split(&self, style: &Self::Style) -> Option<pane_grid::Line> {
        match style {
            PaneGrid::Default => Some(pane_grid::Line {
                color: self.colors().action.base,
                width: 4.0,
            }),
        }
    }

    fn hovered_split(&self, style: &Self::Style) -> Option<pane_grid::Line> {
        match style {
            PaneGrid::Default => Some(pane_grid::Line {
                color: self.colors().action.base,
                width: 4.0,
            }),
        }
    }
}

#[derive(Default)]
pub enum TextInput {
    #[default]
    Default,
    Error,
}

impl text_input::StyleSheet for Theme {
    type Style = TextInput;

    fn active(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default => text_input::Appearance {
                background: Background::Color(self.colors().background.darker),
                border: Border {
                    radius: 4.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                    // XXX Not currently displayed in application.
                },
                icon_color: self.colors().info.base,
            },
            TextInput::Error => text_input::Appearance {
                border: Border {
                    width: 1.0,
                    color: self.colors().error.base,
                    ..Default::default()
                },
                ..self.active(&TextInput::Default)
            },
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }

    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }

    fn selection_color(&self, style: &Self::Style) -> Color {
        match style {
            TextInput::Default | TextInput::Error => self.colors().accent.high_alpha,
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> Color {
        match style {
            TextInput::Default | TextInput::Error => self.colors().text.low_alpha,
        }
    }

    fn value_color(&self, style: &Self::Style) -> Color {
        match style {
            TextInput::Default | TextInput::Error => self.colors().text.base,
        }
    }

    fn disabled_color(&self, style: &Self::Style) -> Color {
        match style {
            TextInput::Default | TextInput::Error => self.colors().text.base,
        }
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        match style {
            TextInput::Default | TextInput::Error => text_input::Appearance {
                background: Background::Color(self.colors().background.light),
                border: Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                    // XXX Not currently displayed in application.
                },
                icon_color: self.colors().info.base,
            },
        }
    }
}

impl selectable_text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: &Self::Style) -> selectable_text::Appearance {
        let color = <Theme as text::StyleSheet>::appearance(self, style.clone()).color;

        let selection_color = self.colors().accent.high_alpha;

        selectable_text::Appearance {
            color,
            selection_color,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Menu {
    #[default]
    Default,
    ComboBox,
}

impl overlay::menu::StyleSheet for Theme {
    type Style = Menu;

    fn appearance(&self, style: &Self::Style) -> overlay::menu::Appearance {
        match style {
            Menu::Default => overlay::menu::Appearance {
                text_color: self.colors().text.base,
                background: Background::Color(self.colors().background.base),
                border: Border {
                    width: 1.0,
                    radius: 4.0.into(),
                    color: self.colors().action.base,
                },
                selected_text_color: self.colors().text.high_alpha,
                selected_background: Background::Color(self.colors().background.high_alpha),
            },
            Menu::ComboBox => overlay::menu::Appearance {
                text_color: self.colors().text.base,
                background: Background::Color(self.colors().background.base),
                border: Border {
                    width: 1.0,
                    radius: 4.0.into(),
                    color: if self.colors().is_dark_theme() {
                        self.colors().background.lighter
                    } else {
                        self.colors().background.darker
                    },
                },
                selected_text_color: self.colors().text.base,
                selected_background: Background::Color(self.colors().background.dark),
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ComboBox {
    #[default]
    Default,
}

impl From<ComboBox> for TextInput {
    fn from(combo_box: ComboBox) -> Self {
        match combo_box {
            ComboBox::Default => TextInput::Default,
        }
    }
}

impl From<ComboBox> for Menu {
    fn from(combo_box: ComboBox) -> Self {
        match combo_box {
            ComboBox::Default => Menu::ComboBox,
        }
    }
}
