use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{HashMap, hash_map};
use std::ops::Range;
use std::path::PathBuf;
use std::{fs, io};

use chrono::{self, DateTime, Local, NaiveDate, Utc};
use futures::FutureExt;
use futures::future::{self, BoxFuture};
use iced::Task;
use itertools::Itertools;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::filter::{Filter, FilterChain};
use super::reroute::RerouteRules;
use super::{
    Id, Kind, Metadata, ReadMarker, Request, find_message_by_history_id,
    find_message_by_id, find_message_mut_by_history_id, find_message_mut_by_id,
    model, position_message_after_date_time, position_message_by_history_id,
    position_message_by_id, smart_filter_internal_message,
    smart_filter_message, smart_filter_repeat,
};
use crate::buffer::{self, BuffersContext};
use crate::client::ClientsContext;
use crate::config::buffer::OnMessage;
use crate::message::{
    self, MessageReferences, Searchable, Source, Temporal, broadcast,
    highlight, source,
};
use crate::target::{self, Target};
use crate::time::Posix;
use crate::user::Nick;
use crate::{
    Config, Notification, Server, client, compression, config, environment,
    input, isupport, reaction, redaction,
};

/// Max # messages to persist; TODO: make configurable (alter message store when
/// set to zero?)
pub(crate) const MAX_SAVED_MESSAGES: usize = 10_000;
/// Duration to wait after last received update before saving/flushing to disk
const SAVE_AFTER_DURATION_SINCE_UPDATE: Duration = Duration::from_secs(10);
/// # of pending updates to trigger save/flush to disk even if
/// SAVE_AFTER_DURATION_SINCE_UPDATE has not passed
const SAVE_AFTER_UPDATE_COUNT: usize = 500;

#[derive(Debug)]
pub enum Message {
    Loaded(Kind, Result<Vec<message::Message>, Error>),
    Saved(Kind, Result<usize, Error>),
    DraftsSaved(Result<usize, Error>),
    Highlights(Vec<message::MessageWithContext>),
    Exited(
        HashMap<Kind, Result<usize, Error>>,
        Option<Result<usize, Error>>,
    ),
}

#[derive(Debug)]
pub enum Event {
    History(Message),
    Model(model::Message),
    Notification(Server, Notification),
    Client(client::Message),
}

#[derive(Debug)]
pub enum Update {
    Message(message::MessageWithContext),
    Broadcast(message::BroadcastWithContext),
    Reaction(reaction::ReactionWithContext),
    Redaction(redaction::RedactionWithContext),
    Remove(Kind, Id, message::Time),
    ShowPreview(Kind, Id, message::Time, url::Url),
    HidePreview(Kind, Id, message::Time, url::Url),
}

#[derive(Debug, Default)]
pub struct PostWriteUpdate {
    events: Vec<Event>,
    highlight: Option<message::MessageWithContext>,
    read_marker_update: Option<ReadMarkerUpdate>,
}

#[derive(Debug)]
pub enum ReadMarkerUpdate {
    Canonical,
    Display,
}

#[derive(Debug)]
pub struct Manager {
    storage: HashMap<Kind, Storage>,
    input_storage: input::Storage,
    filters: Vec<Filter>,
    reroute_rules: RerouteRules,
    event_sender: mpsc::UnboundedSender<Vec<Event>>,
}

impl Manager {
    pub fn new(config: &Config) -> (Self, Task<Vec<Event>>) {
        let mut input_storage = input::Storage::default();

        input_storage.load(config);

        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        (
            Self {
                storage: HashMap::new(),
                input_storage,
                filters: Vec::new(),
                reroute_rules: RerouteRules::default(),
                event_sender,
            },
            Task::stream(UnboundedReceiverStream::new(event_receiver)),
        )
    }

    #[cfg(test)]
    pub fn test() -> Self {
        let (event_sender, _) = mpsc::unbounded_channel();

        Self {
            storage: HashMap::new(),
            input_storage: input::Storage::default(),
            filters: Vec::new(),
            reroute_rules: RerouteRules::default(),
            event_sender,
        }
    }

    /// Write all pending updates for all history to the message store (not save
    /// to disk).
    pub fn write(
        &mut self,
        server: Option<&Server>,
        updates: Vec<Update>,
        clients_context: &dyn ClientsContext,
        buffers_context: &dyn BuffersContext,
        config: &Config,
    ) {
        let mut updates_by_kind = HashMap::<Kind, Vec<Update>>::new();

        for update in updates.into_iter() {
            if let Some(kind) = match &update {
                Update::Message(message) => {
                    Kind::from_message(&message.inner, server)
                }
                Update::Broadcast(_) => None,
                Update::Reaction(reaction) => server.map(|server| {
                    Kind::from_target(server.clone(), reaction.target.clone())
                }),
                Update::Redaction(redaction) => server.map(|server| {
                    Kind::from_target(server.clone(), redaction.target.clone())
                }),
                Update::Remove(kind, ..)
                | Update::ShowPreview(kind, ..)
                | Update::HidePreview(kind, ..) => Some(kind.clone()),
            } {
                let kind_updates =
                    updates_by_kind.entry(kind).or_insert(vec![]);

                kind_updates.push(update);
            } else if matches!(update, Update::Broadcast(_))
                && let Some(server) = server
            {
                let Update::Broadcast(mut broadcast) = update else {
                    unreachable!();
                };

                let mut targets = std::mem::take(&mut broadcast.in_channels)
                    .into_iter()
                    .map(|channel| message::Target::Channel { channel })
                    .collect::<Vec<message::Target>>();

                if broadcast.in_server {
                    targets.push(message::Target::Server);
                }

                match std::mem::take(&mut broadcast.in_queries) {
                    broadcast::Queries::All => {
                        targets.extend(self.storage.keys().filter_map(
                            |kind| {
                                if let Kind::Query(kind_server, query) = kind
                                    && kind_server == server
                                {
                                    Some(message::Target::Query {
                                        query: query.clone(),
                                    })
                                } else {
                                    None
                                }
                            },
                        ));
                    }
                    broadcast::Queries::WithNick(nick) => {
                        let kind = Kind::from_target(
                            server.clone(),
                            Target::from(nick.clone()),
                        );

                        if self.storage.contains_key(&kind) {
                            targets.push(message::Target::Query {
                                query: target::Query::from(nick),
                            });
                        }
                    }
                    broadcast::Queries::None => (),
                }

                for message in broadcast.into_messages(
                    targets,
                    clients_context.get_server_casemapping_or_default(server),
                    config,
                ) {
                    if let Some(kind) =
                        Kind::from_server_message(server, &message)
                    {
                        let update =
                            Update::Message(message::MessageWithContext {
                                inner: message,
                                highlight: None,
                                historical: false,
                                labeled_response_context: None,
                                notification_allowed: false,
                            });

                        let kind_updates =
                            updates_by_kind.entry(kind).or_insert(vec![]);

                        kind_updates.push(update);
                    } else {
                        log::error!(
                            "unexpected broadcast target {:?}",
                            message.target
                        );
                    }
                }
            } else {
                log::error!(
                    "missing server context for storage update {update:?}"
                );
            }
        }

        let events = updates_by_kind
            .into_iter()
            .flat_map(|(kind, updates)| {
                let (kind_storage, filter_chain) =
                    self.get_or_load_mut_with_filter_chain(kind);

                kind_storage.write(
                    updates,
                    filter_chain,
                    clients_context,
                    buffers_context,
                    config,
                )
            })
            .collect();

        let _ = self.event_sender.send(events);
    }

    pub fn record_draft(&mut self, raw_input: input::RawInput) {
        self.input_storage.store_draft(raw_input);
    }

    pub fn record_input_history(
        &mut self,
        buffer: &buffer::Upstream,
        text: String,
    ) {
        self.input_storage.record(buffer, text);
    }

