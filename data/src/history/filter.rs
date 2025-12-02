use super::Kind;
use crate::message::{self, Source, source};
use crate::server::Map as ServerMap;
use crate::target::{Channel, Query};
use crate::user::Nick;
use crate::{Message, Server, User, client, isupport};

#[derive(Debug, Clone)]
pub struct Filter {
    target: FilterTarget,
    class: FilterClass,
}

#[derive(Debug, Clone)]
enum FilterClass {
    Channel(Server, Channel),
    Server(Server),
}

#[derive(Debug, Clone)]
enum FilterTarget {
    User(User),
}

impl FilterTarget {
    pub fn from_nick(nick: Nick) -> Self {
        Self::User(User::from(nick))
    }
}

impl Filter {
    pub fn list_from_servers(
        servers: &ServerMap,
        clients: &client::Map,
    ) -> Vec<Self> {
        let mut new_filters = Vec::new();
        servers.entries().for_each(|entry| {
            let Some(filters) = &entry.config.filters else {
                return;
            };

            for idx in 0..filters.ignore.len() {
                let chantypes = clients.get_chantypes(&entry.server);
                let casemapping = clients.get_casemapping(&entry.server);

                new_filters.push(Filter::from_str_with_server(
                    &entry.server,
                    chantypes,
                    casemapping,
                    &filters.ignore[idx],
                ));
            }
        });
        new_filters
    }

    fn from_str_with_server(
        server: &Server,
        chantypes: &[char],
        casemapping: isupport::CaseMap,
        value: &str,
    ) -> Self {
        let (class, target) = match value.split_once(' ') {
            Some((channel, nick)) => {
                let channel =
                    Channel::from_str(channel, chantypes, casemapping);

                let target =
                    FilterTarget::from_nick(Nick::from_str(nick, casemapping));

                (FilterClass::Channel(server.clone(), channel), target)
            }
            None => (
                FilterClass::Server(server.clone()),
                FilterTarget::from_nick(Nick::from_str(value, casemapping)),
            ),
        };

        Self { class, target }
    }

    pub fn match_user(&self, user: &User, channel: Option<&Channel>) -> bool {
        match &self.target {
            FilterTarget::User(filter_user) => {
                user.as_normalized_str() == filter_user.as_normalized_str()
                    && (match &self.class {
                        FilterClass::Channel(_, filter_channel) => channel
                            .is_some_and(|channel| {
                                channel.as_normalized_str()
                                    == filter_channel.as_normalized_str()
                            }),
                        FilterClass::Server(_) => true,
                    })
            }
        }
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
                    msg_user.nickname() == user.nickname()
                }
                Source::Server(Some(server)) => {
                    // Match server messages from the filtered user, except
                    // for nick change messages in order to alert the Halloy
                    // user that the filtered user has a new nickname.
                    server.nick().is_some_and(|nick| user.nickname() == *nick)
                        && !matches!(
                            server.kind(),
                            source::server::Kind::ChangeNick
                        )
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
                FilterClass::Channel(_, _) => false,
                FilterClass::Server(_) => {
                    user.nickname().as_normalized_str()
                        == query.as_normalized_str()
                }
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
            FilterClass::Channel(target_server, target_channel) => match kind {
                Kind::Channel(server, channel) => {
                    target_channel == channel && target_server == server
                }
                _ => false,
            },
            FilterClass::Server(target_server) => match kind {
                Kind::Server(server)
                | Kind::Channel(server, _)
                | Kind::Query(server, _) => target_server == server,
                Kind::Highlights | Kind::Logs => false,
            },
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
            FilterClass::Channel(target_server, ..)
            | FilterClass::Server(target_server) => target_server == server,
        }
    }

    fn sync_isupport(
        &mut self,
        target_server: &Server,
        chantypes: &[char],
        casemapping: isupport::CaseMap,
    ) {
        match &mut self.target {
            FilterTarget::User(user) => {
                user.renormalize(casemapping);
            }
        }

        match &self.class {
            FilterClass::Channel(server, channel) => {
                if target_server == server {
                    let updated_channel = Channel::from_str(
                        channel.as_str(),
                        chantypes,
                        casemapping,
                    );

                    self.class =
                        FilterClass::Channel(server.clone(), updated_channel);
                }
            }
            FilterClass::Server(_) => (),
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

    pub fn filter_user(&self, user: &User, channel: Option<&Channel>) -> bool {
        self.filters.iter().any(|f| f.match_user(user, channel))
    }

    pub fn filter_query(&self, kind: &Query) -> bool {
        self.filters.iter().any(|f| f.match_query(kind))
    }

    pub fn filter_message_of_kind(&self, message: &mut Message, kind: &Kind) {
        message.blocked = self
            .filters
            .iter()
            .filter(|f| {
                if let message::Target::Highlights {
                    server, channel, ..
                } = &message.target
                {
                    f.match_kind(&Kind::Channel(
                        server.clone(),
                        channel.clone(),
                    ))
                } else {
                    f.match_kind(kind)
                }
            })
            .any(|f| f.match_message(message));
    }

    pub fn sync_isupport(
        filters: &'f mut [Filter],
        server: &Server,
        chantypes: &[char],
        casemapping: isupport::CaseMap,
    ) {
        log::debug!("[{server}] updating filter ISUPPORT");
        filters
            .iter_mut()
            .filter(|filter| filter.match_server(server))
            .for_each(|filter| {
                filter.sync_isupport(server, chantypes, casemapping);
            });
    }

    pub fn filter_message(&self, message: &Message) -> bool {
        self.filters.iter().any(|f| f.match_message(message))
    }
}
