use std::cmp::Ordering;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{fs, io};

use chrono::{self, DateTime, Local, NaiveDate, Utc};
use hashbrown::{HashMap, HashSet, hash_map};
use iced::Task;
use itertools::Itertools;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::filter::{Filter, FilterChain};
use super::reroute::RerouteRules;
use super::{
    Id, Kind, Metadata, ReadMarker, Request, database, model,
    smart_filter_internal_message, smart_filter_message, smart_filter_repeat,
};
use crate::buffer::{self, BuffersContext};
use crate::client::ClientsContext;
use crate::config::buffer::OnMessage;
use crate::message::{self, Searchable, Source, Temporal, broadcast, source};
use crate::target::{self, Target};
use crate::user::Nick;
use crate::{
    Config, Notification, Server, client, config, environment, input, reaction,
    redaction, server,
};

mod batch;
mod cache;
mod worker;
use cache::{MessageCache, ReadCache};

const CLEAR_AFTER_DURATION_SINCE_CLOSED: Duration = Duration::from_secs(8);

#[derive(Debug)]
pub enum Message {
    Initialized(Kind, Metadata),
    Read(Kind, cache::Read, database::Window),
    Committed(Vec<batch::Committed>),
    Importing(Kind),
    Failed(String),
    Unrecorded(batch::Batch),
    DraftsSaved(Result<usize, Error>),
    Exited(Result<(), String>, Option<Result<usize, Error>>),
}

#[derive(Debug)]
pub enum Event {
    History(Message),
    Model(model::Message),
    Notification(Server, Notification),
    Client(client::Message),
}

#[derive(Debug, Clone, Copy)]
pub enum ReferenceQuery {
    Oldest,
    Before(DateTime<Utc>),
}

#[derive(Debug)]
pub enum Update {
    Message(Option<Server>, message::MessageWithContext),
    Broadcast(Server, message::BroadcastWithContext),
    Reaction(Server, reaction::ReactionWithContext),
    Redaction(Server, redaction::RedactionWithContext),
    Remove(Kind, Id, message::Time),
    ShowPreview(Kind, Id, message::Time, url::Url),
    HidePreview(Kind, Id, message::Time, url::Url),
    ReadMarker(Kind, ReadMarker),
    ShowInSidebar(Kind, bool),
}

#[derive(Debug)]
pub struct Manager {
    storage: HashMap<Kind, Storage>,
    worker: worker::Worker,
    input_storage: input::Storage,
    filters: Vec<Filter>,
    reroute_rules: RerouteRules,
    event_sender: mpsc::UnboundedSender<Vec<Event>>,
    monitored: Vec<Kind>,
    failure: Option<String>,
    exiting: bool,
}

impl Manager {
    pub fn new(config: &Config) -> (Self, Task<Vec<Event>>) {
        let mut input_storage = input::Storage::default();

        input_storage.load(config);

        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        (
            Self {
                storage: HashMap::new(),
                worker: worker::Worker::new(event_sender.clone()),
                input_storage,
                filters: Vec::new(),
                reroute_rules: RerouteRules::default(),
                event_sender,
                monitored: vec![],
                failure: None,
                exiting: false,
            },
            Task::stream(UnboundedReceiverStream::new(event_receiver)),
        )
    }

    #[cfg(test)]
    pub fn test() -> Self {
        let (event_sender, _) = mpsc::unbounded_channel();

        Self {
            storage: HashMap::new(),
            worker: worker::Worker::test(event_sender.clone()),
            input_storage: input::Storage::default(),
            filters: Vec::new(),
            reroute_rules: RerouteRules::default(),
            event_sender,
            monitored: vec![],
            failure: None,
            exiting: false,
        }
    }

