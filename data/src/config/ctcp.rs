use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Ctcp {
    pub ping: bool,
    pub source: bool,
    pub time: bool,
    pub version: bool,
    pub userinfo: Option<String>,
}

impl Default for Ctcp {
    fn default() -> Self {
        Self {
            ping: true,
            source: true,
            time: true,
            version: true,
            userinfo: Option::default(),
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

        if self.userinfo.is_some() {
            commands.push("USERINFO");
        }

        commands.join(" ")
    }
}
