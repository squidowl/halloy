use std::cmp::Ordering;
use std::collections::HashMap;
use std::io;
use std::time::Duration;

use chrono::{DateTime, Utc};
use futures::FutureExt;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

pub use self::kind::Kind;
pub use self::manager::{
    EchoEvent, Manager, ReactionToEcho, ReplyToEcho, Resource,
};
pub use self::metadata::{Metadata, ReadMarker};
pub use self::model::Model;
pub use self::storage::Storage;
use crate::capabilities::LabeledResponseContext;
use crate::message::{self, Direction, MessageReferences};
use crate::reaction::Reaction;
use crate::redaction::Redaction;
use crate::target::Target;
use crate::user::Nick;
use crate::{
    Message, Server, compression, config, isupport, reaction, redaction,
};

pub mod filter;
mod kind;
pub mod manager;
pub mod metadata;
pub mod model;
pub mod reroute;
pub mod storage;

// TODO: Make this configurable?
/// Max # messages to persist
pub(crate) const MAX_MESSAGES: usize = 10_000;
/// # messages to truncate after hitting [`MAX_MESSAGES`]
const TRUNC_COUNT: usize = 500;
/// Duration to wait after receiving last message before flushing
const FLUSH_AFTER_LAST_RECEIVED: Duration = Duration::from_secs(5);
/// # new messages to trigger flush even if FLUSH_AFTER_LAST_RECEIVED has not passed
const FLUSH_COUNT: usize = 1000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Id {
    #[default]
    Undetermined,
    Determined(u64),
}

pub(crate) fn truncate_messages(messages: &mut Vec<Message>) {
    if messages.len() > MAX_MESSAGES {
        messages.drain(0..messages.len() - (MAX_MESSAGES - TRUNC_COUNT));
    }
}

#[derive(Debug)]
pub struct Loaded {
    pub messages: Vec<Message>,
    pub metadata: Metadata,
}

pub enum Seed {
    Single(isupport::CaseMap),
    Multiple(HashMap<Server, isupport::CaseMap>),
}

fn renormalize_messages<'a>(
    messages: impl Iterator<Item = &'a mut Message>,
    seed: Seed,
) {
    match seed {
        Seed::Multiple(casemappings) => {
            messages.for_each(|message| {
                if let message::Target::Highlights { server, .. }
                | message::Target::ChannelMonitor { server, .. } =
                    &message.target
                    && let Some(casemapping) = casemappings.get(server)
                {
                    message.renormalize(*casemapping);
                }
            });
        }
        Seed::Single(casemapping) => {
            messages.for_each(|message| message.renormalize(casemapping));
        }
    }
}

pub async fn overwrite(
    kind: &Kind,
    messages: &[Message],
    read_marker: Option<ReadMarker>,
    chathistory_references: Option<MessageReferences>,
) -> Result<(), Error> {
    if messages.is_empty() {
        return metadata::save(
            kind,
            messages,
            read_marker,
            chathistory_references,
        )
        .await;
    }

    let messages = write_messages(kind, messages).await?;

    metadata::save(kind, messages, read_marker, chathistory_references).await?;

    Ok(())
}

pub async fn append(
    kind: &Kind,
    seed: Option<Seed>,
    pending_messages: Vec<(Message, Option<LabeledResponseContext>)>,
    read_marker: Option<ReadMarker>,
    max_triggers_unread: Option<DateTime<Utc>>,
    max_triggers_highlight: Option<DateTime<Utc>>,
    chathistory_references: Option<MessageReferences>,
    pending_reactions: HashMap<message::Id, reaction::Pending>,
    pending_redactions: HashMap<message::Id, redaction::Pending>,
) -> Result<Vec<EchoEvent>, Error> {
    let loaded = load(kind.clone(), seed).await?;

    let mut echo_events: Vec<EchoEvent> = vec![];

    let mut all_messages = loaded.messages;

    // pending reactions should only exist for unloaded history entries
    for (id, pending) in pending_reactions.into_iter() {
        if let Some(server_time) = pending.server_time()
            && let Some(message) =
                find_message_mut_by_id(&mut all_messages, &id, &server_time)
        {
            if message.is_echo
                && message.direction == Direction::Received
                && let Ok(target) = Target::try_from(message.target.clone())
            {
                let message_text = message.text();
                for pending_reaction in pending.reactions.iter() {
                    if pending_reaction.notification_enabled {
                        let reaction_to_echo = ReactionToEcho {
                            reaction: reaction::Context {
                                inner: pending_reaction.reaction.clone(),
                                target: target.clone(),
                                in_reply_to: id.clone(),
                                is_echo: pending_reaction.is_echo,
                                deduplicate: pending_reaction.deduplicate,
                            },
                            message_text: message_text.to_string(),
                        };

                        echo_events.push(EchoEvent::Reaction(reaction_to_echo));
                    }
                }
            }

            for pending_reaction in pending.reactions.into_iter() {
                insert_reaction(
                    &mut message.reactions,
                    pending_reaction.reaction,
                    pending_reaction.is_echo,
                    pending_reaction.deduplicate,
                    pending_reaction.labeled_response_context,
                );
            }
        }
    }

    for (id, pending) in pending_redactions.into_iter() {
        if let Some(message) =
            find_message_mut_by_id(&mut all_messages, &id, &pending.server_time)
        {
            message.redaction = Some(pending.redaction);
        }
    }

    for (message, _) in &pending_messages {
        if !message.is_echo
            && !message.deduplicate
            && let Some(reply_id) = &message.reply_to
            && let Some(original) = find_message_by_id(
                &all_messages,
                reply_id,
                &message.server_time,
            )
            && original.is_echo
            && original.direction == Direction::Received
        {
            echo_events.push(EchoEvent::Reply(ReplyToEcho {
                message: message.clone(),
            }));
        }
    }

    pending_messages.into_iter().for_each(
        |(message, labeled_response_context)| {
            insert_message(
                &mut all_messages,
                message,
                labeled_response_context,
            );
        },
    );

    let _ = write_messages(kind, &all_messages).await?;

    // Update metadata directly, without referencing all messages, since all
    // messages have not been processed (and so do not have their blocked state
    // set → blocked messages may incorrectly update metadata).

    metadata::update(
        kind,
        read_marker,
        max_triggers_unread,
        max_triggers_highlight,
        chathistory_references,
    )
    .await?;

    Ok(echo_events)
}

