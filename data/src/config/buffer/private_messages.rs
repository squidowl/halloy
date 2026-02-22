use serde::Deserialize;

use crate::{isupport, target};

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PrivateMessages {
    pub reroute: Vec<RerouteRule>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RerouteRule {
    Channel { user: String, channel: String },
}

impl PrivateMessages {
    pub fn has_rule_for(&self, user: &str, channel: &str) -> bool {
        self.reroute.iter().any(|rule| match rule {
            RerouteRule::Channel {
                user: rule_user,
                channel: rule_channel,
            } => {
                rule_user.eq_ignore_ascii_case(user)
                    && rule_channel.eq_ignore_ascii_case(channel)
            }
        })
    }

    pub fn channel_for_query(
        &self,
        query: &target::Query,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Option<target::Channel> {
        self.reroute.iter().find_map(|rule| {
            let (user, channel) = match rule {
                RerouteRule::Channel { user, channel } => (user, channel),
            };

            let target =
                target::Query::parse(user, chantypes, statusmsg, casemapping)
                    .ok()?;

            if target.as_normalized_str() != query.as_normalized_str() {
                return None;
            }

            target::Channel::parse(channel, chantypes, statusmsg, casemapping)
                .ok()
        })
    }
}