    pub fn input<'a>(&'a self, buffer: &buffer::Upstream) -> input::Cache<'a> {
        self.input_storage.get(buffer)
    }

    pub fn tick(&mut self, now: Instant, config: &Config) {
        if let Some(save_future) = self.input_storage.tick(now, config) {
            let event_sender = self.event_sender.clone();

            tokio::task::spawn(async move {
                let saved = save_future.await;

                let _ = event_sender
                    .send(vec![Event::History(Message::DraftsSaved(saved))]);
            });
        }

        for kind_storage in self.storage.values_mut() {
            if let Some(save_future) = kind_storage.tick(now) {
                let event_sender = self.event_sender.clone();

                tokio::task::spawn(async move {
                    let (kind, saved) = save_future.await;

                    let _ = event_sender.send(vec![Event::History(
                        Message::Saved(kind, saved),
                    )]);
                });
            }
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients_context: &dyn ClientsContext,
        buffers_context: &dyn BuffersContext,
        config: &Config,
    ) {
        match message {
            Message::Loaded(kind, Ok(messages)) => {
                log::debug!(
                    "loaded history {kind}: {} messages",
                    messages.len()
                );

                let (kind_storage, filter_chain) =
                    self.get_or_load_mut_with_filter_chain(kind.clone());

                kind_storage.loaded(messages);

                let mut events = kind_storage.flush(
                    filter_chain,
                    clients_context,
                    buffers_context,
                    config,
                    false,
                );

                if events.is_empty() {
                    kind_storage.read(
                        true,
                        filter_chain,
                        clients_context,
                        &config.buffer,
                    );

                    events.push(Event::Model(model::Message::Update(
                        kind,
                        kind_storage.model_update(),
                    )));
                }

                let _ = self.event_sender.send(events);
            }
            Message::Loaded(kind, Err(error)) => {
                log::error!("failed to load history {kind}: {error}");

                let kind_storage = self.get_or_load_mut(kind);

                kind_storage.messages = Some(vec![]);
            }
            Message::Saved(kind, result) => {
                match result {
                    Ok(message_count) => {
                        log::debug!(
                            "saved history {kind}: {message_count} messages"
                        );
                    }
                    Err(error) => {
                        log::error!("failed to save history {kind}: {error}");
                    }
                }

                let kind_storage = self.get_or_load_mut(kind);

                kind_storage.saved();
            }
            Message::DraftsSaved(result) => {
                match result {
                    Ok(draft_count) => {
                        log::debug!("saved input drafts: {draft_count} drafts");
                    }
                    Err(error) => {
                        log::error!("failed to save input drafts: {error}");
                    }
                }

                self.input_storage.saved();
            }
            Message::Highlights(highlights) => {
                let (kind_storage, filter_chain) =
                    self.get_or_load_mut_with_filter_chain(Kind::Highlights);

                let updates =
                    highlights.into_iter().map(Update::Message).collect();

                let events = kind_storage.write(
                    updates,
                    filter_chain,
                    clients_context,
                    buffers_context,
                    config,
                );

                let _ = self.event_sender.send(events);
            }
            Message::Exited(results, input_result) => {
                for (kind, result) in results {
                    match result {
                        Ok(message_count) => {
                            log::debug!(
                                "saved history {kind}: {message_count} messages"
                            );
                        }
                        Err(error) => {
                            log::error!(
                                "failed to save history {kind}: {error}"
                            );
                        }
                    }
                }

                if let Some(input_result) = input_result {
                    match input_result {
                        Ok(draft_count) => {
                            log::debug!(
                                "saved input drafts: {draft_count} drafts"
                            );
                        }
                        Err(error) => {
                            log::error!("failed to save input drafts: {error}");
                        }
                    }
                }
            }
        }
    }

    pub fn mark_as_read(&mut self, kind: Kind) {
        let kind_storage = self.get_or_load_mut(kind);

        let events = kind_storage.mark_as_read().into_iter().collect();

        let _ = self.event_sender.send(events);
    }

    pub fn mark_server_as_read(&mut self, server: &Server) {
        let events = self
            .get_server_mut(server)
            .iter_mut()
            .filter_map(|server_storage| server_storage.mark_as_read())
            .collect();

        let _ = self.event_sender.send(events);
    }

    pub fn clear(
        &mut self,
        kind: Kind,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) {
        let (kind_storage, filter_chain) =
            self.get_or_load_mut_with_filter_chain(kind);

        let event =
            kind_storage.clear(filter_chain, clients_context, buffer_config);

        let _ = self.event_sender.send(vec![event]);
    }

    pub fn last_received_chathistory_targets(
        &self,
        server: &Server,
    ) -> Option<DateTime<Utc>> {
        let path = chathistory_targets_path(server).ok()?;

        let bytes = fs::read(path).ok()?;

        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn update_last_received_chathistory_targets(
        &self,
        server: &Server,
        timestamp: DateTime<Utc>,
    ) {
        match write_chathistory_targets_timestamp(server, timestamp) {
            Ok(()) => {
                log::debug!(
                    "updated targets timestamp for {server} to {timestamp}"
                );
            }
            Err(error) => {
                log::warn!(
                    "failed to update targets timestamp for {server} to {timestamp}: {error}"
                );
            }
        }
    }

    /// If no message reference of an allowed message reference type is
    /// available, then None will be returned.
    pub fn last_can_reference_before(
        &mut self,
        server_time: DateTime<Utc>,
        kind: Kind,
        message_reference_types: &[isupport::MessageReferenceType],
    ) -> Option<isupport::MessageReference> {
        self.get_or_load(kind)
            .last_can_reference_before(server_time, message_reference_types)
    }

    /// If no reference of an allowed type is available, then a pseudo-reference
    /// will be returned at the provided server_time if
    /// `MessageReferenceType::Timestamp` is allowed.  Otherwise None will be
    /// returned.
    pub fn last_can_reference_before_or_at(
        &mut self,
        server_time: DateTime<Utc>,
        kind: Kind,
        message_reference_types: &[isupport::MessageReferenceType],
    ) -> Option<isupport::MessageReference> {
        self.get_or_load(kind).last_can_reference_before_or_at(
            server_time,
            message_reference_types,
        )
    }

    pub fn find_message_by_history_id(
        &self,
        history_id: &Id,
        kind: &Kind,
        time: &message::Time,
    ) -> Option<&message::Message> {
        self.storage.get(kind).and_then(|kind_storage| {
            kind_storage.find_message_by_history_id(history_id, time)
        })
    }

    pub fn find_message_by_id(
        &self,
        id: &message::Id,
        kind: &Kind,
        time: &message::Time,
    ) -> Option<&message::Message> {
        self.storage
            .get(kind)
            .and_then(|kind_storage| kind_storage.find_message_by_id(id, time))
    }

    fn get_or_load(&mut self, kind: Kind) -> &Storage {
        match self.storage.entry(kind) {
            hash_map::Entry::Occupied(entry) => entry.into_mut(),
            hash_map::Entry::Vacant(entry) => {
                Manager::load_message_store(
                    self.event_sender.clone(),
                    entry.key().clone(),
                );

                let storage = Storage::from(entry.key().clone());

                entry.insert(storage)
            }
        }
    }

    fn get_or_load_mut(&mut self, kind: Kind) -> &mut Storage {
        match self.storage.entry(kind) {
            hash_map::Entry::Occupied(entry) => entry.into_mut(),
            hash_map::Entry::Vacant(entry) => {
                Manager::load_message_store(
                    self.event_sender.clone(),
                    entry.key().clone(),
                );

                let storage = Storage::from(entry.key().clone());

                entry.insert(storage)
            }
        }
    }

    fn get_or_load_mut_with_filter_chain(
        &mut self,
        kind: Kind,
    ) -> (&mut Storage, FilterChain<'_>) {
        (
            match self.storage.entry(kind) {
                hash_map::Entry::Occupied(entry) => entry.into_mut(),
                hash_map::Entry::Vacant(entry) => {
                    Manager::load_message_store(
                        self.event_sender.clone(),
                        entry.key().clone(),
                    );

                    let storage = Storage::from(entry.key().clone());

                    entry.insert(storage)
                }
            },
            FilterChain::borrow(&self.filters),
        )
    }

    fn get_server_mut(&mut self, server: &Server) -> Vec<&mut Storage> {
        self.storage
            .iter_mut()
            .filter_map(|(kind, storage)| {
                kind.as_server()
                    .is_some_and(|kind_server| kind_server == server)
                    .then_some(storage)
            })
            .collect()
    }

    fn load_message_store(
        event_sender: mpsc::UnboundedSender<Vec<Event>>,
        kind: Kind,
    ) {
        tokio::task::spawn(async move {
            let loaded = Storage::load_message_store(&kind).await;

            let _ = event_sender
                .send(vec![Event::History(Message::Loaded(kind, loaded))]);
        });
    }

    #[must_use]
    pub fn exit(
        &mut self,
        clients_context: &dyn ClientsContext,
        buffers_context: &dyn BuffersContext,
        config: &Config,
    ) -> Vec<Event> {
        let mut flush_events = self
            .storage
            .values_mut()
            .flat_map(|kind_storage| {
                kind_storage.flush(
                    FilterChain::borrow(&self.filters),
                    clients_context,
                    buffers_context,
                    config,
                    true,
                )
            })
            .collect::<Vec<Event>>();

        let mut markread_events =
            if config.buffer.mark_as_read.on_application_exit {
                self.storage.values_mut().collect::<Vec<&mut Storage>>()
            } else {
                self.storage
                    .values_mut()
                    .filter(|kind_storage| {
                        config.buffer.mark_as_read.on_buffer_close.mark_as_read(
                            buffers_context
                                .is_open_and_at_bottom(&kind_storage.kind),
                        )
                    })
                    .collect::<Vec<&mut Storage>>()
            }
            .into_iter()
            .filter_map(Storage::mark_as_read)
            .collect::<Vec<Event>>();

        for mut markread_event in markread_events.iter_mut() {
            // Check if there is already a markread event in `flush_events`, and
            // if so remove the markread event from `flush_events` and update
            // the markread event in `markread_events`

            if let Event::Client(client::Message::SendMarkread(
                server,
                target,
                read_marker,
            )) = &mut markread_event
            {
                flush_events.retain(|flush_event| {
                    if let Event::Client(client::Message::SendMarkread(
                        flush_server,
                        flush_target,
                        flush_read_marker,
                    )) = &flush_event
                        && flush_server == server
                        && flush_target == target
                    {
                        *read_marker = (*read_marker).max(*flush_read_marker);

                        false
                    } else {
                        true
                    }
                });
            }
        }

        let save_futures = self
            .storage
            .values_mut()
            .filter_map(Storage::save)
            .collect::<Vec<_>>();

        let input_save_future = self.input_storage.save(config);

        if !save_futures.is_empty() || input_save_future.is_some() {
            let event_sender = self.event_sender.clone();

            tokio::task::spawn(async move {
                let saved = future::join_all(save_futures).await;

                let input_saved =
                    if let Some(input_save_future) = input_save_future {
                        Some(input_save_future.await)
                    } else {
                        None
                    };

                let _ = event_sender.send(vec![Event::History(
                    Message::Exited(saved.into_iter().collect(), input_saved),
                )]);
            });
        }

        flush_events.into_iter().chain(markread_events).collect()
    }

    pub fn get_filters(&self) -> &[Filter] {
        &self.filters
    }

    pub fn get_reroute_rules(&self) -> &RerouteRules {
        &self.reroute_rules
    }

    pub fn get_last_seen(
        &self,
        buffer: &buffer::Upstream,
    ) -> Option<&HashMap<Nick, DateTime<Utc>>> {
        let kind = Kind::from(buffer.clone());

        self.storage
            .get(&kind)
            .map(|kind_storage| &kind_storage.last_seen)
    }
}

#[derive(Debug)]
pub struct Storage {
    kind: Kind,
    show_in_sidebar: bool,
    latest: Option<DateTime<Utc>>,
    latest_triggers_unread: Option<DateTime<Utc>>,
    latest_triggers_highlight: Option<DateTime<Utc>>,
    display_read_marker: Option<ReadMarker>,
    read_marker: Option<ReadMarker>,
    chathistory_references: Option<MessageReferences>,
    messages: Option<Vec<message::Message>>, // TODO: Replace with database connection
    read_cache: ReadCache,
    write_buffer: WriteBuffer,
    last_seen: HashMap<Nick, DateTime<Utc>>,
}

impl From<Kind> for Storage {
    fn from(kind: Kind) -> Self {
        let show_in_sidebar = match &kind {
            Kind::Server(_) => true,
            Kind::Channel(_, _) => false,
            Kind::Query(_, _) => false,
            Kind::Logs => true,
            Kind::Highlights => true,
            Kind::ChannelMonitor => true,
        };

        let metadata = Storage::load_metadata(&kind).unwrap_or_default();

        Self {
            kind,
            show_in_sidebar,
            latest: metadata.latest,
            latest_triggers_unread: metadata.latest_triggers_unread,
            latest_triggers_highlight: metadata.latest_triggers_highlight,
            display_read_marker: metadata.read_marker,
            read_marker: metadata.read_marker,
            chathistory_references: metadata.chathistory_references,
            messages: None,
            read_cache: ReadCache::default(),
            write_buffer: WriteBuffer::default(),
            last_seen: HashMap::new(),
        }
    }
}

impl Storage {
    fn load_metadata(kind: &Kind) -> Result<Metadata, Error> {
        let path = kind_metadata_path(kind)?;

        if let Ok(bytes) = fs::read(path) {
            Ok(serde_json::from_slice(&bytes).unwrap_or_default())
        } else {
            Ok(Metadata::default())
        }
    }

    async fn load_message_store(
        kind: &Kind,
    ) -> Result<Vec<message::Message>, Error> {
        let path = kind_path(kind)?;

        if let Ok(bytes) = tokio::fs::read(path).await {
            Ok(compression::decompress(&bytes)?)
        } else {
            Ok(vec![])
        }
    }

    fn loaded(&mut self, messages: Vec<message::Message>) {
        // If not already loaded, should always be the case
        if self.messages.is_none() {
            self.messages = Some(messages);
        } else {
            log::debug!("unexpected repeat loading of history {}", self.kind);
        }
    }

    fn read(
        &mut self,
        force: bool,
        filter_chain: FilterChain,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) {
        if let Some(messages) = self.messages.as_ref() {
            self.read_cache.read(
                &self.kind,
                messages,
                &self.display_read_marker,
                force,
                filter_chain,
                clients_context,
                buffer_config,
            );
        }
    }

    /// Write updates to this history's message store (not save to disk).
    #[must_use]
    fn write(
        &mut self,
        updates: Vec<Update>,
        filter_chain: FilterChain,
        clients_context: &dyn ClientsContext,
        buffers_context: &dyn BuffersContext,
        config: &Config,
    ) -> Vec<Event> {
        self.write_buffer.updates.extend(updates);

        // TODO: Do we want/need a buffer here once we've switched to SQLite?
        self.flush(
            filter_chain,
            clients_context,
            buffers_context,
            config,
            false,
        )
    }

    #[must_use]
    fn flush(
        &mut self,
        filter_chain: FilterChain,
        clients_context: &dyn ClientsContext,
        buffers_context: &dyn BuffersContext,
        config: &Config,
        exiting: bool,
    ) -> Vec<Event> {
        if !self.write_buffer.updates.is_empty()
            && let Some(messages) = self.messages.as_mut()
        {
            let updates = std::mem::take(&mut self.write_buffer.updates);

            self.write_buffer.unsaved_update_count = self
                .write_buffer
                .unsaved_update_count
                .saturating_add(updates.len());
            self.write_buffer.newest_unsaved_update_at = Some(Instant::now());

            let server = self.kind.as_server();

            let mut post_write_updates =
                HashMap::<(Id, message::Time), PostWriteUpdate>::new();

            for update in updates {
                // Update when user(s) in were last seen in this history (since
                // last application launch).
                let mut update_last_seen =
                    |nick: &Nick, date_time: DateTime<Utc>| {
                        if let Some(last_seen) = self.last_seen.get_mut(nick) {
                            *last_seen = (*last_seen).max(date_time);
                        } else {
                            self.last_seen.insert(nick.to_owned(), date_time);
                        }
                    };

                match &update {
                    Update::Message(message) => {
                        if let Source::User(user) | Source::Action(Some(user)) =
                            &message.inner.source
                        {
                            update_last_seen(
                                user.nickname(),
                                message.time().utc,
                            );
                        }
                    }
                    Update::Reaction(reaction) => {
                        update_last_seen(
                            &reaction.inner.sender,
                            reaction.inner.time.utc,
                        );
                    }
                    Update::Redaction(redaction) => {
                        update_last_seen(
                            &redaction.inner.from,
                            redaction.time.utc,
                        );
                    }
                    Update::Broadcast(_)
                    | Update::Remove(..)
                    | Update::ShowPreview(..)
                    | Update::HidePreview(..) => (),
                }

                match update {
                    Update::Message(mut message) => {
                        let casemapping = clients_context
                            .get_maybe_server_casemapping_or_default(server);

                        determine_history_id(&mut message.inner);

                        let history_id = *message.history_id();
                        let message_time = *message.time();

                        let post_write_update = post_write_updates
                            .entry((history_id, message_time))
                            .or_default();

                        if let Some(highlight) =
                            std::mem::take(&mut message.highlight)
                            && let Some(server) = server
                            && let Some(channel) =
                                message.inner.target.as_channel()
                        {
                            if message.notification_allowed
                                && let Some(user) = message.inner.user()
                            {
                                let (description, sound) = match highlight {
                                    highlight::Kind::Nick => {
                                        ("highlighted you".to_string(), None)
                                    }
                                    highlight::Kind::Match {
                                        matching,
                                        sound,
                                    } => (
                                        format!("matched highlight {matching}"),
                                        sound,
                                    ),
                                };

                                post_write_update.events.push(
                                    Event::Notification(
                                        server.clone(),
                                        Notification::Highlight {
                                            user: user.clone(),
                                            channel: channel.clone(),
                                            casemapping,
                                            message: message
                                                .inner
                                                .text()
                                                .into(),
                                            description,
                                            sound,
                                        },
                                    ),
                                );
                            }

                            post_write_update.highlight =
                                Some(message::MessageWithContext {
                                    inner: message::Message {
                                        target: message::Target::Highlights {
                                            server: server.clone(),
                                            channel: channel.clone(),
                                        },
                                        ..message.inner.clone()
                                    },
                                    highlight: None,
                                    historical: message.historical,
                                    labeled_response_context: message
                                        .labeled_response_context
                                        .clone(),
                                    notification_allowed: false,
                                });
                        } else if message.notification_allowed
                            && let Some(user) = message.inner.user()
                            && let Some(server) = server
                        {
                            if let Some(channel) =
                                message.inner.target.as_channel()
                            {
                                if let Some(reply_to_id) =
                                    message.inner.reply_to.as_ref()
                                    && let Some(reply_to_message) =
                                        find_message_by_id(
                                            messages,
                                            reply_to_id,
                                            message.time(),
                                        )
                                    && reply_to_message.is_ours()
                                {
                                    post_write_update.events.push(
                                        Event::Notification(
                                            server.clone(),
                                            Notification::Reply {
                                                user: user.clone(),
                                                channel: channel.clone(),
                                                casemapping,
                                                message: message
                                                    .inner
                                                    .text()
                                                    .into(),
                                            },
                                        ),
                                    );
                                } else if let Some(channel_notifications_config) =
                                    config
                                        .notifications
                                        .channels
                                        .get(channel.as_str())
                                    && channel_notifications_config
                                        .should_notify(
                                            user,
                                            None,
                                            server,
                                            casemapping,
                                        )
                                {
                                    post_write_update.events.push(
                                        Event::Notification(
                                            server.clone(),
                                            Notification::Channel {
                                                user: user.clone(),
                                                channel: channel.clone(),
                                                casemapping,
                                                message: message
                                                    .inner
                                                    .text()
                                                    .into(),
                                            },
                                        ),
                                    );
                                }
                            } else if matches!(
                                message.inner.target,
                                message::Target::Query { .. }
                            ) {
                                post_write_update.events.push(
                                    Event::Notification(
                                        server.clone(),
                                        Notification::DirectMessage {
                                            user: user.clone(),
                                            casemapping,
                                            message: message
                                                .inner
                                                .text()
                                                .to_string(),
                                        },
                                    ),
                                );
                            }
                        }

                        if post_write_update.read_marker_update.is_none()
                            && config.buffer.mark_as_read.on_message_sent
                            && message.inner.is_sent()
                        {
                            post_write_update.read_marker_update =
                                Some(ReadMarkerUpdate::Display);
                        }

                        if match config.buffer.mark_as_read.on_message {
                            OnMessage::Focused => buffers_context
                                .is_focused_and_at_bottom(&self.kind),
                            OnMessage::Open => buffers_context
                                .is_open_and_at_bottom_in_focused_window(
                                    &self.kind,
                                ),
                            OnMessage::None => false,
                        } {
                            post_write_update.read_marker_update =
                                Some(ReadMarkerUpdate::Canonical);
                        }

                        if insert_message(messages, message)
                            && config.buffer.mark_as_read.on_message_sent
                        {
                            post_write_update.read_marker_update =
                                Some(ReadMarkerUpdate::Canonical);
                        }
                    }
                    Update::Reaction(reaction) => {
                        if let Some(message) = find_message_mut_by_id(
                            messages,
                            &reaction.in_reply_to,
                            &reaction.inner.time,
                        ) {
                            if message.is_ours()
                                && reaction.notification_allowed
                                && !reaction.is_ours()
                                && let Some(server) = self.kind.as_server()
                            {
                                let casemapping = clients_context
                                    .get_server_casemapping_or_default(server);

                                let post_write_update = post_write_updates
                                    .entry((message.history_id, message.time))
                                    .or_default();

                                post_write_update.events.push(
                                    Event::Notification(
                                        server.clone(),
                                        Notification::Reaction {
                                            reaction: reaction.clone(),
                                            casemapping,
                                            message_text: message.text().into(),
                                        },
                                    ),
                                );
                            }

                            insert_reaction(&mut message.reactions, reaction);
                        }
                    }
                    Update::Broadcast(_) => {
                        log::error!(
                            "storage update not expanded by storage manager {update:?}"
                        );
                    }
                    Update::Redaction(redaction) => {
                        if let Some(position) = position_message_by_id(
                            messages,
                            &redaction.redacts,
                            &redaction.time,
                        ) {
                            // TODO: Notification when message.is_ours()?
                            messages[position].redaction =
                                Some(redaction.into());
                        }
                    }
                    Update::Remove(_, history_id, time) => {
                        if let Some(position) = position_message_by_history_id(
                            messages,
                            &history_id,
                            &time,
                        ) {
                            messages.remove(position);
                        }
                    }
                    Update::ShowPreview(_, history_id, time, url) => {
                        if let Some(message) = find_message_mut_by_history_id(
                            messages,
                            &history_id,
                            &time,
                        ) {
                            message.hidden_urls.remove(&url);
                        }
                    }
                    Update::HidePreview(_, history_id, time, url) => {
                        if let Some(message) = find_message_mut_by_history_id(
                            messages,
                            &history_id,
                            &time,
                        ) {
                            message.hidden_urls.insert(url);
                        }
                    }
                }
            }

            let delay_read =
                self.read_cache.requested.as_ref().is_some_and(|requested| {
                    matches!(requested.limit, message::Limit::Backlog(_))
                });

            if delay_read {
                self.read_cache.unload();
            } else {
                // TODO: A targeted update of the read cache
                self.read_cache.read(
                    &self.kind,
                    messages,
                    &self.display_read_marker,
                    true,
                    filter_chain,
                    clients_context,
                    &config.buffer,
                );
            }

            let mut write_events = vec![];

            let mut display_read_marker_update = None;
            let mut read_marker_update = None;

            let mut highlights = vec![];

            for ((history_id, message_time), post_write_update) in
                post_write_updates.into_iter()
            {
                let message = if let Some(message) = self
                    .read_cache
                    .get_message_by_history_id(&history_id, &message_time)
                {
                    Some(Cow::Borrowed(message))
                } else {
                    process_message(
                        &self.kind,
                        messages,
                        &history_id,
                        &message_time,
                        filter_chain,
                        clients_context,
                        &config.buffer,
                    )
                    .map(Cow::Owned)
                };

                let Some(message) = message else {
                    log::error!("unable to find stored message");

                    continue;
                };

                if !message.blocked || message.inner.is_ours() {
                    self.show_in_sidebar = true;

                    self.latest = self.latest.max(Some(message.time().utc));

                    if message.inner.triggers_unread() {
                        self.latest_triggers_unread = self
                            .latest_triggers_unread
                            .max(Some(message.time().utc));
                    }

                    if message.inner.triggers_highlight() {
                        self.latest_triggers_highlight = self
                            .latest_triggers_highlight
                            .max(Some(message.time().utc));
                    }

                    match post_write_update.read_marker_update {
                        Some(ReadMarkerUpdate::Canonical) => {
                            display_read_marker_update =
                                display_read_marker_update.max(Some(
                                    ReadMarker::from(&message.inner),
                                ));
                            read_marker_update = read_marker_update
                                .max(Some(ReadMarker::from(&message.inner)));
                        }
                        Some(ReadMarkerUpdate::Display) => {
                            display_read_marker_update =
                                display_read_marker_update.max(Some(
                                    ReadMarker::from(&message.inner),
                                ));
                        }
                        None => (),
                    }

                    if let Some(highlight) = post_write_update.highlight {
                        highlights.push(highlight);
                    }

                    write_events.extend(post_write_update.events);
                }
            }

            if !highlights.is_empty() {
                write_events
                    .push(Event::History(Message::Highlights(highlights)));
            }

            self.display_read_marker =
                self.display_read_marker.max(display_read_marker_update);

            if read_marker_update > self.read_marker {
                self.read_marker = read_marker_update;

                if let Some(read_marker_update) = read_marker_update
                    && let Some(server) = server
                    && let Some(target) = self.kind.target()
                {
                    write_events.push(Event::Client(
                        client::Message::SendMarkread(
                            server.clone(),
                            target,
                            read_marker_update,
                        ),
                    ));
                }
            }

            self.display_read_marker =
                self.display_read_marker.max(display_read_marker_update);

            if delay_read {
                self.read_cache.read(
                    &self.kind,
                    messages,
                    &self.display_read_marker,
                    true,
                    filter_chain,
                    clients_context,
                    &config.buffer,
                );
            }

            write_events.push(Event::Model(model::Message::Update(
                self.kind.clone(),
                self.model_update(),
            )));

            if exiting {
                write_events
                    .into_iter()
                    .filter(|event| {
                        // Filter out model and notification events, since we are
                        // exiting
                        match event {
                            Event::History(_) => true,
                            Event::Model(_) => false,
                            Event::Notification(_, _) => false,
                            Event::Client(_) => true,
                        }
                    })
                    .collect()
            } else {
                write_events
            }
        } else {
            vec![]
        }
    }

    #[must_use]
    fn tick(
        &mut self,
        now: Instant,
    ) -> Option<BoxFuture<'static, (Kind, Result<usize, Error>)>> {
        if let Some(newest_unsaved_update) =
            self.write_buffer.newest_unsaved_update_at
            && (now.duration_since(newest_unsaved_update)
                >= SAVE_AFTER_DURATION_SINCE_UPDATE
                || self.write_buffer.unsaved_update_count
                    > SAVE_AFTER_UPDATE_COUNT)
        {
            self.save()
        } else {
            None
        }
    }

    fn save(
        &mut self,
    ) -> Option<BoxFuture<'static, (Kind, Result<usize, Error>)>> {
        if self.write_buffer.newest_unsaved_update_at.is_some()
            && !self.write_buffer.saving
        {
            self.write_buffer.saving = true;

            self.write_buffer.newest_unsaved_update_at = None;
            self.write_buffer.unsaved_update_count = 0;

            let kind = self.kind.clone();

            let metadata = Metadata {
                read_marker: self.read_marker,
                latest: self.latest,
                latest_triggers_unread: self.latest_triggers_unread,
                latest_triggers_highlight: self.latest_triggers_highlight,
                chathistory_references: self.chathistory_references.clone(),
            };

            let messages = self
                .messages
                .as_ref()
                .map(|messages| {
                    messages
                        [messages.len().saturating_sub(MAX_SAVED_MESSAGES)..]
                        .to_vec()
                })
                .unwrap_or_default();

            Some(
                async move {
                    let saved = match Storage::save_metadata(&kind, &metadata) {
                        Ok(()) => Storage::save_message_store(&kind, &messages)
                            .await
                            .map(|()| messages.len()),
                        Err(error) => Err(error),
                    };

                    (kind, saved)
                }
                .boxed(),
            )
        } else {
            None
        }
    }

    fn save_metadata(kind: &Kind, metadata: &Metadata) -> Result<(), Error> {
        let bytes = serde_json::to_vec(metadata)?;

        let path = kind_metadata_path(kind)?;

        Ok(fs::write(path, &bytes)?)
    }

    async fn save_message_store(
        kind: &Kind,
        messages: &Vec<message::Message>,
    ) -> Result<(), Error> {
        let compressed = compression::compress(messages)?;

        let path = kind_path(kind)?;

        Ok(tokio::fs::write(path, &compressed).await?)
    }

    fn saved(&mut self) {
        self.write_buffer.saving = false;
    }

    #[must_use]
    fn clear(
        &mut self,
        filter_chain: FilterChain,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) -> Event {
        self.read_cache.clear();

        self.read(false, filter_chain, clients_context, buffer_config);

        Event::Model(model::Message::Update(
            self.kind.clone(),
            self.model_update(),
        ))
    }

    #[must_use]
    fn mark_as_read(&mut self) -> Option<Event> {
        if self.latest.as_ref()
            > self.read_marker.as_ref().map(ReadMarker::as_date_time)
            && let Some(latest) = self.latest
        {
            let read_marker = ReadMarker::from(latest);

            self.read_marker = Some(read_marker);

            if let Some(server) = self.kind.as_server()
                && let Some(target) = self.kind.target()
            {
                return Some(Event::Client(client::Message::SendMarkread(
                    server.clone(),
                    target,
                    read_marker,
                )));
            }
        }

        None
    }

    /// Block, condense, and populate reply-previews for the `Storage`'s messages.
    #[allow(dead_code)]
    fn process(
        &mut self,
        filter_chain: FilterChain,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) -> Option<model::Update> {
        if let MessageCache::Loaded { messages, .. } =
            &mut self.read_cache.message_cache
        {
            process_messages(
                &self.kind,
                messages,
                filter_chain,
                clients_context,
                buffer_config,
            );

            log::debug!("processed messages in {}", self.kind);

            Some(self.model_update())
        } else {
            None
        }
    }

    #[must_use]
    fn model_update(&self) -> model::Update {
        let pane_update =
            self.read_cache.model_update(&self.display_read_marker);

        model::Update {
            show_in_sidebar: self.show_in_sidebar,
            read_marker: self.read_marker,
            display_read_marker: self.display_read_marker,
            latest: self.latest,
            latest_triggers_unread: self.latest_triggers_unread,
            latest_triggers_highlight: self.latest_triggers_highlight,
            pane: pane_update,
        }
    }

    fn last_can_reference_before(
        &self,
        server_time: DateTime<Utc>,
        message_reference_types: &[isupport::MessageReferenceType],
    ) -> Option<isupport::MessageReference> {
        self.last_can_reference(server_time, false, message_reference_types)
    }

    fn last_can_reference_before_or_at(
        &self,
        server_time: DateTime<Utc>,
        message_reference_types: &[isupport::MessageReferenceType],
    ) -> Option<isupport::MessageReference> {
        self.last_can_reference(server_time, true, message_reference_types)
    }

    fn last_can_reference(
        &self,
        server_time: DateTime<Utc>,
        allow_at_if_not_before: bool,
        message_reference_types: &[isupport::MessageReferenceType],
    ) -> Option<isupport::MessageReference> {
        let (before_message, at_message) = self
            .messages
            .as_ref()
            .map(|messages| {
                let can_reference_before = |message: &message::Message| {
                    message.can_reference(message_reference_types)
                        && !message.is_rerouted()
                        && message.time.utc < server_time
                };

                let mut at_message = None;

                let can_reference_at = |message: &message::Message| {
                    message.can_reference(message_reference_types)
                        && !message.is_rerouted()
                        && message.time.utc == server_time
                };

                let before_message = messages.iter().rev().find(|message| {
                    if allow_at_if_not_before
                        && at_message.is_none()
                        && can_reference_at(message)
                    {
                        at_message = Some(*message);
                    }

                    can_reference_before(message)
                });

                (before_message, at_message)
            })
            .unzip();

        // If a reference before server_time exists, then return that reference.
        if let Some(message_references) = before_message
            .flatten()
            .map(message::Message::references)
            .max(
                if self.chathistory_references.as_ref().is_some_and(
                    |chathistory_references| {
                        chathistory_references
                            .timestamp
                            .is_some_and(|timestamp| timestamp < server_time)
                    },
                ) {
                    self.chathistory_references.clone()
                } else {
                    None
                },
            )
        {
            return message_references
                .message_reference(message_reference_types);
        }

        // Else, if a reference at server_time is allowed and exists, then
        // return that reference.
        if allow_at_if_not_before
            && let Some(message_references) =
                at_message.flatten().map(message::Message::references).or(
                    if self.chathistory_references.as_ref().is_some_and(
                        |chathistory_references| {
                            chathistory_references.timestamp.is_some_and(
                                |timestamp| timestamp == server_time,
                            )
                        },
                    ) {
                        self.chathistory_references.clone()
                    } else {
                        None
                    },
                )
        {
            return message_references
                .message_reference(message_reference_types);
        }

        if allow_at_if_not_before
            && message_reference_types
                .contains(&isupport::MessageReferenceType::Timestamp)
        {
            Some(isupport::MessageReference::Timestamp(server_time))
        } else {
            None
        }
    }

    fn find_message_by_history_id(
        &self,
        history_id: &Id,
        time: &message::Time,
    ) -> Option<&message::Message> {
        self.messages.as_ref().and_then(|messages| {
            find_message_by_history_id(messages, history_id, time)
        })
    }

    fn find_message_by_id(
        &self,
        id: &message::Id,
        time: &message::Time,
    ) -> Option<&message::Message> {
        self.messages
            .as_ref()
            .and_then(|messages| find_message_by_id(messages, id, time))
    }
}

