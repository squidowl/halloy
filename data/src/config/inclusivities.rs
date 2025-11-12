use irc::proto;
use serde::{Deserialize, Deserializer};

use crate::target::{Channel, Query, Target};
use crate::user::User;

pub fn is_target_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    target: &Target,
) -> bool {
    let is_inclusive =
        |inclusivities: Option<&Inclusivities>, target: &Target| -> bool {
            inclusivities.is_some_and(|inclusivities| match target {
                Target::Channel(channel) => {
                    inclusivities.is_channel_inclusive(channel)
                }
                Target::Query(query) => inclusivities.is_query_inclusive(query),
            })
        };

    let is_included = is_inclusive(include, target);
    let is_excluded = is_inclusive(exclude, target);

    is_included || !is_excluded
}

pub fn is_user_included(
    include: Option<&Inclusivities>,
    exclude: Option<&Inclusivities>,
    user: &User,
    channel: Option<&Channel>,
) -> bool {
    let is_inclusive = |inclusivities: Option<&Inclusivities>,
                        user: &User,
                        channel: Option<&Channel>|
     -> bool {
        inclusivities.is_some_and(|inclusivities| {
            inclusivities.is_user_inclusive(user)
                && channel.is_none_or(|channel| {
                    inclusivities.is_channel_inclusive(channel)
                })
        })
    };

    let is_included = is_inclusive(include, user, channel);
    let is_excluded = is_inclusive(exclude, user, channel);

    is_included || !is_excluded
}

#[derive(Debug, Clone)]
pub struct Inclusivities {
    pub users: Option<Inclusivity>,
    pub channels: Option<Inclusivity>,
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
            },
            Legacy(Vec<String>),
        }

        match Format::deserialize(deserializer)? {
            Format::Inclusivities { users, channels } => {
                Ok(Inclusivities { users, channels })
            }
            Format::Legacy(strings) => Ok(Inclusivities::parse(strings)),
        }
    }
}

impl Inclusivities {
    pub fn all() -> Self {
        Self {
            users: Some(Inclusivity::All),
            channels: Some(Inclusivity::All),
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
        }
    }

    pub fn is_channel_inclusive(&self, channel: &Channel) -> bool {
        self.channels
            .as_ref()
            .is_some_and(|inclusivity| match inclusivity {
                Inclusivity::All => true,
                Inclusivity::Any(inclusivity_channels) => {
                    inclusivity_channels.iter().any(|channel_inclusive| {
                        channel.as_str() == channel_inclusive.as_str()
                    })
                }
            })
    }

    pub fn is_query_inclusive(&self, query: &Query) -> bool {
        self.users
            .as_ref()
            .is_some_and(|inclusivity| match inclusivity {
                Inclusivity::All => true,
                Inclusivity::Any(inclusivity_users) => {
                    inclusivity_users.iter().any(|user_inclusive| {
                        query.as_str() == user_inclusive.as_str()
                    })
                }
            })
    }

    pub fn is_user_inclusive(&self, user: &User) -> bool {
        self.users
            .as_ref()
            .is_some_and(|inclusivity| match inclusivity {
                Inclusivity::All => true,
                Inclusivity::Any(inclusivity_users) => {
                    inclusivity_users.iter().any(|user_inclusive| {
                        user.as_str() == user_inclusive.as_str()
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
