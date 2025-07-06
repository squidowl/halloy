use regex::Regex;
use thiserror::Error;

use crate::{
    Message, Server, Target, User, isupport,
    message::{Content, Source},
    target::{Channel, Query},
    user::TryFromUserError,
};

use super::Kind;

#[derive(Debug, Clone)]
pub struct Filter {
    pub target: FilterTarget,
    pub class: FilterClass,
    // pub content: Option,
}

#[derive(Debug, Clone)]
pub enum FilterClass {
    Server(Server),
    Channel((Server, Channel)),
    Any,
}

#[derive(Debug, Clone)]
pub enum FilterTarget {
    User((Source, Source)),
    // Regex(Regex),
    Any,
}

impl FilterTarget {
    pub fn user_try_from_str(nick: &str) -> Result<Self, FilterError> {
        User::try_from(nick)
            .map(|u| {
                Self::User((
                    Source::User(u.clone()),
                    Source::Action(Some(u.clone())),
                ))
            })
            .map_err(|e| FilterError::TryFromUserError(e))
    }
}

#[derive(Error, Debug)]
pub enum FilterError {
    #[error("Unable to generate filter from nickname {0:?}")]
    TryFromUserError(TryFromUserError),
    // #[error("Unable to generate filter from regex \"{0}\", message: {1:?}")]
    // RegexError(String, regex::Error),
}

impl Filter {
    pub fn try_from_str_with_server(
        server: &Server,
        value: &str,
    ) -> Result<Self, FilterError> {
        let channel_test = Regex::new("^[#&][^\x07, ]{1,49}").expect("Failed to compile static regex, if you see this, something has gone wrong.");
        let (class, target) = match channel_test.find(value) {
            Some(channel_match) => {
                let offset = channel_match.end();
                let channel = Channel::from_str(
                    channel_match.as_str(),
                    isupport::CaseMap::default(),
                );

                let target = match value.get(offset..) {
                    Some(nick) => FilterTarget::user_try_from_str(nick)?,
                    None => FilterTarget::Any,
                };

                (FilterClass::Channel((server.clone(), channel)), target)
            }
            None => (FilterClass::Any, FilterTarget::user_try_from_str(value)?),
        };

        Ok(Self { class, target })
    }

    /// Tests a [`Message`] against the filter's predicate.
    ///
    /// This function returns `true` when the message matches predicate, false
    /// otherwise.
    ///
    /// [`Message`]:crate::Message
    pub fn match_message(&self, message: &Message) -> bool {
        match &self.target {
            FilterTarget::User((
                Source::User(user),
                Source::Action(action),
            )) => match &message.content {
                Content::Fragments(fragments) => {
                    fragments.iter().any(|frag| match frag {
                        crate::message::Fragment::User(u, _) => u == user,
                        _ => false,
                    })
                }
                Content::Plain(_) => match &message.target.source() {
                    Source::Action(msg_action) => msg_action == action,
                    Source::User(msg_user) => msg_user == user,
                    _ => false,
                },
                _ => false,
            },
            // FilterTarget::Regex(re) => re.is_match(&message.text()),
            FilterTarget::Any => true,
            _ => false,
        }
    }

    /// Tests a [`Target`] against the filter's predicate.
    ///
    /// This function returns `true` when the target matches predicate, false
    /// otherwise.
    ///
    /// [`Target`]:crate::Target
    pub fn match_target(&self, target: &Target) -> bool {
        match &self.target {
            FilterTarget::User((Source::User(user), _)) => {
                if let Target::Query(q) = target {
                    q.as_str() == user.as_str()
                } else {
                    false
                }
            }
            // FilterTarget::Regex(re) => re.is_match(&target.as_str()),
            FilterTarget::Any => true,
            _ => false,
        }
    }

    /// Tests a [`Query`] against the filter's predicate.
    ///
    /// This function returns `true` when the query matches predicate, false
    /// otherwise.
    ///
    /// [`Query`]:crate::Query
    pub fn match_query(&self, query: &Query) -> bool {
        match &self.target {
            FilterTarget::User((Source::User(user), ..)) => {
                user.as_str() == query.as_str()
            }
            // FilterTarget::Any => true, // this could hav
            _ => false,
        }
    }

    pub fn match_kind(&self, kind: &Kind) -> bool {
        match &self.class {
            FilterClass::Server(target_server) => match kind {
                Kind::Channel(server, _ch) => target_server == server,
                // history::Kind::Server narrows to server messages only
                Kind::Server(server) => target_server == server,
                Kind::Highlights => true,
                _ => false,
            },
            FilterClass::Channel((target_server, target_channel)) => match kind
            {
                Kind::Channel(server, channel) => {
                    target_channel == channel && target_server == server
                }
                Kind::Highlights => true,
                _ => false,
            },
            FilterClass::Any => true,
            // _ => false,
        }
    }

    pub fn is_user(&self) -> bool {
        match self.target {
            FilterTarget::User(_) => true,
            _ => false,
        }
    }
}

pub struct FilterChain<'f> {
    filters: &'f Vec<Filter>,
}

impl<'f> FilterChain<'f> {
    pub fn borrow(filters: &'f Vec<Filter>) -> Self {
        Self { filters }
    }

    pub fn filter_message(&self, message: &Message) -> bool {
        self.filters.iter().any(|f| f.match_message(message))
    }
    pub fn filter_target(&self, target: &Target) -> bool {
        self.filters.iter().any(|f| f.match_target(target))
    }
}
