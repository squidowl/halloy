use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Local, NaiveDate, Utc};
use iced::Task;
use itertools::Itertools;
use tokio::fs;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};

use super::filter::{Filter, FilterChain};
use super::reroute::RerouteRules;
use super::{
    Kind, Metadata, ReadMarker, metadata, model, smart_filter_internal_message,
    smart_filter_message, smart_filter_repeat,
};
use crate::message::{MessageReferences, Source, source};
use crate::user::Nick;
use crate::{
    Config, Notification, Server, compression, config, environment, isupport,
    message, reaction, redaction,
};

/// Duration to wait after last received update before flushing
const FLUSH_AFTER_DURATION_SINCE_LAST_UPDATED: Duration =
    Duration::from_secs(5);
/// # of pending updates to trigger flush even if FLUSH_AFTER_DURATION_SINCE_LAST_UPDATED has not passed
const FLUSH_AFTER_UPDATE_COUNT: usize = 1000;

pub enum Message {
    Loaded(Kind, Result<(Metadata, Vec<message::Message>), Error>),
}

pub enum Event {
    Model(model::Message),
    Notification(Notification),
}

pub enum Update {
    Message(message::MessageWithContext),
    Reaction(reaction::ReactionWithContext),
    Redaction(redaction::RedactionWithContext),
}

#[derive(Debug)]
pub struct Manager {
    storage: HashMap<Kind, Storage>,
    config: Arc<Config>,
    filters: Vec<Filter>,
    reroute_rules: RerouteRules,
    message_sender: mpsc::UnboundedSender<Message>,
    event_sender: mpsc::Sender<Event>,
}

impl Manager {
    pub fn new(config: Arc<Config>) -> (Self, Task<Message>, Task<Event>) {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        let (event_sender, event_receiver) = mpsc::channel(300);

        (
            Self {
                storage: HashMap::new(),
                config: config.clone(),
                filters: Vec::new(),
                reroute_rules: RerouteRules::default(),
                message_sender,
                event_sender,
            },
            Task::stream(UnboundedReceiverStream::new(message_receiver)),
            Task::stream(ReceiverStream::new(event_receiver)),
        )
    }

    pub fn write(&mut self, server: Option<&Server>, updates: Vec<Update>) {
        let mut updates_by_kind: HashMap<Kind, Vec<Update>> = HashMap::new();

        for update in updates.into_iter() {
            if let Some(kind) = match &update {
                Update::Message(message) => {
                    Kind::from_message(&message.inner, server)
                }
                Update::Reaction(reaction) => server.map(|server| {
                    Kind::from_target(server.clone(), reaction.target.clone())
                }),
                Update::Redaction(redaction) => server.map(|server| {
                    Kind::from_target(server.clone(), redaction.target.clone())
                }),
            } {
                updates_by_kind
                    .entry(kind)
                    .and_modify(|updates| updates.push(update))
                    .or_insert(vec![update]);
            } else {
                log::error!(
                    "unable to determine history kind for storage update {update:?}"
                );
            }
        }

        for (kind, updates) in updates_by_kind.into_iter() {
            let kind_storage = self.get_mut(&kind);

            kind_storage.write(updates);
        }
    }

    pub fn tick(&mut self, now: Instant) -> Vec<Event> {
        self.storage
            .values()
            .flat_map(|kind_storage| kind_storage.tick(now))
            .collect()
    }

    pub async fn update(
        &mut self,
        message: Message,
        config: &Config,
    ) -> Option<Event> {
        match message {
            Message::Loaded(kind, Ok((metadata, messages))) => {
                let kind_storage = self.get_mut(&kind);

                kind_storage.loaded(metadata, messages);

                Some(Event::Model(model::Message::Update(
                    kind,
                    kind_storage.model_update(),
                )))
            }
            Message::Loaded(kind, Err(error)) => {
                log::error!("failed to load history {kind:?}");

                None
            }
        }
    }

    fn get_mut(&mut self, kind: &Kind) -> &mut Storage {
        self.storage.get_mut(kind).unwrap_or({
            let storage = Storage::from(kind.clone());

            self.load(kind.clone());

            self.storage.insert(kind.clone(), storage);

            self.storage.get_mut(kind).unwrap_or_else(|| panic!("expected to get_mut {kind} storage after insert into storage::Manager.storage"))
        })
    }

    pub fn load(&self, kind: Kind) {
        let message_sender = self.message_sender.clone();

        tokio::task::spawn(async move {
            let loaded = Storage::load(&kind).await;

            message_sender.send(Message::Loaded(kind, loaded));
        });
    }

