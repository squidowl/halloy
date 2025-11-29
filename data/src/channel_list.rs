use std::collections::HashMap;

use crate::Server;

#[derive(Default, Clone, Debug)]
pub struct Manager {
    items: HashMap<Server, Vec<String>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct ChannelInformation {
    pub channel: String, // TODO: Convert to Channel maybe?
    pub topic: String,
    pub user_count: String,
}