    /// Queue a batch. Models and side effects update after its transaction commits.
    pub fn write(
        &mut self,
        updates: Vec<Update>,
        clients_context: &dyn ClientsContext,
        buffers_context: &dyn BuffersContext,
        focused_window: &Option<iced::window::Id>,
        config: &Config,
    ) {
        let mut updates_by_kind = HashMap::<Kind, Vec<Update>>::new();

        for update in updates.into_iter() {
            if let Some(kind) = match &update {
                Update::Message(server, message) => {
                    Kind::from_message(&message.inner, server.as_ref())
                }
                Update::Broadcast(..) => None,
                Update::Reaction(server, reaction) => Some(Kind::from_target(
                    server.clone(),
                    reaction.target.clone(),
                )),
                Update::Redaction(server, redaction) => Some(
                    Kind::from_target(server.clone(), redaction.target.clone()),
                ),
                Update::Remove(kind, ..)
                | Update::ShowPreview(kind, ..)
                | Update::HidePreview(kind, ..)
                | Update::ReadMarker(kind, ..)
                | Update::ShowInSidebar(kind, ..) => Some(kind.clone()),
            } {
                let kind_updates =
                    updates_by_kind.entry(kind).or_insert(vec![]);

                kind_updates.push(update);
            } else if let Update::Broadcast(server, mut broadcast) = update {
                let mut targets = match &mut broadcast.in_channels {
                    broadcast::Channels::All => self
                        .storage
                        .keys()
                        .filter_map(|kind| {
                            if let Kind::Channel(kind_server, channel) = kind
                                && *kind_server == server
                            {
                                Some(message::Target::Channel {
                                    channel: channel.clone(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<message::Target>>(),
                    broadcast::Channels::Vec(channels) => {
                        std::mem::take(channels)
                            .into_iter()
                            .map(|channel| message::Target::Channel { channel })
                            .collect::<Vec<message::Target>>()
                    }
                };

                if broadcast.in_server {
                    targets.push(message::Target::Server);
                }

                match std::mem::take(&mut broadcast.in_queries) {
                    broadcast::Queries::All => {
                        targets.extend(self.storage.keys().filter_map(
                            |kind| {
                                if let Kind::Query(kind_server, query) = kind
                                    && *kind_server == server
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
                    clients_context.get_server_casemapping_or_default(&server),
                    config,
                ) {
                    if let Some(kind) =
                        Kind::from_server_message(&server, &message)
                    {
                        let update = Update::Message(
                            Some(server.clone()),
                            message::MessageWithContext {
                                inner: message,
                                highlight: None,
                                historical: false,
                                labeled_response_context: None,
                                notification_allowed: false,
                            },
                        );

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

        if self.exiting {
            return;
        }
        for (kind, updates) in &updates_by_kind {
            if let Kind::Channel(server, channel) = kind
                && config.channel_monitor.is_channel_included(
                    server,
                    channel,
                    clients_context.get_server_casemapping_or_default(server),
                )
                && !self.monitored.contains(kind)
            {
                self.monitored.push(kind.clone());
            }
            let storage = self.get_or_load_mut(kind.clone());
            for update in updates {
                let seen = match update {
                    Update::Message(_, message) => message
                        .inner
                        .user()
                        .map(|user| (user.nickname(), message.inner.time.utc)),
                    Update::Reaction(_, reaction) => {
                        Some((&reaction.inner.sender, reaction.inner.time.utc))
                    }
                    Update::Redaction(_, redaction) => {
                        Some((&redaction.inner.from, redaction.time.utc))
                    }
                    _ => None,
                };
                if let Some((nick, time)) = seen {
                    storage
                        .last_seen
                        .entry(nick.clone())
                        .and_modify(|seen| *seen = (*seen).max(time))
                        .or_insert(time);
                }
            }
        }
        let batch = batch::Batch::capture(
            updates_by_kind,
            &self.filters,
            clients_context,
            buffers_context,
            focused_window,
            config,
        );
        for kind in batch.kinds() {
            self.get_or_load_mut(kind);
        }
        if self.failure.is_some() {
            self.unrecorded(batch, clients_context, &config.buffer);
        } else {
            self.worker.send(worker::Command::Write(batch));
        }
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
            let events = self.event_sender.clone();
            tokio::task::spawn(async move {
                let _ = events.send(vec![Event::History(
                    Message::DraftsSaved(save_future.await),
                )]);
            });
        }
        for storage in self.storage.values_mut() {
            if let Request::Closed { at: Some(closed) } =
                storage.read_cache.requested
                && self.failure.is_none()
                && now.duration_since(closed)
                    >= CLEAR_AFTER_DURATION_SINCE_CLOSED
            {
                storage.read_cache.clear();
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
            Message::Initialized(kind, metadata) => {
                let storage = self
                    .storage
                    .entry(kind.clone())
                    .or_insert_with(|| Storage::from(kind.clone()));
                storage.importing = false;
                storage.apply_metadata(metadata, None);
                self.refresh(&kind);
                self.publish(&kind);
            }
            Message::Read(kind, read, window) => {
                if self.failure.is_some() {
                    return;
                }
                let visible_through = window
                    .messages
                    .last()
                    .map(|message| ReadMarker::from(&message.time));
                let newest_visible = !window.has_more_newer;
                let older =
                    read.extension.is_some_and(|extension| extension.older);
                let Some(storage) = self.storage.get_mut(&kind) else {
                    return;
                };
                let accepted = storage.read_cache.loaded(
                    &read,
                    window,
                    &kind,
                    FilterChain::borrow(&self.filters),
                    clients_context,
                    &config.buffer,
                );
                let mark = if accepted && !older {
                    let at_bottom = match config.buffer.mark_as_read.on_message
                    {
                        OnMessage::Focused => {
                            buffers_context.is_focused_and_at_bottom(&kind)
                        }
                        OnMessage::Open => buffers_context
                            .is_open_and_at_bottom_in_focused_window(&kind),
                        OnMessage::None => false,
                    };
                    let admitted = storage.auto_read.take();
                    if at_bottom && newest_visible {
                        admitted
                            .filter(|marker| Some(*marker) <= visible_through)
                            .and_then(|marker| {
                                storage.read_cache.visible_through(marker)
                            })
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(marker) = mark {
                    self.worker.send(worker::Command::MarkRead(
                        kind.clone(),
                        Some(marker),
                        true,
                    ));
                }
                self.refresh(&kind);
                if accepted {
                    self.publish(&kind);
                }
            }
            Message::Committed(committed) => {
                let mut events = vec![];
                let mut monitor_changed = false;
                for committed in committed {
                    let kind = committed.kind;
                    let storage = self
                        .storage
                        .entry(kind.clone())
                        .or_insert_with(|| Storage::from(kind.clone()));
                    storage.apply_metadata(
                        committed.metadata,
                        committed.display_read_marker,
                    );
                    if let Some(show) = committed.show_in_sidebar {
                        storage.show_in_sidebar = show;
                    }
                    if matches!(
                        storage.read_cache.requested,
                        Request::Open { .. }
                    ) {
                        storage.auto_read = storage.auto_read.max(
                            committed.admitted_latest.map(ReadMarker::from),
                        );
                    }
                    match committed.change {
                        batch::Change::Metadata => (),
                        batch::Change::Append => {
                            storage.read_cache.committed(true);
                        }
                        batch::Change::Replace => {
                            storage.read_cache.committed(false);
                            monitor_changed |= self.monitored.contains(&kind);
                        }
                    }
                    events.extend(committed.events);
                    self.refresh(&kind);
                    self.publish(&kind);
                }
                if monitor_changed
                    && let Some(storage) =
                        self.storage.get_mut(&Kind::ChannelMonitor)
                {
                    storage.read_cache.changed();
                    self.refresh(&Kind::ChannelMonitor);
                }
                if !self.exiting {
                    let _ = self.event_sender.send(events);
                } else {
                    let _ = self.event_sender.send(
                        events
                            .into_iter()
                            .filter(|event| matches!(event, Event::Client(_)))
                            .collect(),
                    );
                }
            }
            Message::Importing(kind) => {
                if let Some(storage) = self.storage.get_mut(&kind) {
                    storage.importing = true;
                }
                self.publish(&kind);
            }
            Message::Failed(error) => {
                log::error!(
                    "History is not being recorded. New messages are available only in this session. Restart Halloy after resolving: {error}"
                );
                self.failure = Some(error);
                for storage in self.storage.values_mut() {
                    storage.importing = false;
                    storage.read_cache.failed();
                    let _ = self.event_sender.send(vec![Event::Model(
                        model::Message::Update(
                            storage.kind.clone(),
                            storage.model_update(),
                        ),
                    )]);
                }
            }
            Message::Unrecorded(batch) => {
                self.unrecorded(batch, clients_context, &config.buffer);
            }
            Message::DraftsSaved(result) => {
                if let Err(error) = result {
                    log::error!("failed to save input drafts: {error}");
                }
                self.input_storage.saved();
            }
            Message::Exited(result, input_result) => {
                if let Err(error) = result {
                    log::error!("history shutdown failed: {error}");
                }
                if let Some(Err(error)) = input_result {
                    log::error!("failed to save input drafts: {error}");
                }
            }
        }
    }

    fn unrecorded(
        &mut self,
        batch: batch::Batch,
        clients: &dyn ClientsContext,
        config: &config::Buffer,
    ) {
        let mut changed = HashSet::new();
        for (kind, message) in batch.transient() {
            let storage = self
                .storage
                .entry(kind.clone())
                .or_insert_with(|| Storage::from(kind.clone()));
            if self.failure.is_some() {
                storage.read_cache.failed();
            }
            if !FilterChain::borrow(&self.filters)
                .filter_message_of_kind(&message, &kind)
                || message.is_ours()
            {
                storage.show_in_sidebar = true;
            }
            storage.read_cache.unrecorded(
                message,
                &kind,
                FilterChain::borrow(&self.filters),
                clients,
                config,
            );
            changed.insert(kind);
        }
        for kind in changed {
            self.publish(&kind);
        }
    }

    fn refresh(&mut self, kind: &Kind) {
        if self.failure.is_some() || self.exiting {
            return;
        }
        let Some(storage) = self.storage.get_mut(kind) else {
            return;
        };
        if let Some(read) =
            storage.read_cache.request(storage.display_read_marker)
        {
            self.worker.send(worker::Command::Read(
                kind.clone(),
                read,
                if *kind == Kind::ChannelMonitor {
                    self.monitored.clone()
                } else {
                    vec![]
                },
            ));
        }
    }

    fn publish(&self, kind: &Kind) {
        if let Some(storage) = self.storage.get(kind) {
            let _ = self.event_sender.send(vec![Event::Model(
                model::Message::Update(kind.clone(), storage.model_update()),
            )]);
        }
    }

    pub fn show_in_sidebar(&mut self, kind: Kind, show: bool) {
        self.get_or_load_mut(kind.clone()).show_in_sidebar = show;
        self.publish(&kind);
    }

    pub fn mark_as_read(&mut self, kind: Kind) {
        self.get_or_load_mut(kind.clone());
        if self.failure.is_none() && !self.exiting {
            self.worker
                .send(worker::Command::MarkRead(kind, None, true));
        }
    }

    pub fn mark_server_as_read(&mut self, server: &Server) {
        let kinds = self
            .storage
            .keys()
            .filter(|kind| kind.as_server() == Some(server))
            .cloned()
            .collect::<Vec<_>>();
        for kind in kinds {
            self.mark_as_read(kind);
        }
    }

    pub fn set_model_limit(&mut self, kind: Kind, limit: message::Limit) {
        self.get_or_load_mut(kind.clone())
            .read_cache
            .set_model_limit(limit);
        self.refresh(&kind);
        self.publish(&kind);
    }

    pub fn clear_model(&mut self, kind: Kind) {
        self.get_or_load_mut(kind.clone()).read_cache.clear_model();
        self.refresh(&kind);
        self.publish(&kind);
    }

    pub fn close_model(
        &mut self,
        kind: Kind,
        is_scrolled_to_bottom: bool,
        hide_in_sidebar: bool,
        config: &Config,
    ) {
        let storage = self.get_or_load_mut(kind.clone());
        storage.read_cache.close_model();
        storage.auto_read = None;
        if hide_in_sidebar {
            storage.show_in_sidebar = false;
        }
        if self.failure.is_none()
            && !self.exiting
            && config
                .buffer
                .mark_as_read
                .on_buffer_close
                .mark_as_read(is_scrolled_to_bottom)
        {
            self.worker.send(worker::Command::MarkRead(
                kind.clone(),
                None,
                true,
            ));
        }
        self.publish(&kind);
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

    pub fn request_chathistory_reference(
        &mut self,
        lookup: client::ChathistoryLookup,
    ) {
        if self.failure.is_some() || self.exiting {
            let _ = self.event_sender.send(vec![Event::Client(
                client::Message::ChathistoryReference(lookup, None),
            )]);
        } else {
            self.worker.send(worker::Command::Reference(lookup));
        }
    }

    pub fn request_highlight_source(
        &mut self,
        mut navigation: model::Navigation,
        highlight: Id,
    ) {
        navigation.message = None;
        if self.failure.is_some() || self.exiting {
            let _ = self.event_sender.send(vec![Event::Model(
                model::Message::GoToMessage(navigation),
            )]);
        } else {
            self.worker
                .send(worker::Command::HighlightSource(navigation, highlight));
        }
    }

    pub fn request_resend_message(&mut self, lookup: client::ResendLookup) {
        if self.failure.is_some() || self.exiting {
            let _ = self.event_sender.send(vec![Event::Client(
                client::Message::ResendMessage(lookup, None),
            )]);
        } else {
            self.worker.send(worker::Command::Resend(lookup));
        }
    }

    fn get_or_load_mut(&mut self, kind: Kind) -> &mut Storage {
        match self.storage.entry(kind) {
            hash_map::Entry::Occupied(entry) => entry.into_mut(),
            hash_map::Entry::Vacant(entry) => {
                if self.failure.is_none() && !self.exiting {
                    self.worker
                        .send(worker::Command::Initialize(entry.key().clone()));
                }
                let mut storage = Storage::from(entry.key().clone());
                if self.failure.is_some() {
                    storage.read_cache.failed();
                }
                entry.insert(storage)
            }
        }
    }

    pub fn exit(&mut self, buffers: &dyn BuffersContext, config: &Config) {
        self.exiting = true;
        let histories = self
            .storage
            .keys()
            .map(|kind| {
                (
                    kind.clone(),
                    config.buffer.mark_as_read.on_application_exit
                        || config
                            .buffer
                            .mark_as_read
                            .on_buffer_close
                            .mark_as_read(buffers.is_open_and_at_bottom(kind)),
                )
            })
            .collect();
        let save = self.input_storage.save(config);
        let worker = self.worker.clone();
        tokio::task::spawn(async move {
            let result = if let Some(save) = save {
                Some(save.await)
            } else {
                None
            };
            worker.send(worker::Command::Exit(histories, result));
        });
    }

    pub fn get_filters(&self) -> &[Filter] {
        &self.filters
    }

    pub fn get_filters_mut(&mut self) -> &mut Vec<Filter> {
        &mut self.filters
    }

    pub fn set_filters(
        &mut self,
        servers: &server::Map,
        clients: &client::Map,
        buffer_config: &config::Buffer,
    ) {
        for storage in self.storage.values_mut() {
            storage.read_cache.invalidate_processing();
        }
        let new_filters = Filter::list_from_servers(servers, clients);

        let mut servers: HashSet<Server> = HashSet::new();

        for new_filter in new_filters.iter() {
            if !self.filters.contains(new_filter) {
                servers.insert(new_filter.server());
            }
        }

        for filter in self.filters.iter() {
            if !new_filters.contains(filter) {
                servers.insert(filter.server());
            }
        }

        self.filters = new_filters;

        if !servers.is_empty() {
            self.reprocess_message_feed_history(clients, buffer_config);
        }

        for server in servers {
            self.reprocess_server_history(&server, clients, buffer_config);
        }
    }

    pub fn reload_configuration(
        &mut self,
        servers: &server::Map,
        clients: &client::Map,
        buffer_config: &config::Buffer,
    ) {
        self.filters = Filter::list_from_servers(servers, clients);
        let filters = FilterChain::borrow(&self.filters);
        let events: Vec<_> = self
            .storage
            .values_mut()
            .filter_map(|storage| {
                storage.read_cache.invalidate_processing();
                storage.process(filters, clients, buffer_config)
            })
            .collect();
        for event in &events {
            if let Event::Model(model::Message::Update(kind, _)) = event {
                self.refresh(kind);
            }
        }
        if !events.is_empty() {
            let _ = self.event_sender.send(events);
        }
    }

    pub fn get_reroute_rules(&self) -> &RerouteRules {
        &self.reroute_rules
    }

    pub fn get_reroute_rules_mut(&mut self) -> &mut RerouteRules {
        &mut self.reroute_rules
    }

    pub fn set_reroute_rules(
        &mut self,
        servers: &server::Map,
        clients: &client::Map,
    ) {
        self.reroute_rules = RerouteRules::from_server_map(servers, clients);
    }

    pub fn renormalize_server_history(
        &mut self,
        server: &Server,
        clients_context: &dyn ClientsContext,
    ) {
        let events = self
            .storage
            .values_mut()
            .filter_map(|kind_storage| {
                if kind_storage
                    .as_server()
                    .is_some_and(|kind_server| kind_server == server)
                    || kind_storage.is_message_feed()
                {
                    kind_storage.renormalize(clients_context)
                } else {
                    None
                }
            })
            .collect::<Vec<Event>>();

        if !events.is_empty() {
            let _ = self.event_sender.send(events);
        }
    }

    pub fn renormalize_message_feed_history(
        &mut self,
        clients_context: &dyn ClientsContext,
    ) {
        let events = self
            .storage
            .values_mut()
            .filter_map(|kind_storage| {
                if kind_storage.is_message_feed() {
                    kind_storage.renormalize(clients_context)
                } else {
                    None
                }
            })
            .collect::<Vec<Event>>();

        if !events.is_empty() {
            let _ = self.event_sender.send(events);
        }
    }

    pub fn reprocess_server_history(
        &mut self,
        server: &Server,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) {
        let filter_chain = FilterChain::borrow(&self.filters);

        let events = self
            .storage
            .values_mut()
            .filter_map(|kind_storage| {
                if kind_storage
                    .as_server()
                    .is_some_and(|kind_server| kind_server == server)
                    || kind_storage.is_message_feed()
                {
                    kind_storage.process(
                        filter_chain,
                        clients_context,
                        buffer_config,
                    )
                } else {
                    None
                }
            })
            .collect::<Vec<Event>>();

        for event in &events {
            if let Event::Model(model::Message::Update(kind, _)) = event {
                self.refresh(kind);
            }
        }
        if !events.is_empty() {
            let _ = self.event_sender.send(events);
        }
    }

    pub fn reprocess_message_feed_history(
        &mut self,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) {
        let filter_chain = FilterChain::borrow(&self.filters);

        let events = self
            .storage
            .values_mut()
            .filter_map(|kind_storage| {
                if kind_storage.is_message_feed() {
                    kind_storage.process(
                        filter_chain,
                        clients_context,
                        buffer_config,
                    )
                } else {
                    None
                }
            })
            .collect::<Vec<Event>>();

        for event in &events {
            if let Event::Model(model::Message::Update(kind, _)) = event {
                self.refresh(kind);
            }
        }
        if !events.is_empty() {
            let _ = self.event_sender.send(events);
        }
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

    pub fn reload_channel_monitor(
        &mut self,
        clients: &dyn ClientsContext,
        config: &config::ChannelMonitor,
    ) {
        self.get_or_load_mut(Kind::ChannelMonitor);
        self.monitored = self
            .storage
            .keys()
            .filter(|kind| match kind {
                Kind::Channel(server, channel) => config.is_channel_included(
                    server,
                    channel,
                    clients.get_server_casemapping_or_default(server),
                ),
                _ => false,
            })
            .cloned()
            .collect();
        if let Some(storage) = self.storage.get_mut(&Kind::ChannelMonitor) {
            storage.read_cache.invalidate();
        }
        self.refresh(&Kind::ChannelMonitor);
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
    read_cache: ReadCache,
    last_seen: HashMap<Nick, DateTime<Utc>>,
    auto_read: Option<ReadMarker>,
    importing: bool,
}

impl From<Kind> for Storage {
    fn from(kind: Kind) -> Self {
        Self {
            show_in_sidebar: !matches!(
                kind,
                Kind::Channel(..) | Kind::Query(..)
            ),
            kind,
            latest: None,
            latest_triggers_unread: None,
            latest_triggers_highlight: None,
            display_read_marker: None,
            read_marker: None,
            read_cache: ReadCache::default(),
            last_seen: HashMap::new(),
            auto_read: None,
            importing: false,
        }
    }
}

impl Storage {
    fn as_server(&self) -> Option<&Server> {
        self.kind.as_server()
    }
    fn is_message_feed(&self) -> bool {
        matches!(self.kind, Kind::Highlights | Kind::ChannelMonitor)
    }
    fn apply_metadata(
        &mut self,
        metadata: Metadata,
        display: Option<ReadMarker>,
    ) {
        self.latest = self.latest.max(metadata.latest);
        self.latest_triggers_unread = self
            .latest_triggers_unread
            .max(metadata.latest_triggers_unread);
        self.latest_triggers_highlight = self
            .latest_triggers_highlight
            .max(metadata.latest_triggers_highlight);
        self.read_marker = self.read_marker.max(metadata.read_marker);
        let marker = self
            .display_read_marker
            .max(metadata.read_marker)
            .max(display);
        if self.display_read_marker != marker
            && matches!(
                self.read_cache.requested,
                Request::Open {
                    limit: message::Limit::Backlog(_),
                    ..
                }
            )
        {
            self.read_cache.invalidate();
        }
        self.display_read_marker = marker;
    }
    /// Apply normalization to the `Storage`'s messages.
    fn renormalize(
        &mut self,
        clients_context: &dyn ClientsContext,
    ) -> Option<Event> {
        self.read_cache.invalidate_processing();
        if let MessageCache::Loaded { messages, .. } =
            &mut self.read_cache.message_cache
        {
            renormalize_messages(&self.kind, messages, clients_context);

            log::debug!("renormalized messages in {}", self.kind);

            Some(Event::Model(model::Message::Update(
                self.kind.clone(),
                self.model_update(),
            )))
        } else {
            None
        }
    }

    /// Block, condense, and populate reply-previews for the `Storage`'s messages.
    fn process(
        &mut self,
        filter_chain: FilterChain,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) -> Option<Event> {
        self.read_cache.config.clone_from(buffer_config);
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

            self.read_cache.processed = true;
            log::debug!("processed messages in {}", self.kind);

            Some(Event::Model(model::Message::Update(
                self.kind.clone(),
                self.model_update(),
            )))
        } else {
            None
        }
    }

    #[must_use]
    fn model_update(&self) -> model::Update {
        let pane_update =
            self.read_cache.model_update(&self.display_read_marker);
        let pane_update =
            if self.importing && matches!(pane_update, model::Pane::Loading) {
                model::Pane::Migrating
            } else {
                pane_update
            };

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
}

/// Normalize loaded messages using the server's current casemapping. Some
/// fields omit normalization when serialized; others may use an older mapping.
fn renormalize_messages(
    kind: &Kind,
    messages: &mut [message::MessageDisplay],
    clients_context: &dyn ClientsContext,
) {
    match kind {
        Kind::Highlights | Kind::ChannelMonitor => {
            for message in messages.iter_mut() {
                if let message::Target::Highlights { server, .. }
                | message::Target::ChannelMonitor { server, .. } =
                    &message.inner.target
                    && let Some(casemapping) =
                        clients_context.get_server_casemapping(server)
                {
                    database::normalize(
                        Arc::make_mut(&mut message.inner),
                        casemapping,
                    );
                }
            }
        }
        _ if let Some(server) = kind.as_server() => {
            if let Some(casemapping) =
                clients_context.get_server_casemapping(server)
            {
                for message in messages.iter_mut() {
                    database::normalize(
                        Arc::make_mut(&mut message.inner),
                        casemapping,
                    );
                }
            }
        }
        _ => (),
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
    for message in messages.iter_mut() {
        message.condensed = None;
        message.reply_preview = None;
    }
    block_messages(
        kind,
        messages,
        filter_chain,
        clients_context,
        buffer_config,
    );

    condense_messages(messages, buffer_config);

    populate_messages_reply_previews(messages, 0);
}

fn process_appended_messages(
    kind: &Kind,
    messages: &mut [message::MessageDisplay],
    appended_at: usize,
    filter_chain: FilterChain,
    clients_context: &dyn ClientsContext,
    buffer_config: &config::Buffer,
) {
    let contextual = messages.iter().enumerate().any(|(index, message)| {
        match &message.inner.source {
            Source::Server(source) => {
                index >= appended_at
                    && buffer_config
                        .server_messages
                        .smart(source.as_ref().map(|source| source.kind))
                        .is_some()
            }
            Source::Internal(source::Internal::Status(status)) => {
                buffer_config.internal_messages.smart(status).is_some()
            }
            _ => false,
        }
    });
    if contextual {
        process_messages(
            kind,
            messages,
            filter_chain,
            clients_context,
            buffer_config,
        );
        return;
    }

    block_messages(
        kind,
        &mut messages[appended_at..],
        filter_chain,
        clients_context,
        buffer_config,
    );
    // Only a new condensable row can extend the preceding group. Blocked
    // events do not split groups; condensation_range also checks local dates.
    let boundary = messages[appended_at..]
        .iter()
        .position(|message| !message.blocked)
        .and_then(|offset| {
            message::MessageDisplay::condensation_range(
                messages,
                appended_at + offset,
                &buffer_config.server_messages.condense,
            )
        })
        .map_or(appended_at, |range| range.start.min(appended_at));
    for message in &mut messages[boundary..] {
        message.condensed = None;
    }
    condense_messages(&mut messages[boundary..], buffer_config);

    // A parent arriving later can resolve an existing reply (including a
    // nested preview).
    let late_parent = {
        let appended_ids: HashSet<&message::Id> = messages[appended_at..]
            .iter()
            .filter_map(|message| message.inner.id.as_ref())
            .collect();
        !appended_ids.is_empty()
            && messages[..appended_at]
                .iter()
                .filter_map(|message| message.inner.reply_to.as_ref())
                .any(|id| appended_ids.contains(id))
    };
    let from = if late_parent { 0 } else { appended_at };
    for message in &mut messages[from..] {
        message.reply_preview = None;
    }
    populate_messages_reply_previews(messages, from);
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

                    let source_kind = source.as_ref().map(|source| source.kind);

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
                        && let Some(nick) = source
                            .as_ref()
                            .and_then(|source| source.nick.as_ref())
                    {
                        // Check if server message is smart filtered.
                        match source_kind {
                            Some(message::Kind::Away) => {
                                message.blocked = smart_filter_repeat(
                                    message.inner.as_ref(),
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
                                    message.inner.as_ref(),
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
                            message.inner.as_ref(),
                            &seconds,
                            &current_time,
                        );
                    }
                }
                _ => (),
            }
        }

        message.blocked = message.blocked
            || filter_chain
                .filter_message_of_kind(message.inner.as_ref(), kind);
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
            CondensationKey::Singular => {
                for message in chunk {
                    message.condensed = None;
                }
            }
        });
}

// TODO: Retrieve reply previews for messages outside read cache
/// Backfill previews for replies at or after `from` in a history batch
fn populate_messages_reply_previews(
    messages: &mut [message::MessageDisplay],
    from: usize,
) {
    let mut parents: HashMap<&message::Id, Vec<usize>> = HashMap::new();
    for message in &messages[from..] {
        if let Some(id) = &message.inner.reply_to {
            parents.entry(id).or_default();
        }
    }
    if parents.is_empty() {
        return;
    }
    for (position, message) in messages.iter().enumerate() {
        if let Some(id) = &message.inner.id
            && let Some(positions) = parents.get_mut(id)
        {
            positions.push(position);
        }
    }
    let position_pairs: Vec<(usize, usize)> = messages
        .iter()
        .enumerate()
        .skip(from)
        .filter_map(|(position, message)| {
            let positions = parents.get(message.inner.reply_to.as_ref()?)?;
            if positions.is_empty() {
                return None;
            }
            let start = message.time().utc + chrono::Duration::seconds(1);
            let split = messages
                .binary_search_by(|stored| stored.time().utc.cmp(&start))
                .unwrap_or_else(|position| position);
            let before =
                positions.partition_point(|position| *position < split);
            let parent = if before > 0 {
                positions[before - 1]
            } else {
                *positions.last()?
            };
            Some((position, parent))
        })
        .collect();

    drop(parents);

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
/// historical messages without a message ID or unlabled echoes) the messages
/// must have an exact match + target & / content.
///
/// The return values are the history ID of the message, the time of the
/// message, and whether a message sent from this client was was replaced by an
/// echo by the insert.
pub(super) fn reconcile_message(
    messages: &mut Vec<message::Message>,
    mut message: message::MessageWithContext,
) -> (Id, message::Time, bool) {
    let mut history_id = message.inner.history_id;
    let time = message.inner.time;

    if messages.is_empty() {
        messages.push(message.into());

        return (history_id, time, false);
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

        let start_index =
            messages.partition_point(|stored| stored.time.utc < start);
        let end_index =
            messages.partition_point(|stored| stored.time.utc < end);

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
            history_id = messages.remove(index).history_id;
            message.inner.history_id = history_id;
            replaced_sent = true;
        }
    }

    let start = message.time().utc - fuzz_seconds;
    let end = message.time().utc + fuzz_seconds;

    let start_index =
        messages.partition_point(|stored| stored.time.utc < start);
    let end_index = messages.partition_point(|stored| stored.time.utc < end);

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
        history_id = messages[index].history_id;
        message.inner.history_id = history_id;
        if message_is_unlabeled_echo && messages[index].is_sent() {
            replaced_sent = true;
        }

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
                    history_id: messages[index].history_id,
                    id: message.id().clone().or(messages[index].id.clone()),
                    ..message.into()
                };
            }
        } else {
            messages.remove(index);
            let insert_at =
                messages.partition_point(|stored| stored.time <= time);
            messages.insert(insert_at, message.into());
        }
    } else {
        messages.insert(insert_at, message.into());
    }

    (history_id, time, replaced_sent)
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
            match source.kind {
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

pub(super) fn get_before_and_after_count_from_position(
    count: usize,
    position: usize,
    len: usize,
) -> (usize, usize) {
    if position < count / 2 {
        // Since there aren't enough messages before position to account for
        // ~half of count, shift that count towards messages after position.
        let before_count = position;

        let after_count = count - position;

        (before_count, after_count)
    } else if position > len.saturating_sub(count / 2) {
        // Since there aren't enough messages after position to account for
        // ~half of count, shift that count towards messages before position.
        let after_count = len.saturating_sub(position);

        let before_count = count - after_count;

        (before_count, after_count)
    } else {
        (count / 2, count - count / 2 - 1)
    }
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

fn dir_path() -> Result<PathBuf, Error> {
    let dir = environment::data_dir().join("msdb");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] database::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}
