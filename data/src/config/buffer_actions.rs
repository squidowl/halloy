use serde::Deserialize;

use crate::dashboard::{BufferAction, BufferFocusedAction};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct BufferActions {
    #[serde(default)]
    pub click_buffer: BufferAction,
    #[serde(default)]
    pub click_focused_buffer: Option<BufferFocusedAction>,
    #[serde(default)]
    pub click_channel_name: BufferAction,
    #[serde(default)]
    pub click_highlight: BufferAction,
    #[serde(default)]
    pub click_user_name: BufferAction,
    #[serde(default)]
    pub local_buffer: BufferAction,
    #[serde(default)]
    pub message_channel: BufferAction,
    #[serde(default)]
    pub message_user: BufferAction,
}
