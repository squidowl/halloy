use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Ctcp {
    pub ping: bool,
    pub source: bool,
    pub time: bool,
    pub version: bool,
}

impl Default for Ctcp {
    fn default() -> Self {
        Self {
            ping: true,
            source: true,
            time: true,
            version: true,
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
