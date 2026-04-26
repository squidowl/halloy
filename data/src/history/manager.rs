use std::cmp::Ord;
use std::collections::{HashMap, HashSet, hash_map};
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, Utc};
use futures::future::BoxFuture;
use futures::{Future, FutureExt, future};
use itertools::Itertools;
use tokio::time::Instant;

use super::filter::{Filter, FilterChain};
use super::reroute::RerouteRules;
use crate::capabilities::LabeledResponseContext;
use crate::history::{self, History, MessageReferences, ReadMarker, metadata};
use crate::message::broadcast::{self, Broadcast};
use crate::message::{self, Limit};
use crate::target::{self, Target};
use crate::user::Nick;
use crate::{
    Config, Server, buffer, client, config, input, isupport, reaction, server,
};

const DRAFT_SAVE_EVERY: Duration = Duration::from_secs(10);

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReactionToEcho {
    pub reaction: reaction::Context,
    pub message_text: String,
}

#[derive(Debug)]
pub enum Message {
    LoadFull(history::Kind, Result<history::Loaded, history::Error>),
    UpdatePartial(history::Kind, Result<history::Metadata, history::Error>),
    UpdateChatHistoryReferences(
        history::Kind,
        MessageReferences,
        Result<(), history::Error>,
    ),
    UpdateReadMarker(
        history::Kind,
        history::ReadMarker,
        Result<(), history::Error>,
    ),
    Closed(history::Kind, Result<(), history::Error>),
    Flushed(history::Kind, Result<Vec<ReactionToEcho>, history::Error>),
    Exited(Vec<(history::Kind, Result<(), history::Error>)>),
    SentMessageUpdated(history::Kind, history::ReadMarker),
    ResendMessage(history::Kind, message::Message),
    DraftsSaved,
    ReactionsToEcho(Server, Vec<ReactionToEcho>),
}

pub enum Event {
    Loaded(history::Kind),
    Exited,
    SentMessageUpdated(history::Kind, history::ReadMarker),
    ResendMessage(history::Kind, message::Message),
    ReactionsToEcho(Server, Vec<ReactionToEcho>),
}

#[derive(Debug, Default)]
pub struct Manager {
    resources: HashSet<Resource>,
    filters: Vec<Filter>,
    reroute_rules: RerouteRules,
    data: Data,
    last_draft_changed: Option<tokio::time::Instant>,
}

