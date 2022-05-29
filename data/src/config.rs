use std::ops::Deref;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config(irc::client::data::Config);

impl Deref for Config {
    type Target = irc::client::data::Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
