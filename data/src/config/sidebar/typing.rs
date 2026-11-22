use serde::{Deserialize, Deserializer};

use crate::config::animation::{
    AnimationEnabled, deserialize_typing_animation_enabled,
};
use crate::serde::deserialize_u32_positive_integer;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Typing {
    pub show: Option<TypingShow>,
    #[serde(deserialize_with = "deserialize_typing_position")]
    pub position: TypingPosition,
    #[serde(deserialize_with = "deserialize_u32_positive_integer")]
    pub icon_height: u32,
    pub animation: Animation,
}

impl Default for Typing {
    fn default() -> Self {
        Self {
            show: None,
            position: TypingPosition::default(),
            icon_height: 4,
            animation: Animation::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TypingShow {
    All,
    Channels,
    #[default]
    Queries,
    None,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TypingBuffer {
    Channel,
    Query,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TypingPosition {
    Left,
    LeftPriority(TypingIndicatorPriority),
    #[default]
    Right,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TypingLeftPosition {
    #[serde(deserialize_with = "deserialize_typing_left_position")]
    pub left: TypingLeftPositionOptions,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TypingLeftPositionOptions {
    #[default]
    TypingGutter,
    TypingPriority(TypingIndicatorPriority),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TypingLeftPriority {
    pub priority: TypingIndicatorPriority,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TypingIndicatorPriority {
    #[default]
    Highlight,
    None,
    Unread,
}

pub fn deserialize_typing_position<'de, D>(
    deserializer: D,
) -> Result<TypingPosition, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Inner {
        String(TypingPosition),
        TypingLeftPosition(TypingLeftPosition),
    }

    match Inner::deserialize(deserializer)? {
        Inner::String(string) => Ok(string),
        Inner::TypingLeftPosition(position) => match position.left {
            TypingLeftPositionOptions::TypingGutter => Ok(TypingPosition::Left),
            TypingLeftPositionOptions::TypingPriority(priority) => {
                Ok(TypingPosition::LeftPriority(priority))
            }
        },
    }
}

pub fn deserialize_typing_left_position<'de, D>(
    deserializer: D,
) -> Result<TypingLeftPositionOptions, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Inner {
        String(TypingLeftPositionOptions),
        TypingLeftPriority(TypingLeftPriority),
    }

    match Inner::deserialize(deserializer)? {
        Inner::String(string) => Ok(string),
        Inner::TypingLeftPriority(left) => {
            Ok(TypingLeftPositionOptions::TypingPriority(left.priority))
        }
    }
}

impl Typing {
    pub fn can_show(&self, is_horizontal: bool, buffer: TypingBuffer) -> bool {
        match &self.show {
            Some(show) => match show {
                TypingShow::All => true,
                _ => match buffer {
                    TypingBuffer::Channel => *show == TypingShow::Channels,
                    TypingBuffer::Query => *show == TypingShow::Queries,
                },
            },
            None => !is_horizontal && buffer == TypingBuffer::Query,
        }
    }

    pub fn is_position_right(&self) -> bool {
        matches!(self.position, TypingPosition::Right)
    }

    pub fn is_position_left(&self) -> bool {
        self.is_position_left_gutter() || self.is_position_left_priority()
    }

    pub fn is_position_left_gutter(&self) -> bool {
        matches!(self.position, TypingPosition::Left)
    }

    pub fn is_position_left_priority(&self) -> bool {
        matches!(self.position, TypingPosition::LeftPriority(..))
    }

    pub fn typing_indicator_priority(
        &self,
        show_highlight: bool,
        show_unread: bool,
    ) -> bool {
        let priority =
            if let TypingPosition::LeftPriority(priority) = self.position {
                priority
            } else {
                TypingIndicatorPriority::None
            };

        match priority {
            TypingIndicatorPriority::None => true,
            TypingIndicatorPriority::Highlight => !show_highlight,
            TypingIndicatorPriority::Unread => !show_unread && !show_highlight,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Animation {
    #[serde(deserialize_with = "deserialize_typing_animation_enabled")]
    pub enabled: AnimationEnabled,
}

impl Animation {
    pub fn is_disabled(&self) -> bool {
        matches!(self.enabled, AnimationEnabled::Never)
    }

    pub fn is_enabled(&self, is_focused_window: bool) -> bool {
        matches!(self.enabled, AnimationEnabled::Always)
            || (matches!(self.enabled, AnimationEnabled::Focused)
                && is_focused_window)
    }
}
