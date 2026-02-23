use fancy_regex::Regex;

use super::Kind;
use crate::config::server::Ignore;
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
    UserRegex(Regex),
    MessageRegex(Regex),
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
        servers
            .entries()
            .filter_map(|entry| {
                entry.config.filters.as_ref().map(|filters| {
                    let chantypes = clients.get_chantypes(&entry.server);
                    let casemapping = clients.get_casemapping(&entry.server);

                    filters
                        .ignore
                        .iter()
                        .map(|ignore| {
                            let filter = match &ignore {
                                // Use from_str_with_server for backwards compatibility
                                Ignore::User(user) => {
                                    Filter::from_str_with_server(
                                        &entry.server,
                                        chantypes,
                                        casemapping,
                                        user,
                                    )
                                }
                                Ignore::UserInChannel { user, channel } => {
                                    let channel = Channel::from_str(
                                        channel,
                                        chantypes,
                                        casemapping,
                                    );

                                    let target = FilterTarget::from_nick(
                                        Nick::from_str(user, casemapping),
                                    );

                                    Self {
                                        class: FilterClass::Channel(
                                            entry.server.clone(),
                                            channel,
                                        ),
                                        target,
                                    }
                                }
                                Ignore::Regex { regex } => Self {
                                    class: FilterClass::Server(
                                        entry.server.clone(),
                                    ),
                                    target: FilterTarget::UserRegex(
                                        regex.clone().into(),
                                    ),
                                },
                                Ignore::RegexInChannel { regex, channel } => {
                                    let channel = Channel::from_str(
                                        channel,
                                        chantypes,
                                        casemapping,
                                    );

                                    Self {
                                        class: FilterClass::Channel(
                                            entry.server.clone(),
                                            channel,
                                        ),
                                        target: FilterTarget::UserRegex(
                                            regex.clone().into(),
                                        ),
                                    }
                                }
                            };

                            if let FilterTarget::User(filter_user) = &filter.target {
                                match &filter.class {
                                    FilterClass::Server(server) => {
                                        log::debug!(
                                            "[{server}] loaded ignore user filter raw={:?} normalized={:?} scope=server",
                                            filter_user.as_str(),
                                            filter_user.as_normalized_str(),
                                        );
                                    }
                                    FilterClass::Channel(server, channel) => {
                                        log::debug!(
                                            "[{server}] loaded ignore user filter raw={:?} normalized={:?} scope=channel:{channel}",
                                            filter_user.as_str(),
                                            filter_user.as_normalized_str(),
                                        );
                                    }
                                }
                            }

                            filter
                        })
                        .chain(filters.regex.iter().map(|regex| Self {
                            class: FilterClass::Server(entry.server.clone()),
                            target: FilterTarget::MessageRegex(
                                regex.clone().into(),
                            ),
                        }))
                        .collect::<Vec<Self>>()
                })
            })
            .flatten()
            .collect()
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

    pub fn match_user(
        &self,
        user: &User,
        channel: Option<&Channel>,
        server: &Server,
    ) -> bool {
        match &self.target {
            FilterTarget::User(filter_user) => {
                user.as_normalized_str() == filter_user.as_normalized_str()
                    && (match &self.class {
                        FilterClass::Channel(filter_server, filter_channel) => {
                            channel.is_some_and(|channel| {
                                channel.as_normalized_str()
                                    == filter_channel.as_normalized_str()
                            }) && filter_server == server
                        }
                        FilterClass::Server(filter_server) => {
                            filter_server == server
                        }
                    })
            }
            FilterTarget::UserRegex(regex) => {
                regex.is_match(user.as_str()).is_ok_and(|is_match| is_match)
                    && (match &self.class {
                        FilterClass::Channel(filter_server, filter_channel) => {
                            channel.is_some_and(|channel| {
                                channel.as_normalized_str()
                                    == filter_channel.as_normalized_str()
                            }) && filter_server == server
                        }
                        FilterClass::Server(filter_server) => {
                            filter_server == server
                        }
                    })
            }
            FilterTarget::MessageRegex(_) => false,
        }
    }

    /// Tests a [`Message`] against the filter's predicate.
    ///
    /// This function returns `true` when the message matches predicate, false
    /// otherwise.
    ///
    /// [`Message`]:crate::MessageRegex
    pub fn match_message(&self, message: &Message) -> bool {
        match &self.target {
            FilterTarget::User(user) => match &message.target.source() {
                Source::Action(Some(msg_user)) | Source::User(msg_user) => {
                    let matched = msg_user.nickname() == user.nickname();

                    log::debug!(
                        "filter match_message user-compare filter_raw={:?} filter_norm={:?} msg_raw={:?} msg_norm={:?} matched={} source={:?}",
                        user.nickname().as_str(),
                        user.nickname().as_normalized_str(),
                        msg_user.nickname().as_str(),
                        msg_user.nickname().as_normalized_str(),
                        matched,
                        message.target.source(),
                    );

                    matched
                }
                Source::Server(Some(server)) => {
                    // Match server messages from the filtered user, except for
                    // nick change messages in order to alert the Halloy user
                    // that the filtered user has a new nickname.
                    let matched = server
                        .nick()
                        .is_some_and(|nick| user.nickname() == *nick)
                        && !matches!(
                            server.kind(),
                            source::server::Kind::ChangeNick
                        );

                    log::debug!(
                        "filter match_message server-user-compare filter_raw={:?} filter_norm={:?} server_nick={:?} matched={} kind={:?}",
                        user.nickname().as_str(),
                        user.nickname().as_normalized_str(),
                        server.nick().map(|nick| nick.as_str()),
                        matched,
                        server.kind(),
                    );

                    matched
                }
                _ => false,
            },
            FilterTarget::UserRegex(regex) => match &message.target.source() {
                Source::Action(Some(msg_user)) | Source::User(msg_user) => {
                    regex
                        .is_match(msg_user.as_str())
                        .is_ok_and(|is_match| is_match)
                }
                Source::Server(Some(server)) => {
                    // Match server messages from the filtered user, except for
                    // nick change messages in order to alert the Halloy user
                    // that the filtered user has a new nickname.
                    server.nick().is_some_and(|nick| {
                        regex
                            .is_match(nick.as_str())
                            .is_ok_and(|is_match| is_match)
                    }) && !matches!(
                        server.kind(),
                        source::server::Kind::ChangeNick
                    )
                }
                _ => false,
            },
            FilterTarget::MessageRegex(regex) => regex
                .is_match(&message.text())
                .is_ok_and(|is_match| is_match),
        }
    }

    /// Tests a [`Query`] against the filter's predicate.
    ///
    /// This function returns `true` when the query matches predicate, false
    /// otherwise.
    ///
    /// [`Query`]:crate::Query
    pub fn match_query(&self, query: &Query, server: &Server) -> bool {
        match &self.target {
            FilterTarget::User(user) => match &self.class {
                FilterClass::Channel(_, _) => false,
                FilterClass::Server(filter_server) => {
                    user.nickname().as_normalized_str()
                        == query.as_normalized_str()
                        && filter_server == server
                }
            },
            FilterTarget::UserRegex(regex) => match &self.class {
                FilterClass::Channel(_, _) => false,
                FilterClass::Server(filter_server) => {
                    regex
                        .is_match(query.as_str())
                        .is_ok_and(|is_match| is_match)
                        && filter_server == server
                }
            },
            FilterTarget::MessageRegex(_) => false,
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
            FilterTarget::UserRegex(_) | FilterTarget::MessageRegex(_) => (),
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

    pub fn filter_user(
        &self,
        user: &User,
        channel: Option<&Channel>,
        server: &Server,
    ) -> bool {
        self.filters
            .iter()
            .any(|f| f.match_user(user, channel, server))
    }

    pub fn filter_query(&self, query: &Query, server: &Server) -> bool {
        self.filters.iter().any(|f| f.match_query(query, server))
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
}
