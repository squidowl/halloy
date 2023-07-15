use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, PartialOrd, Eq, Hash)]
pub enum Event {
    Connected,
    Reconnected,
    Disconnected,
    // TODO: Add more alert types.
    // Highlighted
    // ..
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub sound: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct List(HashMap<Event, Config>);

impl List {
    pub fn get(&self, event: Event) -> Option<&Config> {
        self.0.get(&event)
    }
}