#[derive(Debug)]
pub enum History {
    Partial {
        kind: Kind,
        pending_messages: Vec<(Message, Option<LabeledResponseContext>)>, // Unordered
        last_updated_at: Option<Instant>,
        max_triggers_unread: Option<DateTime<Utc>>,
        max_triggers_highlight: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
        chathistory_references: Option<MessageReferences>,
        last_seen: HashMap<Nick, DateTime<Utc>>,
        pending_reactions: HashMap<message::Id, reaction::Pending>,
        pending_redactions: HashMap<message::Id, redaction::Pending>,
        show_in_sidebar: bool,
        flushing_messages: Vec<(Message, Option<LabeledResponseContext>)>,
        flushing_reactions: HashMap<message::Id, reaction::Pending>,
        flushing_redactions: HashMap<message::Id, redaction::Pending>,
    },
    Full {
        kind: Kind,
        messages: Vec<Message>, // Sorted by Message.server_time
        last_updated_at: Option<Instant>,
        read_marker: Option<ReadMarker>,
        display_read_marker: Option<ReadMarker>,
        chathistory_references: Option<MessageReferences>,
        last_seen: HashMap<Nick, DateTime<Utc>>,
        cleared: bool,
        last_flushed_at: usize,
    },
}

impl History {
    fn partial(kind: Kind) -> Self {
        Self::Partial {
            kind,
            pending_messages: vec![],
            last_updated_at: None,
            max_triggers_unread: None,
            max_triggers_highlight: None,
            read_marker: None,
            chathistory_references: None,
            last_seen: HashMap::new(),
            pending_reactions: HashMap::new(),
            pending_redactions: HashMap::new(),
            show_in_sidebar: false,
            flushing_messages: vec![],
            flushing_reactions: HashMap::new(),
            flushing_redactions: HashMap::new(),
        }
    }

    pub fn update_partial(&mut self, metadata: Metadata) {
        if let Self::Partial {
            max_triggers_unread,
            max_triggers_highlight,
            read_marker,
            chathistory_references,
            ..
        } = self
        {
            *read_marker = (*read_marker).max(metadata.read_marker);
            *max_triggers_unread =
                (*max_triggers_unread).max(metadata.last_triggers_unread);
            *max_triggers_highlight =
                (*max_triggers_highlight).max(metadata.last_triggers_highlight);
            *chathistory_references = chathistory_references
                .clone()
                .max(metadata.chathistory_references);
        }
    }

    fn has_unread(&self) -> bool {
        match self {
            History::Partial {
                max_triggers_unread,
                read_marker,
                ..
            } => {
                // Read marker is prior to last known message which triggers unread
                if let Some(read_marker) = read_marker {
                    max_triggers_unread
                        .is_some_and(|max| read_marker.date_time() < max)
                }
                // Default state == unread if theres messages that trigger indicator
                else {
                    max_triggers_unread.is_some()
                }
            }
            History::Full {
                messages,
                display_read_marker,
                ..
            } => {
                let latest = metadata::latest_triggers_unread(messages);

                if let Some(display_read_marker) = display_read_marker {
                    latest.is_some_and(|latest| {
                        display_read_marker.date_time() < latest
                    })
                } else {
                    latest.is_some()
                }
            }
        }
    }

    fn has_highlight(&self) -> bool {
        match self {
            History::Partial {
                max_triggers_highlight,
                read_marker,
                ..
            } => {
                // Read marker is prior to last known message which triggers highlight
                if let Some(read_marker) = read_marker {
                    max_triggers_highlight
                        .is_some_and(|max| read_marker.date_time() < max)
                }
                // Default state == highlight if theres messages that trigger indicator
                else {
                    max_triggers_highlight.is_some()
                }
            }
            History::Full {
                messages,
                read_marker,
                ..
            } => {
                let latest = metadata::latest_triggers_highlight(messages);

                if let Some(read_marker) = read_marker {
                    latest
                        .is_some_and(|latest| read_marker.date_time() < latest)
                } else {
                    latest.is_some()
                }
            }
        }
    }

