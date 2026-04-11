use std::collections::HashMap;
use std::sync::Arc;

use crate::user::Nick;
use crate::{Server, client, config, isupport, message, server, target};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RerouteRules {
    pub direct_messages: HashMap<Server, Vec<RerouteRule>>,
    pub direct_notices: HashMap<Server, Vec<RerouteRule>>,
}

impl RerouteRules {
    pub fn from_server_map(
        servers: &server::Map,
        clients: &client::Map,
    ) -> Self {
        let direct_messages = servers
            .entries()
            .filter_map(|entry| {
                let chantypes =
                    clients.get_server_chantypes_or_default(&entry.server);
                let statusmsg =
                    clients.get_server_statusmsg_or_default(&entry.server);
                let casemapping =
                    clients.get_server_casemapping_or_default(&entry.server);

                let reroute_rules = parse_reroute_rules(
                    &entry.config.reroute.private_messages,
                    chantypes,
                    statusmsg,
                    casemapping,
                );

                (!reroute_rules.is_empty())
                    .then_some((entry.server.clone(), reroute_rules))
            })
            .collect();

        let direct_notices = servers
            .entries()
            .filter_map(|entry| {
                let chantypes =
                    clients.get_server_chantypes_or_default(&entry.server);
                let statusmsg =
                    clients.get_server_statusmsg_or_default(&entry.server);
                let casemapping =
                    clients.get_server_casemapping_or_default(&entry.server);

                let reroute_rules = parse_reroute_rules(
                    &entry.config.reroute.private_notices,
                    chantypes,
                    statusmsg,
                    casemapping,
                );

                (!reroute_rules.is_empty())
                    .then_some((entry.server.clone(), reroute_rules))
            })
            .collect();

        Self {
            direct_messages,
            direct_notices,
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
        let reroute_rules = parse_reroute_rules(
            &config.reroute.private_messages,
            chantypes,
            statusmsg,
            casemapping,
        );

        if reroute_rules.is_empty() {
            self.direct_messages.remove(server);
        } else {
            self.direct_messages.insert(server.clone(), reroute_rules);
        }

        let reroute_rules = parse_reroute_rules(
            &config.reroute.private_notices,
            chantypes,
            statusmsg,
            casemapping,
        );

        if reroute_rules.is_empty() {
            self.direct_notices.remove(server);
        } else {
            self.direct_notices.insert(server.clone(), reroute_rules);
        }
    }
}

fn parse_reroute_rules(
    config: &[config::server::RerouteRule],
    chantypes: &[char],
    statusmsg: &[char],
    casemapping: isupport::CaseMap,
) -> Vec<RerouteRule> {
    config
        .iter()
        .filter_map(|reroute_rule| {
            if reroute_rule.user == "*" {
                // Ignore catch-all "*" user until second pass, to
                // ensure it is found after any direct match.
                return None;
            }

            RerouteRule::try_from_config(
                reroute_rule,
                chantypes,
                statusmsg,
                casemapping,
            )
        })
        .chain(config.iter().filter_map(|reroute_rule| {
            if reroute_rule.user != "*" {
                return None;
            }

            RerouteRule::try_from_config(
                reroute_rule,
                chantypes,
                statusmsg,
                casemapping,
            )
        }))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RerouteRule {
    pub from: Nick,
    pub to: RerouteTarget,
}

impl RerouteRule {
    fn try_from_config(
        reroute_rule: &config::server::RerouteRule,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Option<Self> {
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
                .map(RerouteTarget::Channel)
            }
            config::server::RerouteTarget::Server => {
                Some(RerouteTarget::Server)
            }
        }
        .map(|target| RerouteRule {
            from: nick,
            to: target,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RerouteTarget {
    Server,
    Channel(target::Channel),
}

impl RerouteRules {
    pub fn has_reroute_rule_for_direct_message(
        &self,
        query: &target::Query,
        server: &Server,
    ) -> bool {
        has_reroute_rule_for_query(&self.direct_messages, query, server)
    }

    pub fn target_for_direct_message(
        &self,
        query: &target::Query,
        server: &Server,
        source: &message::Source,
    ) -> Option<message::Target> {
        target_for_query(&self.direct_messages, query, server, source)
    }

    pub fn has_reroute_rule_for_direct_notice(
        &self,
        query: &target::Query,
        server: &Server,
    ) -> bool {
        has_reroute_rule_for_query(&self.direct_notices, query, server)
    }

    pub fn target_for_direct_notice(
        &self,
        query: &target::Query,
        server: &Server,
        source: &message::Source,
    ) -> Option<message::Target> {
        target_for_query(&self.direct_notices, query, server, source)
    }
}

fn has_reroute_rule_for_query(
    reroute_rules: &HashMap<Server, Vec<RerouteRule>>,
    query: &target::Query,
    server: &Server,
) -> bool {
    reroute_rules.get(server).is_some_and(|reroute_rules| {
        reroute_rules.iter().any(|reroute_rule| {
            query.as_normalized_str() == reroute_rule.from.as_normalized_str()
        })
    })
}

fn target_for_query(
    reroute_rules: &HashMap<Server, Vec<RerouteRule>>,
    query: &target::Query,
    server: &Server,
    source: &message::Source,
) -> Option<message::Target> {
    reroute_rules.get(server).and_then(|reroute_rules| {
        reroute_rules.iter().find_map(|reroute_rule| {
            ((query.as_normalized_str()
                == reroute_rule.from.as_normalized_str())
                || reroute_rule.from.as_str() == "*")
                .then_some(match &reroute_rule.to {
                    RerouteTarget::Channel(channel) => {
                        message::Target::Channel {
                            channel: channel.clone(),
                            source: source.clone(),
                        }
                    }
                    RerouteTarget::Server => message::Target::Server {
                        source: source.clone(),
                    },
                })
        })
    })
}