#[derive(Debug, Default)]
pub struct ReadCache {
    requested: Option<Request>,
    message_cache: MessageCache,
}

#[derive(Debug, Default)]
pub enum MessageCache {
    #[default]
    Unloaded,
    Loaded {
        has_more_older_messages: bool,
        has_more_newer_messages: bool,
        messages: Vec<message::MessageDisplay>,
        limit: message::Limit,
        clear: Option<DateTime<Utc>>,
    },
}

impl ReadCache {
    pub fn read(
        &mut self,
        kind: &Kind,
        messages: &[message::Message],
        display_read_marker: &Option<ReadMarker>,
        force: bool,
        filter_chain: FilterChain,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) {
        const OK_LOADED_FACTOR: usize = 2;
        const LOAD_FACTOR: usize = 4;

        let Some(Request {
            limit: requested_limit,
            clear: requested_clear,
        }) = &self.requested
        else {
            return;
        };

        let messages = if let Some(requested_clear) = &requested_clear {
            &messages
                [position_message_after_date_time(messages, requested_clear)..]
        } else {
            messages
        };

        let force = force
            || match &self.message_cache {
                MessageCache::Unloaded => true,
                MessageCache::Loaded { clear, .. } => clear != requested_clear,
            };

        let loaded = match &self.message_cache {
            MessageCache::Unloaded => None,
            MessageCache::Loaded {
                limit,
                clear,
                messages,
                ..
            } => Some((limit, clear, messages.len())),
        };

        let load_limit = match requested_limit {
            message::Limit::Top(requested_count) => {
                if !force
                    && let Some((
                        message::Limit::Top(loaded_count),
                        _,
                        loaded_messages_len,
                    )) = loaded
                    && *loaded_count
                        >= requested_count.saturating_mul(OK_LOADED_FACTOR)
                    && loaded_messages_len == *loaded_count
                {
                    return;
                }

                let load_count = requested_count.saturating_mul(LOAD_FACTOR);

                message::Limit::Top(load_count)
            }
            message::Limit::Bottom(requested_count) => {
                if !force
                    && let Some((
                        message::Limit::Bottom(loaded_count),
                        _,
                        loaded_messages_len,
                    )) = loaded
                    && *loaded_count
                        >= requested_count.saturating_mul(OK_LOADED_FACTOR)
                    && loaded_messages_len == *loaded_count
                {
                    return;
                }

                let load_count = requested_count.saturating_mul(LOAD_FACTOR);

                message::Limit::Bottom(load_count)
            }
            message::Limit::Around(requested_count, history_id) => {
                if !force {
                    let (_, requested_target) =
                        get_range_of_messages_by_message_limit(
                            messages,
                            display_read_marker,
                            requested_limit,
                            requested_clear,
                        );

                    if let Some((loaded_limit, loaded_clear, _)) = loaded {
                        let (loaded_range, _) =
                            get_range_of_messages_by_message_limit(
                                messages,
                                display_read_marker,
                                loaded_limit,
                                loaded_clear,
                            );

                        if (requested_target.saturating_sub(loaded_range.start)
                            >= requested_count.saturating_mul(OK_LOADED_FACTOR)
                            || loaded_range.start == 0)
                            && (loaded_range
                                .end
                                .saturating_sub(requested_target)
                                >= requested_count
                                    .saturating_mul(OK_LOADED_FACTOR)
                                || loaded_range.end == messages.len())
                        {
                            return;
                        }
                    }
                }

                let load_count = requested_count.saturating_mul(LOAD_FACTOR);

                message::Limit::Around(load_count, *history_id)
            }
            message::Limit::Backlog(requested_count) => {
                if !force {
                    let (_, requested_target) =
                        get_range_of_messages_by_message_limit(
                            messages,
                            display_read_marker,
                            requested_limit,
                            requested_clear,
                        );

                    if let Some((loaded_limit, loaded_clear, _)) = loaded {
                        let (loaded_range, _) =
                            get_range_of_messages_by_message_limit(
                                messages,
                                display_read_marker,
                                loaded_limit,
                                loaded_clear,
                            );

                        if (requested_target.saturating_sub(loaded_range.start)
                            >= requested_count.saturating_mul(OK_LOADED_FACTOR)
                            || loaded_range.start == 0)
                            && (loaded_range
                                .end
                                .saturating_sub(requested_target)
                                >= requested_count
                                    .saturating_mul(OK_LOADED_FACTOR)
                                || loaded_range.end == messages.len())
                        {
                            return;
                        }
                    }
                }

                let load_count = requested_count.saturating_mul(LOAD_FACTOR);

                message::Limit::Backlog(load_count)
            }
        };

        let load_clear = *requested_clear;

        let (load_range, _) = get_range_of_messages_by_message_limit(
            messages,
            display_read_marker,
            &load_limit,
            &load_clear,
        );

        let has_more_older_messages = load_range.start != 0;
        let has_more_newer_messages = load_range.end != messages.len();
        let mut load_messages = messages[load_range]
            .iter()
            .map(message::MessageDisplay::from)
            .collect::<Vec<message::MessageDisplay>>();

        process_messages(
            kind,
            &mut load_messages,
            filter_chain,
            clients_context,
            buffer_config,
        );

        self.message_cache = MessageCache::Loaded {
            has_more_older_messages,
            has_more_newer_messages,
            messages: load_messages,
            limit: load_limit,
            clear: load_clear,
        };
    }