    fn add_message(
        &mut self,
        message: Message,
        labeled_response_context: Option<LabeledResponseContext>,
    ) -> Option<ReadMarker> {
        if let History::Partial {
            show_in_sidebar,
            max_triggers_unread,
            ..
        } = self
            && (matches!(message.direction, message::Direction::Sent)
                || ((message.triggers_unread()
                    || (message.is_echo && !message.deduplicate))
                    && Some(message.server_time) > *max_triggers_unread))
        {
            *show_in_sidebar = true;
        }

        if message.triggers_unread()
            && let History::Partial {
                max_triggers_unread,
                ..
            } = self
        {
            *max_triggers_unread =
                (*max_triggers_unread).max(Some(message.server_time));
        }

        if message.triggers_highlight()
            && !message.blocked
            && let History::Partial {
                max_triggers_highlight,
                ..
            } = self
        {
            *max_triggers_highlight =
                (*max_triggers_highlight).max(Some(message.server_time));
        }

        match self {
            History::Partial {
                last_updated_at, ..
            }
            | History::Full {
                last_updated_at, ..
            } => {
                *last_updated_at = Some(Instant::now());
            }
        }

        if matches!(
            self,
            History::Partial {
                kind: Kind::ChannelMonitor,
                ..
            }
        ) {
            return None;
        }

        match self {
            History::Partial { last_seen, .. }
            | History::Full { last_seen, .. } => {
                update_last_seen(last_seen, &message);
            }
        }

        match self {
            History::Partial {
                pending_messages, ..
            } => {
                pending_messages.push((message, labeled_response_context));

                None
            }
            History::Full { messages, .. } => {
                insert_message(messages, message, labeled_response_context)
            }
        }
    }

    pub fn find_message_by_hash(
        &self,
        hash: message::Hash,
        server_time: &DateTime<Utc>,
    ) -> Option<&Message> {
        match self {
            History::Partial {
                pending_messages, ..
            } => pending_messages
                .iter()
                .find(|(m, _)| m.hash == hash)
                .map(|(m, _)| m),
            History::Full { messages, .. } => {
                find_message_by_hash(messages, hash, server_time)
            }
        }
    }

    pub fn find_message_by_id(
        &self,
        id: &message::Id,
        server_time: &DateTime<Utc>,
    ) -> Option<&Message> {
        match self {
            History::Partial {
                pending_messages, ..
            } => pending_messages
                .iter()
                .find(|(m, _)| m.id.as_deref() == Some(id))
                .map(|(m, _)| m),
            History::Full { messages, .. } => {
                find_message_by_id(messages, id, server_time)
            }
        }
    }

    pub fn find_message_mut_by_id(
        &mut self,
        id: &message::Id,
        server_time: &DateTime<Utc>,
    ) -> Option<&mut Message> {
        match self {
            History::Partial {
                pending_messages, ..
            } => pending_messages
                .iter_mut()
                .find(|(m, _)| m.id.as_deref() == Some(id))
                .map(|(m, _)| m),
            History::Full { messages, .. } => {
                find_message_mut_by_id(messages, id, server_time)
            }
        }
    }

    pub(crate) fn is_our_message(
        &self,
        id: &message::Id,
        server_time: &DateTime<Utc>,
    ) -> bool {
        self.find_message_by_id(id, server_time)
            .is_some_and(|msg| msg.direction == Direction::Sent || msg.is_echo)
    }

    fn remove_message(
        &mut self,
        server_time: DateTime<Utc>,
        history_id: Id,
    ) -> Option<Message> {
        match self {
            History::Partial {
                pending_messages, ..
            } => pending_messages
                .iter()
                .position(|(message, _)| message.history_id == history_id)
                .map(|index| {
                    let (message, _) = pending_messages.remove(index);
                    message
                }),
            History::Full { messages, .. } => {
                if messages.is_empty() {
                    return None;
                }

                let fuzz_seconds = chrono::Duration::seconds(1);

                let start = server_time - fuzz_seconds;
                let end = server_time + fuzz_seconds;

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

                messages[start_index..end_index]
                    .iter()
                    .position(|message| message.history_id == history_id)
                    .map(|slice_index| {
                        messages.remove(start_index + slice_index)
                    })
            }
        }
    }

    // Find the first message in the condensation, then return all messages in
    // the condensation
    fn get_expansion_messages(
        &mut self,
        server_time: DateTime<Utc>,
        history_id: Id,
        config: &config::buffer::Condensation,
    ) -> Vec<&mut Message> {
        match self {
            History::Partial { .. } => (),
            History::Full { messages, .. } => {
                if messages.is_empty() {
                    return vec![];
                }

                let fuzz_seconds = chrono::Duration::seconds(1);

                let start = server_time - fuzz_seconds;
                let end = server_time + fuzz_seconds;

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

                if let Some(index) = messages[start_index..end_index]
                    .iter()
                    .enumerate()
                    .find_map(|(slice_index, message)| {
                        (message.history_id == history_id)
                            .then_some(start_index + slice_index)
                    })
                {
                    if messages[index].redaction.is_some() {
                        return vec![&mut messages[index]];
                    } else if let Some(first_index) = messages[..=index]
                        .iter()
                        .rev()
                        .position(|message| message.condensed.is_some())
                        .map(|position| index - position)
                    {
                        return messages[first_index..]
                            .iter_mut()
                            .filter(|message| !message.blocked)
                            .scan(true, |is_first_message, message| {
                                if *is_first_message {
                                    *is_first_message = false;
                                    Some(message)
                                } else {
                                    (message.can_condense(config)
                                        && message.condensed.is_none())
                                    .then_some(message)
                                }
                            })
                            .collect();
                    }
                }
            }
        }

        vec![]
    }

