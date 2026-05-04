use data::buffer::Brackets;
use data::config::buffer::{AccessLevelFormat, Dimmed};
use data::config::display::nickname::Metadata;
use data::target::{Query, TargetRef};
use data::{Config, User, metadata};
use iced::Color;
use iced::widget::{container, row};
use unicode_segmentation::UnicodeSegmentation;

use super::{Element, selectable_text};
use crate::{Theme, font, theme, widget};

pub struct UserDisplay {
    base: UserDisplayData,
    tooltip: Option<UserDisplayData>,
}

impl UserDisplay {
    pub fn new(
        user: &User,
        show_access_levels: AccessLevelFormat,
        show_bot_icon: bool,
        registry: &dyn metadata::Registry,
        enabled: &[Metadata],
        truncate: Option<u16>,
        truncation_character: char,
        brackets: Option<&Brackets>,
    ) -> Self {
        let full = UserDisplayData::new(
            user,
            show_access_levels,
            show_bot_icon,
            registry,
            enabled,
        );

        if let Some(truncated) = truncate.and_then(|truncation_length| {
            full.truncate(truncation_length as usize, truncation_character)
        }) {
            Self {
                base: truncated.bracket(brackets),
                tooltip: Some(full),
            }
        } else if full.bot_icon && brackets.is_some() {
            Self {
                base: full.clone().bracket(brackets),
                tooltip: Some(full),
            }
        } else {
            let tooltip = full.bot_icon.then_some(full.clone());

            Self {
                base: full.bracket(brackets),
                tooltip,
            }
        }
    }

    pub fn into_element<'a, M: 'a>(
        self,
        user: &User,
        is_away: bool,
        is_offline: bool,
        dimmed: Option<(Dimmed, Color)>,
        theme: &'a Theme,
        config: &'a Config,
    ) -> Element<'a, M> {
        let base = self
            .base
            .into_element(user, is_away, is_offline, dimmed, theme, config);

        if let Some(tooltip) = self.tooltip {
            iced::widget::tooltip(
                base,
                container(container(if tooltip.bot_icon {
                    row![
                        tooltip.into_element(
                            user, false, false, None, theme, config,
                        ),
                        selectable_text(String::from(
                            " has marked itself as a bot"
                        ))
                    ]
                    .spacing(theme::ICON_SPACE)
                    .into()
                } else {
                    tooltip
                        .into_element(user, false, false, None, theme, config)
                }))
                .style(theme::container::tooltip)
                .padding(8),
                iced::widget::tooltip::Position::Top,
            )
            .delay(iced::time::Duration::ZERO)
            .into()
        } else {
            base
        }
    }

    pub fn width(&self, config: &Config) -> f32 {
        self.base.width(config)
    }
}

#[derive(Clone)]
pub struct UserDisplayData {
    left: String,
    bot_icon: bool,
    right: Option<String>,
}

impl UserDisplayData {
    pub fn new(
        user: &User,
        show_access_levels: AccessLevelFormat,
        show_bot_icon: bool,
        registry: &dyn metadata::Registry,
        enabled: &[Metadata],
    ) -> Self {
        let prefixed_nick = user.display(show_access_levels, false, None, '\0');

        let bot_icon = user.is_bot() && show_bot_icon;

        let query = Query::from(user);

        let display_name = if enabled.contains(&Metadata::DisplayName)
            && let Some(display_name) =
                registry.display_name(TargetRef::Query(&query))
            && !display_name.is_empty()
        {
            Some(display_name)
        } else {
            None
        };

        let pronouns = if enabled.contains(&Metadata::Pronouns)
            && let Some(pronouns) = registry.pronouns(&query)
            && !pronouns.is_empty()
        {
            Some(pronouns)
        } else {
            None
        };

        if bot_icon {
            let (left, right) = if let Some(display_name) = display_name {
                (
                    format!("{display_name} ({prefixed_nick}"),
                    if let Some(pronouns) = pronouns {
                        Some(format!("{pronouns})"))
                    } else {
                        Some(String::from(")"))
                    },
                )
            } else {
                (
                    prefixed_nick,
                    pronouns.map(|pronouns| format!(" ({pronouns})")),
                )
            };

            Self {
                left,
                bot_icon,
                right,
            }
        } else {
            let left = match (display_name, pronouns) {
                (Some(display_name), Some(pronouns)) => {
                    format!("{display_name} ({prefixed_nick}, {pronouns})")
                }
                (Some(display_name), None) => {
                    format!("{display_name} ({prefixed_nick})")
                }
                (None, Some(pronouns)) => {
                    format!("{prefixed_nick} ({pronouns})")
                }
                (None, None) => prefixed_nick,
            };

            Self {
                left,
                bot_icon,
                right: None,
            }
        }
    }

