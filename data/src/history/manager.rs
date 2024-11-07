use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::{future, Future, FutureExt};
use tokio::time::Instant;

use crate::history::{self, History};
use crate::message::{self, Limit};
use crate::user::Nick;
use crate::{buffer, config, input};
use crate::{server, Config, Input, Server, User};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Resource {
    pub kind: history::Kind,
}

impl Resource {
    pub fn logs() -> Self {
        Self {
            kind: history::Kind::Logs,
        }
    }

    pub fn highlights() -> Self {
        Self {
            kind: history::Kind::Highlights,
        }
    }
}

#[derive(Debug)]
pub enum Message {
    LoadFull(history::Kind, Result<history::Loaded, history::Error>),
    UpdatePartial(history::Kind, Result<history::Metadata, history::Error>),
    UpdateReadMarker(
        history::Kind,
        history::ReadMarker,
        Result<(), history::Error>,
    ),
    Closed(
        history::Kind,
        Result<Option<history::ReadMarker>, history::Error>,
    ),
    Flushed(history::Kind, Result<(), history::Error>),
    Exited(
        Vec<(
            history::Kind,
            Result<Option<history::ReadMarker>, history::Error>,
        )>,
    ),
}

pub enum Event {
    Loaded(history::Kind),
    Closed(history::Kind, Option<history::ReadMarker>),
    Exited(Vec<(history::Kind, Option<history::ReadMarker>)>),
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
                history::load(resource.kind.clone())
                    .map(move |result| Message::LoadFull(resource.kind, result))
                    .await
            }
            .boxed()
        });

        let removed = removed.into_iter().filter_map(|resource| {
            self.data.untrack(&resource.kind).map(|task| {
                task.map(|result| Message::Closed(resource.kind, result))
                    .boxed()
            })
        });

        let tasks = added.chain(removed).collect();

        self.resources = new_resources;

        tasks
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::LoadFull(kind, Ok(loaded)) => {
                log::debug!(
                    "loaded history for {kind}: {} messages",
                    loaded.messages.len()
                );
                self.data.load_full(kind.clone(), loaded);
                return Some(Event::Loaded(kind));
            }
            Message::LoadFull(kind, Err(error)) => {
                log::warn!("failed to load history for {kind}: {error}");
            }
            Message::Closed(kind, Ok(read_marker)) => {
                log::debug!("closed history for {kind}",);
                return Some(Event::Closed(kind, read_marker));
            }
            Message::Closed(kind, Err(error)) => {
                log::warn!("failed to close history for {kind}: {error}")
            }
            Message::Flushed(kind, Ok(_)) => {
                // Will cause flush loop if we emit a log every time we flush logs
                if !matches!(kind, history::Kind::Logs) {
                    log::debug!("flushed history for {kind}",);
                }
            }
            Message::Flushed(kind, Err(error)) => {
                log::warn!("failed to flush history for {kind}: {error}")
            }
            Message::UpdatePartial(kind, Ok(metadata)) => {
                log::debug!("loaded metadata for {kind}");
                self.data.update_partial(kind, metadata);
            }
            Message::UpdatePartial(kind, Err(error)) => {
                log::warn!("failed to load metadata for {kind}: {error}");
            }
            Message::UpdateReadMarker(kind, read_marker, Ok(_)) => {
                log::debug!("updated read marker for {kind} to {read_marker}");
            }
            Message::UpdateReadMarker(kind, read_marker, Err(error)) => {
                log::warn!("failed to update read marker for {kind} to {read_marker}: {error}");
            }
            Message::Exited(results) => {
                let mut output = vec![];

                for (kind, result) in results {
                    match result {
                        Ok(marker) => {
                            log::debug!("closed history for {kind}",);
                            output.push((kind, marker));
                        }
                        Err(error) => {
                            log::warn!("failed to close history for {kind}: {error}");
                            output.push((kind, None));
                        }
                    }
                }

                return Some(Event::Exited(output));
            }
        }

        None
    }

    pub fn tick(&mut self, now: Instant) -> Vec<BoxFuture<'static, Message>> {
        self.data.flush_all(now)
    }

    pub fn close(&mut self, kind: history::Kind) -> Option<impl Future<Output = Message>> {
        let history = self.data.map.remove(&kind)?;

        Some(history.close().map(|result| Message::Closed(kind, result)))
    }

    pub fn exit(&mut self) -> impl Future<Output = Message> {
        let map = std::mem::take(&mut self.data).map;

        async move {
            let tasks = map
                .into_iter()
                .map(|(kind, state)| state.close().map(move |result| (kind, result)));

            Message::Exited(future::join_all(tasks).await)
        }
    }

    pub fn record_input(
        &mut self,
        input: Input,
        user: User,
        channel_users: &[User],
        chantypes: &[char],
        statusmsg: &[char],
    ) -> Vec<impl Future<Output = Message>> {
        let mut tasks = vec![];

        if let Some(messages) = input.messages(user, channel_users, chantypes, statusmsg) {
            for message in messages {
                tasks.extend(self.record_message(input.server(), message));
            }
        }

        if let Some(text) = input.raw() {
            self.data.input.record(&input.buffer, text.to_string());
        }

        tasks
    }

    pub fn record_draft(&mut self, draft: input::Draft) {
        self.data.input.store_draft(draft);
    }

    pub fn record_message(
        &mut self,
        server: &Server,
        message: crate::Message,
    ) -> Option<impl Future<Output = Message>> {
        history::Kind::from_server_message(server.clone(), &message)
            .and_then(|kind| self.data.add_message(kind, message))
    }

    pub fn record_log(
        &mut self,
        record: crate::log::Record,
    ) -> Option<impl Future<Output = Message>> {
        self.data
            .add_message(history::Kind::Logs, crate::Message::log(record))
    }

    pub fn record_highlight(
        &mut self,
        message: crate::Message,
    ) -> Option<impl Future<Output = Message>> {
        self.data.add_message(history::Kind::Highlights, message)
    }

    pub fn update_read_marker(
        &mut self,
        kind: impl Into<history::Kind>,
        read_marker: history::ReadMarker,
    ) -> Option<impl Future<Output = Message>> {
        self.data.update_read_marker(kind, read_marker)
    }

    pub fn channel_joined(
        &mut self,
        server: Server,
        channel: String,
    ) -> Option<impl Future<Output = Message>> {
        self.data.channel_joined(server, channel)
    }

    pub fn get_messages(
        &self,
        kind: &history::Kind,
        limit: Option<Limit>,
        buffer_config: &config::Buffer,
    ) -> Option<history::View<'_>> {
        self.data.history_view(kind, limit, buffer_config)
    }

    pub fn get_unique_queries(&self, server: &Server) -> Vec<&Nick> {
        let queries = self
            .data
            .map
            .keys()
            .filter_map(|kind| match kind {
                history::Kind::Query(s, user) => (s == server).then_some(user),
                _ => None,
            })
            .collect::<Vec<_>>();

        queries
    }

    pub fn has_unread(&self, kind: &history::Kind) -> bool {
        self.data
            .map
            .get(kind)
            .map(|history| history.has_unread())
            .unwrap_or_default()
    }

    pub fn read_marker(&self, kind: &history::Kind) -> Option<history::ReadMarker> {
        self.data
            .map
            .get(kind)
            .map(|history| history.read_marker())
            .unwrap_or_default()
    }

    pub fn broadcast(
        &mut self,
        server: &Server,
        broadcast: Broadcast,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) -> Vec<impl Future<Output = Message>> {
        let channels = self
            .data
            .map
            .keys()
            .filter_map(|kind| {
                if let history::Kind::Channel(s, channel) = kind {
                    (s == server).then_some(channel)
                } else {
                    None
                }
            })
            .cloned();
        let mut queries = self
            .data
            .map
            .keys()
            .filter_map(|kind| {
                if let history::Kind::Query(s, nick) = kind {
                    (s == server).then_some(nick)
                } else {
                    None
                }
            })
            .cloned();

        let messages = match broadcast {
            Broadcast::Connecting => message::broadcast::connecting(sent_time),
            Broadcast::Connected => message::broadcast::connected(sent_time),
            Broadcast::ConnectionFailed { error } => {
                message::broadcast::connection_failed(error, sent_time)
            }
            Broadcast::Disconnected { error } => {
                message::broadcast::disconnected(channels, queries, error, sent_time)
            }
            Broadcast::Reconnected => message::broadcast::reconnected(channels, queries, sent_time),
            Broadcast::Quit {
                user,
                comment,
                user_channels,
            } => {
                let user_query = queries.find(|nick| user.nickname() == *nick);

                message::broadcast::quit(
                    user_channels,
                    user_query,
                    &user,
                    &comment,
                    config,
                    sent_time,
                )
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
                        sent_time,
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
                        sent_time,
                    )
                }
            }
            Broadcast::Invite {
                inviter,
                channel,
                user_channels,
            } => message::broadcast::invite(inviter, channel, user_channels, sent_time),
            Broadcast::ChangeHost {
                old_user,
                new_username,
                new_hostname,
                ourself,
                user_channels,
            } => {
                if ourself {
                    // If ourself, broadcast to all query channels (since we are in all of them)
                    message::broadcast::change_host(
                        user_channels,
                        queries,
                        &old_user,
                        &new_username,
                        &new_hostname,
                        ourself,
                        sent_time,
                    )
                } else {
                    // Otherwise just the query channel of the user w/ host change
                    let user_query = queries.find(|nick| old_user.nickname() == *nick);
                    message::broadcast::change_host(
                        user_channels,
                        user_query,
                        &old_user,
                        &new_username,
                        &new_hostname,
                        ourself,
                        sent_time,
                    )
                }
            }
        };

        messages
            .into_iter()
            .filter_map(|message| self.record_message(server, message))
            .collect()
    }

    pub fn input<'a>(&'a self, buffer: &buffer::Upstream) -> input::Cache<'a> {
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
    map: HashMap<history::Kind, History>,
    input: input::Storage,
}

