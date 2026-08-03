use serde::{Deserialize, Deserializer};

use crate::dashboard::{BufferAction, BufferFocusedAction};

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Actions {
    pub sidebar: Sidebar,
    pub buffer: Buffer,
    pub nicklist: Nicklist,
    pub notification: Notification,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Buffer {
    pub click_channel_name: ChannelClickAction,
    pub click_highlight: ChannelClickAction,
    pub click_channel_discovery: ChannelClickAction,
    #[serde(alias = "click_nickname")]
    pub click_username: NicknameClickAction,
    pub join_channel: BufferAction,
    #[serde(alias = "local")]
    pub open_internal: BufferAction,
    pub message_channel: BufferAction,
    pub message_user: BufferAction,
    pub only_contract_expanded_message: bool,
    pub click_image_url: ImageClickAction,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            click_channel_name: ChannelClickAction::default(),
            click_highlight: ChannelClickAction::default(),
            click_channel_discovery: ChannelClickAction::default(),
            click_username: NicknameClickAction::default(),
            join_channel: BufferAction::default(),
            open_internal: BufferAction::default(),
            message_channel: BufferAction::default(),
            message_user: BufferAction::default(),
            only_contract_expanded_message: true,
            click_image_url: ImageClickAction::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub buffer: BufferAction,
    pub channel: Option<BufferAction>,
    pub query: Option<BufferAction>,
    pub focused_buffer: Option<BufferFocusedAction>,
    pub cycle: CycleAction,
    #[serde(default = "default_buffer_with_modifier")]
    pub buffer_with_modifier: BufferAction,
    pub channel_with_modifier: Option<BufferAction>,
    pub query_with_modifier: Option<BufferAction>,
    pub focused_buffer_with_modifier: Option<BufferFocusedAction>,
}

fn default_buffer_with_modifier() -> BufferAction {
    BufferAction::NewWindow
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Nicklist {
    #[serde(alias = "click_nickname")]
    pub click_username: Option<NicknameClickAction>,
}

#[derive(Debug, Copy, Clone)]
pub enum NicknameClickAction {
    OpenQuery(BufferAction),
    InsertNickname,
    Noop,
}

impl Default for NicknameClickAction {
    fn default() -> Self {
        Self::OpenQuery(BufferAction::default())
    }
}

impl<'de> Deserialize<'de> for NicknameClickAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum ClickAction {
            OpenQuery(BufferAction),
            InsertNickname,
            #[serde(alias = "no-action")]
            Noop,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Action {
            ClickAction(ClickAction),
            BufferAction(BufferAction),
        }

        match Action::deserialize(deserializer)? {
            Action::ClickAction(click_action) => match click_action {
                ClickAction::OpenQuery(buffer_action) => {
                    Ok(NicknameClickAction::OpenQuery(buffer_action))
                }
                ClickAction::InsertNickname => {
                    Ok(NicknameClickAction::InsertNickname)
                }
                ClickAction::Noop => Ok(NicknameClickAction::Noop),
            },
            Action::BufferAction(buffer_action) => {
                Ok(NicknameClickAction::OpenQuery(buffer_action))
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ChannelClickAction {
    OpenChannel(BufferAction),
    Noop,
}

impl Default for ChannelClickAction {
    fn default() -> Self {
        Self::OpenChannel(BufferAction::default())
    }
}

impl<'de> Deserialize<'de> for ChannelClickAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum ClickAction {
            OpenChannel(BufferAction),
            #[serde(alias = "no-action")]
            Noop,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Action {
            ClickAction(ClickAction),
            BufferAction(BufferAction),
        }

        match Action::deserialize(deserializer)? {
            Action::ClickAction(click_action) => match click_action {
                ClickAction::OpenChannel(buffer_action) => {
                    Ok(ChannelClickAction::OpenChannel(buffer_action))
                }
                ClickAction::Noop => Ok(ChannelClickAction::Noop),
            },
            Action::BufferAction(buffer_action) => {
                Ok(ChannelClickAction::OpenChannel(buffer_action))
            }
        }
    }
}

impl ChannelClickAction {
    pub fn buffer_action(&self) -> Option<BufferAction> {
        match self {
            ChannelClickAction::OpenChannel(buffer_action) => {
                Some(*buffer_action)
            }
            ChannelClickAction::Noop => None,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Notification {
    pub default: NotificationAction,
    pub open_buffer: BufferAction,
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationAction {
    OpenBuffer,
    #[default]
    ActivateApplication,
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageClickAction {
    #[default]
    OpenUrl,
    Preview,
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CycleAction {
    #[default]
    IntoCollapsed,
    SkipCollapsed,
}

impl CycleAction {
    pub fn include_collapsed(&self) -> bool {
        matches!(self, Self::IntoCollapsed)
    }
}
