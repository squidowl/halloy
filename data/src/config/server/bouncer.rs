use indexmap::IndexMap;
use serde::Deserialize;

use crate::bouncer::BouncerNetwork;
use crate::config::Server;
use crate::config::server::icon::Icon;
use crate::config::sidebar::OrderChannelsBy;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default, transparent)]
pub struct BouncerConfig {
    pub networks: IndexMap<String, NetworkConfig>,
}

impl BouncerConfig {
    pub fn overlay(
        &self,
        bouncer_network: &BouncerNetwork,
        server: &Server,
    ) -> Server {
        let mut server = server.clone();

        if let Some(bouncer_network_config) =
            self.networks.get(&bouncer_network.name)
        {
            if let Some(icon) = &bouncer_network_config.icon {
                server.icon = icon.clone();
            }

            if let Some(channels) = &bouncer_network_config.channels {
                server.channels = channels.clone();
            }

            if let Some(order_channels_by) =
                bouncer_network_config.order_channels_by
            {
                server.order_channels_by = Some(order_channels_by);
            }
        }

        server
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct NetworkConfig {
    pub icon: Option<Icon>,
    pub channels: Option<Vec<String>>,
    pub order_channels_by: Option<OrderChannelsBy>,
}
