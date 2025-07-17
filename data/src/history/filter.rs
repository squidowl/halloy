use thiserror::Error;

use crate::{
    Config, Message, Server, User, isupport,
    message::Source,
    target::{Channel, Query},
    user::Nick,
};

use super::Kind;

#[derive(Debug, Clone)]
pub struct Filter {
    pub target: FilterTarget,
    pub class: FilterClass,
}

#[derive(Debug, Clone)]
pub enum FilterClass {
    Server(Server),
    Channel((Server, Channel)),
    Any,
}

#[derive(Debug, Clone)]
pub enum FilterTarget {
    User(Source),
    Any,
}

impl From<&str> for FilterTarget {
    fn from(nick: &str) -> Self {
        Self::User(Source::User(User::from(Nick::from(nick))))
    }
}

#[derive(Error, Debug)]
pub enum FilterError {
    #[error("Unable to generate filter from nickname {0:?}")]
    TryFromUserError(String),
}

impl Filter {
    pub fn list_from_config(config: &Config) -> Vec<Self> {
        let mut new_filters = Vec::new();
        config.servers.entries().for_each(|entry| {
            let Some(filters) = &entry.config.filters else {
                return;
            };

            for idx in 0..filters.ignore.len() {
                new_filters.push(Filter::from_str_with_server(
                    &entry.server,
                    &filters.ignore[idx],
                ));
            }
        });
        new_filters
    }

    fn from_str_with_server(server: &Server, value: &str) -> Self {
        let (class, target) = match value.split_once(' ') {
            Some((channel, nick)) => {
                let channel =
                    Channel::from_str(channel, isupport::CaseMap::default());

                let target = FilterTarget::from(nick);

                (FilterClass::Channel((server.clone(), channel)), target)
            }
            None => (FilterClass::Any, FilterTarget::from(value)),
        };

        Self { class, target }
    }

    /// Tests a [`Message`] against the filter's predicate.
    ///
    /// This function returns `true` when the message matches predicate, false
    /// otherwise.
    ///
    /// [`Message`]:crate::Message
    pub fn match_message(&self, message: &Message) -> bool {
        match &self.target {
            FilterTarget::User(Source::User(user)) => {
                match &message.target.source() {
                    Source::Action(Some(msg_user)) | Source::User(msg_user) => {
                        msg_user == user
                    }
                    _ => false,
                }
            }
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
            FilterTarget::User(Source::User(user)) => match &self.class {
                FilterClass::Channel((_, _)) => false,
                _ => user.as_str() == query.as_str(),
            },
            FilterTarget::User(_) => false,
            FilterTarget::Any => false,
        }
    }

    pub fn match_kind(&self, kind: &Kind) -> bool {
        match &self.class {
            FilterClass::Server(target_server) => match kind {
                Kind::Channel(server, _ch) => target_server == server,
                // history::Kind::Server narrows to server messages only,
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
        }
    }

    pub fn match_server(&self, server: &Server) -> bool {
        match &self.class {
            FilterClass::Server(target_server)
            | FilterClass::Channel((target_server, ..)) => {
                target_server == server
            }
            FilterClass::Any => true,
        }
    }

    fn sync_casemapping(
        &mut self,
        target_server: &Server,
        casemapping: isupport::CaseMap,
    ) {
        let FilterClass::Channel((server, channel)) = &self.class else {
            return;
        };

        if target_server == server {
            let updated_channel =
                Channel::from_str(channel.as_str(), casemapping);

            self.class =
                FilterClass::Channel((server.clone(), updated_channel));
        }
    }

    pub fn is_user(&self) -> bool {
        matches!(self.target, FilterTarget::User(_))
    }
}

pub struct FilterChain<'f> {
    filters: &'f Vec<Filter>,
}

impl<'f> FilterChain<'f> {
    pub fn borrow(filters: &'f Vec<Filter>) -> Self {
        Self { filters }
    }

    pub fn filter_query(&self, kind: &Query) -> bool {
        self.filters.iter().any(|f| f.match_query(kind))
    }

    pub fn filter_message_of_kind(
        &self,
        message: &Message,
        kind: &Kind,
    ) -> bool {
        self.filters
            .iter()
            .filter(|f| f.match_kind(kind))
            .any(|f| f.match_message(message))
    }

    pub fn sync_channels(
        filters: &'f mut [Filter],
        server: &Server,
        casemapping: isupport::CaseMap,
    ) {
        log::debug!("updating casemap for {server:?}");
        filters
            .iter_mut()
            .filter(|filter| filter.match_server(server))
            .for_each(|filter| {
                log::debug!("{filter:?}");
                filter.sync_casemapping(server, casemapping);
            });
    }

    pub fn filter_message(&self, message: &Message) -> bool {
        self.filters.iter().any(|f| f.match_message(message))
    }
}
