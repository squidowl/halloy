use irc::proto;
use serde::{Deserialize, Deserializer};

use crate::isupport;
use crate::server::Server;
use crate::target::{Channel, Query, Target};
use crate::user::User;

pub fn is_target_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    target: &Target,
    server: &Server,
    casemapping: isupport::CaseMap,
) -> bool {
    let is_inclusive = |inclusivities: Option<&Inclusivities>,
                        target: &Target,
                        server: &Server|
     -> bool {
        inclusivities.is_some_and(|inclusivities| match target {
                Target::Channel(channel) => {
                    inclusivities.is_channel_inclusive(channel, casemapping)
                }
                Target::Query(query) => {
                    inclusivities.is_query_inclusive(query, casemapping)
                }
            } || inclusivities.is_server_inclusive(server))
    };

    let is_included = is_inclusive(include, target, server);
    let is_excluded = is_inclusive(exclude, target, server);

    is_included || !is_excluded
}

pub fn is_user_channel_server_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    user: &User,
    channel: Option<&Channel>,
    server: &Server,
    casemapping: isupport::CaseMap,
) -> bool {
    let is_inclusive = |inclusivities: Option<&Inclusivities>,
                        user: &User,
                        channel: Option<&Channel>,
                        server: &Server|
     -> bool {
        inclusivities.is_some_and(|inclusivities| {
            inclusivities.is_user_inclusive(user, casemapping)
                || channel.is_none_or(|channel| {
                    inclusivities.is_channel_inclusive(channel, casemapping)
                })
                || inclusivities.is_server_inclusive(server)
        })
    };

    let is_included = is_inclusive(include, user, channel, server);
    let is_excluded = is_inclusive(exclude, user, channel, server);

    is_included || !is_excluded
}

#[derive(Debug, Clone)]
pub struct Inclusivities {
    pub users: Option<Inclusivity>,
    pub channels: Option<Inclusivity>,
    pub servers: Option<Inclusivity>,
}

impl<'de> Deserialize<'de> for Inclusivities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        pub enum Format {
            Inclusivities {
                #[serde(default)]
                users: Option<Inclusivity>,
                #[serde(default)]
                channels: Option<Inclusivity>,
                #[serde(default)]
                servers: Option<Inclusivity>,
            },
            Legacy(Vec<String>),
        }

        match Format::deserialize(deserializer)? {
            Format::Inclusivities {
                users,
                channels,
                servers,
            } => Ok(Inclusivities {
                users,
                channels,
                servers,
            }),
            Format::Legacy(strings) => Ok(Inclusivities::parse(strings)),
        }
    }
}

impl Inclusivities {
    pub fn all() -> Self {
        Self {
            users: Some(Inclusivity::All),
            channels: Some(Inclusivity::All),
            servers: Some(Inclusivity::All),
        }
    }

    pub fn parse(mut strings: Vec<String>) -> Self {
        let channels = strings
            .extract_if(.., |string| {
                proto::is_channel(string, proto::DEFAULT_CHANNEL_PREFIXES)
            })
            .collect::<Vec<_>>();
        let users = strings;

        Inclusivities {
            users: (!users.is_empty()).then_some(Inclusivity::Any(users)),
            channels: (!channels.is_empty())
                .then_some(Inclusivity::Any(channels)),
            servers: Some(Inclusivity::All),
        }
    }

    pub fn is_channel_inclusive(
        &self,
        channel: &Channel,
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.channels
            .as_ref()
            .is_some_and(|inclusivity| match inclusivity {
                Inclusivity::All => true,
                Inclusivity::Any(inclusivity_channels) => {
                    inclusivity_channels.iter().any(|inclusivity_channel| {
                        channel.as_normalized_str()
                            == casemapping
                                .normalize(inclusivity_channel)
                                .as_str()
                    })
                }
            })
    }

    pub fn is_query_inclusive(
        &self,
        query: &Query,
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.users
            .as_ref()
            .is_some_and(|inclusivity| match inclusivity {
                Inclusivity::All => true,
                Inclusivity::Any(inclusivity_users) => {
                    inclusivity_users.iter().any(|inclusivity_user| {
                        query.as_normalized_str()
                            == casemapping.normalize(inclusivity_user).as_str()
                    })
                }
            })
    }

    pub fn is_server_inclusive(&self, server: &Server) -> bool {
        self.servers
            .as_ref()
            .is_some_and(|inclusivity| match inclusivity {
                Inclusivity::All => true,
                Inclusivity::Any(inclusivity_servers) => {
                    inclusivity_servers.iter().any(|inclusivity_server| {
                        server.name.as_ref() == inclusivity_server.as_str()
                    })
                }
            })
    }

    pub fn is_user_inclusive(
        &self,
        user: &User,
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.users
            .as_ref()
            .is_some_and(|inclusivity| match inclusivity {
                Inclusivity::All => true,
                Inclusivity::Any(inclusivity_users) => {
                    inclusivity_users.iter().any(|inclusivity_user| {
                        user.as_normalized_str()
                            == casemapping.normalize(inclusivity_user).as_str()
                    })
                }
            })
    }
}

#[derive(Debug, Clone)]
pub enum Inclusivity {
    All,
    Any(Vec<String>),
}

impl<'de> Deserialize<'de> for Inclusivity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        pub enum Format {
            All(String),
            Any(Vec<String>),
        }

        match Format::deserialize(deserializer)? {
            Format::All(string) => {
                if string == "all" || string == "*" {
                    Ok(Inclusivity::All)
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(&string),
                        &"'all' or '*'",
                    ))
                }
            }
            Format::Any(strings) => Ok(Inclusivity::Any(strings)),
        }
    }
}
