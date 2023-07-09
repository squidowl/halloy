use serde::Deserialize;

use crate::dashboard::DefaultAction;

#[derive(Debug, Copy, Default, Clone, Deserialize)]
pub struct Dashboard {
    #[serde(default)]
    pub sidebar_default_action: DefaultAction,
}
