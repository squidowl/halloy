use std::collections::HashMap;

use crate::Server;

#[derive(Default, Clone, Debug)]
pub struct Manager {
    items: HashMap<Server, Vec<ChannelInformation>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn push_for_server(
        &mut self,
        server: Server,
        channel_information: ChannelInformation,
    ) {
        self.items
            .entry(server)
            .or_default()
            .push(channel_information);
    }

    pub fn get_for_server(&self, server: &Server) -> &[ChannelInformation] {
        self.items.get(server).map_or(&[], |items| items.as_slice())
    }

    pub fn clear(&mut self, server: &Server) {
        self.items.remove(server);
    }
}

#[derive(Default, Clone, Debug)]
pub struct ChannelInformation {
    pub channel: String, // TODO: Convert to Channel maybe?
    pub topic: String,
    pub user_count: String,
}