    pub fn into_element<'a, M: 'a>(
        self,
        user: &User,
        is_away: bool,
        is_offline: bool,
        dimmed: Option<(Dimmed, Color)>,
        theme: &'a Theme,
        config: &'a Config,
    ) -> Element<'a, M> {
        let style = theme::selectable_text::dimmed(
            theme::selectable_text::nickname(
                theme, config, user, is_away, is_offline,
            ),
            theme,
            dimmed,
        );

        self.render(style, is_offline, None, theme)
    }

    pub(crate) fn into_element_sized<'a, M: 'a>(
        self,
        user: &User,
        size: f32,
        theme: &'a Theme,
        config: &'a Config,
    ) -> Element<'a, M> {
        let style =
            theme::selectable_text::nickname(theme, config, user, false, false);

        self.render(style, false, Some(size), theme)
    }

    fn render<'a, M: 'a>(
        self,
        style: crate::widget::selectable_text::Style,
        is_offline: bool,
        size: Option<f32>,
        theme: &'a Theme,
    ) -> Element<'a, M> {
        let font =
            theme::font_style::nickname(theme, is_offline).map(font::get);

        if self.bot_icon {
            row![
                selectable_text(self.left)
                    .style(move |_| style)
                    .font_maybe(font.clone())
                    .size_maybe(size),
                widget::bot_icon(move |_| style),
                self.right.map(|right| selectable_text(right)
                    .style(move |_| style)
                    .font_maybe(font)
                    .size_maybe(size)),
            ]
            .spacing(theme::ICON_SPACE)
            .into()
        } else {
            selectable_text(self.left)
                .style(move |_| style)
                .font_maybe(font)
                .size_maybe(size)
                .into()
        }
    }

    pub fn width(&self, config: &Config) -> f32 {
        let mut width = font::width_from_str(self.left.as_str(), &config.font);

        if self.bot_icon {
            width += theme::ICON_SPACE + theme::ICON_SIZE;

            if let Some(right) = self.right.as_ref() {
                width += theme::ICON_SPACE
                    + font::width_from_str(right.as_str(), &config.font);
            }
        }

        width
    }

    pub fn truncate(
        &self,
        truncation_length: usize,
        truncation_character: char,
    ) -> Option<Self> {
        let left_length =
            UnicodeSegmentation::graphemes(self.left.as_str(), true).count();

        if truncation_length < left_length {
            return Some(Self {
                left: format!(
                    "{}{truncation_character}",
                    UnicodeSegmentation::graphemes(self.left.as_str(), true)
                        .take(truncation_length.saturating_sub(1))
                        .collect::<String>()
                ),
                bot_icon: false,
                right: None,
            });
        } else if self.bot_icon {
            if truncation_length < left_length.saturating_add(2) {
                return Some(Self {
                    left: format!(
                        "{}{truncation_character}",
                        self.left.as_str()
                    ),
                    bot_icon: false,
                    right: None,
                });
            } else {
                let right_length = self
                    .right
                    .as_ref()
                    .map(|right| {
                        UnicodeSegmentation::graphemes(right.as_str(), true)
                            .count()
                    })
                    .unwrap_or_default();

                if truncation_length
                    < left_length.saturating_add(2).saturating_add(right_length)
                {
                    return Some(Self {
                        left: self.left.clone(),
                        bot_icon: true,
                        right: self.right.as_ref().map(|right| {
                            format!(
                                "{}{truncation_character}",
                                UnicodeSegmentation::graphemes(
                                    right.as_str(),
                                    true
                                )
                                .take(
                                    truncation_length
                                        .saturating_sub(left_length)
                                        .saturating_sub(2)
                                        .saturating_sub(1)
                                )
                                .collect::<String>()
                            )
                        }),
                    });
                }
            }
        }

        None
    }

    pub fn bracket(self, brackets: Option<&Brackets>) -> Self {
        if let Some(brackets) = brackets {
            if self.bot_icon {
                UserDisplayData {
                    left: format!("{}{}", brackets.left, self.left),
                    bot_icon: true,
                    right: Some(
                        self.right
                            .map(|right| format!("{right}{}", brackets.right))
                            .unwrap_or(brackets.right.clone()),
                    ),
                }
            } else {
                UserDisplayData {
                    left: format!(
                        "{}{}{}",
                        brackets.left, self.left, brackets.right
                    ),
                    bot_icon: false,
                    right: None,
                }
            }
        } else {
            self
        }
    }
}
