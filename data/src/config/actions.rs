use serde::Deserialize;

use crate::dashboard::{BufferAction, BufferFocusedAction};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Actions {
    #[serde(default)]
    pub sidebar: Sidebar,
    #[serde(default)]
    pub buffer: Buffer,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Buffer {
    #[serde(default)]
    pub click_channel_name: BufferAction,
    #[serde(default)]
    pub click_highlight: BufferAction,
    #[serde(default)]
    pub click_username: BufferAction,
    #[serde(default)]
    pub local: BufferAction,
    #[serde(default)]
    pub message_channel: BufferAction,
    #[serde(default)]
    pub message_user: BufferAction,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Sidebar {
    #[serde(default)]
    pub buffer: BufferAction,
    #[serde(default)]
    pub focused_buffer: Option<BufferFocusedAction>,
}