    fn get_message_by_history_id(
        &self,
        history_id: &Id,
        time: &message::Time,
    ) -> Option<&message::MessageDisplay> {
        match &self.message_cache {
            MessageCache::Loaded { messages, .. } => {
                find_message_by_history_id(messages, history_id, time)
            }
            MessageCache::Unloaded => None,
        }
    }

    fn unload(&mut self) {
        self.message_cache = MessageCache::Unloaded;
    }

    fn clear(&mut self) {
        if let Some(Request { clear, .. }) = &mut self.requested {
            *clear = Some(Utc::now());
        }
    }

    fn model_update(
        &self,
        display_read_marker: &Option<ReadMarker>,
    ) -> model::Pane {
        if let Some(Request {
            limit: requested_limit,
            clear: requested_clear,
        }) = &self.requested
        {
            match &self.message_cache {
                MessageCache::Loaded {
                    has_more_older_messages,
                    has_more_newer_messages,
                    messages,
                    ..
                } => {
                    let (range, target) =
                        get_range_of_messages_by_message_limit(
                            messages,
                            display_read_marker,
                            requested_limit,
                            requested_clear,
                        );

                    let limit = match requested_limit {
                        message::Limit::Top(requested_count) => {
                            let count = range.end.saturating_sub(range.start);

                            message::Limit::Top(if *has_more_newer_messages {
                                count
                            } else {
                                *requested_count
                            })
                        }
                        message::Limit::Bottom(requested_count) => {
                            let count = range.end.saturating_sub(range.start);

                            message::Limit::Bottom(
                                if *has_more_older_messages {
                                    count
                                } else {
                                    *requested_count
                                },
                            )
                        }
                        message::Limit::Around(requested_count, history_id) => {
                            let before_count = if *has_more_older_messages {
                                target.saturating_sub(range.start)
                            } else {
                                *requested_count
                            };

                            let after_count = if *has_more_newer_messages {
                                range
                                    .end
                                    .saturating_sub(target)
                                    .saturating_add(1)
                            } else {
                                *requested_count
                            };

                            message::Limit::Around(
                                before_count.min(after_count),
                                *history_id,
                            )
                        }
                        message::Limit::Backlog(requested_count) => {
                            let before_count = if *has_more_older_messages {
                                target.saturating_sub(range.start)
                            } else {
                                *requested_count
                            };

                            let after_count = if *has_more_newer_messages {
                                range
                                    .end
                                    .saturating_sub(target)
                                    .saturating_add(1)
                            } else {
                                *requested_count
                            };

                            message::Limit::Backlog(
                                before_count.min(after_count),
                            )
                        }
                    };

                    if range.is_empty() {
                        model::Pane::Loading
                    } else {
                        let has_more_older_messages =
                            range.start > 0 || *has_more_older_messages;
                        let has_more_newer_messages = range.end
                            < messages.len()
                            || *has_more_newer_messages;

                        model::Pane::Open {
                            has_more_older_messages,
                            has_more_newer_messages,
                            messages: messages[range].to_vec(),
                            limit,
                            clear: *requested_clear,
                        }
                    }
                }
                MessageCache::Unloaded => model::Pane::Loading,
            }
        } else {
            model::Pane::Closed
        }
    }
}

