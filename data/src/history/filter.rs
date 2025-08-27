use crate::server::Map as ServerMap;
use crate::{
    Message, Server, User, isupport,
    message::Source,
    target::{Channel, Query},
    user::Nick,
};

use super::Kind;

#[derive(Debug, Clone)]
pub struct Filter {
    target: FilterTarget,
    class: FilterClass,
}

#[derive(Debug, Clone)]
enum FilterClass {
    Channel((Server, Channel)),
    Any,
}

#[derive(Debug, Clone)]
enum FilterTarget {
    User(User),
}

impl From<&str> for FilterTarget {
    fn from(nick: &str) -> Self {
        Self::User(User::from(Nick::from(nick)))
    }
}

impl Filter {
    pub fn list_from_servers(servers: &ServerMap) -> Vec<Self> {
        let mut new_filters = Vec::new();
        servers.entries().for_each(|entry| {
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
            FilterTarget::User(user) => match &message.target.source() {
                Source::Action(Some(msg_user)) | Source::User(msg_user) => {
                    msg_user == user
                }
                _ => false,
            },
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
            FilterTarget::User(user) => match &self.class {
                FilterClass::Channel((_, _)) => false,
                _ => user.as_str() == query.as_str(),
            },
        }
    }

    /// Tests a [`history::Kind`] against the filter's predicate.
    ///
    /// This function returns `true` when the query matches predicate, false
    /// otherwise.
    ///
    /// Note: matching against [`history::Kind::Server`] is not the same as
    /// matching against [`Server`] - `history::Kind::Server` matches only for
    /// messages from the server itself, not 'any message coming from a server'.
    /// Use `Filter::match_server` if you need to test against 'any message from
    /// a particular server'
    ///
    /// [`history::Kind`]:crate::history::Kind
    /// [`history::Kind::Server`]:crate::history::Kind::Server
    /// [`Server`]:crate::Server
    pub fn match_kind(&self, kind: &Kind) -> bool {
        match &self.class {
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

    /// Tests a [`Server`] against the filter's predicate.
    ///
    /// This function returns `true` when the query matches predicate, false
    /// otherwise.
    ///
    /// [`Server`]:crate::Server
    pub fn match_server(&self, server: &Server) -> bool {
        match &self.class {
            FilterClass::Channel((target_server, ..)) => {
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
        log::debug!("[{server}] updating filter casemapping");
        filters
            .iter_mut()
            .filter(|filter| filter.match_server(server))
            .for_each(|filter| {
                filter.sync_casemapping(server, casemapping);
            });
    }

    pub fn filter_message(&self, message: &Message) -> bool {
        self.filters.iter().any(|f| f.match_message(message))
    }
}
