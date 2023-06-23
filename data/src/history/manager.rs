use std::collections::{HashMap, HashSet};
use std::time::Duration;

use futures::future::BoxFuture;
use futures::{future, Future, FutureExt, Stream, StreamExt};
use itertools::Itertools;
use tokio::time::Instant;

use crate::history::{self, History};
use crate::message::{self, Limit};
use crate::time::Posix;
use crate::user::Nick;
use crate::{server, Server};

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
    Tick(Instant),
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

    pub fn update(&mut self, message: Message) -> Vec<BoxFuture<'static, Message>> {
        match message {
            Message::Loaded(server, kind, Ok(messages)) => {
                log::debug!(
                    "loaded history for {kind} on {server}: {} messages",
                    messages.len()
                );
                self.data.loaded(server, kind, messages)
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
            Message::Tick(now) => {
                return self.data.flush_all(now);
            }
        }

        vec![]
    }

    pub fn close(&mut self) -> impl Future<Output = ()> {
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

    pub fn record_message(&mut self, server: &Server, message: crate::Message) {
        self.data.add_message(
            server.clone(),
            history::Kind::from(message.source.clone()),
            message,
        );
    }

    pub fn get_channel_messages(
        &self,
        server: &Server,
        channel: &str,
        limit: Option<Limit>,
    ) -> (usize, Vec<&crate::Message>) {
        self.data
            .messages(server, &history::Kind::Channel(channel.to_string()))
            .map(|messages| {
                let total = messages.len();

                (total, with_limit(limit, messages.iter()))
            })
            .unwrap_or_else(|| (0, vec![]))
    }

    pub fn get_server_messages(
        &self,
        server: &Server,
        limit: Option<Limit>,
    ) -> (usize, Vec<&crate::Message>) {
        self.data
            .messages(server, &history::Kind::Server)
            .map(|messages| {
                let total = messages.len();

                (total, with_limit(limit, messages.iter()))
            })
            .unwrap_or_else(|| (0, vec![]))
    }

    pub fn get_query_messages(
        &self,
        server: &Server,
        nick: &Nick,
        limit: Option<Limit>,
    ) -> (usize, Vec<&crate::Message>) {
        self.data
            .messages(server, &history::Kind::Query(nick.clone()))
            .map(|messages| {
                let total = messages.len();

                (total, with_limit(limit, messages.iter()))
            })
            .unwrap_or_else(|| (0, vec![]))
    }

    pub fn get_unique_queries(&self, server: &Server) -> Vec<&Nick> {
        let Some(map) = self.data.map.get(server) else {
            return vec![]
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

    pub fn broadcast(&mut self, server: &Server, broadcast: Broadcast) {
        let Some(map) = self.data.map.get(server) else {
            return;
        };

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
        let queries = map
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
            Broadcast::Disconnected => {
                message::broadcast::disconnected(channels, queries).collect::<Vec<_>>()
            }
            Broadcast::Reconnected => {
                message::broadcast::reconnected(channels, queries).collect::<Vec<_>>()
            }
        };

        messages.into_iter().for_each(|message| {
            self.record_message(server, message);
        });
    }
}

pub fn tick() -> impl Stream<Item = Message> {
    use tokio::time::interval_at;
    use tokio_stream::wrappers::IntervalStream;

    IntervalStream::new(interval_at(
        Instant::now() + Duration::from_secs(1),
        Duration::from_secs(1),
    ))
    .map(Message::Tick)
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
            .skip_while(|message| Posix::from(message.datetime) < timestamp)
            .collect(),
        None => messages.collect(),
    }
}

#[derive(Debug, Default)]
struct Data {
    map: HashMap<server::Server, HashMap<history::Kind, History>>,
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
                    ..
                } => {
                    let last_received_at = *last_received_at;
                    messages.extend(std::mem::take(new_messages));
                    entry.insert(History::Full {
                        server,
                        kind,
                        messages,
                        last_received_at,
                    });
                }
                _ => {
                    entry.insert(History::Full {
                        server,
                        kind,
                        messages,
                        last_received_at: None,
                    });
                }
            },
            hash_map::Entry::Vacant(entry) => {
                entry.insert(History::Full {
                    server,
                    kind,
                    messages,
                    last_received_at: None,
                });
            }
        }
    }

    fn messages(&self, server: &server::Server, kind: &history::Kind) -> Option<&[crate::Message]> {
        self.map
            .get(server)
            .and_then(|map| map.get(kind))
            .map(History::messages)
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
            .or_insert_with(|| History::partial(server, kind))
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

#[derive(Debug, Clone, Copy)]
pub enum Broadcast {
    Disconnected,
    Reconnected,
}
