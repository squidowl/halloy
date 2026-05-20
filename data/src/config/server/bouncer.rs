use indexmap::IndexMap;
use serde::Deserialize;

use crate::bouncer::BouncerNetwork;
use crate::config::Server;
use crate::config::server::icon::Icon;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct BouncerConfig {
    pub networks: IndexMap<String, NetworkConfig>,
}

impl BouncerConfig {
    pub fn apply(
        &self,
        bouncer_network: &BouncerNetwork,
        server: &Server,
    ) -> Server {
        let mut server = server.clone();

        #[allow(clippy::collapsible_if)]
        if let Some(bouncer_network_config) =
            self.networks.get(&bouncer_network.name)
        {
            if let Some(icon) = &bouncer_network_config.icon {
                server.icon = icon.clone();
            }
        }

        server
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct NetworkConfig {
    pub icon: Option<Icon>,
}
