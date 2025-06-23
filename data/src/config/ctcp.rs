use serde::Deserialize;

use crate::serde::default_bool_true;

#[derive(Debug, Clone, Deserialize)]
pub struct Ctcp {
    #[serde(default = "default_bool_true")]
    pub ping: bool,
    #[serde(default = "default_bool_true")]
    pub source: bool,
    #[serde(default = "default_bool_true")]
    pub time: bool,
    #[serde(default = "default_bool_true")]
    pub version: bool,
}

impl Default for Ctcp {
    fn default() -> Self {
        Self {
            ping: default_bool_true(),
            source: default_bool_true(),
            time: default_bool_true(),
            version: default_bool_true(),
        }
    }
}

impl Ctcp {
    pub fn client_info(&self) -> String {
        let mut commands = vec!["ACTION", "CLIENTINFO", "DCC"];

        if self.ping {
            commands.push("PING");
        }

        if self.source {
            commands.push("SOURCE");
        }

        if self.time {
            commands.push("TIME");
        }

        if self.version {
            commands.push("VERSION");
        }

        commands.join(" ")
    }
}
