use std::collections::{HashMap, HashSet, hash_map};

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::{Future, FutureExt, future};
use tokio::time::Instant;

use super::filter::{Filter, FilterChain};
use crate::history::{self, History, MessageReferences, ReadMarker};
use crate::message::{self, Hash, Limit};
use crate::target::{self, Target};
use crate::user::{ChannelUsers, Nick};
use crate::{
    Config, Input, Server, User, buffer, config, input, isupport, server,
};

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
    SentMessageUpdated(history::Kind, history::ReadMarker),
}

pub enum Event {
    Loaded(history::Kind),
    Closed(history::Kind, Option<history::ReadMarker>),
    Exited(Vec<(history::Kind, Option<history::ReadMarker>)>),
    SentMessageUpdated(history::Kind, history::ReadMarker),
}

#[derive(Debug, Default)]
pub struct Manager {
    resources: HashSet<Resource>,
    filters: Vec<Filter>,
    data: Data,
}

impl Manager {
    pub fn clear_messages(
        &mut self,
        kind: history::Kind,
    ) -> Option<BoxFuture<'static, Message>> {
        if let Some(history) = self.data.map.get_mut(&kind) {
            let task = history.flush(None);

            match history {
                History::Full {
                    messages, cleared, ..
                } => {
                    messages.clear();
                    *cleared = true;
                }
                History::Partial { messages, .. } => {
                    messages.clear();
                }
            }

            self.data.blocked_messages_index.remove(&kind);

            log::debug!("cleared messages for {kind}");

            return task.map(move |task| {
                task.map(move |result| Message::Flushed(kind, result))
                    .boxed()
            });
        }

