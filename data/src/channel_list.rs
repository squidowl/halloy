use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Default, Clone, Debug)]
pub struct Manager {
    pub channels: HashMap<String, (String, usize)>,
    pub last_updated: Option<DateTime<Utc>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            last_updated: None,
        }
    }

    pub fn items(&self) -> impl Iterator<Item = (&'_ String, &'_ String, &'_ usize)> {
        self.channels.iter().map(|(channel, (topic, user_count))| (channel, topic, user_count))
    }
}