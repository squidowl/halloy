use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Reroute {
    pub private_messages: Vec<RerouteRule>,
    pub private_notices: Vec<RerouteRule>,
}

impl Default for Reroute {
    fn default() -> Self {
        Self {
            private_messages: Vec::default(),
            private_notices: vec![RerouteRule {
                user: "*".to_string(),
                target: RerouteTarget::Server,
            }],
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RerouteRule {
    pub user: String,
    pub target: RerouteTarget,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RerouteTarget {
    Channel(String),
    Server,
}