    // If now is None then history will be flushed regardless of time
    // since last received
    fn flush(
        &mut self,
        now: Option<Instant>,
        seed: Option<Seed>,
    ) -> Option<BoxFuture<'static, Result<Vec<EchoEvent>, Error>>> {
        match self {
            History::Partial {
                kind,
                pending_messages,
                last_updated_at,
                read_marker,
                max_triggers_unread,
                max_triggers_highlight,
                chathistory_references,
                pending_reactions,
                pending_redactions,
                flushing_messages,
                flushing_reactions,
                flushing_redactions,
                ..
            } => {
                if let Some(last_received) = *last_updated_at
                    && (now.is_none_or(|now| {
                        now.duration_since(last_received)
                            >= FLUSH_AFTER_LAST_RECEIVED
                    }) || pending_messages.len() > FLUSH_COUNT)
                    && flushing_messages.is_empty()
                    && flushing_reactions.is_empty()
                    && flushing_redactions.is_empty()
                {
                    let kind = kind.clone();
                    let read_marker = *read_marker;
                    let max_triggers_unread = *max_triggers_unread;
                    let max_triggers_highlight = *max_triggers_highlight;

                    *last_updated_at = None;

                    if matches!(kind, Kind::ChannelMonitor) {
                        return Some(
                            async move {
                                metadata::update(
                                    &kind,
                                    read_marker,
                                    max_triggers_unread,
                                    max_triggers_highlight,
                                    None,
                                )
                                .await
                                .map(|()| Vec::<EchoEvent>::new())
                            }
                            .boxed(),
                        );
                    }

                    let pending_messages = std::mem::take(pending_messages);
                    *flushing_messages = pending_messages.clone();
                    let chathistory_references = chathistory_references.clone();
                    let pending_reactions = std::mem::take(pending_reactions);
                    *flushing_reactions = pending_reactions.clone();
                    let pending_redactions = std::mem::take(pending_redactions);
                    *flushing_redactions = pending_redactions.clone();

                    return Some(
                        async move {
                            append(
                                &kind,
                                seed,
                                pending_messages,
                                read_marker,
                                max_triggers_unread,
                                max_triggers_highlight,
                                chathistory_references,
                                pending_reactions,
                                pending_redactions,
                            )
                            .await
                        }
                        .boxed(),
                    );
                }

                None
            }
            History::Full {
                kind,
                messages,
                last_updated_at,
                read_marker,
                chathistory_references,
                last_flushed_at,
                ..
            } => {
                if let Some(last_received) = *last_updated_at
                    && (now.is_none_or(|now| {
                        now.duration_since(last_received)
                            >= FLUSH_AFTER_LAST_RECEIVED
                    }) || messages.len().saturating_sub(*last_flushed_at)
                        > FLUSH_COUNT)
                    && !messages.is_empty()
                {
                    let kind = kind.clone();
                    let read_marker = *read_marker;

                    *last_updated_at = None;

                    if matches!(kind, Kind::ChannelMonitor) {
                        let max_triggers_unread =
                            metadata::latest_triggers_unread(messages);
                        let max_triggers_highlight =
                            metadata::latest_triggers_highlight(messages);

                        return Some(
                            async move {
                                metadata::update(
                                    &kind,
                                    read_marker,
                                    max_triggers_unread,
                                    max_triggers_highlight,
                                    None,
                                )
                                .await
                                .map(|()| Vec::<EchoEvent>::new())
                            }
                            .boxed(),
                        );
                    }

                    let chathistory_references = chathistory_references.clone();

                    truncate_messages(messages);

                    let messages = messages.clone();

                    return Some(
                        async move {
                            overwrite(
                                &kind,
                                &messages,
                                read_marker,
                                chathistory_references,
                            )
                            .await
                            .map(|()| vec![])
                        }
                        .boxed(),
                    );
                }

                None
            }
        }
    }

    fn make_partial(
        &mut self,
    ) -> Option<BoxFuture<'static, Result<(), Error>>> {
        match self {
            History::Partial { .. } => None,
            History::Full {
                kind,
                messages,
                read_marker,
                chathistory_references,
                last_seen,
                ..
            } => {
                let kind = kind.clone();
                let read_marker = *read_marker;
                let max_triggers_unread =
                    metadata::latest_triggers_unread(messages);
                let max_triggers_highlight =
                    metadata::latest_triggers_highlight(messages);

                if matches!(kind, Kind::ChannelMonitor) {
                    *self = Self::Partial {
                        kind: kind.clone(),
                        pending_messages: vec![],
                        last_updated_at: None,
                        read_marker,
                        max_triggers_unread,
                        max_triggers_highlight,
                        chathistory_references: None,
                        last_seen: HashMap::new(),
                        pending_reactions: HashMap::new(),
                        pending_redactions: HashMap::new(),
                        show_in_sidebar: true,
                        flushing_messages: vec![],
                        flushing_reactions: HashMap::new(),
                        flushing_redactions: HashMap::new(),
                    };

                    return Some(
                        async move {
                            metadata::update(
                                &kind,
                                read_marker,
                                max_triggers_unread,
                                max_triggers_highlight,
                                None,
                            )
                            .await
                        }
                        .boxed(),
                    );
                }

                let last_seen = last_seen.clone();
                let chathistory_references =
                    metadata::latest_can_reference(messages)
                        .max(chathistory_references.clone());

                let full_history = std::mem::replace(
                    self,
                    Self::Partial {
                        kind,
                        pending_messages: vec![],
                        last_updated_at: None,
                        read_marker,
                        max_triggers_unread,
                        max_triggers_highlight,
                        chathistory_references: chathistory_references.clone(),
                        last_seen,
                        pending_reactions: HashMap::new(),
                        pending_redactions: HashMap::new(),
                        show_in_sidebar: true,
                        flushing_messages: vec![],
                        flushing_reactions: HashMap::new(),
                        flushing_redactions: HashMap::new(),
                    },
                );

                match full_history {
                    History::Partial { .. } => None,
                    History::Full { kind, messages, .. } => Some(
                        async move {
                            overwrite(
                                &kind,
                                &messages,
                                read_marker,
                                chathistory_references,
                            )
                            .await
                        }
                        .boxed(),
                    ),
                }
            }
        }
    }

    async fn close(self, seed: Option<Seed>) -> Result<(), Error> {
        match self {
            History::Partial {
                kind,
                pending_messages,
                read_marker,
                max_triggers_unread,
                max_triggers_highlight,
                chathistory_references,
                pending_reactions,
                pending_redactions,
                ..
            } => append(
                &kind,
                seed,
                pending_messages,
                read_marker,
                max_triggers_unread,
                max_triggers_highlight,
                chathistory_references,
                pending_reactions,
                pending_redactions,
            )
            .await
            .map(|_| ()),
            History::Full {
                kind,
                messages,
                read_marker,
                chathistory_references,
                ..
            } => {
                overwrite(&kind, &messages, read_marker, chathistory_references)
                    .await
            }
        }
    }

    pub fn mark_as_read(&mut self) -> Option<ReadMarker> {
        let (read_marker, latest) = match self {
            History::Partial {
                max_triggers_unread,
                read_marker,
                ..
            } => (read_marker, max_triggers_unread.map(ReadMarker::from)),
            History::Full {
                messages,
                read_marker,
                display_read_marker,
                ..
            } => {
                let latest = ReadMarker::latest(messages);

                if latest > *display_read_marker {
                    *display_read_marker = latest;
                }

                (read_marker, latest)
            }
        };

        if latest > *read_marker {
            *read_marker = latest;

            latest
        } else {
            None
        }
    }

    pub fn can_mark_as_read(&self) -> bool {
        match self {
            History::Partial { .. } => self.has_unread(),
            History::Full {
                messages,
                read_marker,
                ..
            } => {
                if messages.is_empty() {
                    false
                } else {
                    *read_marker < ReadMarker::latest(messages)
                }
            }
        }
    }

    pub fn first_can_reference(&self) -> Option<&Message> {
        let can_reference = |message: &Message| {
            message.can_reference() && !message.is_rerouted()
        };

        match self {
            History::Partial {
                pending_messages, ..
            } => pending_messages.iter().find_map(|(message, _)| {
                can_reference(message).then_some(message)
            }),
            History::Full { messages, .. } => {
                messages.iter().find(|message| can_reference(message))
            }
        }
    }

    pub fn last_can_reference_before_or_at(
        &self,
        server_time: DateTime<Utc>,
        allow_at: bool,
        message_reference_types: &[isupport::MessageReferenceType],
    ) -> Option<isupport::MessageReference> {
        let can_reference_before = |message: &Message| {
            message.can_reference()
                && !message.is_rerouted()
                && message.server_time < server_time
        };

        let mut at_message = None;

        let can_reference_at = |message: &Message| {
            message.can_reference()
                && !message.is_rerouted()
                && message.server_time == server_time
        };

        let (before_message, chathistory_references) = match self {
            History::Partial {
                pending_messages,
                chathistory_references,
                ..
            } => (
                pending_messages.iter().rev().find_map(|(message, _)| {
                    if at_message.is_none() && can_reference_at(message) {
                        at_message = Some(message);
                    }

                    can_reference_before(message).then_some(message)
                }),
                chathistory_references,
            ),
            History::Full {
                messages,
                chathistory_references,
                ..
            } => (
                messages.iter().rev().find(|message| {
                    if at_message.is_none() && can_reference_at(message) {
                        at_message = Some(message);
                    }

                    can_reference_before(message)
                }),
                chathistory_references,
            ),
        };

        // If a reference before server_time exists, then return that reference.
        if let Some(message_references) =
            before_message.map(Message::references).max(
                if chathistory_references.as_ref().is_some_and(
                    |chathistory_references| {
                        chathistory_references.timestamp < server_time
                    },
                ) {
                    chathistory_references.clone()
                } else {
                    None
                },
            )
        {
            message_references.message_reference(message_reference_types)
        // Else, if a reference at server_time is allowed, exists, and timestamp
        // references are supported, then return a timestamp reference at
        // server_time.
        } else if allow_at
            && message_reference_types
                .contains(&isupport::MessageReferenceType::Timestamp)
            && (at_message.is_some()
                || chathistory_references.as_ref().is_some_and(
                    |chathistory_references| {
                        chathistory_references.timestamp == server_time
                    },
                ))
        {
            Some(isupport::MessageReference::Timestamp(server_time))
        } else {
            None
        }
    }

    pub fn update_chathistory_references(
        &mut self,
        chathistory_references: MessageReferences,
    ) {
        let (stored, last_updated_at) = match self {
            History::Partial {
                chathistory_references: stored_chathistory_references,
                last_updated_at,
                ..
            } => (stored_chathistory_references, last_updated_at),
            History::Full {
                chathistory_references: stored_chathistory_references,
                last_updated_at,
                ..
            } => (stored_chathistory_references, last_updated_at),
        };

        if stored
            .as_ref()
            .is_none_or(|stored| chathistory_references > *stored)
        {
            *stored = Some(chathistory_references);
            *last_updated_at = Some(Instant::now());
        }
    }

    pub fn update_read_marker(&mut self, read_marker: ReadMarker) -> bool {
        let stored = match self {
            History::Partial {
                read_marker: stored_read_marker,
                ..
            } => stored_read_marker,
            History::Full {
                display_read_marker,
                read_marker: stored_read_marker,
                ..
            } => {
                *display_read_marker =
                    (*display_read_marker).max(Some(read_marker));
                stored_read_marker
            }
        };

        if Some(read_marker) > *stored {
            *stored = Some(read_marker);
            true
        } else {
            false
        }
    }

    pub fn read_marker(&self) -> Option<ReadMarker> {
        match self {
            History::Partial { read_marker, .. }
            | History::Full { read_marker, .. } => *read_marker,
        }
    }

    pub fn update_display_read_marker(&mut self, read_marker: ReadMarker) {
        if let History::Full {
            display_read_marker,
            ..
        } = self
        {
            *display_read_marker =
                (*display_read_marker).max(Some(read_marker));
        }
    }

    pub fn display_read_marker(&self) -> Option<ReadMarker> {
        match self {
            History::Partial { .. } => None,
            History::Full {
                display_read_marker,
                ..
            } => *display_read_marker,
        }
    }

    pub fn hide_preview(&mut self, history_id: Id, url: url::Url) {
        if let Self::Full {
            messages,
            last_updated_at,
            ..
        } = self
            && let Some(message) =
                messages.iter_mut().find(|m| m.history_id == history_id)
        {
            message.hidden_urls.insert(url);

            *last_updated_at = Some(Instant::now());
        }
    }

    pub fn show_preview(&mut self, history_id: Id, url: &url::Url) {
        if let Self::Full {
            messages,
            last_updated_at,
            ..
        } = self
            && let Some(message) =
                messages.iter_mut().find(|m| m.history_id == history_id)
        {
            message.hidden_urls.remove(url);

            *last_updated_at = Some(Instant::now());
        }
    }

    pub fn add_reaction(
        &mut self,
        reaction: reaction::Context,
        notification_enabled: bool,
        labeled_response_context: Option<LabeledResponseContext>,
    ) -> Option<ReactionToEcho> {
        match self {
            History::Partial {
                pending_messages,
                last_updated_at,
                pending_reactions,
                ..
            } => {
                if let Some(message) =
                    pending_messages.iter_mut().rev().find_map(|(m, _)| {
                        (m.id.as_deref() == Some(&*reaction.in_reply_to))
                            .then_some(m)
                    })
                {
                    let message_text = if message.is_echo
                        && message.direction == Direction::Received
                    {
                        Some(message.text().to_string())
                    } else {
                        None
                    };

                    message.reactions.push(reaction.inner.clone());

                    if let Some(message_text) = message_text
                        && notification_enabled
                    {
                        return Some(ReactionToEcho {
                            reaction,
                            message_text,
                        });
                    } else {
                        return None;
                    }
                } else {
                    let pending = pending_reactions
                        .entry(reaction.in_reply_to)
                        .or_insert(reaction::Pending::default());

                    pending.reactions.push(reaction::PendingReaction {
                        reaction: reaction.inner,
                        is_echo: reaction.is_echo,
                        deduplicate: reaction.deduplicate,
                        labeled_response_context,
                        notification_enabled,
                    });
                }

                *last_updated_at = Some(Instant::now());
            }
            History::Full {
                messages,
                last_updated_at,
                ..
            } => {
                let message = find_message_mut_by_id(
                    messages,
                    &reaction.in_reply_to,
                    &reaction.inner.server_time,
                )?;
                message.reactions.push(reaction.inner.clone());

                *last_updated_at = Some(Instant::now());

                if message.is_echo
                    && message.direction == Direction::Received
                    && notification_enabled
                {
                    return Some(ReactionToEcho {
                        reaction,
                        message_text: message.text().to_string(),
                    });
                } else {
                    return None;
                };
            }
        }
        None
    }

    pub fn redact_message(
        &mut self,
        id: message::Id,
        redaction: Redaction,
        server_time: DateTime<Utc>,
        display_redacted: bool,
    ) {
        match self {
            History::Partial {
                pending_messages,
                last_updated_at,
                pending_redactions,
                ..
            } => {
                if let Some(message) =
                    pending_messages.iter_mut().rev().find_map(|(m, _)| {
                        (m.id.as_deref() == Some(&*id)).then_some(m)
                    })
                {
                    message.redaction = Some(redaction);
                } else {
                    let pending = pending_redactions.entry(id).or_insert(
                        redaction::Pending::new(redaction, server_time),
                    );

                    pending.server_time =
                        (pending.server_time).min(server_time);
                }

                *last_updated_at = Some(Instant::now());
            }
            History::Full {
                messages,
                last_updated_at,
                ..
            } => {
                let Some(position) =
                    position_message_by_id(messages, &id, &server_time)
                else {
                    return;
                };

                messages[position].redaction = Some(redaction);

                if !display_redacted {
                    messages[position].blocked = true;
                }

                let updated_reply = messages[position].as_reply_preview();
                for message in messages.iter_mut() {
                    if message.reply_to.as_deref() == Some(&*id) {
                        message.reply_preview = Some(updated_reply.clone());
                    }
                }

                *last_updated_at = Some(Instant::now());
            }
        }
    }

    pub fn last_seen(&self) -> HashMap<Nick, DateTime<Utc>> {
        match self {
            History::Partial { last_seen, .. }
            | History::Full { last_seen, .. } => last_seen.clone(),
        }
    }

    pub fn renormalize_messages(&mut self, seed: Seed) {
        match self {
            History::Full { messages, .. } => {
                renormalize_messages(messages.iter_mut(), seed);
            }
            History::Partial {
                pending_messages, ..
            } => renormalize_messages(
                pending_messages.iter_mut().map(|(message, _)| message),
                seed,
            ),
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
/// For non-echoes a search window of +/- 1 second around the server time of the
/// incoming message is used.  For labeled echoes the exact time should be
/// either be stored locally, or the message was sent from another client.  For
/// unlabled echoes a search window of +/- 300s is used to account for transit
/// time and potential clock skew. For matching methods that do not have an
/// identifier (i.e. when matching historical messages without an ID or unlabled
/// echoes) the messages must have an exact match + target & / content.
///
/// A non-None return value indicates a message sent from this client was was
/// replaced by an echo (and the replacement's server_time corresponds to the
/// ReadMarker).
pub fn insert_message(
    messages: &mut Vec<Message>,
    message: Message,
    labeled_response_context: Option<LabeledResponseContext>,
) -> Option<ReadMarker> {
    if messages.is_empty() {
        messages.push(message);

        return None;
    }

    let message_is_unlabeled_echo =
        matches!(message.direction, message::Direction::Received)
            && message.is_echo
            && labeled_response_context.is_none();

    let fuzz_seconds = if message_is_unlabeled_echo {
        chrono::Duration::seconds(300)
    } else {
        chrono::Duration::seconds(1)
    };

    let mut read_marker = None;

    if let Some(labeled_response_context) = &labeled_response_context {
        let start = labeled_response_context.server_time - fuzz_seconds;
        let end = labeled_response_context.server_time + fuzz_seconds;

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

        if let Some(index) = messages[start_index..end_index]
            .iter()
            .enumerate()
            .find_map(|(slice_index, stored)| {
                (stored.id.as_ref().is_some_and(|id| {
                    *id == labeled_response_context.label_as_id
                }) && stored.target == message.target)
                    .then_some(start_index + slice_index)
            })
        {
            messages.remove(index);

            read_marker = Some(ReadMarker::from(&message));
        }
    }

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

    let mut insert_at = start_index;
    let mut replace_at = None;

    for (current_index, stored) in
        (start_index..).zip(messages[start_index..end_index].iter())
    {
        if replace_at.is_none() && labeled_response_context.is_none() {
            let use_echo_cmp =
                matches!(stored.direction, message::Direction::Sent)
                    && message_is_unlabeled_echo;

            let check_for_matching_content = (stored.id.is_none()
                || message.id.is_none())
                && ((message.deduplicate
                    && stored.server_time == message.server_time)
                    || use_echo_cmp);

            if (message.id.is_some() && stored.id == message.id)
                || (check_for_matching_content
                    && has_matching_content(stored, &message, use_echo_cmp))
            {
                replace_at = Some(current_index);
            }
        }

        if message.server_time >= stored.server_time {
            insert_at = current_index + 1;
        }
    }

    if let Some(index) = replace_at {
        if messages[index].server_time == message.server_time {
            if message.deduplicate
                && has_matching_content(&messages[index], &message, false)
            {
                // Perform a minimal update if this is a message from
                // chathistory (or ZNC playback) and has the same raw content,
                // since the newly received message will have been parsed
                // without historical state.
                if messages[index].id.is_none() {
                    messages[index].id = message.id;
                }
                messages[index].direction = message::Direction::Received;
                messages[index].command = None;
                messages[index].received_at = message.received_at;
            } else {
                messages[index] = Message {
                    id: message.id.or(messages[index].id.clone()),
                    ..message
                };
            }
        } else {
            if message_is_unlabeled_echo {
                read_marker = Some(ReadMarker::from(&message));
            }

            match insert_at.cmp(&index) {
                Ordering::Less => {
                    messages.remove(index);
                    messages.insert(insert_at, message);
                }
                Ordering::Equal => messages[index] = message,
                Ordering::Greater => {
                    messages.insert(insert_at, message);
                    messages.remove(index);
                }
            }
        }
    } else {
        messages.insert(insert_at, message);
    }

    read_marker
}

/// The content of JOIN, PART, and QUIT messages may be dependent on how
/// the user attributes are resolved.  Match those messages based on Nick
/// alone (covered by comparing target components) to avoid false negatives.
fn has_matching_content(
    message: &Message,
    other: &Message,
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
    reactions: &mut Vec<Reaction>,
    reaction: Reaction,
    is_echo: bool,
    deduplicate: bool,
    labeled_response_context: Option<LabeledResponseContext>,
) {
    if reactions.is_empty() {
        reactions.push(reaction);

        return;
    }

    if let Some(labeled_response_context) = &labeled_response_context {
        if let Some(index) = reactions.iter().position(|reaction| {
            reaction
                .id
                .as_ref()
                .is_some_and(|id| *id == labeled_response_context.label_as_id)
        }) {
            reactions.remove(index);
        }

        reactions.push(reaction);
    } else if let Some(index) = reactions.iter().position(|stored| {
        (stored.id.is_some() && stored.id == reaction.id)
            || (deduplicate
                && (stored.server_time == reaction.server_time || is_echo)
                && stored.sender == reaction.sender
                && stored.text == reaction.text
                && stored.unreact == reaction.unreact)
    }) {
        reactions[index] = reaction;
    } else {
        reactions.push(reaction);
    }
}

pub fn find_message_by_hash<'a>(
    messages: &'a [Message],
    hash: message::Hash,
    server_time: &DateTime<Utc>,
) -> Option<&'a Message> {
    position_message(messages, |message| message.hash == hash, server_time)
        .and_then(|position| messages.get(position))
}

