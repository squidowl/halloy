use data::buffer::Brackets;
use data::config::buffer::{AccessLevelFormat, Dimmed};
use data::config::display::nickname::Metadata;
use data::target::{Query, TargetRef};
use data::user::AccessLevel;
use data::{Config, User, metadata};
use iced::Color;
use iced::widget::{container, row, tooltip};
use unicode_segmentation::UnicodeSegmentation;

use super::{Element, selectable_text};
use crate::{Theme, font, theme, widget};

pub struct UserDisplay {
    full: UserDisplayData,
    truncated: Option<UserDisplayData>,
}

impl UserDisplay {
    pub fn new(
        user: &User,
        show_access_levels: AccessLevelFormat,
        show_bot_icon: bool,
        registry: &dyn metadata::Registry,
        truncate: Option<u16>,
        brackets: Option<&Brackets>,
        config: &Config,
    ) -> Self {
        let full = UserDisplayData::new(
            user,
            show_access_levels,
            show_bot_icon,
            registry,
            config,
        );

        if let Some(truncated) = truncate.and_then(|truncation_length| {
            full.truncate(
                truncation_length as usize,
                config.display.truncation_character,
            )
        }) {
            Self {
                full,
                truncated: Some(truncated.bracket(brackets)),
            }
        } else {
            Self {
                full: full.bracket(brackets),
                truncated: None,
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
        let (base_user_display, tooltip_user_display) =
            if let Some(truncated) = self.truncated {
                (truncated, Some(self.full))
            } else {
                let tooltip_user_display =
                    self.full.bot_icon.then_some(self.full.clone());

                (self.full, tooltip_user_display)
            };

        let base_user_display = base_user_display
            .into_element(user, is_away, is_offline, dimmed, theme, config);

        if let Some(tooltip_user_display) = tooltip_user_display {
            tooltip(
                base_user_display,
                container(container(if tooltip_user_display.bot_icon {
                    row![
                        tooltip_user_display.into_element(
                            user, false, false, None, theme, config,
                        ),
                        selectable_text(String::from(
                            " has marked itself as a bot"
                        ))
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(theme::ICON_SPACE)
                    .into()
                } else {
                    tooltip_user_display
                        .into_element(user, false, false, None, theme, config)
                }))
                .style(theme::container::tooltip)
                .padding(8),
                tooltip::Position::Top,
            )
            .delay(iced::time::Duration::ZERO)
            .into()
        } else {
            base_user_display
        }
    }

    pub fn width(&self, config: &Config) -> f32 {
        self.truncated
            .as_ref()
            .map_or(self.full.width(config), |truncated| {
                truncated.width(config)
            })
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
        config: &Config,
    ) -> Self {
        let access_levels = match show_access_levels {
            AccessLevelFormat::All => {
                let access_levels = user
                    .access_levels()
                    .filter_map(AccessLevel::char)
                    .collect::<String>();

                if access_levels.is_empty() {
                    None
                } else {
                    Some(access_levels)
                }
            }
            AccessLevelFormat::Highest => {
                user.highest_access_level().char().map(String::from)
            }
            AccessLevelFormat::None => None,
        }
        .unwrap_or_default();

        let nickname = user.nickname();

        let bot_icon = user.is_bot() && show_bot_icon;

        let query = Query::from(user);

        let display_name =
            if config.display.nickname.contains(&Metadata::DisplayName)
                && let Some(display_name) =
                    registry.display_name(TargetRef::Query(&query))
                && !display_name.is_empty()
            {
                Some(display_name)
            } else {
                None
            };

        let pronouns = if config.display.nickname.contains(&Metadata::Pronouns)
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
                    format!("{display_name} ({access_levels}{nickname}"),
                    if let Some(pronouns) = pronouns {
                        Some(format!("{pronouns})"))
                    } else {
                        Some(String::from(")"))
                    },
                )
            } else {
                (
                    format!("{access_levels}{nickname}"),
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
                    format!(
                        "{display_name} ({access_levels}{nickname}, {pronouns})",
                    )
                }
                (Some(display_name), None) => {
                    format!("{display_name} ({access_levels}{nickname})",)
                }
                (None, Some(pronouns)) => {
                    format!("{access_levels}{nickname} ({pronouns})",)
                }
                (None, None) => format!("{access_levels}{nickname}",),
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

        let font =
            theme::font_style::nickname(theme, is_offline).map(font::get);

        if self.bot_icon {
            row![
                selectable_text(self.left)
                    .style(move |_| style)
                    .font_maybe(font.clone()),
                widget::bot_icon(move |_| style),
                self.right.map(|right| selectable_text(right)
                    .style(move |_| style)
                    .font_maybe(font)),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(theme::ICON_SPACE)
            .into()
        } else {
            selectable_text(self.left)
                .style(move |_| style)
                .font_maybe(font)
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
