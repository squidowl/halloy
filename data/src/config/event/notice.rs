use serde::Deserialize;

use crate::config::event::Route;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Notice {
    pub route_to: Option<Route>,
}
