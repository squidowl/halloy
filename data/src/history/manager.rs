use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::{future, Future, FutureExt};
use itertools::Itertools;
use tokio::time::Instant;

use crate::config::buffer::{Exclude, ServerMessages};
use crate::history::{self, History};
use crate::message::{self, Limit};
use crate::time::Posix;
use crate::user::{Nick, NickRef};
use crate::{server, Buffer, Config, Input, Server, User};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Resource {
    pub server: server::Server,
    pub kind: history::Kind,
}

#[derive(Debug)]
pub enum Message {
    Loaded(
        server::Server,
        history::Kind,
        Result<Vec<crate::Message>, history::Error>,
    ),
    Closed(server::Server, history::Kind, Result<(), history::Error>),
    Flushed(server::Server, history::Kind, Result<(), history::Error>),
}

#[derive(Debug, Default)]
pub struct Manager {
    resources: HashSet<Resource>,
    data: Data,
}

impl Manager {
    pub fn track(&mut self, new_resources: HashSet<Resource>) -> Vec<BoxFuture<'static, Message>> {
        let added = new_resources.difference(&self.resources).cloned();
        let removed = self.resources.difference(&new_resources).cloned();

        let added = added.into_iter().map(|resource| {
            async move {
                history::load(&resource.server.clone(), &resource.kind.clone())
                    .map(move |result| Message::Loaded(resource.server, resource.kind, result))
                    .await
            }
            .boxed()
        });

        let removed = removed.into_iter().filter_map(|resource| {
            self.data
                .untrack(&resource.server, &resource.kind)
                .map(|task| {
                    task.map(|result| Message::Closed(resource.server, resource.kind, result))
                        .boxed()
                })
        });

        let tasks = added.chain(removed).collect();

        self.resources = new_resources;