impl Manager {
    pub fn clear_messages(
        &mut self,
        kind: history::Kind,
        clients: &client::Map,
    ) -> Option<BoxFuture<'static, Message>> {
        if let Some(history) = self.data.map.get_mut(&kind) {
            let task = history.flush(None, clients.get_seed(&kind));

            if let History::Full {
                messages, cleared, ..
            } = history
            {
                messages.clear();
                *cleared = true;
            }

            log::debug!("cleared messages for {kind}");

            return task.map(move |task| {
                task.map(move |result| {
                    Message::Flushed(kind, result.map(|_| vec![]))
                })
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
            Message::Flushed(kind, Ok(reactions)) => {
                // Will cause flush loop if we emit a log every time we flush logs
                if !matches!(kind, history::Kind::Logs) {
                    log::debug!("flushed history for {kind}",);
                }
                if !reactions.is_empty()
                    && let Some(server) = kind.server()
                {
                    return Some(Event::ReactionsToEcho(
                        server.clone(),
                        reactions,
                    ));
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
            Message::UpdateChatHistoryReferences(
                kind,
                chathistory_references,
                Ok(()),
            ) => {
                log::debug!(
                    "updated chathistory references for {kind} to {chathistory_references:?}"
                );
            }
            Message::UpdateChatHistoryReferences(
                kind,
                chathistory_references,
                Err(error),
            ) => {
                log::warn!(
                    "failed to update chathistory references for {kind} to {chathistory_references:?}: {error}"
                );
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
            Message::ResendMessage(kind, message) => {
                return Some(Event::ResendMessage(kind, message));
            }
            Message::DraftsSaved => {}
            Message::ReactionsToEcho(server, reactions) => {
                return Some(Event::ReactionsToEcho(server, reactions));
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

    pub fn filters(&self) -> &[Filter] {
        &self.filters
    }

    pub fn get_reroute_rules_mut(&mut self) -> &mut RerouteRules {
        &mut self.reroute_rules
    }

    pub fn get_reroute_rules(&self) -> &RerouteRules {
        &self.reroute_rules
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

    pub fn open(&mut self, kind: history::Kind) {
        self.data
            .map
            .entry(kind.clone())
            .or_insert(History::partial(kind.clone()));
    }

    pub fn exit(
        &mut self,
        clients: &client::Map,
    ) -> impl Future<Output = Message> + use<> {
        let data = std::mem::take(&mut self.data);
        let drafts = data.input.clone_draft_map();
        let seeds: Vec<Option<history::Seed>> =
            data.map.keys().map(|kind| clients.get_seed(kind)).collect();
        let seeded_map = data.map.into_iter().zip(seeds);

        async move {
            let tasks = seeded_map.into_iter().map(|((kind, state), seed)| {
                state.close(seed).map(move |result| (kind, result))
            });

            let results = future::join_all(tasks).await;
            input::save_drafts(drafts).await;
            Message::Exited(results)
        }
    }

    pub fn maybe_save_drafts(
        &mut self,
        now: tokio::time::Instant,
    ) -> Option<BoxFuture<'static, Message>> {
        let last_changed = self.last_draft_changed?;

        if now.duration_since(last_changed) < DRAFT_SAVE_EVERY {
            return None;
        }

        self.last_draft_changed = None;
        let drafts = self.data.input.clone_draft_map();

        Some(
            async move {
                input::save_drafts(drafts).await;
                Message::DraftsSaved
            }
            .boxed(),
        )
    }

    pub fn preload_drafts(
        &mut self,
        drafts: HashMap<buffer::Upstream, String>,
    ) {
        self.data.input.load_into(drafts);
    }

    pub fn record_input_message(
        &mut self,
        message: message::Message,
        labeled_response_context: Option<LabeledResponseContext>,
        server: &Server,
        casemapping: isupport::CaseMap,
        config: &Config,
    ) -> Vec<BoxFuture<'static, Message>> {
        let mut tasks = vec![];

        let message =
            message.with_labeled_response_context(labeled_response_context);

        if config.buffer.mark_as_read.on_message_sent
            && let Some(kind) =
                history::Kind::from_server_message(server, &message)
        {
            self.update_display_read_marker(
                kind,
                history::ReadMarker::from(&message),
            );
        }

        tasks.extend(self.block_and_record_message(
            server,
            casemapping,
            message,
            None,
            &config.buffer,
        ));

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
        // Only set if None, so drafts save on an interval
        if self.last_draft_changed.is_none() {
            self.last_draft_changed = Some(tokio::time::Instant::now());
        }
    }

    // The message's blocked state should be determined prior to using this
    // function. In most cases, the best way to do that is by using the
    // block_and_record_message function.
    pub fn record_message(
        &mut self,
        server: &Server,
        message: crate::Message,
        labeled_response_context: Option<LabeledResponseContext>,
        buffer_config: &config::Buffer,
    ) -> Vec<BoxFuture<'static, Message>> {
        history::Kind::from_server_message_rerouted_from(server, &message)
            .and_then(|kind| {
                if message.can_reference() {
                    self.data
                        .update_chathistory_references(
                            kind,
                            message.references(),
                        )
                        .map(futures::FutureExt::boxed)
                } else {
                    None
                }
            })
            .into_iter()
            .chain(
                history::Kind::from_server_message(server, &message).and_then(
                    |kind| {
                        let condensers = (message.can_condense(
                            &buffer_config.server_messages.condense,
                        ) && !message.blocked)
                            .then_some((kind.clone(), message.clone()));

                        let future = self.data.add_message(
                            kind,
                            message,
                            labeled_response_context,
                        );

                        if let Some((kind, message)) = condensers {
                            self.condense_message(
                                message,
                                &kind,
                                &buffer_config.server_messages.condense,
                            );
                        }

                        future.map(futures::FutureExt::boxed)
                    },
                ),
            )
            .collect()
    }

    pub fn record_reaction(
        &mut self,
        server: &Server,
        reaction: reaction::Context,
        notification_enabled: bool,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.data
            .add_reaction(server.clone(), reaction, notification_enabled)
    }

    pub fn block_and_record_message(
        &mut self,
        server: &Server,
        casemapping: isupport::CaseMap,
        mut message: crate::Message,
        labeled_response_context: Option<LabeledResponseContext>,
        buffer_config: &config::Buffer,
    ) -> Vec<BoxFuture<'static, Message>> {
        if let Some(kind) = history::Kind::from_server_message(server, &message)
        {
            self.block_message(
                &mut message,
                &kind,
                server,
                casemapping,
                buffer_config,
            );
        }

        self.record_message(
            server,
            message,
            labeled_response_context,
            buffer_config,
        )
    }

    pub fn record_log(
        &mut self,
        record: crate::log::Record,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.data.add_message(
            history::Kind::Logs,
            crate::Message::log(record),
            None,
        )
    }

    // Unlike block_and_record_message, the message's blocked status should be
    // determined before recording a highlight in order to block highlight
    // notifications.
    pub fn record_highlight(
        &mut self,
        message: crate::Message,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.data
            .add_message(history::Kind::Highlights, message, None)
    }

    pub fn remove_message(
        &mut self,
        kind: history::Kind,
        server_time: DateTime<Utc>,
        hash: message::Hash,
        resend: bool,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.data.remove_message(kind, server_time, hash, resend)
    }

    pub fn expand_condensed_message(
        &mut self,
        kind: history::Kind,
        server_time: DateTime<Utc>,
        hash: message::Hash,
        config: &config::buffer::Condensation,
    ) {
        self.data
            .expand_condensed_message(kind, server_time, hash, config);
    }

    pub fn contract_condensed_message(
        &mut self,
        kind: history::Kind,
        server_time: DateTime<Utc>,
        hash: message::Hash,
        config: &config::buffer::Condensation,
    ) {
        self.data
            .contract_condensed_message(kind, server_time, hash, config);
    }

    pub fn update_chathistory_references<T: Into<history::Kind>>(
        &mut self,
        kind: T,
        chathistory_references: MessageReferences,
    ) -> Option<impl Future<Output = Message> + use<T>> {
        self.data
            .update_chathistory_references(kind, chathistory_references)
    }

    pub fn update_read_marker<T: Into<history::Kind>>(
        &mut self,
        kind: T,
        read_marker: history::ReadMarker,
    ) -> Option<impl Future<Output = Message> + use<T>> {
        self.data.update_read_marker(kind, read_marker)
    }

    pub fn update_display_read_marker<T: Into<history::Kind>>(
        &mut self,
        kind: T,
        read_marker: history::ReadMarker,
    ) {
        self.data.update_display_read_marker(kind, read_marker);
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
        config: &Config,
    ) -> Option<history::View<'_>> {
        self.data.history_view(kind, limit, config)
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
            .iter()
            .filter_map(|(kind, history)| match kind {
                #[allow(clippy::bool_comparison)] // easy to miss exclamation
                history::Kind::Query(s, query) => (s == server
                    && self.filters.iter().all(|filter| {
                        filter.match_query(query, server) == false
                    })
                    && match history {
                        History::Full { .. } => true,
                        History::Partial {
                            show_in_sidebar, ..
                        } => *show_in_sidebar,
                    })
                .then_some(query),
                _ => None,
            })
            .sorted_by(Ord::cmp)
            .collect()
    }

    pub fn server_kinds(&self, server: Server) -> Vec<history::Kind> {
        self.data
            .map
            .keys()
            .filter(|kind| kind.server().is_some_and(|s| *s == server))
            .cloned()
            .collect()
    }

    pub fn kinds(&self) -> Vec<history::Kind> {
        self.data.map.keys().cloned().collect()
    }

    pub fn server_has_unread(&self, server: &Server) -> bool {
        self.data
            .map
            .iter()
            .filter_map(|(kind, history)| {
                if kind.server().is_some_and(|s| *s == *server) {
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
            .flat_map(|message| {
                self.block_and_record_message(
                    server,
                    casemapping,
                    message,
                    None,
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

    pub fn show_preview(
        &mut self,
        kind: impl Into<history::Kind>,
        message: message::Hash,
        url: &url::Url,
    ) {
        self.data.show_preview(&kind.into(), message, url);
    }

    pub fn block_message(
        &self,
        message: &mut crate::Message,
        kind: &history::Kind,
        server: &Server,
        casemapping: isupport::CaseMap,
        buffer_config: &config::Buffer,
    ) {
        message.blocked = false;

        if let message::Source::Server(source) = message.target.source() {
            // Check if target is included/excluded.
            let target_ref = match &message.target {
                message::Target::Channel { channel, .. }
                | message::Target::Highlights { channel, .. } => {
                    Some(channel.as_target_ref())
                }

                message::Target::Query { query, .. } => {
                    Some(query.as_target_ref())
                }
                message::Target::Server { .. }
                | message::Target::Logs { .. } => None,
            };

            if let Some(target_ref) = target_ref
                && !buffer_config.server_messages.should_send_message(
                    source.as_ref(),
                    target_ref,
                    server,
                    casemapping,
                )
            {
                message.blocked = true;
                return;
            }

            let source_kind =
                source.as_ref().map(message::source::server::Server::kind);

            if let Some(seconds) =
                buffer_config.server_messages.smart(source_kind)
                && let Some(nick) =
                    match source.as_ref().and_then(|source| source.nick()) {
                        Some(nick) => Some(nick.clone()),
                        None => message.plain().and_then(|s| {
                            s.split(' ')
                                .nth(1)
                                .map(|nick| Nick::from_str(nick, casemapping))
                        }),
                    }
                // These blocks are currently only relevant for open panes,
                // since the associated messages do not trigger UI
                // (unread/notifications/etc) and will be processed if/when the
                // pane is opened.
                && let Some(History::Full { messages, .. }) =
                    self.data.map.get(kind)
            {
                if matches!(source_kind, Some(message::Kind::Away)) {
                    message.blocked = messages
                        .iter()
                        .rev()
                        .find_map(|historical_message| {
                            if let crate::message::Source::Server(
                                historical_source,
                            ) = historical_message.target.source()
                                && let Some(historical_source_kind) = source
                                    .as_ref()
                                    .map(message::source::server::Server::kind)
                                && matches!(
                                    historical_source_kind,
                                    message::Kind::Away
                                )
                                && let Some(historical_nick) = historical_source
                                    .as_ref()
                                    .and_then(|historical_source| {
                                        historical_source.nick()
                                    })
                                && *historical_nick == nick
                            {
                                return Some(smart_filter_repeat(
                                    message,
                                    &seconds,
                                    Some(&historical_message.server_time),
                                ));
                            }

                            if !smart_filter_repeat(
                                message,
                                &seconds,
                                Some(&historical_message.server_time),
                            ) {
                                return Some(false);
                            }

                            None
                        })
                        .unwrap_or(false);
                } else {
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
                let insert_date =
                    message.server_time.with_timezone(&Local).date_naive();

                let start = messages
                    .iter()
                    .take(insert_position)
                    .rev()
                    .position(|message| {
                        !message.blocked
                            && (!message.can_condense(config)
                                || message
                                    .server_time
                                    .with_timezone(&Local)
                                    .date_naive()
                                    != insert_date)
                    })
                    .map_or(0, |position| insert_position - position);

                let end = messages
                    .iter()
                    .skip(insert_position)
                    .position(|message| {
                        !message.blocked
                            && (!message.can_condense(config)
                                || message
                                    .server_time
                                    .with_timezone(&Local)
                                    .date_naive()
                                    != insert_date)
                    })
                    .map_or(messages.len(), |position| {
                        insert_position + position
                    });

                let mut condensable_messages = messages[start..end]
                    .iter_mut()
                    .filter(|message| !message.blocked)
                    .collect::<Vec<&mut message::Message>>();

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

        if let Some(History::Full { messages, .. }) =
            self.data.map.get_mut(&kind)
        {
            let mut last_seen = HashMap::<Nick, DateTime<Utc>>::new();
            let mut last_away = HashMap::<Nick, DateTime<Utc>>::new();

            messages.iter_mut().for_each(|message| {
                message.blocked = false;

                match message.target.source() {
                    message::Source::Server(source) => {
                        let server = if let Some(server) = kind.server() {
                            Some(server)
                        } else if let message::Target::Highlights {
                            server,
                            ..
                        } = &message.target
                        {
                            Some(server)
                        } else {
                            None
                        };

                        let casemapping = clients
                            .get_maybe_server_casemapping_or_default(server);

                        // Check if target is included/excluded.
                        let target_ref = match &message.target {
                            message::Target::Channel { channel, .. }
                            | message::Target::Highlights { channel, .. } => {
                                Some(channel.as_target_ref())
                            }

                            message::Target::Query { query, .. } => {
                                Some(query.as_target_ref())
                            }
                            message::Target::Server { .. }
                            | message::Target::Logs { .. } => None,
                        };

                        let source_kind = source
                            .as_ref()
                            .map(message::source::server::Server::kind);

                        if let Some(target_ref) = target_ref
                            && let Some(server) = server
                            && !buffer_config
                                .server_messages
                                .should_send_message(
                                    source.as_ref(),
                                    target_ref,
                                    server,
                                    casemapping,
                                )
                        {
                            message.blocked = true;
                        } else if let Some(seconds) =
                            buffer_config.server_messages.smart(source_kind)
                        {
                            let nick = match source
                                .as_ref()
                                .and_then(|source| source.nick())
                            {
                                Some(nick) => Some(nick.clone()),
                                None => message.plain().and_then(|s| {
                                    s.split(' ').nth(1).map(|nick| {
                                        Nick::from_str(nick, casemapping)
                                    })
                                }),
                            };

                            if let Some(nick) = nick {
                                match source_kind {
                                    Some(message::Kind::Away) => {
                                        message.blocked = smart_filter_repeat(
                                            message,
                                            &seconds,
                                            last_away.get(&nick),
                                        );

                                        if !message.blocked {
                                            last_away.insert(
                                                nick.clone(),
                                                message.server_time,
                                            );
                                        }
                                    }
                                    _ => {
                                        message.blocked = smart_filter_message(
                                            message,
                                            &seconds,
                                            last_seen.get(&nick),
                                        );
                                    }
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
                        if !buffer_config.internal_messages.enabled(status) {
                            message.blocked = true;
                        } else if let Some(seconds) =
                            buffer_config.internal_messages.smart(status)
                        {
                            message.blocked = smart_filter_internal_message(
                                message, &seconds,
                            );
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
                    CondensationKey::Singular => chunk
                        .collect::<Vec<&mut message::Message>>()
                        .iter_mut()
                        .for_each(|message| message.condensed = None),
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
            history.renormalize_messages(seed);
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
        Some(Limit::Around(n, hash)) => {
            let collected = messages.collect::<Vec<_>>();
            let length = collected.len();
            let center = collected
                .iter()
                .position(|m| m.hash == hash)
                .unwrap_or(length.saturating_sub(1));
            let start =
                center.saturating_sub(n / 2).min(length.saturating_sub(n));
            let end = (start + n).min(length);
            collected[start..end].to_vec()
        }
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
                    pending_messages,
                    last_updated_at,
                    read_marker: partial_read_marker,
                    chathistory_references: partial_chathistory_references,
                    last_seen,
                    pending_reactions,
                    ..
                } => {
                    let read_marker =
                        (*partial_read_marker).max(metadata.read_marker);

                    let chathistory_references = partial_chathistory_references
                        .clone()
                        .max(metadata.chathistory_references)
                        .max(metadata::latest_can_reference(&messages));

                    let last_updated_at = *last_updated_at;

                    let mut last_seen = last_seen.clone();

                    for (id, pending) in pending_reactions.iter_mut() {
                        if let Some(message) = history::find_reaction_target(
                            &mut messages,
                            id,
                            &pending.server_time,
                        ) {
                            message.reactions.append(
                                &mut pending
                                    .reactions
                                    .iter()
                                    .map(|(reaction, _)| reaction.clone())
                                    .collect(),
                            );
                        }
                    }

                    for (message, labeled_response_context) in
                        std::mem::take(pending_messages)
                    {
                        history::update_last_seen(&mut last_seen, &message);

                        history::insert_message(
                            &mut messages,
                            message,
                            labeled_response_context,
                        );
                    }

                    entry.insert(History::Full {
                        kind,
                        messages,
                        last_updated_at,
                        read_marker,
                        display_read_marker: read_marker,
                        chathistory_references,
                        last_seen,
                        cleared: false,
                    });
                }
                _ => {
                    let chathistory_references = metadata
                        .chathistory_references
                        .max(metadata::latest_can_reference(&messages));

                    let last_seen = history::get_last_seen(&messages);

                    entry.insert(History::Full {
                        kind,
                        messages,
                        last_updated_at: None,
                        read_marker: metadata.read_marker,
                        display_read_marker: metadata.read_marker,
                        chathistory_references,
                        last_seen,
                        cleared: false,
                    });
                }
            },
            hash_map::Entry::Vacant(entry) => {
                let chathistory_references = metadata
                    .chathistory_references
                    .max(metadata::latest_can_reference(&messages));

                let last_seen = history::get_last_seen(&messages);

                entry.insert(History::Full {
                    kind,
                    messages,
                    last_updated_at: None,
                    read_marker: metadata.read_marker,
                    display_read_marker: metadata.read_marker,
                    chathistory_references,
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
        config: &Config,
    ) -> Option<history::View<'_>> {
        let History::Full {
            messages,
            display_read_marker,
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
                    .can_condense(&config.buffer.server_messages.condense)
                {
                    if message.expanded {
                        Some(message)
                    } else {
                        message
                            .condensed
                            .as_ref()
                            .map(std::convert::AsRef::as_ref)
                    }
                } else {
                    match message.target.source() {
                        message::Source::Internal(
                            message::source::Internal::Status(status),
                        ) => {
                            if !config.buffer.internal_messages.enabled(status)
                            {
                                return None;
                            } else if let Some(seconds) =
                                config.buffer.internal_messages.smart(status)
                            {
                                return (!smart_filter_internal_message(
                                    message, &seconds,
                                ))
                                .then_some(message);
                            }

                            Some(message)
                        }
                        _ => Some(message),
                    }
                }
            })
            .collect::<Vec<_>>();

        let total = processed.len();
        let with_access_levels = config.buffer.nickname.show_access_levels;
        let truncate = config.buffer.nickname.truncate;

        let max_nick_chars =
            config.buffer.nickname.alignment.is_right().then(|| {
                processed
                    .iter()
                    .filter_map(|message| {
                        if let message::Source::User(user) =
                            message.target.source()
                            && !user.is_bot()
                        {
                            Some(
                                config
                                    .buffer
                                    .nickname
                                    .brackets
                                    .format(user.display(
                                        with_access_levels,
                                        config.buffer.nickname.show_bot_icon,
                                        truncate,
                                        config.display.truncation_character,
                                    ))
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

        let max_bot_nick_chars =
            config.buffer.nickname.alignment.is_right().then(|| {
                processed
                    .iter()
                    .filter_map(|message| {
                        if let message::Source::User(user) =
                            message.target.source()
                            && user.is_bot()
                        {
                            Some(
                                config
                                    .buffer
                                    .nickname
                                    .brackets
                                    .format(user.display(
                                        with_access_levels,
                                        config.buffer.nickname.show_bot_icon,
                                        truncate,
                                        config.display.truncation_character,
                                    ))
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
            config.buffer.nickname.alignment.is_right().then(|| {
                if matches!(kind, history::Kind::Channel(..)) {
                    processed
                        .iter()
                        .filter_map(|message| {
                            message.target.prefixes().map(|prefixes| {
                                config
                                    .buffer
                                    .status_message_prefix
                                    .brackets
                                    .format(prefixes.iter().collect::<String>())
                                    .chars()
                                    .count()
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
        let range_end_timestamp_chars =
            (config.buffer.nickname.alignment.is_right()
                && config.buffer.server_messages.condense.any())
            .then(|| {
                processed
                    .iter()
                    .find_map(|message| {
                        if let message::Source::Internal(
                            message::source::Internal::Condensed(
                                end_server_time,
                            ),
                        ) = message.target.source()
                            && message.server_time != *end_server_time
                        {
                            config
                                .buffer
                                .format_range_end_timestamp(end_server_time)
                                .map(|(dash, end_timestamp)| {
                                    dash.chars().count()
                                        + end_timestamp.chars().count()
                                })
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

        let split_at = display_read_marker.map_or(0, |display_read_marker| {
            limited
                .iter()
                .rev()
                .position(|message| {
                    message.server_time <= display_read_marker.date_time()
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
                without_limit.hash != with_limit.hash
            });
        let has_more_newer_messages = last_without_limit
            .zip(last_with_limit)
            .is_some_and(|(without_limit, with_limit)| {
                without_limit.hash != with_limit.hash
            });

        Some(history::View {
            total,
            has_more_older_messages,
            has_more_newer_messages,
            old_messages: old.to_vec(),
            new_messages: new.to_vec(),
            max_nick_chars,
            max_bot_nick_chars,
            max_prefix_chars,
            range_end_timestamp_chars,
            cleared: *cleared,
        })
    }

    fn add_message(
        &mut self,
        kind: history::Kind,
        message: crate::Message,
        labeled_response_context: Option<LabeledResponseContext>,
    ) -> Option<impl Future<Output = Message> + use<>> {
        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                let read_marker = entry
                    .get_mut()
                    .add_message(message, labeled_response_context);

                // Update the read marker immediately so the split is correct
                if let Some(read_marker) = read_marker
                    && entry.get_mut().update_read_marker(read_marker)
                {
                    Some(
                        async move {
                            Message::SentMessageUpdated(
                                kind.clone(),
                                read_marker,
                            )
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
                    .add_message(message, labeled_response_context);

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

    fn remove_message(
        &mut self,
        kind: history::Kind,
        server_time: DateTime<Utc>,
        hash: message::Hash,
        resend: bool,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.map.get_mut(&kind).and_then(|history| {
            history
                .remove_message(server_time, hash)
                .and_then(|message| {
                    resend.then_some(
                        async move { Message::ResendMessage(kind, message) }
                            .boxed(),
                    )
                })
        })
    }

    fn expand_condensed_message(
        &mut self,
        kind: history::Kind,
        server_time: DateTime<Utc>,
        hash: message::Hash,
        config: &config::buffer::Condensation,
    ) {
        if let Some(history) = self.map.get_mut(&kind) {
            history
                .get_condensed_messages(server_time, hash, config)
                .iter_mut()
                .for_each(|message| {
                    message.expanded = true;
                });
        }
    }

    fn contract_condensed_message(
        &mut self,
        kind: history::Kind,
        server_time: DateTime<Utc>,
        hash: message::Hash,
        config: &config::buffer::Condensation,
    ) {
        if let Some(history) = self.map.get_mut(&kind) {
            history
                .get_condensed_messages(server_time, hash, config)
                .iter_mut()
                .for_each(|message| {
                    message.expanded = false;
                });
        }
    }

    fn update_chathistory_references<T: Into<history::Kind>>(
        &mut self,
        kind: T,
        chathistory_references: MessageReferences,
    ) -> Option<impl Future<Output = Message> + use<T>> {
        use std::collections::hash_map;

        let kind = kind.into();

        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                entry
                    .get_mut()
                    .update_chathistory_references(chathistory_references);

                None
            }
            hash_map::Entry::Vacant(_) => Some(
                async move {
                    let updated =
                        history::metadata::update_chathistory_references(
                            &kind,
                            &chathistory_references,
                        )
                        .await;

                    Message::UpdateChatHistoryReferences(
                        kind,
                        chathistory_references,
                        updated,
                    )
                }
                .boxed(),
            ),
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
                    let updated = history::metadata::update_read_marker(
                        &kind,
                        &read_marker,
                    )
                    .await;

                    Message::UpdateReadMarker(kind, read_marker, updated)
                }
                .boxed(),
            ),
        }
    }

    fn update_display_read_marker<T: Into<history::Kind>>(
        &mut self,
        kind: T,
        read_marker: history::ReadMarker,
    ) {
        use std::collections::hash_map;

        let kind = kind.into();

        match self.map.entry(kind) {
            hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().update_display_read_marker(read_marker);
            }
            hash_map::Entry::Vacant(_) => (),
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

    fn show_preview(
        &mut self,
        kind: &history::Kind,
        message: message::Hash,
        url: &url::Url,
    ) {
        if let Some(history) = self.map.get_mut(kind) {
            history.show_preview(message, url);
        }
    }

    fn add_reaction(
        &mut self,
        server: Server,
        reaction: reaction::Context,
        notification_enabled: bool,
    ) -> Option<impl Future<Output = Message> + use<>> {
        let kind =
            history::Kind::from_target(server.clone(), reaction.target.clone());
        match self.map.entry(kind.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                let reactions = entry
                    .get_mut()
                    .add_reaction(reaction, notification_enabled);

                if notification_enabled {
                    reactions.map(|reaction| {
                        async move {
                            Message::ReactionsToEcho(server, vec![reaction])
                        }
                        .boxed()
                    })
                } else {
                    None
                }
            }
            hash_map::Entry::Vacant(entry) => {
                entry
                    .insert(History::partial(kind.clone()))
                    .add_reaction(reaction, notification_enabled);

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

fn smart_filter_repeat(
    message: &crate::Message,
    seconds: &i64,
    last_seen_server_time: Option<&DateTime<Utc>>,
) -> bool {
    let Some(server_time) = last_seen_server_time else {
        return false;
    };

    let duration_seconds = message
        .server_time
        .signed_duration_since(*server_time)
        .num_seconds();

    duration_seconds <= *seconds
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