    // Block, condense, and populate reply-previews for history's messages
    pub fn process_history(
        &mut self,
        kind: &Kind,
        clients_context: &dyn ClientsContext,
        buffer_config: &config::Buffer,
    ) -> Option<Vec<Event>> {
        self.storage.get_mut(&kind).and_then(|kind_storage| {
            process_messages(
                &kind,
                self.read_cache.messages,
                FilterChain::borrow(&self.filters),
                clients_context,
                buffer_config,
            );

            log::debug!("processed messages in {kind}");

            kind_storage.model_full_update()
        })
    }

    pub fn get_reroute_rules(&self) -> &RerouteRules {
        &self.reroute_rules
    }
}

#[derive(Debug)]
pub struct Storage {
    kind: Kind,
    show_in_sidebar: bool,
    latest_triggers_unread: Option<DateTime<Utc>>,
    latest_triggers_highlight: Option<DateTime<Utc>>,
    display_read_marker: Option<ReadMarker>,
    read_marker: Option<ReadMarker>,
    chathistory_references: Option<MessageReferences>,
    messages: Option<Vec<message::Message>>,
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

        Self {
            kind,
            show_in_sidebar,
            latest_triggers_unread: None,
            latest_triggers_highlight: None,
            display_read_marker: None,
            read_marker: None,
            chathistory_references: None,
            messages: None,
            read_cache: ReadCache::default(),
            write_buffer: WriteBuffer::default(),
            last_seen: HashMap::new(),
        }
    }
}

impl Storage {
    async fn load(kind: &Kind) -> Result<(Metadata, Vec<message::Message>)> {
        let path = path(&kind).await?;

        let metadata = metadata::load(&kind).await.unwrap_or_default();

        let mut messages = read_all(&path).await.unwrap_or_default();

        Ok((metadata, messages))
    }

    fn loaded(&mut self, metadata: Metadata, messages: Vec<message::Message>) {
        // If not already loaded, should always be the case
        if self.messages.is_none() {
            self.latest_triggers_unread = metadata.last_triggers_unread;
            self.latest_triggers_highlight = metadata.last_triggers_highlight;
            self.display_read_marker = metadata.read_marker;
            self.read_marker = metadata.read_marker;
            self.chathistory_references = metadata.chathistory_references;

            self.messages = Some(messages);
        } else {
            log::debug!("unexpected repeat loading of {:?}", self.kind);
        }
    }

    pub fn write(&mut self, updates: Vec<Update>) {
        self.write_buffer.updates.extend(updates);

        if self.read_cache.limit.is_some()
            || self.write_buffer.updates.len() > FLUSH_AFTER_UPDATE_COUNT
        {
            self.flush();
        } else {
            self.write_buffer.updates.last_updated_at = Some(Instant::now());
        }
    }

    pub fn update_read_marker(&mut self, read_marker: ReadMarker) -> Event {
        self.display_read_marker =
            (self.display_read_marker).max(Some(read_marker));
        self.read_marker = Some(read_marker);

        Event::Model(model::Message::Update(
            self.kind.clone(),
            self.model_update(),
        ))
    }

    pub fn tick(&mut self, now: Instant) -> Vec<Event> {
        if let Some(last_updated_at) = self.write_buffer.last_updated_at
            && (now.duration_since(last_updated_at)
                >= FLUSH_AFTER_DURATION_SINCE_LAST_UPDATED
                || self.write_buffer.updates.len() > FLUSH_AFTER_UPDATE_COUNT)
        {
            self.flush()
        } else {
            vec![]
        }
    }

    pub fn model_update(&self) -> model::Update {
        let pane_update = self.read_cache.model_update();

        model::Update {
            show_in_sidebar: self.show_in_sidebar,
            read_marker: self.read_marker,
            display_read_marker: self.display_read_marker,
            latest_triggers_unread: self.latest_triggers_unread,
            latest_triggers_highlight: self.latest_triggers_highlight,
            pane: pane_update,
        }
    }

