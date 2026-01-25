use irc::proto;
use serde::{Deserialize, Deserializer};

use crate::isupport;
use crate::message::Source;
use crate::server::Server;
use crate::target::{Channel, Query, Target};
use crate::user::NickRef;

// Skips inclusivity checks without a source, as those are expected to be
// performed elsewhere
pub fn is_source_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    source: &Source,
    channel: Option<&Channel>,
    server: Option<&Server>,
    casemapping: isupport::CaseMap,
) -> bool {
    let is_inclusive = |inclusivities: Option<&Inclusivities>,
                        source: &Source,
                        channel: Option<&Channel>,
                        server: Option<&Server>,
                        casemapping: isupport::CaseMap|
     -> bool {
        inclusivities.is_some_and(|inclusivities| {
            inclusivities.is_source_inclusive(source)
                || inclusivities.criteria.iter().any(|criterion| {
                    criterion.is_source_channel_server_inclusive(
                        Some(source),
                        channel,
                        server,
                        casemapping,
                    )
                })
        })
    };

    let is_included =
        is_inclusive(include, source, channel, server, casemapping);
    let is_excluded =
        is_inclusive(exclude, source, channel, server, casemapping);

    is_included || !is_excluded
}

pub fn is_target_channel_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    user: Option<NickRef>,
    channel: &Channel,
    server: &Server,
    casemapping: isupport::CaseMap,
) -> bool {
    let is_inclusive = |inclusivities: Option<&Inclusivities>,
                        channel: &Channel,
                        server: &Server|
     -> bool {
        inclusivities.is_some_and(|inclusivities| {
            inclusivities.is_channel_inclusive(channel, casemapping)
                || inclusivities.criteria.iter().any(|criterion| {
                    criterion.is_user_channel_server_inclusive(
                        user,
                        Some(channel),
                        Some(server),
                        casemapping,
                    )
                })
                || inclusivities.is_server_inclusive(server)
        })
    };

    let is_included = is_inclusive(include, channel, server);
    let is_excluded = is_inclusive(exclude, channel, server);

    is_included || !is_excluded
}

pub fn is_target_query_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    query: &Query,
    server: &Server,
    casemapping: isupport::CaseMap,
) -> bool {
    let is_inclusive = |inclusivities: Option<&Inclusivities>,
                        query: &Query,
                        server: &Server|
     -> bool {
        inclusivities.is_some_and(|inclusivities| {
            inclusivities.is_query_inclusive(query, casemapping)
                || inclusivities.criteria.iter().any(|criterion| {
                    criterion.is_query_server_inclusive(
                        query,
                        server,
                        casemapping,
                    )
                })
                || inclusivities.is_server_inclusive(server)
        })
    };

    let is_included = is_inclusive(include, query, server);
    let is_excluded = is_inclusive(exclude, query, server);

    is_included || !is_excluded
}

pub fn is_target_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    user: Option<NickRef>,
    target: &Target,
    server: &Server,
    casemapping: isupport::CaseMap,
) -> bool {
    match target {
        Target::Channel(channel) => is_target_channel_included(
            include,
            exclude,
            user,
            channel,
            server,
            casemapping,
        ),
        Target::Query(query) => is_target_query_included(
            include,
            exclude,
            query,
            server,
            casemapping,
        ),
    }
}

pub fn is_user_channel_server_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    user: NickRef,
    channel: Option<&Channel>,
    server: &Server,
    casemapping: isupport::CaseMap,
) -> bool {
    let is_inclusive = |inclusivities: Option<&Inclusivities>,
                        user: NickRef,
                        channel: Option<&Channel>,
                        server: &Server|
     -> bool {
        inclusivities.is_some_and(|inclusivities| {
            inclusivities.is_user_inclusive(user, casemapping)
                || channel.is_none_or(|channel| {
                    inclusivities.is_channel_inclusive(channel, casemapping)
                })
                || inclusivities.is_server_inclusive(server)
                || inclusivities.criteria.iter().any(|criterion| {
                    criterion.is_user_channel_server_inclusive(
                        Some(user),
                        channel,
                        Some(server),
                        casemapping,
                    )
                })
        })
    };

    let is_included = is_inclusive(include, user, channel, server);
    let is_excluded = is_inclusive(exclude, user, channel, server);

    is_included || !is_excluded
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inclusivities {
    pub users: Option<Inclusivity>,
    pub channels: Option<Inclusivity>,
    pub servers: Option<Inclusivity>,
    pub server_messages: Option<Inclusivity>,
    pub criteria: Vec<Criterion>,
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
                #[serde(default)]
                server_messages: Option<Inclusivity>,
                #[serde(default)]
                criteria: Vec<Criterion>,
            },
            Legacy(Vec<String>),
            All(String),
        }

        match Format::deserialize(deserializer)? {
            Format::Inclusivities {
                users,
                channels,
                servers,
                server_messages,
                criteria,
            } => Ok(Inclusivities {
                users,
                channels,
                servers,
                server_messages,
                criteria,
            }),
            Format::Legacy(strings) => Ok(Inclusivities::parse(strings)),
            Format::All(string) => {
                if string == "all" || string == "*" {
                    Ok(Inclusivities::all())
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(&string),
                        &"'all' or '*'",
                    ))
                }
            }
        }
    }
}