        None
    }

    pub fn track(
        &mut self,
        new_resources: HashSet<Resource>,
        config: &Config,
    ) -> Vec<BoxFuture<'static, Message>> {
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
            self.data.untrack(&resource.kind, config).map(|task| {
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
                let len = loaded.messages.len();
                self.data.load_full(kind.clone(), loaded);
                log::debug!("loaded history for {kind}: {len} messages");

                if !self.data.has_blocked_message_cache(&kind) {
                    self.rebuild_blocked_message_cache(kind.clone());
                }

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
                log::warn!("failed to close history for {kind}: {error}");
            }
            Message::Flushed(kind, Ok(())) => {
                // Will cause flush loop if we emit a log every time we flush logs
                if !matches!(kind, history::Kind::Logs) {
                    log::debug!("flushed history for {kind}",);
                }
            }
            Message::Flushed(kind, Err(error)) => {
                log::warn!("failed to flush history for {kind}: {error}");
            }
            Message::UpdatePartial(kind, Ok(metadata)) => {
                log::debug!("loaded metadata for {kind}");
                self.data.update_partial(kind, metadata);
            }
            Message::UpdatePartial(kind, Err(error)) => {
                log::warn!("failed to load metadata for {kind}: {error}");
            }
            Message::UpdateReadMarker(kind, read_marker, Ok(())) => {
                log::debug!("updated read marker for {kind} to {read_marker}");
            }
            Message::UpdateReadMarker(kind, read_marker, Err(error)) => {
                log::warn!(
                    "failed to update read marker for {kind} to {read_marker}: {error}"
                );
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
                            log::warn!(
                                "failed to close history for {kind}: {error}"
                            );
                            output.push((kind, None));
                        }
                    }
                }

                return Some(Event::Exited(output));
            }
            Message::SentMessageUpdated(kind, read_marker) => {
                return Some(Event::SentMessageUpdated(kind, read_marker));
            }
        }

        None
    }

    pub fn set_filters(&mut self, mut new_filters: Vec<Filter>) {
        self.filters.clear();
        self.filters.append(&mut new_filters);
        self.data.clear_blocked_message_cache();
        log::debug!(
            "set new filters to history manager, reset all cached channel flags."
        );
    }

    pub fn get_filters(&mut self) -> &mut Vec<Filter> {
        &mut self.filters
    }

    pub fn tick(&mut self, now: Instant) -> Vec<BoxFuture<'static, Message>> {
        self.data.flush_all(now)
    }

    pub fn close(
        &mut self,
        kind: history::Kind,
        mark_as_read: bool,
    ) -> Option<impl Future<Output = Message> + use<>> {
        let history = self.data.map.remove(&kind)?;

        Some(
            history
                .close(mark_as_read)
                .map(|result| Message::Closed(kind, result)),
        )
    }

    pub fn exit(
        &mut self,
        mark_partial_as_read: bool,
        mark_full_as_read: bool,
    ) -> impl Future<Output = Message> + use<> {
        let map = std::mem::take(&mut self.data).map;

        async move {
            let tasks = map.into_iter().map(|(kind, state)| {
                match state {
                    History::Partial { .. } => {
                        state.close(mark_partial_as_read)
                    }
                    History::Full { .. } => state.close(mark_full_as_read),
                }
                .map(move |result| (kind, result))
            });

            Message::Exited(future::join_all(tasks).await)
        }
    }

    pub fn record_input_message(
        &mut self,
        input: Input,
        user: User,
        channel_users: Option<&ChannelUsers>,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
        config: &Config,
    ) -> Vec<BoxFuture<'static, Message>> {
        let mut tasks = vec![];

        if let Some(messages) = input.messages(
            user,
            channel_users,
            chantypes,
            statusmsg,
            casemapping,
            config,
        ) {
            for message in messages {
                if config.buffer.mark_as_read.on_message_sent
                    && let Some(kind) = history::Kind::from_server_message(
                        input.server().clone(),
                        &message,
                    )
                {
                    tasks.extend(
                        self.update_read_marker(
                            kind,
                            history::ReadMarker::from_date_time(
                                message.server_time,
                            ),
                        )
                        .map(futures::FutureExt::boxed),
                    );
                }

                tasks.extend(
                    self.record_message(input.server(), message)
                        .map(futures::FutureExt::boxed),
                );
            }
        }

        tasks
    }

    pub fn record_input_history(
        &mut self,
        buffer: &buffer::Upstream,
        text: String,
    ) {
        self.data.input.record(buffer, text);
    }

    pub fn record_draft(&mut self, raw_input: input::RawInput) {
        self.data.input.store_draft(raw_input);
    }

    pub fn record_text(&mut self, raw_input: input::RawInput) {
        self.data.input.store_text(raw_input);
    }

    pub fn record_message(
        &mut self,
        server: &Server,
        message: crate::Message,
    ) -> Option<impl Future<Output = Message> + use<>> {
        history::Kind::from_server_message(server.clone(), &message).and_then(
            |kind| {
                let blocked = FilterChain::borrow(&self.filters)
                    .filter_message_of_kind(&message, &kind);

                self.data.add_message(kind, message, blocked)
            },
        )
    }

    pub fn record_log(
        &mut self,
        record: crate::log::Record,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.data.add_message(
            history::Kind::Logs,
            crate::Message::log(record),
            false,
        )
    }

    pub fn record_highlight(
        &mut self,
        message: crate::Message,
    ) -> Option<impl Future<Output = Message> + use<>> {
        let blocked = FilterChain::borrow(&self.filters)
            .filter_message_of_kind(&message, &history::Kind::Highlights);

        self.data
            .add_message(history::Kind::Highlights, message, blocked)
    }

    pub fn update_read_marker<T: Into<history::Kind>>(
        &mut self,
        kind: T,
        read_marker: history::ReadMarker,
    ) -> Option<impl Future<Output = Message> + use<T>> {
        self.data.update_read_marker(kind, read_marker)
    }

    pub fn load_metadata(
        &mut self,
        server: Server,
        target: Target,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.data.load_metadata(server, target)
    }

    pub fn first_can_reference(
        &self,
        server: Server,
        target: Target,
    ) -> Option<&crate::Message> {
        self.data.first_can_reference(server, target)
    }

    pub fn last_can_reference_before(
        &self,
        server: Server,
        target: Target,
        server_time: DateTime<Utc>,
    ) -> Option<MessageReferences> {
        self.data
            .last_can_reference_before(server, target, server_time)
    }

    pub fn mark_as_read(&mut self, kind: &history::Kind) -> Option<ReadMarker> {
        self.data.mark_as_read(kind)
    }

    pub fn can_mark_as_read(&self, kind: &history::Kind) -> bool {
        self.data.can_mark_as_read(kind)
    }

    pub fn get_messages(
        &self,
        kind: &history::Kind,
        limit: Option<Limit>,
        buffer_config: &config::Buffer,
    ) -> Option<history::View<'_>> {
        self.data.history_view(kind, limit, buffer_config)
    }

    pub fn get_last_seen(
        &self,
        buffer: &buffer::Upstream,
    ) -> HashMap<Nick, DateTime<Utc>> {
        let kind = history::Kind::from_input_buffer(buffer.clone());

        self.data
            .map
            .get(&kind)
            .map(History::last_seen)
            .unwrap_or_default()
    }

    pub fn get_unique_queries(&self, server: &Server) -> Vec<&target::Query> {
        self.data
            .map
            .keys()
            .filter_map(|kind| match kind {
                #[allow(clippy::bool_comparison)] // easy to miss exclaimation
                history::Kind::Query(s, query) => (s == server
                    && self
                        .filters
                        .iter()
                        .all(|filter| filter.match_query(query) == false))
                .then_some(query),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub fn server_kinds(&self, server: Server) -> Vec<history::Kind> {
        self.data
            .map
            .iter()
            .filter_map(|(kind, _)| {
                if kind.server().is_some_and(|s| *s == server) {
                    Some(kind.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn server_has_unread(&self, server: Server) -> bool {
        self.data
            .map
            .iter()
            .filter_map(|(kind, history)| {
                if kind.server().is_some_and(|s| *s == server) {
                    Some(history)
                } else {
                    None
                }
            })
            .any(History::has_unread)
    }

    pub fn has_unread(&self, kind: &history::Kind) -> bool {
        self.data.map.get(kind).is_some_and(History::has_unread)
    }

    pub fn read_marker(
        &self,
        kind: &history::Kind,
    ) -> Option<history::ReadMarker> {
        self.data
            .map
            .get(kind)
            .map(History::read_marker)
            .unwrap_or_default()
    }

    pub fn broadcast(
        &mut self,
        server: &Server,
        broadcast: Broadcast,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) -> Vec<impl Future<Output = Message> + use<>> {
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
                message::broadcast::disconnected(
                    channels, queries, error, sent_time,
                )
            }
            Broadcast::Reconnected => {
                message::broadcast::reconnected(channels, queries, sent_time)
            }
            Broadcast::Quit {
                user,
                comment,
                user_channels,
            } => {
                let user_query =
                    queries.find(|query| user.as_str() == query.as_str());

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
                    let user_query = queries
                        .find(|query| old_nick.as_ref() == query.as_str());
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
            } => message::broadcast::invite(
                inviter,
                channel,
                user_channels,
                sent_time,
            ),
            Broadcast::ChangeHost {
                old_user,
                new_username,
                new_hostname,
                ourself,
                logged_in,
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
                        logged_in,
                        sent_time,
                    )
                } else {
                    // Otherwise just the query channel of the user w/ host change
                    let user_query = queries
                        .find(|query| old_user.as_str() == query.as_str());
                    message::broadcast::change_host(
                        user_channels,
                        user_query,
                        &old_user,
                        &new_username,
                        &new_hostname,
                        ourself,
                        logged_in,
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

    pub fn hide_preview(
        &mut self,
        kind: impl Into<history::Kind>,
        message: message::Hash,
        url: url::Url,
    ) {
        self.data.hide_preview(&kind.into(), message, url);
    }

    pub fn rebuild_blocked_message_cache(&mut self, kind: history::Kind) {
        let chain = FilterChain::borrow(&self.filters);

        if let Some(history) = self.data.map.get(&kind) {
            let messages = match history {
                History::Full { messages, .. } => messages,
                History::Partial { messages, .. } => messages,
            };

            let blocked_message_index = messages
                .iter()
                .filter_map(|message| {
                    chain
                        .filter_message_of_kind(message, &kind)
                        .then_some(message.hash)
                })
                .collect();

            self.data
                .blocked_messages_index
                .entry(kind.clone())
                .insert_entry(blocked_message_index);
        };
        log::debug!("rebuilt blocked message cache for {kind}");
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
            .skip_while(|message| message.server_time < timestamp)
            .collect(),
        None => messages.collect(),
    }
}

#[derive(Debug, Default)]
struct Data {
    map: HashMap<history::Kind, History>,
    pub blocked_messages_index: HashMap<history::Kind, HashSet<Hash>>,
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
                    last_seen,
                    ..
                } => {
                    let read_marker =
                        (*partial_read_marker).max(metadata.read_marker);

                    let last_updated_at = *last_updated_at;

                    let mut last_seen = last_seen.clone();

                    std::mem::take(new_messages).into_iter().for_each(
                        |message| {
                            history::update_last_seen(&mut last_seen, &message);

                            history::insert_message(&mut messages, message);
                        },
                    );

                    entry.insert(History::Full {
                        kind,
                        messages,
                        last_updated_at,
                        read_marker,
                        last_seen,
                        cleared: false,
                    });
                }
                _ => {
                    let last_seen = history::get_last_seen(&messages);

                    entry.insert(History::Full {
                        kind,
                        messages,
                        last_updated_at: None,
                        read_marker: metadata.read_marker,
                        last_seen,
                        cleared: false,
                    });
                }
            },
            hash_map::Entry::Vacant(entry) => {
                let last_seen = history::get_last_seen(&messages);

                entry.insert(History::Full {
                    kind,
                    messages,
                    last_updated_at: None,
                    read_marker: metadata.read_marker,
                    last_seen,
                    cleared: false,
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
    ) -> Option<history::View<'_>> {
        let History::Full {
            messages,
            read_marker,
            cleared,
            ..
        } = self.map.get(kind)?
        else {
            return None;
        };

        let mut last_seen = HashMap::<Nick, DateTime<Utc>>::new();

        let filtered = messages
            .iter()
            .filter(|message| {
                !self
                    .blocked_messages_index
                    .get(kind)
                    .is_some_and(|blocklist| blocklist.contains(&message.hash))
            })
            .filter(|message| match message.target.source() {
                message::Source::Server(Some(source)) => {
                    if let Some(server_message) =
                        buffer_config.server_messages.get(source)
                    {
                        // Check if target is a channel, and if included/excluded.
                        if let message::Target::Channel { channel, .. } =
                            &message.target
                            && !server_message
                                .should_send_message(channel.as_str())
                        {
                            return false;
                        }

                        if let Some(seconds) = server_message.smart {
                            let nick = match source.nick() {
                                Some(nick) => nick.clone(),
                                None => {
                                    if let Some(nickname) = message
                                        .plain()
                                        .and_then(|s| s.split(' ').nth(1))
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
                                last_seen.get(&nick),
                            );
                        }
                    }

                    true
                }
                crate::message::Source::User(message_user) => {
                    last_seen.insert(
                        message_user.nickname().to_owned(),
                        message.server_time,
                    );

                    true
                }
                message::Source::Internal(
                    message::source::Internal::Status(status),
                ) => {
                    if let Some(internal_message) =
                        buffer_config.internal_messages.get(status)
                    {
                        if !internal_message.enabled {
                            return false;
                        }

                        if let Some(seconds) = internal_message.smart {
                            return !smart_filter_internal_message(
                                message, &seconds,
                            );
                        }
                    }

                    true
                }
                _ => true,
            })
            .collect::<Vec<_>>();

        let total = filtered.len();
        let with_access_levels = buffer_config.nickname.show_access_levels;
        let truncate = buffer_config.nickname.truncate;

        let max_nick_chars =
            buffer_config.nickname.alignment.is_right().then(|| {
                filtered
                    .iter()
                    .filter_map(|message| {
                        if let message::Source::User(user) =
                            message.target.source()
                        {
                            Some(
                                buffer_config
                                    .nickname
                                    .brackets
                                    .format(
                                        user.display(
                                            with_access_levels,
                                            truncate,
                                        ),
                                    )
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

        let max_prefix_chars =
            buffer_config.nickname.alignment.is_right().then(|| {
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

        let first_without_limit = filtered.first().copied();
        let last_without_limit = filtered.last().copied();

        let limited = with_limit(limit, filtered.into_iter());

        let first_with_limit = limited.first();
        let last_with_limit = limited.last();

        let split_at = read_marker.map_or(0, |read_marker| {
            limited
                .iter()
                .rev()
                .position(|message| {
                    message.server_time <= read_marker.date_time()
                })
                .map_or_else(
                    || {
                        // Backlog is before this limit view of messages
                        if has_read_messages { 0 } else { limited.len() }
                    },
                    |position| limited.len() - position,
                )
        });

        let (old, new) = limited.split_at(split_at);

        let has_more_older_messages = first_without_limit
            .zip(first_with_limit)
            .is_some_and(|(without_limit, with_limit)| {
                without_limit.server_time < with_limit.server_time
            });
        let has_more_newer_messages = last_without_limit
            .zip(last_with_limit)
            .is_some_and(|(without_limit, with_limit)| {
                without_limit.server_time > with_limit.server_time
            });

        Some(history::View {
            total,
            has_more_older_messages,
            has_more_newer_messages,
            old_messages: old.to_vec(),
            new_messages: new.to_vec(),
            max_nick_chars,
            max_prefix_chars,
            cleared: *cleared,
        })
    }

    fn add_message(
        &mut self,
        kind: history::Kind,
        message: crate::Message,
        blocked: bool,
    ) -> Option<impl Future<Output = Message> + use<>> {
        if blocked {
            self.blocked_messages_index
                .entry(kind.clone())
                .and_modify(|cache| {
                    cache.insert(message.hash);
                })
                .or_insert_with(|| {
                    let mut new_cache = HashSet::new();
                    new_cache.insert(message.hash);
                    new_cache
                });
        }

        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                let read_marker = entry.get_mut().add_message(message, blocked);

                read_marker.map(|read_marker| {
                    async move {
                        Message::SentMessageUpdated(kind.clone(), read_marker)
                    }
                    .boxed()
                })
            }
            hash_map::Entry::Vacant(entry) => {
                let _ = entry
                    .insert(History::partial(kind.clone()))
                    .add_message(message, blocked);

                Some(
                    async move {
                        let loaded =
                            history::metadata::load(kind.clone()).await;
                        Message::UpdatePartial(kind, loaded)
                    }
                    .boxed(),
                )
            }
        }
    }

    fn update_read_marker<T: Into<history::Kind>>(
        &mut self,
        kind: T,
        read_marker: history::ReadMarker,
    ) -> Option<impl Future<Output = Message> + use<T>> {
        use std::collections::hash_map;

        let kind = kind.into();

        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().update_read_marker(read_marker);

                None
            }
            hash_map::Entry::Vacant(_) => Some(
                async move {
                    let updated =
                        history::metadata::update(&kind, &read_marker).await;

                    Message::UpdateReadMarker(kind, read_marker, updated)
                }
                .boxed(),
            ),
        }
    }

    fn load_metadata(
        &mut self,
        server: server::Server,
        target: Target,
    ) -> Option<impl Future<Output = Message> + use<>> {
        use std::collections::hash_map;

        let kind = history::Kind::from_target(server, target);

        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(_) => None,
            hash_map::Entry::Vacant(entry) => {
                entry.insert(History::partial(kind.clone()));

                Some(
                    async move {
                        let loaded =
                            history::metadata::load(kind.clone()).await;

                        Message::UpdatePartial(kind, loaded)
                    }
                    .boxed(),
                )
            }
        }
    }

    fn first_can_reference(
        &self,
        server: server::Server,
        target: Target,
    ) -> Option<&crate::Message> {
        let kind = history::Kind::from_target(server, target);

        self.map
            .get(&kind)
            .and_then(|history| history.first_can_reference())
    }

    fn last_can_reference_before(
        &self,
        server: Server,
        target: Target,
        server_time: DateTime<Utc>,
    ) -> Option<MessageReferences> {
        let kind = history::Kind::from_target(server, target);

        self.map
            .get(&kind)
            .and_then(|history| history.last_can_reference_before(server_time))
    }

    fn mark_as_read(&mut self, kind: &history::Kind) -> Option<ReadMarker> {
        self.map.get_mut(kind).and_then(History::mark_as_read)
    }

    fn can_mark_as_read(&self, kind: &history::Kind) -> bool {
        self.map.get(kind).is_some_and(History::can_mark_as_read)
    }

    fn clear_blocked_message_cache(&mut self) {
        self.blocked_messages_index.clear();
    }

    pub fn has_blocked_message_cache(&self, kind: &history::Kind) -> bool {
        self.blocked_messages_index.contains_key(kind)
    }

    fn untrack(
        &mut self,
        kind: &history::Kind,
        config: &Config,
    ) -> Option<
        impl Future<Output = Result<Option<history::ReadMarker>, history::Error>>
        + use<>,
    > {
        self.map.get_mut(kind).and_then(|history| {
            History::make_partial(
                history,
                config.buffer.mark_as_read.on_buffer_close,
            )
        })
    }

    fn flush_all(&mut self, now: Instant) -> Vec<BoxFuture<'static, Message>> {
        self.map
            .iter_mut()
            .filter_map(|(kind, state)| {
                let kind = kind.clone();

                state.flush(Some(now)).map(move |task| {
                    task.map(move |result| Message::Flushed(kind, result))
                        .boxed()
                })
            })
            .collect()
    }

    fn hide_preview(
        &mut self,
        kind: &history::Kind,
        message: message::Hash,
        url: url::Url,
    ) {
        if let Some(history) = self.map.get_mut(kind) {
            history.hide_preview(message, url);
        }
    }
}

fn smart_filter_message(
    message: &crate::Message,
    seconds: &i64,
    last_seen_server_time: Option<&DateTime<Utc>>,
) -> bool {
    let Some(server_time) = last_seen_server_time else {
        return true;
    };

    let duration_seconds = message
        .server_time
        .signed_duration_since(*server_time)
        .num_seconds();

    duration_seconds > *seconds
}

fn smart_filter_internal_message(
    message: &crate::Message,
    seconds: &i64,
) -> bool {
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
        user_channels: Vec<target::Channel>,
    },
    Nickname {
        old_nick: Nick,
        new_nick: Nick,
        ourself: bool,
        user_channels: Vec<target::Channel>,
    },
    Invite {
        inviter: Nick,
        channel: target::Channel,
        user_channels: Vec<target::Channel>,
    },
    ChangeHost {
        old_user: User,
        new_username: String,
        new_hostname: String,
        ourself: bool,
        logged_in: bool,
        user_channels: Vec<target::Channel>,
    },
}
