use std::collections::{HashMap, HashSet, hash_map};

use chrono::{DateTime, Local, NaiveDate, Utc};
use futures::future::BoxFuture;
use futures::{Future, FutureExt, future};
use itertools::Itertools;
use tokio::time::Instant;

use super::filter::{Filter, FilterChain};
use crate::history::{self, History, MessageReferences, ReadMarker};
use crate::message::broadcast::{self, Broadcast};
use crate::message::{self, Limit};
use crate::target::{self, Target};
use crate::user::{ChannelUsers, Nick};
use crate::{
    Config, Input, Server, User, buffer, client, config, input, isupport,
    server,
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
    Closed(history::Kind, Result<(), history::Error>),
    Flushed(history::Kind, Result<(), history::Error>),
    Exited(Vec<(history::Kind, Result<(), history::Error>)>),
    SentMessageUpdated(history::Kind, history::ReadMarker),
}

pub enum Event {
    Loaded(history::Kind),
    Exited,
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
        clients: &client::Map,
    ) -> Option<BoxFuture<'static, Message>> {
        if let Some(history) = self.data.map.get_mut(&kind) {
            let task = history.flush(None, clients.get_seed(&kind));

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
        clients: Option<&client::Map>,
    ) -> Vec<BoxFuture<'static, Message>> {
        let added = new_resources.difference(&self.resources).cloned();
        let removed = self.resources.difference(&new_resources).cloned();

        let added = added.into_iter().map(|resource| {
            let seed =
                clients.and_then(|clients| clients.get_seed(&resource.kind));

            async move {
                history::load(resource.kind.clone(), seed)
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

    pub fn update(
        &mut self,
        message: Message,
        clients: &client::Map,
        buffer_config: &config::Buffer,
    ) -> Option<Event> {
        match message {
            Message::LoadFull(kind, Ok(loaded)) => {
                let len = loaded.messages.len();
                self.data.load_full(kind.clone(), loaded);
                log::debug!("loaded history for {kind}: {len} messages");

                self.process_messages(kind.clone(), clients, buffer_config);

                return Some(Event::Loaded(kind));
            }
            Message::LoadFull(kind, Err(error)) => {
                log::warn!("failed to load history for {kind}: {error}");
            }
            Message::Closed(kind, Ok(())) => {
                log::debug!("closed history for {kind}",);
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
                for (kind, result) in results {
                    match result {
                        Ok(()) => {
                            log::debug!("closed history for {kind}",);
                        }
                        Err(error) => {
                            log::warn!(
                                "failed to close history for {kind}: {error}"
                            );
                        }
                    }
                }

                return Some(Event::Exited);
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
        log::debug!(
            "set new filters to history manager, reset all cached channel flags."
        );
    }

    pub fn get_filters(&mut self) -> &mut Vec<Filter> {
        &mut self.filters
    }

    pub fn tick(
        &mut self,
        now: Instant,
        clients: &client::Map,
    ) -> Vec<BoxFuture<'static, Message>> {
        self.data.flush_all(now, clients)
    }

    pub fn close(
        &mut self,
        kind: history::Kind,
        clients: &client::Map,
    ) -> Option<impl Future<Output = Message> + use<>> {
        let history = self.data.map.remove(&kind)?;

        Some(
            history
                .close(clients.get_seed(&kind))
                .map(|result| Message::Closed(kind, result)),
        )
    }

    pub fn exit(
        &mut self,
        clients: &client::Map,
    ) -> impl Future<Output = Message> + use<> {
        let map = std::mem::take(&mut self.data).map;
        let seeds: Vec<Option<history::Seed>> =
            map.keys().map(|kind| clients.get_seed(kind)).collect();
        let seeded_map = map.into_iter().zip(seeds);

        async move {
            let tasks = seeded_map.into_iter().map(|((kind, state), seed)| {
                state.close(seed).map(move |result| (kind, result))
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
                    self.record_message(
                        input.server(),
                        casemapping,
                        message,
                        &config.buffer,
                    )
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
        casemapping: isupport::CaseMap,
        mut message: crate::Message,
        buffer_config: &config::Buffer,
    ) -> Option<impl Future<Output = Message> + use<>> {
        history::Kind::from_server_message(server.clone(), &message).and_then(
            |kind| {
                self.block_message(
                    &mut message,
                    &kind,
                    casemapping,
                    buffer_config,
                );

                let condensers = (message
                    .can_condense(&buffer_config.server_messages.condense)
                    && !message.blocked)
                    .then_some((kind.clone(), message.clone()));

                let future = self.data.add_message(kind, message);

                if let Some((kind, message)) = condensers {
                    self.condense_message(
                        message,
                        &kind,
                        &buffer_config.server_messages.condense,
                    );
                }

                future
            },
        )
    }

    pub fn record_log(
        &mut self,
        record: crate::log::Record,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.data
            .add_message(history::Kind::Logs, crate::Message::log(record))
    }

    // Unlike record_message, the message's blocked status should be determined
    // before recording a highlight in order to block highlight notifications.
    pub fn record_highlight(
        &mut self,
        message: crate::Message,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.data.add_message(history::Kind::Highlights, message)
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

    pub fn kinds(&self) -> Vec<history::Kind> {
        self.data.map.keys().cloned().collect()
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

    pub fn has_highlight(&self, kind: &history::Kind) -> bool {
        self.data.map.get(kind).is_some_and(History::has_highlight)
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
        casemapping: isupport::CaseMap,
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
        let queries = self
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

        let messages = broadcast::into_messages(
            broadcast, config, sent_time, channels, queries,
        );

        messages
            .into_iter()
            .filter_map(|message| {
                self.record_message(
                    server,
                    casemapping,
                    message,
                    &config.buffer,
                )
            })
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

    pub fn block_message(
        &self,
        message: &mut crate::Message,
        kind: &history::Kind,
        casemapping: isupport::CaseMap,
        buffer_config: &config::Buffer,
    ) {
        message.blocked = false;

        if let message::Source::Server(Some(source)) = message.target.source()
            && let Some(server_message) =
                buffer_config.server_messages.get(source)
        {
            // Check if target is a channel, and if included/excluded.
            if let message::Target::Channel { channel, .. } = &message.target
                && !server_message.should_send_message(channel.as_str())
            {
                message.blocked = true;
                return;
            }

            if let Some(seconds) = server_message.smart {
                let nick = match source.nick() {
                    Some(nick) => Some(nick.clone()),
                    None => message.plain().and_then(|s| {
                        s.split(' ')
                            .nth(1)
                            .map(|nick| Nick::from_str(nick, casemapping))
                    }),
                };

                if let Some(nick) = nick
                    && let Some(history) = self.data.map.get(kind)
                {
                    let messages = match history {
                        History::Full { messages, .. } => messages,
                        History::Partial { messages, .. } => messages,
                    };

                    message.blocked = messages
                        .iter()
                        .rev()
                        .find_map(|historical_message| {
                            if let crate::message::Source::User(
                                historical_message_user,
                            ) = historical_message.target.source()
                                && historical_message_user.nickname() == nick
                            {
                                return Some(smart_filter_message(
                                    message,
                                    &seconds,
                                    Some(&historical_message.server_time),
                                ));
                            }

                            if smart_filter_message(
                                message,
                                &seconds,
                                Some(&historical_message.server_time),
                            ) {
                                return Some(true);
                            }

                            None
                        })
                        .unwrap_or(true);
                }
            }
        }

        if message.blocked {
            return;
        }

        FilterChain::borrow(&self.filters)
            .filter_message_of_kind(message, kind);
    }

    // Whether the message can & should be condensed should be determined prior
    // to calling this function
    pub fn condense_message(
        &mut self,
        message: crate::Message,
        kind: &history::Kind,
        config: &config::buffer::Condensation,
    ) {
        if let Some(History::Full { messages, .. }) =
            self.data.map.get_mut(kind)
        {
            let fuzz_seconds = chrono::Duration::seconds(1);

            let start = message.server_time - fuzz_seconds;
            let end = message.server_time + fuzz_seconds;

            let start_index = match messages
                .binary_search_by(|stored| stored.server_time.cmp(&start))
            {
                Ok(match_index) => match_index,
                Err(sorted_insert_index) => sorted_insert_index,
            };
            let end_index = match messages
                .binary_search_by(|stored| stored.server_time.cmp(&end))
            {
                Ok(match_index) => match_index,
                Err(sorted_insert_index) => sorted_insert_index,
            };

            if let Some(insert_position) = messages[start_index..end_index]
                .iter()
                .position(|stored| stored.hash == message.hash)
                .map(|position| position + start_index)
            {
                let start = messages
                    .iter()
                    .take(insert_position)
                    .rev()
                    .position(|message| {
                        !message.blocked && !message.can_condense(config)
                    })
                    .map_or(0, |position| insert_position - position);

                let end = messages
                    .iter()
                    .skip(insert_position)
                    .position(|message| {
                        !message.blocked && !message.can_condense(config)
                    })
                    .map_or(messages.len(), |position| {
                        insert_position + position
                    });

                let mut condensable_messages = messages[start..end]
                    .iter_mut()
                    .collect::<Vec<&mut message::Message>>(
                );

                let condensed_message = message::condense(
                    &condensable_messages
                        .iter()
                        .map(|message| &**message)
                        .collect::<Vec<&message::Message>>(),
                    config,
                );

                condensable_messages
                    .iter_mut()
                    .for_each(|message| message.condensed = None);

                if let Some(first_message) = condensable_messages.first_mut() {
                    first_message.condensed = condensed_message;
                }
            }
        }
    }

    // Block and condense messages
    pub fn process_messages(
        &mut self,
        kind: history::Kind,
        clients: &client::Map,
        buffer_config: &config::Buffer,
    ) {
        #[derive(PartialEq)]
        enum CondensationKey {
            Condensable(NaiveDate),
            Singular,
        }

        if let Some(history) = self.data.map.get_mut(&kind) {
            let messages = match history {
                History::Full { messages, .. } => messages,
                History::Partial { messages, .. } => messages,
            };

            let mut last_seen = HashMap::<Nick, DateTime<Utc>>::new();

            messages.iter_mut().for_each(|message| {
                message.blocked = false;

                match message.target.source() {
                    message::Source::Server(Some(source)) => {
                        if let Some(server_message) =
                            buffer_config.server_messages.get(source)
                        {
                            // Check if target is a channel, and if included/excluded.
                            if let message::Target::Channel { channel, .. }
                            | message::Target::Highlights {
                                channel, ..
                            } = &message.target
                                && !server_message
                                    .should_send_message(channel.as_str())
                            {
                                message.blocked = true;
                            } else if let Some(seconds) = server_message.smart {
                                let nick = match source.nick() {
                                    Some(nick) => Some(nick.clone()),
                                    None => message.plain().and_then(|s| {
                                        let server = if let Some(server) = kind.server() {
                                            Some(server)
                                        } else if let message::Target::Highlights { server, .. } =
                                            &message.target
                                        {
                                            Some(server)
                                        } else {
                                            None
                                        };

                                        let casemapping = clients
                                            .get_casemapping_or_default(server);

                                        s.split(' ').nth(1).map(|nick| {
                                            Nick::from_str(nick, casemapping)
                                        })
                                    }),
                                };

                                if let Some(nick) = nick {
                                    message.blocked = smart_filter_message(
                                        message,
                                        &seconds,
                                        last_seen.get(&nick),
                                    );
                                }
                            }
                        }
                    }
                    crate::message::Source::User(message_user) => {
                        last_seen.insert(
                            message_user.nickname().to_owned(),
                            message.server_time,
                        );
                    }
                    message::Source::Internal(
                        message::source::Internal::Status(status),
                    ) => {
                        if let Some(internal_message) =
                            buffer_config.internal_messages.get(status)
                        {
                            if !internal_message.enabled {
                                message.blocked = true;
                            } else if let Some(seconds) = internal_message.smart
                            {
                                message.blocked = smart_filter_internal_message(
                                    message, &seconds,
                                );
                            }
                        }
                    }
                    _ => (),
                }
            });

            let chain = FilterChain::borrow(&self.filters);

            messages.iter_mut().for_each(|message| {
                if message.blocked {
                    return;
                }

                chain.filter_message_of_kind(message, &kind);
            });

            messages
                .iter_mut()
                .filter(|message| !message.blocked)
                .chunk_by(|message| {
                    if message
                        .can_condense(&buffer_config.server_messages.condense)
                    {
                        CondensationKey::Condensable(
                            message
                                .server_time
                                .with_timezone(&Local)
                                .date_naive(),
                        )
                    } else {
                        CondensationKey::Singular
                    }
                })
                .into_iter()
                .for_each(|(key, chunk)| match key {
                    CondensationKey::Condensable(_) => {
                        let mut condensable_messages =
                            chunk.collect::<Vec<&mut message::Message>>();

                        let condensed_message = message::condense(
                            &condensable_messages
                                .iter()
                                .map(|message| &**message)
                                .collect::<Vec<&message::Message>>(),
                            &buffer_config.server_messages.condense,
                        );

                        condensable_messages
                            .iter_mut()
                            .for_each(|message| message.condensed = None);

                        if let Some(first_message) =
                            condensable_messages.first_mut()
                        {
                            first_message.condensed = condensed_message;
                        }
                    }
                    CondensationKey::Singular => (),
                });
        }

        log::debug!("processed messages in {kind}");
    }

    pub fn renormalize_messages(
        &mut self,
        kind: &history::Kind,
        clients: &client::Map,
    ) {
        if let Some(history) = self.data.map.get_mut(kind)
            && let Some(seed) = clients.get_seed(kind)
        {
            let messages = match history {
                History::Full { messages, .. } => messages,
                History::Partial { messages, .. } => messages,
            };

            history::renormalize_messages(messages, seed);
        }
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

        let processed = messages
            .iter()
            .flat_map(|message| {
                if message.blocked {
                    None
                } else if message
                    .can_condense(&buffer_config.server_messages.condense)
                {
                    message.condensed.as_ref().map(std::convert::AsRef::as_ref)
                } else {
                    match message.target.source() {
                        message::Source::Internal(
                            message::source::Internal::Status(status),
                        ) => {
                            if let Some(internal_message) =
                                buffer_config.internal_messages.get(status)
                            {
                                if !internal_message.enabled {
                                    return None;
                                } else if let Some(seconds) =
                                    internal_message.smart
                                {
                                    return (!smart_filter_internal_message(
                                        message, &seconds,
                                    ))
                                    .then_some(message);
                                }
                            }

                            Some(message)
                        }
                        _ => Some(message),
                    }
                }
            })
            .collect::<Vec<_>>();

        let total = processed.len();
        let with_access_levels = buffer_config.nickname.show_access_levels;
        let truncate = buffer_config.nickname.truncate;

        let max_nick_chars =
            buffer_config.nickname.alignment.is_right().then(|| {
                processed
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
                    processed
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

        // The right-aligned nicknames setting expects timestamps to have a
        // constant character count to function, so we can utilize that
        // expectation in this calculation
        let max_excess_timestamp_chars = (buffer_config
            .nickname
            .alignment
            .is_right()
            && buffer_config.server_messages.condense.any())
        .then(|| {
            processed
                .iter()
                .find_map(|message| {
                    if let message::Source::Internal(
                        message::source::Internal::Condensed(end_server_time),
                    ) = message.target.source()
                        && message.server_time != *end_server_time
                    {
                        Some(
                            buffer_config
                                .format_range_timestamp(
                                    &message.server_time,
                                    end_server_time,
                                )
                                .map(
                                    |(start_timestamp, dash, end_timestamp)| {
                                        start_timestamp.chars().count()
                                            + dash.chars().count()
                                            + end_timestamp.chars().count()
                                    },
                                )
                                .unwrap_or_default()
                                - buffer_config
                                    .format_timestamp(&message.server_time)
                                    .map(|timestamp| timestamp.chars().count())
                                    .unwrap_or_default(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        });

        let first_without_limit = processed.first().copied();
        let last_without_limit = processed.last().copied();

        let limited = with_limit(limit, processed.into_iter());

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
                    || 0, // Backlog is before this limit view of messages
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
            max_excess_timestamp_chars,
            cleared: *cleared,
        })
    }

    fn add_message(
        &mut self,
        kind: history::Kind,
        message: crate::Message,
    ) -> Option<impl Future<Output = Message> + use<>> {
        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                let read_marker = entry.get_mut().add_message(message);

                if let Some(read_marker) = read_marker {
                    // Update the read marker immediately so the split is correct
                    entry.get_mut().update_read_marker(read_marker);

                    Some(
                        async move {
                            Message::SentMessageUpdated(kind.clone(), read_marker)
                        }
                        .boxed(),
                    )
                } else {
                    None
                }
            }
            hash_map::Entry::Vacant(entry) => {
                let _ = entry
                    .insert(History::partial(kind.clone()))
                    .add_message(message);

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

    fn untrack(
        &mut self,
        kind: &history::Kind,
    ) -> Option<impl Future<Output = Result<(), history::Error>> + use<>> {
        self.map.get_mut(kind).and_then(History::make_partial)
    }

    fn flush_all(
        &mut self,
        now: Instant,
        clients: &client::Map,
    ) -> Vec<BoxFuture<'static, Message>> {
        self.map
            .iter_mut()
            .filter_map(|(kind, state)| {
                let kind = kind.clone();

                state.flush(Some(now), clients.get_seed(&kind)).map(
                    move |task| {
                        task.map(move |result| Message::Flushed(kind, result))
                            .boxed()
                    },
                )
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