#[derive(Debug, Default)]
pub struct WriteBuffer {
    updates: Vec<Update>,
    unsaved_update_count: usize,
    newest_unsaved_update_at: Option<Instant>,
    saving: bool,
}

/// Process a `Message`, specified by `history::Id` and `message::Time`
fn process_message(
    kind: &Kind,
    messages: &[message::Message],
    history_id: &Id,
    time: &message::Time,
    filter_chain: FilterChain,
    clients_context: &dyn ClientsContext,
    buffer_config: &config::Buffer,
) -> Option<message::MessageDisplay> {
    if let Some(position) =
        position_message_by_history_id(messages, history_id, time)
    {
        let mut message = message::MessageDisplay::from(&messages[position]);

        if let Source::Server(source) = &message.inner.source {
            let server = kind.as_server().or(message.inner.target.as_server());

            let casemapping =
                clients_context.get_maybe_server_casemapping_or_default(server);

            let target_ref = message.inner.target.as_targetref();

            let source_kind = source.as_ref().map(source::server::Server::kind);

            // Check if server message kind is disabled or target is excluded.
            if let Some(server) = server
                && let Some(target_ref) = target_ref
                && !buffer_config.server_messages.should_show_message(
                    source.as_ref(),
                    target_ref,
                    server,
                    casemapping,
                )
            {
                message.blocked = true;
            } else if let Some(seconds) =
                buffer_config.server_messages.smart(source_kind)
                && let Some(nick) =
                    source.as_ref().and_then(|source| source.nick())
            {
                // Check if server message is smart filtered.
                match source_kind {
                    Some(message::Kind::Away) => {
                        message.blocked = messages[..position]
                            .iter()
                            .rev()
                            .find_map(|historical_message| {
                                if let Source::Server(historical_source) =
                                    &historical_message.source
                                    && let Some(historical_source_kind) = source
                                        .as_ref()
                                        .map(source::server::Server::kind)
                                    && matches!(
                                        historical_source_kind,
                                        message::Kind::Away
                                    )
                                    && let Some(historical_nick) =
                                        historical_source.as_ref().and_then(
                                            |historical_source| {
                                                historical_source.nick()
                                            },
                                        )
                                    && *historical_nick == *nick
                                {
                                    return Some(smart_filter_repeat(
                                        &message.inner,
                                        &seconds,
                                        Some(&historical_message.time.utc),
                                    ));
                                }

                                if !smart_filter_repeat(
                                    &message.inner,
                                    &seconds,
                                    Some(&historical_message.time.utc),
                                ) {
                                    return Some(false);
                                }

                                None
                            })
                            .unwrap_or(false);
                    }
                    _ => {
                        message.blocked = messages[..position]
                            .iter()
                            .rev()
                            .find_map(|historical_message| {
                                if let Source::User(historical_message_user) =
                                    &historical_message.source
                                    && *historical_message_user.nickname()
                                        == *nick
                                {
                                    return Some(smart_filter_message(
                                        &message.inner,
                                        &seconds,
                                        Some(&historical_message.time.utc),
                                    ));
                                }

                                if smart_filter_message(
                                    &message.inner,
                                    &seconds,
                                    Some(&historical_message.time.utc),
                                ) {
                                    return Some(true);
                                }

                                None
                            })
                            .unwrap_or(true);
                    }
                }
            }
        }

        message.blocked = message.blocked
            || !filter_chain.filter_message_of_kind(&message.inner, kind);

        Some(message)
    } else {
        None
    }
}

