use serde::Deserialize;

use crate::dashboard::{BufferAction, BufferFocusedAction};

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Actions {
    pub sidebar: Sidebar,
    pub buffer: Buffer,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Buffer {
    pub click_channel_name: BufferAction,
    pub click_highlight: BufferAction,
    pub click_username: BufferAction,
    pub join_channel: Option<BufferAction>,
    pub list: BufferAction,
    pub local: BufferAction,
    pub message_channel: BufferAction,
    pub message_user: BufferAction,
    pub search: BufferAction,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub buffer: BufferAction,
    pub focused_buffer: Option<BufferFocusedAction>,
}
