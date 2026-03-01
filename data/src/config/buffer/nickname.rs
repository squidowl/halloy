use serde::Deserialize;

use crate::buffer::{Alignment, Brackets, Color};
use crate::config::buffer::{
    AccessLevelFormat, Away, HideConsecutive, NicknameClickAction,
};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Nickname {
    pub away: Away,
    pub offline: Offline,
    pub color: Color,
    pub brackets: Brackets,
    pub alignment: Alignment,
    pub show_access_levels: AccessLevelFormat,
    pub click: NicknameClickAction,
    pub shown_status: ShownStatus,
    pub truncate: Option<u16>,
    pub hide_consecutive: HideConsecutive,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Offline {
    #[default]
    Solid,
    None,
}

impl Offline {
    pub fn is_offline(&self, is_user_offline: bool) -> bool {
        is_user_offline && matches!(self, Offline::Solid)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShownStatus {
    #[default]
    Current,
    Historical,
}