/// Process `MessageDisplay`s, determining their block, condense, and
/// reply-previews.
fn process_messages(
    kind: &Kind,
    messages: &mut [message::MessageDisplay],
    filter_chain: FilterChain,
    clients_context: &dyn ClientsContext,
    buffer_config: &config::Buffer,
) {
    block_messages(
        kind,
        messages,
        filter_chain,
        clients_context,
        buffer_config,
    );

    condense_messages(messages, buffer_config);

    populate_messages_reply_previews(messages);
}

/// Determine which messages should be blocked (hidden).
fn block_messages(
    kind: &Kind,
    messages: &mut [message::MessageDisplay],
    filter_chain: FilterChain,
    clients_context: &dyn ClientsContext,
    buffer_config: &config::Buffer,
) {
    let current_time = Utc::now();

    let mut last_seen = HashMap::<Nick, DateTime<Utc>>::new();
    let mut last_away = HashMap::<Nick, DateTime<Utc>>::new();

    messages.iter_mut().for_each(|message| {
        message.blocked = false;

        if message.inner.has_redaction()
            && !buffer_config.redaction.display.is_visible()
        {
            message.blocked = true;
        } else {
            match &message.inner.source {
                Source::Server(source) => {
                    let server =
                        kind.as_server().or(message.inner.target.as_server());

                    let casemapping = clients_context
                        .get_maybe_server_casemapping_or_default(server);

                    let target_ref = message.inner.target.as_targetref();

                    let source_kind =
                        source.as_ref().map(source::server::Server::kind);

                    // Check if server message kind is disabled or target is
                    // excluded.
                    if let Some(target_ref) = target_ref
                        && let Some(server) = server
                        && !buffer_config.server_messages.should_show_message(
                            source.as_ref(),
                            target_ref,
                            server,
                            casemapping,
                        )
                    {
                        message.blocked = true;
                    } else if let Some(seconds) =
                        buffer_config.server_messages.smart(source_kind)
                        && let Some(nick) =
                            source.as_ref().and_then(|source| source.nick())
                    {
                        // Check if server message is smart filtered.
                        match source_kind {
                            Some(message::Kind::Away) => {
                                message.blocked = smart_filter_repeat(
                                    &message.inner,
                                    &seconds,
                                    last_away.get(nick),
                                );

                                if !message.blocked {
                                    last_away.insert(
                                        nick.clone(),
                                        message.time().utc,
                                    );
                                }
                            }
                            _ => {
                                message.blocked = smart_filter_message(
                                    &message.inner,
                                    &seconds,
                                    last_seen.get(nick),
                                );
                            }
                        }
                    }
                }
                Source::User(message_user) => {
                    last_seen.insert(
                        message_user.nickname().to_owned(),
                        message.time().utc,
                    );
                }
                Source::Internal(source::Internal::Status(status)) => {
                    if !buffer_config.internal_messages.enabled(status) {
                        message.blocked = true;
                    } else if let Some(seconds) =
                        buffer_config.internal_messages.smart(status)
                    {
                        message.blocked = smart_filter_internal_message(
                            &message.inner,
                            &seconds,
                            &current_time,
                        );
                    }
                }
                _ => (),
            }
        }

        message.blocked = message.blocked
            || filter_chain.filter_message_of_kind(&message.inner, kind);
    });
}

