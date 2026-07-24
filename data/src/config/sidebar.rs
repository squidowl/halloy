use serde::{Deserialize, Deserializer};
use serde_untagged::UntaggedEnumVisitor;

use crate::config::Scrollbar;
use crate::config::buffer::ChannelNameCasing;
use crate::config::inclusivities::{
    Inclusivities, is_server_included, is_target_included,
};
use crate::isupport;
use crate::serde::deserialize_u32_positive_integer;
use crate::server::Server;
use crate::target::Target;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub max_width: Option<u16>,
    #[serde(deserialize_with = "deserialize_unread_indicator")]
    pub unread_indicator: UnreadIndicator,
    #[serde(deserialize_with = "deserialize_highlight_indicator")]
    pub highlight_indicator: HighlightIndicator,
    pub position: Position,
    pub order_by: OrderBy,
    pub scrollbar: Scrollbar,
    #[serde(alias = "font_size")]
    pub secondary_font_size: Option<u8>,
    #[serde(
        deserialize_with = "deserialize_primary_icon",
        alias = "server_icon",
        alias = "server_icon_size"
    )]
    pub primary_icon: PrimaryIcon,
    #[serde(alias = "server_font_size")]
    pub primary_font_size: Option<u8>,
    pub user_menu: UserMenu,
    pub collapse_button: CollapseButton,
    pub padding: Padding,
    pub spacing: Spacing,
    pub order_channels_by: OrderChannelsBy,
    pub channel_name_casing: Option<ChannelNameCasing>,
    pub internal_buffers: InternalBuffers,
    pub cycle_into_collapsed: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct InternalBuffers {
    #[serde(deserialize_with = "deserialize_muteable_internal_buffers")]
    pub mute: Vec<InternalBuffer>,
    pub position: InternalBufferPosition,
    pub buffers: Vec<InternalBuffer>,
}

impl InternalBuffers {
    pub fn is_before_servers(&self) -> bool {
        matches!(self.position, InternalBufferPosition::BeforeServers)
    }
}

pub fn deserialize_muteable_internal_buffers<'de, D>(
    deserializer: D,
) -> Result<Vec<InternalBuffer>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum MuteableInternalBuffer {
        Highlights,
        Logs,
    }

    Ok(Vec::<MuteableInternalBuffer>::deserialize(deserializer)?
        .into_iter()
        .map(|muteable_internal_buffer| match muteable_internal_buffer {
            MuteableInternalBuffer::Highlights => InternalBuffer::Highlights,
            MuteableInternalBuffer::Logs => InternalBuffer::Logs,
        })
        .collect())
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum InternalBufferPosition {
    BeforeServers,
    #[default]
    AfterServers,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrimaryIcon {
    Size(u32),
    Hidden,
}

impl Default for PrimaryIcon {
    fn default() -> Self {
        Self::Size(20)
    }
}

#[allow(clippy::redundant_closure_for_method_calls)]
pub fn deserialize_primary_icon<'de, D>(
    deserializer: D,
) -> Result<PrimaryIcon, D::Error>
where
    D: Deserializer<'de>,
{
    UntaggedEnumVisitor::new()
        .u32(|value| {
            if value > 0 {
                Ok(PrimaryIcon::Size(value))
            } else {
                Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Unsigned(u64::from(value)),
                    &"a positive integer",
                ))
            }
        })
        .string(|string| match string {
            "hidden" | "none" => Ok(PrimaryIcon::Hidden),
            _ => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(string),
                &"\"hidden\" or a size (positive integer)",
            )),
        })
        .bool(|value| match value {
            true => Ok(PrimaryIcon::Size(12)),
            false => Ok(PrimaryIcon::Hidden),
        })
        .map(|map| map.deserialize())
        .deserialize(deserializer)
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Padding {
    pub buffer: [u16; 2],
}