impl Data {
    fn load_full(&mut self, kind: history::Kind, data: history::Loaded) {
        use std::collections::hash_map;

        let history::Loaded {
            mut messages,
            metadata,
        } = data;

        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => match entry.get_mut() {
                History::Partial {
                    messages: new_messages,
                    last_updated_at,
                    read_marker: partial_read_marker,
                    ..
                } => {
                    let read_marker = (*partial_read_marker).max(metadata.read_marker);

                    let last_updated_at = *last_updated_at;
                    messages.extend(std::mem::take(new_messages));
                    entry.insert(History::Full {
                        kind,
                        messages,
                        last_updated_at,
                        read_marker,
                    });
                }
                _ => {
                    entry.insert(History::Full {
                        kind,
                        messages,
                        last_updated_at: None,
                        read_marker: metadata.read_marker,
                    });
                }
            },
            hash_map::Entry::Vacant(entry) => {
                entry.insert(History::Full {
                    kind,
                    messages,
                    last_updated_at: None,
                    read_marker: metadata.read_marker,
                });
            }
        }
    }

    fn update_partial(&mut self, kind: history::Kind, data: history::Metadata) {
        if let Some(history) = self.map.get_mut(&kind) {
            history.update_partial(data);
        }
    }

    fn history_view(
        &self,
        kind: &history::Kind,
        limit: Option<Limit>,
        buffer_config: &config::Buffer,
    ) -> Option<history::View> {
        let History::Full {
            messages,
            read_marker,
            ..
        } = self.map.get(kind)?
        else {
            return None;
        };

        let mut most_recent_messages = HashMap::<Nick, DateTime<Utc>>::new();

        let filtered = messages
            .iter()
            .filter(|message| match message.target.source() {
                message::Source::Server(Some(source)) => {
                    if let Some(server_message) = buffer_config.server_messages.get(source) {
                        // Check if target is a channel, and if included/excluded.
                        if let message::Target::Channel { channel, .. } = &message.target {
                            if !server_message.should_send_message(channel.as_ref()) {
                                return false;
                            }
                        }

                        if let Some(seconds) = server_message.smart {
                            let nick = match source.nick() {
                                Some(nick) => nick.clone(),
                                None => {
                                    if let Some(nickname) =
                                        message.plain().and_then(|s| s.split(' ').nth(1))
                                    {
                                        Nick::from(nickname)
                                    } else {
                                        return true;
                                    }
                                }
                            };

                            return !smart_filter_message(
                                message,
                                &seconds,
                                most_recent_messages.get(&nick),
                            );
                        }
                    }

                    true
                }
                crate::message::Source::User(message_user) => {
                    most_recent_messages
                        .insert(message_user.nickname().to_owned(), message.server_time);

                    true
                }
                message::Source::Internal(message::source::Internal::Status(status)) => {
                    if let Some(internal_message) = buffer_config.internal_messages.get(status) {
                        if !internal_message.enabled {
                            return false;
                        }

                        if let Some(seconds) = internal_message.smart {
                            return !smart_filter_internal_message(message, &seconds);
                        }
                    }

                    true
                }
                _ => true,
            })
            .collect::<Vec<_>>();

        let total = filtered.len();
        let with_access_levels = buffer_config.nickname.show_access_levels;

        let max_nick_chars = buffer_config.nickname.alignment.is_right().then(|| {
            filtered
                .iter()
                .filter_map(|message| {
                    if let message::Source::User(user) = message.target.source() {
                        Some(
                            buffer_config
                                .nickname
                                .brackets
                                .format(user.display(with_access_levels))
                                .chars()
                                .count(),
                        )
                    } else {
                        None
                    }
                })
                .max()
                .unwrap_or_default()
        });

        let max_prefix_chars = buffer_config.nickname.alignment.is_right().then(|| {
            if matches!(kind, history::Kind::Channel(..)) {
                filtered
                    .iter()
                    .filter_map(|message| {
                        message.target.prefixes().map(|prefixes| {
                            buffer_config
                                .status_message_prefix
                                .brackets
                                .format(prefixes.iter().collect::<String>())
                                .chars()
                                .count()
                                + 1
                        })
                    })
                    .max()
                    .unwrap_or_default()
            } else {
                0
            }
        });

        let has_read_messages = read_marker
            .map(|marker| {
                filtered
                    .iter()
                    .any(|message| message.server_time <= marker.date_time())
            })
            .unwrap_or_default();

        let limited = with_limit(limit, filtered.into_iter());

        let split_at = read_marker.map_or(0, |read_marker| {
            limited
                .iter()
                .rev()
                .position(|message| message.server_time <= read_marker.date_time())
                .map_or_else(
                    || {
                        // Backlog is before this limit view of messages
                        if has_read_messages {
                            0
                        } else {
                            limited.len()
                        }
                    },
                    |position| limited.len() - position,
                )
        });

        let (old, new) = limited.split_at(split_at);

        Some(history::View {
            total,
            old_messages: old.to_vec(),
            new_messages: new.to_vec(),
            max_nick_chars,
            max_prefix_chars,
        })
    }

    fn add_message(
        &mut self,
        kind: history::Kind,
        message: crate::Message,
    ) -> Option<impl Future<Output = Message>> {
        use std::collections::hash_map;

        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().add_message(message);

                None
            }
            hash_map::Entry::Vacant(entry) => {
                entry
                    .insert(History::partial(kind.clone()))
                    .add_message(message);

                Some(
                    async move {
                        let loaded = history::metadata::load(kind.clone()).await;

                        Message::UpdatePartial(kind, loaded)
                    }
                    .boxed(),
                )
            }
        }
    }

    fn update_read_marker(
        &mut self,
        kind: impl Into<history::Kind>,
        read_marker: history::ReadMarker,
    ) -> Option<impl Future<Output = Message>> {
        use std::collections::hash_map;

        let kind = kind.into();

        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().update_read_marker(read_marker);

                None
            }
            hash_map::Entry::Vacant(_) => Some(
                async move {
                    let updated = history::metadata::update(&kind, &read_marker).await;

                    Message::UpdateReadMarker(kind, read_marker, updated)
                }
                .boxed(),
            ),
        }
    }

    fn channel_joined(
        &mut self,
        server: server::Server,
        channel: String,
    ) -> Option<impl Future<Output = Message>> {
        use std::collections::hash_map;

        let kind = history::Kind::Channel(server, channel);

        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(_) => None,
            hash_map::Entry::Vacant(entry) => {
                entry.insert(History::partial(kind.clone()));

                Some(
                    async move {
                        let loaded = history::metadata::load(kind.clone()).await;

                        Message::UpdatePartial(kind, loaded)
                    }
                    .boxed(),
                )
            }
        }
    }

    fn untrack(
        &mut self,
        kind: &history::Kind,
    ) -> Option<impl Future<Output = Result<Option<history::ReadMarker>, history::Error>>> {
        self.map.get_mut(kind).and_then(History::make_partial)
    }

    fn flush_all(&mut self, now: Instant) -> Vec<BoxFuture<'static, Message>> {
        self.map
            .iter_mut()
            .filter_map(|(kind, state)| {
                let kind = kind.clone();

                state.flush(now).map(move |task| {
                    task.map(move |result| Message::Flushed(kind, result))
                        .boxed()
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

fn smart_filter_internal_message(message: &crate::Message, seconds: &i64) -> bool {
    let current_time = Utc::now();

    let duration_seconds = current_time
        .signed_duration_since(message.server_time)
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
    ChangeHost {
        old_user: User,
        new_username: String,
        new_hostname: String,
        ourself: bool,
        user_channels: Vec<String>,
    },
}