impl Inclusivities {
    pub fn all() -> Self {
        Self {
            users: Some(Inclusivity::All),
            channels: Some(Inclusivity::All),
            servers: Some(Inclusivity::All),
            server_messages: Some(Inclusivity::All),
            // Does not need to be set, since any criterion will be covered by
            // the Inclusivity fields
            criteria: Vec::default(),
        }
    }

    pub fn parse(mut strings: Vec<String>) -> Self {
        if strings
            .iter()
            .any(|string| string == "all" || string == "*")
        {
            return Inclusivities::all();
        }

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
            servers: None,
            server_messages: None,
            criteria: Vec::default(),
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

    pub fn is_source_inclusive(&self, source: &Source) -> bool {
        if let Source::Server(server) = &source {
            self.server_messages.as_ref().is_some_and(|inclusivity| {
                match inclusivity {
                    Inclusivity::All => true,
                    Inclusivity::Any(inclusivity_server_messages) => {
                        server.as_ref().is_some_and(|server| {
                            let kind = server.kind().to_string();

                            inclusivity_server_messages.iter().any(
                                |inclusivity_server_message| {
                                    kind == *inclusivity_server_message
                                },
                            )
                        })
                    }
                }
            })
        } else {
            false
        }
    }

    pub fn is_user_inclusive(
        &self,
        user: NickRef,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Criterion {
    user: Option<String>,
    channel: Option<String>,
    server: Option<String>,
    server_message: Option<String>,
}

impl Criterion {
    pub fn is_source_channel_server_inclusive(
        &self,
        source: Option<&Source>,
        channel: Option<&Channel>,
        server: Option<&Server>,
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.server_message
            .as_ref()
            .is_none_or(|server_message_criterion| {
                if let Some(Source::Server(Some(server))) = source {
                    server.kind().to_string() == *server_message_criterion
                } else {
                    false
                }
            })
            && self.channel.as_ref().is_none_or(|channel_criterion| {
                channel.is_some_and(|channel| {
                    channel.as_normalized_str()
                        == casemapping.normalize(channel_criterion).as_str()
                })
            })
            && self.server.as_ref().is_none_or(|server_criterion| {
                server.is_some_and(|server| {
                    server.name.as_ref() == server_criterion
                })
            })
            && self.user.is_none()
    }

    pub fn is_user_channel_server_inclusive(
        &self,
        user: Option<NickRef>,
        channel: Option<&Channel>,
        server: Option<&Server>,
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.user.as_ref().is_none_or(|user_criterion| {
            user.is_some_and(|user| {
                user.as_normalized_str()
                    == casemapping.normalize(user_criterion).as_str()
            })
        }) && self.channel.as_ref().is_none_or(|channel_criterion| {
            channel.is_some_and(|channel| {
                channel.as_normalized_str()
                    == casemapping.normalize(channel_criterion).as_str()
            })
        }) && self.server.as_ref().is_none_or(|server_criterion| {
            server
                .is_some_and(|server| server.name.as_ref() == server_criterion)
        }) && self.server_message.is_none()
    }

    pub fn is_query_server_inclusive(
        &self,
        query: &Query,
        server: &Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.user.as_ref().is_none_or(|user_criterion| {
            query.as_normalized_str()
                == casemapping.normalize(user_criterion).as_str()
        }) && self.channel.is_none()
            && self.server.as_ref().is_none_or(|server_criterion| {
                server.name.as_ref() == server_criterion
            })
            && self.server_message.is_none()
    }
}
