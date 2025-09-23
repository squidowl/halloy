use serde::{Deserialize, Deserializer};

use crate::dashboard::{BufferAction, BufferFocusedAction};

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Actions {
    pub sidebar: Sidebar,
    pub buffer: Buffer,
    pub nicklist: Nicklist,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Buffer {
    pub click_channel_name: TargetClickAction,
    pub click_highlight: TargetClickAction,
    #[serde(alias = "click_nickname")]
    pub click_username: NicknameClickAction,
    pub join_channel: BufferAction,
    #[serde(alias = "local")]
    pub open_internal: BufferAction,
    pub message_channel: BufferAction,
    pub message_user: BufferAction,
    pub list: BufferAction,
    pub search: BufferAction,
    pub click_search_result: TargetClickAction,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub buffer: BufferAction,
    pub channel: Option<BufferAction>,
    pub query: Option<BufferAction>,
    pub focused_buffer: Option<BufferFocusedAction>,
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
pub enum TargetClickAction {
    OpenChannel(BufferAction),
    Noop,
}

impl Default for TargetClickAction {
    fn default() -> Self {
        Self::OpenChannel(BufferAction::default())
    }
}

impl<'de> Deserialize<'de> for TargetClickAction {
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
                    Ok(TargetClickAction::OpenChannel(buffer_action))
                }
                ClickAction::Noop => Ok(TargetClickAction::Noop),
            },
            Action::BufferAction(buffer_action) => {
                Ok(TargetClickAction::OpenChannel(buffer_action))
            }
        }
    }
}