fn condense_messages(
    messages: &mut [message::MessageDisplay],
    buffer_config: &config::Buffer,
) {
    #[derive(PartialEq)]
    enum CondensationKey {
        Condensable(NaiveDate),
        Singular,
    }

    messages
        .iter_mut()
        .filter(|message| !message.blocked)
        .chunk_by(|message| {
            if message
                .inner
                .can_condense(&buffer_config.server_messages.condense)
            {
                CondensationKey::Condensable(
                    message.time().utc.with_timezone(&Local).date_naive(),
                )
            } else {
                CondensationKey::Singular
            }
        })
        .into_iter()
        .for_each(|(key, chunk)| match key {
            CondensationKey::Condensable(_) => {
                let mut condensable_messages =
                    chunk.collect::<Vec<&mut message::MessageDisplay>>();

                let condensed_message = message::condense(
                    &condensable_messages
                        .iter()
                        .map(|message| &**message)
                        .collect::<Vec<&message::MessageDisplay>>(),
                    &buffer_config.server_messages.condense,
                );

                condensable_messages
                    .iter_mut()
                    .for_each(|message| message.condensed = None);

                if let Some(first_message) = condensable_messages.first_mut() {
                    first_message.condensed = condensed_message;
                }
            }
            CondensationKey::Singular => chunk
                .collect::<Vec<&mut message::MessageDisplay>>()
                .iter_mut()
                .for_each(|message| message.condensed = None),
        });
}

// TODO: Retrieve reply previews for messages outside read cache
/// Backfill previews for replies for messages in a history batch
fn populate_messages_reply_previews(messages: &mut [message::MessageDisplay]) {
    let position_pairs: Vec<(usize, usize)> = messages
        .iter()
        .enumerate()
        .filter_map(|(message_position, message)| {
            message
                .inner
                .reply_to
                .as_ref()
                .and_then(|reply_to_id| {
                    position_message_by_id(
                        messages,
                        reply_to_id,
                        message.time(),
                    )
                })
                .map(|reply_to_position| (message_position, reply_to_position))
        })
        .collect();

    for (message_position, reply_to_position) in position_pairs {
        if let Some(reply_preview) = messages
            .get(reply_to_position)
            .map(message::MessageDisplay::as_reply_preview)
            && let Some(message) = messages.get_mut(message_position)
        {
            message.reply_preview = Some(reply_preview);
        }
    }
}