        tasks
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Loaded(server, kind, Ok(messages)) => {
                log::debug!(
                    "loaded history for {kind} on {server}: {} messages",
                    messages.len()
                );
                self.data.loaded(server, kind, messages);
            }
            Message::Loaded(server, kind, Err(error)) => {
                log::warn!("failed to load history for {kind} on {server}: {error}");
            }
            Message::Closed(server, kind, Ok(_)) => {
                log::debug!("closed history for {kind} on {server}",);
            }
            Message::Closed(server, kind, Err(error)) => {
                log::warn!("failed to close history for {kind} on {server}: {error}")
            }
            Message::Flushed(server, kind, Ok(_)) => {
                log::debug!("flushed history for {kind} on {server}",);
            }
            Message::Flushed(server, kind, Err(error)) => {
                log::warn!("failed to flush history for {kind} on {server}: {error}")
            }
        }
    }

    pub fn tick(&mut self, now: Instant) -> Vec<BoxFuture<'static, Message>> {
        self.data.flush_all(now)
    }

    pub fn close(
        &mut self,
        server: Server,
        kind: history::Kind,
    ) -> Option<impl Future<Output = ()>> {
        let history = self.data.map.get_mut(&server)?.remove(&kind)?;

        Some(async move {
            match history.close().await {
                Ok(_) => {
                    log::debug!("closed history for {kind} on {server}",);
                }
                Err(error) => {
                    log::warn!("failed to close history for {kind} on {server}: {error}");
                }
            }
        })
    }

    pub fn close_server(&mut self, server: Server) -> Option<impl Future<Output = ()>> {
        let map = self.data.map.remove(&server)?;

        Some(async move {
            let tasks = map.into_iter().map(move |(kind, state)| {
                let server = server.clone();
                state.close().map(move |result| (server, kind, result))
            });

            let results = future::join_all(tasks).await;

            for (server, kind, result) in results {
                match result {
                    Ok(_) => {
                        log::debug!("closed history for {kind} on {server}",);
                    }
                    Err(error) => {
                        log::warn!("failed to close history for {kind} on {server}: {error}");
                    }
                }
            }
        })
    }

    pub fn close_all(&mut self) -> impl Future<Output = ()> {
        let map = std::mem::take(&mut self.data).map;

        async move {
            let tasks = map.into_iter().flat_map(|(server, map)| {
                map.into_iter().map(move |(kind, state)| {
                    let server = server.clone();
                    state.close().map(move |result| (server, kind, result))
                })
            });

            let results = future::join_all(tasks).await;

            for (server, kind, result) in results {
                match result {
                    Ok(_) => {
                        log::debug!("closed history for {kind} on {server}",);
                    }
                    Err(error) => {
                        log::warn!("failed to close history for {kind} on {server}: {error}");
                    }
                }
            }
        }
    }

    pub fn record_input(&mut self, input: Input, our_nick: NickRef) {
        if let Some(message) = input.message(our_nick) {
            self.record_message(input.server(), message);
        }

        if let Some(text) = input.raw() {
            self.data.input.push(input.buffer(), text.to_string());
        }
    }

    pub fn record_message(&mut self, server: &Server, message: crate::Message) {
        self.data.add_message(
            server.clone(),
            history::Kind::from(message.target.clone()),
            message,
        );
    }

    pub fn get_channel_messages(
        &self,
        server: &Server,
        channel: &str,
        limit: Option<Limit>,
        server_messages: &ServerMessages,
    ) -> Option<history::View<'_>> {
        self.data.history_view(
            server,
            &history::Kind::Channel(channel.to_string()),
            limit,
            server_messages,
        )
    }

    pub fn get_server_messages(
        &self,
        server: &Server,
        limit: Option<Limit>,
        server_messages: &ServerMessages,
    ) -> Option<history::View<'_>> {
        self.data
            .history_view(server, &history::Kind::Server, limit, server_messages)
    }

    pub fn get_query_messages(
        &self,
        server: &Server,
        nick: &Nick,
        limit: Option<Limit>,
        server_messages: &ServerMessages,
    ) -> Option<history::View<'_>> {
        self.data.history_view(
            server,
            &history::Kind::Query(nick.clone()),
            limit,
            server_messages,
        )
    }

    pub fn get_unique_queries(&self, server: &Server) -> Vec<&Nick> {
        let Some(map) = self.data.map.get(server) else {
            return vec![];
        };

        let queries = map
            .keys()
            .filter_map(|kind| match kind {
                history::Kind::Query(user) => Some(user),
                _ => None,
            })
            .unique()
            .collect::<Vec<_>>();

        queries
    }

    pub fn has_unread(&self, server: &Server, kind: &history::Kind) -> bool {
        self.data
            .map
            .get(server)
            .and_then(|map| map.get(kind))
            .map(|history| {
                matches!(
                    history,
                    History::Partial {
                        unread_message_count,
                        ..
                    } if *unread_message_count > 0
                )
            })
            .unwrap_or_default()
    }

    pub fn broadcast(&mut self, server: &Server, broadcast: Broadcast, config: &Config) {
        let map = self.data.map.entry(server.clone()).or_default();

        let channels = map
            .keys()
            .filter_map(|kind| {
                if let history::Kind::Channel(channel) = kind {
                    Some(channel)
                } else {
                    None
                }
            })
            .cloned();
        let mut queries = map
            .keys()
            .filter_map(|kind| {
                if let history::Kind::Query(nick) = kind {
                    Some(nick)
                } else {
                    None
                }
            })
            .cloned();

        let messages = match broadcast {
            Broadcast::Connecting => message::broadcast::connecting(),
            Broadcast::Connected => message::broadcast::connected(),
            Broadcast::ConnectionFailed { error } => message::broadcast::connection_failed(error),
            Broadcast::Disconnected { error } => {
                message::broadcast::disconnected(channels, queries, error)
            }
            Broadcast::Reconnected => message::broadcast::reconnected(channels, queries),
            Broadcast::Quit {
                user,
                comment,
                user_channels,
            } => {
                let user_query = queries.find(|nick| user.nickname() == *nick);

                message::broadcast::quit(user_channels, user_query, &user, &comment, config)
            }
            Broadcast::Nickname {
                old_nick,
                new_nick,
                ourself,
                user_channels,
            } => {
                if ourself {
                    // If ourself, broadcast to all query channels (since we are in all of them)
                    message::broadcast::nickname(
                        user_channels,
                        queries,
                        &old_nick,
                        &new_nick,
                        ourself,
                    )
                } else {
                    // Otherwise just the query channel of the user w/ nick change
                    let user_query = queries.find(|nick| old_nick == *nick);
                    message::broadcast::nickname(
                        user_channels,
                        user_query,
                        &old_nick,
                        &new_nick,
                        ourself,
                    )
                }
            }
            Broadcast::Invite {
                inviter,
                channel,
                user_channels,
            } => message::broadcast::invite(inviter, channel, user_channels),
        };

        messages.into_iter().for_each(|message| {
            self.record_message(server, message);
        });
    }

    pub fn input_history<'a>(&'a self, buffer: &Buffer) -> &'a [String] {
        self.data.input.get(buffer)
    }
}

fn with_limit<'a>(
    limit: Option<Limit>,
    messages: impl Iterator<Item = &'a crate::Message>,
) -> Vec<&'a crate::Message> {
    match limit {
        Some(Limit::Top(n)) => messages.take(n).collect(),
        Some(Limit::Bottom(n)) => {
            let collected = messages.collect::<Vec<_>>();
            let length = collected.len();
            collected[length.saturating_sub(n)..length].to_vec()
        }
        Some(Limit::Since(timestamp)) => messages
            .skip_while(|message| message.received_at < timestamp)
            .collect(),
        None => messages.collect(),
    }
}

#[derive(Debug, Default)]
struct Data {
    map: HashMap<server::Server, HashMap<history::Kind, History>>,
    input: history::Input,
}

