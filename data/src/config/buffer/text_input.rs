use std::fmt;

use bitflags::bitflags;
use serde::{Deserialize, de};

use crate::config::buffer::AccessLevelFormat;
use crate::message::formatting::encode::Encoding;
use crate::serde::deserialize_usize_positive_integer;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TextInput {
    pub visibility: Visibility,
    pub auto_format: AutoFormat,
    pub autocomplete: Autocomplete,
    pub nickname: Nickname,
    pub key_bindings: KeyBindings,
    pub kill_to_clipboard: bool,
    #[serde(deserialize_with = "deserialize_usize_positive_integer")]
    pub max_lines: usize,
    pub send_line_delay: u64,
    pub persist: bool,
    pub upload_long_paste: bool,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            visibility: Visibility::default(),
            auto_format: AutoFormat::default(),
            autocomplete: Autocomplete::default(),
            nickname: Nickname::default(),
            key_bindings: KeyBindings::default(),
            kill_to_clipboard: true,
            max_lines: 5,
            send_line_delay: 100,
            persist: true,
            upload_long_paste: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyBindings {
    Default,
    Emacs,
}

impl Default for KeyBindings {
    fn default() -> Self {
        if cfg!(target_os = "macos") {
            KeyBindings::Emacs
        } else {
            KeyBindings::Default
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Focused,
    #[default]
    Always,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Nickname {
    pub enabled: bool,
    #[serde(alias = "show_access_level")]
    pub show_access_levels: AccessLevelFormat,
}

impl Default for Nickname {
    fn default() -> Self {
        Self {
            enabled: true,
            show_access_levels: AccessLevelFormat::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutoFormat {
    #[default]
    Disabled,
    Markdown,
    MarkdownFeatures(MarkdownFeatures),
    Token,
    All,
    #[serde(skip)]
    ForceDisabled,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MarkdownFeatures: u8 {
        #[bitflags(flag_name = "bold")]
        const Bold = 1;
        #[bitflags(flag_name = "italics")]
        const Italics = 1 << 1;
        #[bitflags(flag_name = "strikethrough")]
        const Strikethrough = 1 << 2;
        #[bitflags(flag_name = "code")]
        const Code = 1 << 3;
        #[bitflags(flag_name = "spoiler")]
        const Spoiler = 1 << 4;
        #[bitflags(flag_name = "link")]
        const Link = 1 << 5;
    }
}

struct FeatureVisitor();

impl<'de> de::Visitor<'de> for FeatureVisitor {
    type Value = MarkdownFeatures;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Invalid markdown specifier")
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let mut features = MarkdownFeatures::empty();
        while let Some(s) = visitor.next_element()? {
            let Some(x) = MarkdownFeatures::from_name(s) else {
                return Err(de::Error::invalid_value(
                    de::Unexpected::Str(s),
                    &self,
                ));
            };
            features |= x;
        }
        Ok(features)
    }
}

impl<'de> Deserialize<'de> for MarkdownFeatures {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        d.deserialize_seq(FeatureVisitor())
    }
}

impl AutoFormat {
    pub fn encoding(&self) -> Option<Encoding> {
        match self {
            Self::Disabled | Self::ForceDisabled => None,
            Self::Markdown => Some(Encoding::MarkdownOnly),
            Self::MarkdownFeatures(features) => {
                Some(Encoding::MarkdownFeaturesOnly(*features))
            }
            Self::Token => Some(Encoding::TokenOnly),
            Self::All => Some(Encoding::All),
        }
    }

    pub fn encodes_markdown_features(&self, feature: MarkdownFeatures) -> bool {
        match self {
            Self::Token | Self::Disabled | Self::ForceDisabled => false,
            Self::Markdown => true,
            Self::MarkdownFeatures(features) => features.contains(feature),
            Self::All => true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Autocomplete {
    pub order_by: OrderBy,
    pub sort_direction: SortDirection,
    pub completion_suffixes: [String; 2],
    pub include_ignored: bool,
}

impl Default for Autocomplete {
    fn default() -> Self {
        Self {
            order_by: OrderBy::default(),
            sort_direction: SortDirection::default(),
            completion_suffixes: [": ".to_string(), " ".to_string()],
            include_ignored: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderBy {
    Alpha,
    #[default]
    Recent,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}