/// Insert the incoming message into the provided vector, sorted on server
/// time.
///
/// Deduplication is peformed for:
///  - Messages that the server has marked as historical (e.g. chathistory or
///    ZNC-playback)
///  - Messages with an exact ID match
///  - Echoes (labeled via labeled-response, or unlabeled)
///
/// For non-echoes a search window of +/- 1 second around the server time of the
/// incoming message is used.
///
/// For labeled echoes the exact time should be either be stored locally, or the
/// message was sent from another client.
///
/// For unlabled echoes a search window of +/- 300s is used to account for
/// transit time and potential clock skew.
///
/// For matching methods that do not have an identifier (i.e. when matching
/// historical messages without an ID or unlabled echoes) the messages must have
/// an exact match + target & / content.
///
/// The return value is whether a message sent from this client was was replaced
/// by an echo.
pub fn insert_message(
    messages: &mut Vec<message::Message>,
    message: message::MessageWithContext,
) -> bool {
    if messages.is_empty() {
        messages.push(message.into());

        return false;
    }

    let message_is_unlabeled_echo =
        message.inner.is_echo() && message.labeled_response_context.is_none();

    let fuzz_seconds = if message_is_unlabeled_echo {
        chrono::Duration::seconds(300)
    } else {
        chrono::Duration::seconds(1)
    };

    let mut replaced_sent = false;

    if let Some(labeled_response_context) = &message.labeled_response_context {
        let start = labeled_response_context.time.utc - fuzz_seconds;
        let end = labeled_response_context.time.utc + fuzz_seconds;

        let start_index = match messages
            .binary_search_by(|stored| stored.time.utc.cmp(&start))
        {
            Ok(match_index) => match_index,
            Err(sorted_insert_index) => sorted_insert_index,
        };
        let end_index = match messages
            .binary_search_by(|stored| stored.time.utc.cmp(&end))
        {
            Ok(match_index) => match_index,
            Err(sorted_insert_index) => sorted_insert_index,
        };

        if let Some(index) = messages[start_index..end_index]
            .iter()
            .enumerate()
            .find_map(|(slice_index, stored)| {
                (stored.id.as_ref().is_some_and(|id| {
                    *id == labeled_response_context.label_as_id
                }) && stored.source == message.inner.source)
                    .then_some(start_index + slice_index)
            })
        {
            messages.remove(index);

            replaced_sent = true;
        }
    }

    let start = message.time().utc - fuzz_seconds;
    let end = message.time().utc + fuzz_seconds;

    let start_index =
        match messages.binary_search_by(|stored| stored.time.utc.cmp(&start)) {
            Ok(match_index) => match_index,
            Err(sorted_insert_index) => sorted_insert_index,
        };
    let end_index =
        match messages.binary_search_by(|stored| stored.time.utc.cmp(&end)) {
            Ok(match_index) => match_index,
            Err(sorted_insert_index) => sorted_insert_index,
        };

    let mut insert_at = start_index;
    let mut replace_at = None;

    for (current_index, stored) in
        (start_index..).zip(messages[start_index..end_index].iter())
    {
        if replace_at.is_none() && message.labeled_response_context.is_none() {
            let use_echo_cmp = stored.is_sent() && message_is_unlabeled_echo;

            let check_for_matching_content = (stored.id.is_none()
                || message.id().is_none())
                && ((message.historical && stored.time == *message.time())
                    || use_echo_cmp);

            if (message.id().is_some() && stored.id == *message.id())
                || (check_for_matching_content
                    && has_matching_content(
                        stored,
                        &message.inner,
                        use_echo_cmp,
                    ))
            {
                replace_at = Some(current_index);
                break;
            }
        }

        if *message.time() >= stored.time {
            insert_at = current_index + 1;
        }
    }

    if let Some(index) = replace_at {
        if messages[index].time == *message.time() {
            if message.historical
                && has_matching_content(&messages[index], &message.inner, false)
            {
                // Perform a minimal update if this is a message from
                // chathistory (or ZNC playback) and has the same raw content,
                // since the newly received message will have been parsed
                // without historical state.
                if messages[index].id.is_none() {
                    messages[index].id = message.inner.id;
                }

                messages[index].direction = message.inner.direction;
            } else {
                messages[index] = message::Message {
                    id: message.id().clone().or(messages[index].id.clone()),
                    ..message.into()
                };
            }
        } else {
            if message_is_unlabeled_echo {
                replaced_sent = true;
            }

            match insert_at.cmp(&index) {
                Ordering::Less => {
                    messages.remove(index);
                    messages.insert(insert_at, message.into());
                }
                Ordering::Equal => messages[index] = message.into(),
                Ordering::Greater => {
                    messages.insert(insert_at, message.into());
                    messages.remove(index);
                }
            }
        }
    } else {
        messages.insert(insert_at, message.into());
    }

    replaced_sent
}

/// The content of JOIN, PART, and QUIT messages may be dependent on how
/// the user attributes are resolved.  Match those messages based on Nick
/// alone (covered by comparing target components) to avoid false negatives.
fn has_matching_content(
    message: &message::Message,
    other: &message::Message,
    use_echo_cmp: bool,
) -> bool {
    if message.target == other.target {
        if let message::Source::Server(Some(source)) = &message.source {
            match source.kind() {
                message::source::server::Kind::Join
                | message::source::server::Kind::Part
                | message::source::server::Kind::Quit => {
                    return true;
                }
                message::source::server::Kind::JoinTopic
                | message::source::server::Kind::RequestTopic
                | message::source::server::Kind::ChangeHost
                | message::source::server::Kind::ChangeNick
                | message::source::server::Kind::ChangeMode
                | message::source::server::Kind::ChangeTopic
                | message::source::server::Kind::MonitoredOnline
                | message::source::server::Kind::MonitoredOffline
                | message::source::server::Kind::StandardReply(_)
                | message::source::server::Kind::WAllOps
                | message::source::server::Kind::Kick
                | message::source::server::Kind::Away
                | message::source::server::Kind::Invite => (),
            }
        }

        if use_echo_cmp {
            matches!(message.content.echo_cmp(&other.content), Ordering::Equal)
        } else {
            message.content == other.content
        }
    } else {
        false
    }
}

pub fn insert_reaction(
    reactions: &mut Vec<reaction::Reaction>,
    reaction: reaction::ReactionWithContext,
) {
    if reactions.is_empty() {
        reactions.push(reaction.into());

        return;
    }

    if let Some(labeled_response_context) = &reaction.labeled_response_context {
        if let Some(index) = reactions.iter().position(|stored| {
            stored
                .id
                .as_ref()
                .is_some_and(|id| *id == labeled_response_context.label_as_id)
        }) {
            reactions.remove(index);
        }

        reactions.push(reaction.into());
    } else if let Some(index) = reactions.iter().position(|stored| {
        (stored.id.is_some() && stored.id == reaction.inner.id)
            || (reaction.historical
                && (stored.time == reaction.inner.time || reaction.is_echo())
                && stored.sender == reaction.inner.sender
                && stored.text == reaction.inner.text
                && stored.unreact == reaction.inner.unreact)
    }) {
        // Reactions were previously stored without IDs, so deduplicate by
        // matching content for historical reactions.
        reactions[index] = reaction.into();
    } else {
        reactions.push(reaction.into());
    }
}

/// Outputs is the range and anchor position within the messages, as prescribed
/// by the limit.
fn get_range_of_messages_by_message_limit<M>(
    messages: &[M],
    read_marker: &Option<ReadMarker>,
    limit: &message::Limit,
    clear: &Option<DateTime<Utc>>,
) -> (Range<usize>, usize)
where
    M: message::Searchable,
{
    let messages = if let Some(clear) = clear {
        &messages[position_message_after_date_time(messages, clear)..]
    } else {
        messages
    };

    match limit {
        message::Limit::Top(count) => (0..messages.len().min(*count), 0),
        message::Limit::Bottom(count) => (
            messages.len().saturating_sub(*count)..messages.len(),
            messages.len().saturating_sub(1),
        ),
        message::Limit::Around(count, history_id) => {
            // TODO: When upgraded to SQLite make sure this is something
            // performant, unlike this linear search.
            if let Some(position) = messages
                .iter()
                .position(|message| message.history_id() == history_id)
            {
                (
                    position.saturating_sub(*count)
                        ..position.saturating_add(*count).max(messages.len()),
                    position,
                )
            } else {
                (0..0, 0)
            }
        }
        message::Limit::Backlog(count) => {
            if let Some(read_marker) = read_marker {
                let position = position_message_after_date_time(
                    messages,
                    read_marker.as_date_time(),
                );

                (
                    position.saturating_sub(*count)
                        ..position.saturating_add(*count).max(messages.len()),
                    position,
                )
            } else {
                (0..messages.len().min(*count), 0)
            }
        }
    }
}

// TODO: get this from database or UUID
fn determine_history_id(message: &mut message::Message) {
    message.history_id = Id::Determined(Posix::now().as_nanos());
}

fn write_chathistory_targets_timestamp(
    server: &Server,
    timestamp: DateTime<Utc>,
) -> Result<(), Error> {
    let bytes = serde_json::to_vec(&Some(timestamp))?;

    let path = chathistory_targets_path(server)?;

    Ok(fs::write(path, &bytes)?)
}

fn chathistory_targets_path(server: &Server) -> Result<PathBuf, Error> {
    let dir = dir_path()?;

    let name = format!("{server}-targets");

    Ok(dir.join(format!("{name}.json")))
}

fn kind_metadata_path(kind: &Kind) -> Result<PathBuf, Error> {
    let dir = dir_path()?;

    let name = match kind {
        Kind::Server(server) => format!("{server}-metadata"),
        Kind::Channel(server, channel) => {
            format!("{server}channel{}-metadata", channel.as_normalized_str())
        }
        Kind::Query(server, query) => {
            format!("{server}nickname{}-metadata", query.as_normalized_str())
        }
        Kind::Logs => "logs-metadata".to_string(),
        Kind::Highlights => "highlights-metadata".to_string(),
        Kind::ChannelMonitor => "channel-monitor-metadata".to_string(),
    };

    Ok(dir.join(format!("{name}.json")))
}

fn kind_path(kind: &Kind) -> Result<PathBuf, Error> {
    let dir = dir_path()?;

    let name = match kind {
        Kind::Server(server) => format!("{server:b}"),
        Kind::Channel(server, channel) => {
            format!("{server:b}channel{}", channel.as_normalized_str())
        }
        Kind::Query(server, query) => {
            format!("{server:b}nickname{}", query.as_normalized_str())
        }
        Kind::Logs => "logs".to_string(),
        Kind::Highlights => "highlights".to_string(),
        Kind::ChannelMonitor => "channel_monitor".to_string(),
    };

    Ok(dir.join(format!("{name}.json.gz")))
}

fn dir_path() -> Result<PathBuf, Error> {
    let data_dir = environment::data_dir();

    let history_dir = data_dir.join("msdb");

    if !history_dir.exists() {
        fs::create_dir_all(&history_dir)?;
    }

    Ok(history_dir)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Compression(#[from] compression::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}