pub fn find_message_by_id<'a>(
    messages: &'a [Message],
    id: &message::Id,
    server_time: &DateTime<Utc>,
) -> Option<&'a Message> {
    position_message_by_id(messages, id, server_time)
        .and_then(|position| messages.get(position))
}

pub fn find_message_mut_by_id<'a>(
    messages: &'a mut [Message],
    id: &message::Id,
    server_time: &DateTime<Utc>,
) -> Option<&'a mut Message> {
    position_message_by_id(messages, id, server_time)
        .and_then(|position| messages.get_mut(position))
}

pub fn position_message_by_id(
    messages: &[Message],
    id: &message::Id,
    server_time: &DateTime<Utc>,
) -> Option<usize> {
    position_message(
        messages,
        |message| message.id.as_deref() == Some(id),
        server_time,
    )
}

pub fn position_message(
    messages: &[Message],
    is_match: impl Fn(&Message) -> bool,
    server_time: &DateTime<Utc>,
) -> Option<usize> {
    if messages.is_empty() {
        return None;
    }

    // We're either looking for the message at server_time or one that is
    // expected to before (e.g. the message that is reacted or replied to at
    // server_time).  Fuzz ahead one second to ensure all messages at
    // server_time are checked.

    let start = *server_time + chrono::Duration::seconds(1);

    let start_index = match messages
        .binary_search_by(|stored| stored.server_time.cmp(&start))
    {
        Ok(match_index) => match_index,
        Err(sorted_insert_index) => sorted_insert_index,
    };

    // Check messages at server_time, then before server_time, then check for
    // the unlikely scenario where the message we're looking for is after the
    // provided server_time.

    messages
        .iter()
        .take(start_index)
        .rev()
        .position(&is_match)
        .map(|position| start_index - 1 - position)
        .or(messages
            .iter()
            .skip(start_index)
            .rev()
            .position(is_match)
            .map(|position| messages.len() - 1 - position))
}

#[derive(Debug)]
pub struct View<'a> {
    pub total: usize,
    pub has_more_older_messages: bool,
    pub has_more_newer_messages: bool,
    pub old_messages: Vec<&'a Message>,
    pub new_messages: Vec<&'a Message>,
    pub cleared: bool,
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

pub fn smart_filter_message(
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

pub fn smart_filter_repeat(
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

pub fn smart_filter_internal_message(
    message: &message::Message,
    seconds: &i64,
    current_time: &DateTime<Utc>,
) -> bool {
    let duration_seconds = current_time
        .signed_duration_since(message.server_time)
        .num_seconds();

    duration_seconds > *seconds
}