    pub fn update_last_seen(&mut self, message: &message::Message) {
        if let Source::User(user) | Source::Action(Some(user)) = &message.source
        {
            let nickname = user.nickname().to_owned();

            if let Some(date_time) = self.last_seen.get_mut(&nickname) {
                *date_time = (*date_time).max(message.time.utc);
            } else {
                self.last_seen.insert(nickname, message.time.utc);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct ReadCache {
    has_more_older_messages: bool,
    has_more_newer_messages: bool,
    messages: Vec<message::MessageDisplay>,
    limit: Option<message::Limit>,
}

impl ReadCache {
    pub fn model_update(&self) -> model::Pane {
        if let Some(limit) = self.limit {
            model::Pane::Open {
                has_more_older_messages: self.has_more_older_messages,
                has_more_newer_messages: self.has_more_newer_messages,
                messages: self.messages,
                limit: limit,
            }
        } else {
            model::Pane::Closed
        }
    }
}

#[derive(Debug, Default)]
pub struct WriteBuffer {
    last_updated_at: Option<Instant>,
    updates: Vec<Update>,
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

        if message.has_redaction()
            && !buffer_config.redaction.display.is_visible()
        {
            message.blocked = true;
        } else {
            match message.source() {
                Source::Server(source) => {
                    let server = if let Some(server) = kind.server() {
                        Some(server)
                    } else if let message::Target::Highlights {
                        server, ..
                    }
                    | message::Target::ChannelMonitor {
                        server,
                        ..
                    } = message.target()
                    {
                        Some(server)
                    } else {
                        None
                    };

                    let casemapping = clients_context
                        .get_maybe_server_casemapping_or_default(server);

                    // Check if target is included/excluded.
                    let target_ref = match message.target() {
                        message::Target::Channel { channel }
                        | message::Target::ChannelMonitor { channel, .. }
                        | message::Target::Highlights { channel, .. } => {
                            Some(channel.as_target_ref())
                        }

                        message::Target::Query { query } => {
                            Some(query.as_target_ref())
                        }
                        message::Target::Server | message::Target::Logs => None,
                    };

                    let source_kind =
                        source.as_ref().map(source::server::Server::kind);

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
                                        &message.inner,
                                        &seconds,
                                        last_away.get(&nick),
                                    );

                                    if !message.blocked {
                                        last_away.insert(
                                            nick.clone(),
                                            *message.server_time(),
                                        );
                                    }
                                }
                                _ => {
                                    message.blocked = smart_filter_message(
                                        &message.inner,
                                        &seconds,
                                        last_seen.get(&nick),
                                    );
                                }
                            }
                        }
                    }
                }
                Source::User(message_user) => {
                    last_seen.insert(
                        message_user.nickname().to_owned(),
                        *message.server_time(),
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

        if !message.blocked {
            message.blocked =
                filter_chain.filter_message_of_kind(&message.inner, kind);
        }
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
            if message.can_condense(&buffer_config.server_messages.condense) {
                CondensationKey::Condensable(
                    message.server_time().with_timezone(&Local).date_naive(),
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

/// Backfill previews for replies for messages in a history batch
fn populate_messages_reply_previews(messages: &mut [message::MessageDisplay]) {
    let position_pairs: Vec<(usize, usize)> = messages
        .iter()
        .enumerate()
        .filter_map(|(message_position, message)| {
            message.reply_to.as_ref().and_then(|reply_id| {
                position_reply_target(messages, reply_id, &message.server_time)
                    .map(|reply_target_position| {
                        (message_position, reply_target_position)
                    })
            })
        })
        .collect();

    for (message_position, reply_target_position) in position_pairs {
        if let Some(reply_preview) = messages
            .get(reply_target_position)
            .map(message::MessageDisplay::as_reply_preview)
            && let Some(message) = messages.get_mut(message_position)
        {
            message.reply_preview = Some(reply_preview);
        }
    }
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

async fn read_all(path: &PathBuf) -> Result<Vec<message::Message>, Error> {
    let bytes = fs::read(path).await?;
    Ok(compression::decompress(&bytes)?)
}

async fn write_messages<'a>(
    kind: &Kind,
    messages: &'a [message::Message],
) -> Result<&'a [message::Message], Error> {
    let latest_messages =
        &messages[messages.len().saturating_sub(MAX_MESSAGES)..];

    let path = path(kind).await?;
    let compressed = compression::compress(&latest_messages)?;

    fs::write(path, &compressed).await?;

    Ok(latest_messages)
}

pub async fn delete(kind: &Kind) -> Result<(), Error> {
    let path = path(kind).await?;

    fs::remove_file(path).await?;

    Ok(())
}

pub async fn dir_path() -> Result<PathBuf, Error> {
    let data_dir = environment::data_dir();

    let history_dir = data_dir.join("msdb");

    if !history_dir.exists() {
        fs::create_dir_all(&history_dir).await?;
    }

    Ok(history_dir)
}

async fn path(kind: &Kind) -> Result<PathBuf, Error> {
    let dir = dir_path().await?;

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