impl Data {
    fn loaded(
        &mut self,
        server: server::Server,
        kind: history::Kind,
        mut messages: Vec<crate::Message>,
    ) {
        use std::collections::hash_map;

        match self
            .map
            .entry(server.clone())
            .or_default()
            .entry(kind.clone())
        {
            hash_map::Entry::Occupied(mut entry) => match entry.get_mut() {
                History::Partial {
                    messages: new_messages,
                    last_received_at,
                    opened_at,
                    ..
                } => {
                    let last_received_at = *last_received_at;
                    let opened_at = *opened_at;
                    messages.extend(std::mem::take(new_messages));
                    entry.insert(History::Full {
                        server,
                        kind,
                        messages,
                        last_received_at,
                        opened_at,
                    });
                }
                _ => {
                    entry.insert(History::Full {
                        server,
                        kind,
                        messages,
                        last_received_at: None,
                        opened_at: Posix::now(),
                    });
                }
            },
            hash_map::Entry::Vacant(entry) => {
                entry.insert(History::Full {
                    server,
                    kind,
                    messages,
                    last_received_at: None,
                    opened_at: Posix::now(),
                });
            }
        }
    }

    fn history_view(
        &self,
        server: &server::Server,
        kind: &history::Kind,
        limit: Option<Limit>,
        server_messages: &ServerMessages,
    ) -> Option<history::View> {
        let History::Full {
            messages,
            opened_at,
            ..
        } = self.map.get(server)?.get(kind)?
        else {
            return None;
        };

        let mut most_recent_messages = HashMap::<Nick, DateTime<Utc>>::new();

        let filtered = messages
            .iter()
            .filter(|message| match message.target.source() {
                message::Source::Server(Some(source)) => {
                    let source_config = server_messages.get(source);

                    match source_config.exclude {
                        Exclude::All => false,
                        Exclude::None => true,
                        Exclude::Smart(seconds) => {
                            let nick = match source {
                                message::source::Server::Join(nick) => nick,
                                message::source::Server::Part(nick) => nick,
                                message::source::Server::Quit(nick) => nick,
                            };

                            if nick.is_some() {
                                !smart_filter_message(
                                    message,
                                    &seconds,
                                    most_recent_messages.get(&nick.to_owned().unwrap()),
                                )
                            } else if let Some(nickname) =
                                message.text.split(' ').collect::<Vec<_>>().get(1)
                            {
                                let nick = Nick::from(*nickname);

                                !smart_filter_message(
                                    message,
                                    &seconds,
                                    most_recent_messages.get(&nick),
                                )
                            } else {
                                true
                            }
                        }
                    }
                }
                crate::message::Source::User(message_user) => {
                    most_recent_messages
                        .insert(message_user.nickname().to_owned(), message.server_time);

                    true
                }
                _ => true,
            })
            .collect::<Vec<_>>();

        let total = filtered.len();
        let limited = with_limit(limit, filtered.into_iter());

        let split_at = limited
            .iter()
            .position(|message| message.received_at >= *opened_at)
            .unwrap_or(limited.len());

        let (old, new) = limited.split_at(split_at);

        Some(history::View {
            total,
            old_messages: old.to_vec(),
            new_messages: new.to_vec(),
        })
    }

    fn add_message(
        &mut self,
        server: server::Server,
        kind: history::Kind,
        message: crate::Message,
    ) {
        self.map
            .entry(server.clone())
            .or_default()
            .entry(kind.clone())
            .or_insert_with(|| History::partial(server, kind, message.received_at))
            .add_message(message)
    }

    fn untrack(
        &mut self,
        server: &server::Server,
        kind: &history::Kind,
    ) -> Option<impl Future<Output = Result<(), history::Error>>> {
        self.map
            .get_mut(server)
            .and_then(|map| map.get_mut(kind).and_then(History::make_partial))
    }

    fn flush_all(&mut self, now: Instant) -> Vec<BoxFuture<'static, Message>> {
        self.map
            .iter_mut()
            .flat_map(|(server, map)| {
                map.iter_mut().filter_map(|(kind, state)| {
                    let server = server.clone();
                    let kind = kind.clone();

                    state.flush(now).map(move |task| {
                        task.map(move |result| Message::Flushed(server, kind, result))
                            .boxed()
                    })
                })
            })
            .collect()
    }
}

fn smart_filter_message(
    message: &crate::Message,
    seconds: &i64,
    most_recent_message_server_time: Option<&DateTime<Utc>>,
) -> bool {
    let Some(server_time) = most_recent_message_server_time else {
        return true;
    };

    let duration_seconds = message
        .server_time
        .signed_duration_since(*server_time)
        .num_seconds();

    duration_seconds > *seconds
}

#[derive(Debug, Clone)]
pub enum Broadcast {
    Connecting,
    Connected,
    ConnectionFailed {
        error: String,
    },
    Disconnected {
        error: Option<String>,
    },
    Reconnected,
    Quit {
        user: User,
        comment: Option<String>,
        user_channels: Vec<String>,
    },
    Nickname {
        old_nick: Nick,
        new_nick: Nick,
        ourself: bool,
        user_channels: Vec<String>,
    },
    Invite {
        inviter: Nick,
        channel: String,
        user_channels: Vec<String>,
    },
}
