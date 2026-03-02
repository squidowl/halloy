use serde::Deserialize;

use crate::{Server, isupport, target};

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PrivateMessages {
    pub reroute: Vec<RerouteRule>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RerouteRule {
    pub user: String,
    pub target: RerouteTarget,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RerouteTarget {
    Channel { channel: String },
    Server { server: String },
}

impl PrivateMessages {
    pub fn has_reroute_rule_for(&self, user: &str, channel: &str) -> bool {
        self.reroute.iter().any(|rule| match rule {
            RerouteRule {
                user: rule_user,
                target:
                    RerouteTarget::Channel {
                        channel: rule_channel,
                    },
            } => {
                rule_user.eq_ignore_ascii_case(user)
                    && rule_channel.eq_ignore_ascii_case(channel)
            }
            RerouteRule {
                target: RerouteTarget::Server { .. },
                ..
            } => false,
        })
    }

    pub fn has_server_reroute_rule_for(
        &self,
        user: &str,
        server: &Server,
    ) -> bool {
        self.reroute.iter().any(|rule| match rule {
            RerouteRule {
                user: rule_user,
                target:
                    RerouteTarget::Server {
                        server: rule_server,
                    },
            } => {
                rule_user.eq_ignore_ascii_case(user)
                    && rule_server.eq_ignore_ascii_case(&server.name)
            }
            RerouteRule {
                target: RerouteTarget::Channel { .. },
                ..
            } => false,
        })
    }

    pub fn has_reroute_rule_for_query(
        &self,
        query: &target::Query,
        server: &Server,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.target_for_query(query, server, chantypes, statusmsg, casemapping)
            .is_some()
    }

    pub fn target_for_query(
        &self,
        query: &target::Query,
        server: &Server,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Option<&RerouteTarget> {
        self.reroute.iter().find_map(|rule| {
            let user = &rule.user;

            let user_query =
                target::Query::parse(user, chantypes, statusmsg, casemapping)
                    .ok()?;

            if user_query.as_normalized_str() != query.as_normalized_str() {
                return None;
            }

            match &rule.target {
                RerouteTarget::Channel { channel } => target::Channel::parse(
                    channel,
                    chantypes,
                    statusmsg,
                    casemapping,
                )
                .ok()
                .map(|_| &rule.target),
                RerouteTarget::Server {
                    server: rule_server,
                } => rule_server
                    .eq_ignore_ascii_case(&server.name)
                    .then_some(&rule.target),
            }
        })
    }
}
