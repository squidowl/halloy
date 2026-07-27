//! Per-network settings for a bouncer.

use std::collections::HashMap;

use serde::Deserialize;

use super::{
    ConfirmMessageDelivery, Filters, Icon, Muteable, OptionalTyping, Reroute,
    Server, SidebarVisibility,
};
use crate::config::sidebar::OrderChannelsBy;
use crate::metadata;

/// Settings that can be overridden for a bouncer network.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(crate) struct Overrides {
    enabled: Option<bool>,
    filters: Option<Filters>,
    reroute: Option<Reroute>,
    channels: Option<Vec<Muteable>>,
    order_channels_by: Option<OrderChannelsBy>,
    queries: Option<Vec<Muteable>>,
    umodes: Option<String>,
    on_connect: Option<Vec<String>>,
    who_poll_enabled: Option<bool>,
    monitor: Option<Vec<String>>,
    #[serde(alias = "chathistory")]
    automated_chathistory: Option<bool>,
    confirm_message_delivery: Option<ConfirmMessageDelivery>,
    typing: Option<OptionalTyping>,
    metadata: Option<HashMap<metadata::Key, String>>,
    icon: Option<Icon>,
    sidebar_visibility: Option<SidebarVisibility>,
}

impl Overrides {
    pub(super) fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    pub(super) fn apply(&self, config: &mut Server) {
        if let Some(value) = &self.filters {
            config.filters = Some(value.clone());
        }
        if let Some(value) = &self.reroute {
            config.reroute.clone_from(value);
        }
        if let Some(value) = &self.channels {
            config.channels.clone_from(value);
        }
        if let Some(value) = self.order_channels_by {
            config.order_channels_by = Some(value);
        }
        if let Some(value) = &self.queries {
            config.queries.clone_from(value);
        }
        if let Some(value) = &self.umodes {
            config.umodes = Some(value.clone());
        }
        if let Some(value) = &self.on_connect {
            config.on_connect.clone_from(value);
        }
        if let Some(value) = self.who_poll_enabled {
            config.who_poll_enabled = value;
        }
        if let Some(value) = &self.monitor {
            config.monitor.clone_from(value);
        }
        if let Some(value) = self.automated_chathistory {
            config.automated_chathistory = value;
        }
        if let Some(value) = &self.confirm_message_delivery {
            config.confirm_message_delivery.clone_from(value);
        }
        if let Some(value) = &self.typing {
            config.typing.clone_from(value);
        }
        if let Some(value) = &self.metadata {
            config.metadata.clone_from(value);
        }
        if let Some(value) = &self.icon {
            config.icon.clone_from(value);
        }
        if let Some(value) = self.sidebar_visibility {
            config.sidebar_visibility = value;
        }
    }
}