impl Default for Padding {
    fn default() -> Self {
        Self { buffer: [5, 4] }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Spacing {
    pub server: u32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self { server: 2 }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct UserMenu {
    pub enabled: bool,
}

impl Default for UserMenu {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct CollapseButton {
    pub enabled: bool,
}

impl Default for CollapseButton {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UnreadIndicator {
    pub title: bool,
    pub icon: Icon,
    #[serde(deserialize_with = "deserialize_u32_positive_integer")]
    pub icon_size: u32,
    pub show_on_open_buffers: bool,
    pub query_as_highlight: bool,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
}

impl Default for UnreadIndicator {
    fn default() -> Self {
        UnreadIndicator {
            title: false,
            icon: Icon::Dot,
            icon_size: 6,
            show_on_open_buffers: true,
            query_as_highlight: false,
            exclude: None,
            include: None,
        }
    }
}

impl UnreadIndicator {
    pub fn has_icon(&self) -> bool {
        !matches!(self.icon, Icon::None)
    }

    pub fn should_indicate(
        &self,
        target: Option<&Target>,
        server: &Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        if let Some(target) = target {
            is_target_included(
                self.include.as_ref(),
                self.exclude.as_ref(),
                None,
                target.as_target_ref(),
                server,
                casemapping,
            )
        } else {
            is_server_included(
                self.include.as_ref(),
                self.exclude.as_ref(),
                server,
            )
        }
    }
}

pub fn deserialize_unread_indicator<'de, D>(
    deserializer: D,
) -> Result<UnreadIndicator, D::Error>
where
    D: Deserializer<'de>,
{
    #[allow(clippy::redundant_closure_for_method_calls)]
    UntaggedEnumVisitor::new()
        .string(|string| match string {
            "title" => Ok(UnreadIndicator {
                title: true,
                icon: Icon::None,
                ..UnreadIndicator::default()
            }),
            "none" => Ok(UnreadIndicator {
                title: false,
                icon: Icon::None,
                ..UnreadIndicator::default()
            }),
            "dot" => Ok(UnreadIndicator {
                title: false,
                icon: Icon::Dot,
                ..UnreadIndicator::default()
            }),
            _ => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(string),
                &"one of: \"dot\", \"title\", or \"none\"",
            )),
        })
        .map(|map| map.deserialize())
        .deserialize(deserializer)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HighlightIndicator {
    pub title: bool,
    pub icon: Icon,
    #[serde(deserialize_with = "deserialize_u32_positive_integer")]
    pub icon_size: u32,
    pub show_on_open_buffers: bool,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
}

impl Default for HighlightIndicator {
    fn default() -> Self {
        HighlightIndicator {
            title: false,
            icon: Icon::CircleEmpty,
            icon_size: 6,
            show_on_open_buffers: true,
            exclude: None,
            include: None,
        }
    }
}

impl HighlightIndicator {
    pub fn has_icon(&self) -> bool {
        !matches!(self.icon, Icon::None)
    }

    pub fn should_indicate(
        &self,
        target: Option<&Target>,
        server: &Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        if let Some(target) = target {
            is_target_included(
                self.include.as_ref(),
                self.exclude.as_ref(),
                None,
                target.as_target_ref(),
                server,
                casemapping,
            )
        } else {
            is_server_included(
                self.include.as_ref(),
                self.exclude.as_ref(),
                server,
            )
        }
    }
}

pub fn deserialize_highlight_indicator<'de, D>(
    deserializer: D,
) -> Result<HighlightIndicator, D::Error>
where
    D: Deserializer<'de>,
{
    #[allow(clippy::redundant_closure_for_method_calls)]
    UntaggedEnumVisitor::new()
        .string(|string| match string {
            "title" => Ok(HighlightIndicator {
                title: true,
                icon: Icon::None,
                ..HighlightIndicator::default()
            }),
            "none" => Ok(HighlightIndicator {
                title: false,
                icon: Icon::None,
                ..HighlightIndicator::default()
            }),
            "dot" => Ok(HighlightIndicator {
                title: false,
                icon: Icon::Dot,
                ..HighlightIndicator::default()
            }),
            _ => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(string),
                &"one of: \"dot\", \"title\", or \"none\"",
            )),
        })
        .map(|map| map.deserialize())
        .deserialize(deserializer)
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Icon {
    #[default]
    Dot,
    CircleEmpty,
    DotCircled,
    Certificate,
    Asterisk,
    Speaker,
    Lightbulb,
    Star,
    None,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

impl Position {
    pub fn is_horizontal(&self) -> bool {
        match self {
            Position::Left | Position::Right => false,
            Position::Top | Position::Bottom => true,
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum OrderBy {
    #[default]
    Alpha,
    Config,
}

#[derive(Debug, Copy, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OrderChannelsBy {
    #[default]
    Name,
    NameAndPrefix,
    Config,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InternalBuffer {
    ConfigEditor,
    FileTransfers,
    ChannelDiscovery,
    ChannelMonitor,
    Highlights,
    Logs,
}
