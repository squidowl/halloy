use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct Reroute {
    pub query: Vec<RerouteRule>,
    pub notice: Vec<RerouteRule>,
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
