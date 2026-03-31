use serde::Deserialize;

use crate::{isupport, target};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct Reroute {
    pub private_messages: PrivateMessages,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivateMessages {
    pub reroute: Vec<RerouteRule>,
}

impl<'de> Deserialize<'de> for PrivateMessages {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Data {
            Rules(Vec<RerouteRule>),
            Legacy {
                #[serde(default)]
                reroute: Vec<RerouteRule>,
            },
        }

        Ok(match Data::deserialize(deserializer)? {
            Data::Rules(reroute) => Self { reroute },
            Data::Legacy { reroute } => Self { reroute },
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RerouteRule {
    pub user: String,
    pub target: RerouteTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RerouteTarget {
    Channel(String),
    Server(Option<String>),
}

impl<'de> Deserialize<'de> for RerouteTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Data {
            Channel { channel: String },
            Server { server: String },
            Unnamed(String),
        }

        match Data::deserialize(deserializer)? {
            Data::Channel { channel } => Ok(Self::Channel(channel)),
            Data::Server { server } => Ok(Self::Server(Some(server))),
            Data::Unnamed(target) => {
                if target == "server" {
                    Ok(Self::Server(None))
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(&target),
                        &"unnamed target can only be \"server\"",
                    ))
                }
            }
        }
    }
}

impl PrivateMessages {
    pub fn has_reroute_rule_for_query(
        &self,
        query: &target::Query,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.target_for_query(query, chantypes, statusmsg, casemapping)
            .is_some()
    }

    pub fn target_for_query(
        &self,
        query: &target::Query,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Option<&RerouteTarget> {
        self.reroute.iter().find_map(|rule| {
            if !query_matches_user(
                query,
                &rule.user,
                chantypes,
                statusmsg,
                casemapping,
            ) {
                return None;
            }

            match &rule.target {
                RerouteTarget::Channel(channel) => target::Channel::parse(
                    channel,
                    chantypes,
                    statusmsg,
                    casemapping,
                )
                .ok()
                .map(|_| &rule.target),
                RerouteTarget::Server(_) => Some(&rule.target),
            }
        })
    }
}

fn query_matches_user(
    query: &target::Query,
    user: &str,
    chantypes: &[char],
    statusmsg: &[char],
    casemapping: isupport::CaseMap,
) -> bool {
    target::Query::parse(user, chantypes, statusmsg, casemapping)
        .ok()
        .is_some_and(|user_query| {
            user_query.as_normalized_str() == query.as_normalized_str()
        })
}
