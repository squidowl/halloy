use std::collections::HashMap;
use std::sync::Arc;

use crate::user::Nick;
use crate::{
    Server, client, config, history, isupport, message, server, target,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RerouteRules {
    pub direct_messages: HashMap<Server, Vec<DirectMessageRerouteRule>>,
}

impl RerouteRules {
    pub fn from_server_map(
        servers: &server::Map,
        clients: &client::Map,
    ) -> Self {
        Self {
            direct_messages: servers
                .entries()
                .filter_map(|entry| {
                    let chantypes =
                        clients.get_server_chantypes_or_default(&entry.server);
                    let statusmsg =
                        clients.get_server_statusmsg_or_default(&entry.server);
                    let casemapping = clients
                        .get_server_casemapping_or_default(&entry.server);

                    let reroute_rules = parse_reroute_rules(
                        entry.config,
                        chantypes,
                        statusmsg,
                        casemapping,
                    );

                    (!reroute_rules.is_empty())
                        .then_some((entry.server.clone(), reroute_rules))
                })
                .collect(),
        }
    }

    pub fn sync_isupport(
        &mut self,
        server: &Server,
        config: Arc<config::Server>,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) {
        let reroute_rules =
            parse_reroute_rules(config, chantypes, statusmsg, casemapping);

        if reroute_rules.is_empty() {
            self.direct_messages.remove(server);
        } else {
            self.direct_messages.insert(server.clone(), reroute_rules);
        }
    }
}

fn parse_reroute_rules(
    config: Arc<config::Server>,
    chantypes: &[char],
    statusmsg: &[char],
    casemapping: isupport::CaseMap,
) -> Vec<DirectMessageRerouteRule> {
    config
        .reroute
        .private_messages
        .reroute
        .iter()
        .filter_map(|reroute_rule| {
            let nick = Nick::from_str(&reroute_rule.user, casemapping);

            match &reroute_rule.target {
                config::server::RerouteTarget::Channel(config_channel) => {
                    target::Channel::parse(
                        config_channel,
                        chantypes,
                        statusmsg,
                        casemapping,
                    )
                    .ok()
                    .map(DirectMessageRerouteTarget::Channel)
                }
                config::server::RerouteTarget::Server => {
                    Some(DirectMessageRerouteTarget::Server)
                }
            }
            .map(|target| DirectMessageRerouteRule {
                from: nick,
                to: target,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectMessageRerouteRule {
    pub from: Nick,
    pub to: DirectMessageRerouteTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectMessageRerouteTarget {
    Server,
    Channel(target::Channel),
}

impl RerouteRules {
    pub fn has_reroute_rule_for_query(
        &self,
        query: &target::Query,
        server: &Server,
    ) -> bool {
        self.direct_messages
            .get(server)
            .is_some_and(|reroute_rules| {
                reroute_rules.iter().any(|reroute_rule| {
                    query.as_normalized_str()
                        == reroute_rule.from.as_normalized_str()
                })
            })
    }

    pub fn message_target_for_query(
        &self,
        query: &target::Query,
        server: &Server,
        source: &message::Source,
    ) -> Option<message::Target> {
        self.direct_messages.get(server).and_then(|reroute_rules| {
            reroute_rules.iter().find_map(|reroute_rule| {
                (query.as_normalized_str()
                    == reroute_rule.from.as_normalized_str())
                .then_some(match &reroute_rule.to {
                    DirectMessageRerouteTarget::Channel(channel) => {
                        message::Target::Channel {
                            channel: channel.clone(),
                            source: source.clone(),
                        }
                    }
                    DirectMessageRerouteTarget::Server => {
                        message::Target::Server {
                            source: source.clone(),
                        }
                    }
                })
            })
        })
    }

    pub fn history_kind_for_query(
        &self,
        query: &target::Query,
        server: &Server,
    ) -> Option<history::Kind> {
        self.direct_messages.get(server).and_then(|reroute_rules| {
            reroute_rules.iter().find_map(|reroute_rule| {
                (query.as_normalized_str()
                    == reroute_rule.from.as_normalized_str())
                .then_some(match &reroute_rule.to {
                    DirectMessageRerouteTarget::Channel(channel) => {
                        history::Kind::Channel(server.clone(), channel.clone())
                    }
                    DirectMessageRerouteTarget::Server => {
                        history::Kind::Server(server.clone())
                    }
                })
            })
        })
    }
}
