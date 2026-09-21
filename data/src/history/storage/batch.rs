use chrono::{DateTime, Utc};
use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;

use super::{Error, Event, Update};
use crate::buffer::BuffersContext;
use crate::client::ClientsContext;
use crate::history::filter::{Filter, FilterChain};
use crate::history::{
    Id, Kind, Metadata, ReadMarker, database, smart_filter_message,
    smart_filter_repeat,
};
use crate::message::{self, Source, highlight};
use crate::{
    Config, Notification, Server, client, config, isupport, reaction, redaction,
};

#[derive(Debug)]
pub struct Batch {
    histories: Vec<HistoryBatch>,
    filters: Vec<Filter>,
    buffer: config::Buffer,
    captured_at: DateTime<Utc>,
}

#[derive(Debug)]
struct HistoryBatch {
    kind: Kind,
    casemapping: isupport::CaseMap,
    reference_types: Vec<isupport::MessageReferenceType>,
    monitored: bool,
    updates: Vec<Prepared>,
}

#[derive(Debug, Clone)]
enum Prepared {
    Message {
        message: message::MessageWithContext,
        notification: Option<Notification>,
        reply_notification: Option<Notification>,
    },
    Reaction(reaction::ReactionWithContext, bool),
    Redaction(redaction::RedactionWithContext),
    Remove(Id),
    Preview(Id, url::Url, bool),
    ReadMarker(ReadMarker),
    Sidebar(bool),
}

struct Pending {
    message: message::Message,
    notifications: Vec<Notification>,
    highlight: Option<message::MessageWithContext>,
    sent: bool,
    echoed: bool,
}

impl Pending {
    fn new(message: message::Message) -> Self {
        Self {
            message,
            notifications: vec![],
            highlight: None,
            sent: false,
            echoed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Metadata,
    Append,
    Replace,
}

#[derive(Debug)]
pub struct Committed {
    pub change: Change,
    pub kind: Kind,
    pub metadata: Metadata,
    pub display_read_marker: Option<ReadMarker>,
    pub show_in_sidebar: Option<bool>,
    pub admitted_latest: Option<DateTime<Utc>>,
    pub events: Vec<Event>,
}

impl Batch {
    pub(super) fn kinds(&self) -> impl Iterator<Item = Kind> {
        let mut kinds = self
            .histories
            .iter()
            .map(|history| history.kind.clone())
            .collect::<HashSet<_>>();
        if self.histories.iter().any(|history| history.updates.iter().any(|update| {
            matches!(update, Prepared::Message { message, .. } if message.highlight.is_some())
                || matches!(update, Prepared::Redaction(_))
        })) {
            kinds.insert(Kind::Highlights);
        }
        if self.histories.iter().any(|history| history.monitored) {
            kinds.insert(Kind::ChannelMonitor);
        }
        kinds.into_iter()
    }

    pub(super) fn transient(
        &self,
    ) -> impl Iterator<Item = (Kind, message::Message)> + '_ {
        self.histories.iter().flat_map(|history| {
            history.updates.iter().filter_map(|update| {
                if let Prepared::Message { message, .. } = update {
                    Some((history.kind.clone(), message.inner.clone()))
                } else {
                    None
                }
            })
        })
    }

    pub(super) fn capture(
        updates: HashMap<Kind, Vec<Update>>,
        filters: &[Filter],
        clients: &dyn ClientsContext,
        buffers: &dyn BuffersContext,
        focused_window: &Option<iced::window::Id>,
        config: &Config,
    ) -> Self {
        let mut histories = Vec::with_capacity(updates.len());
        for (kind, updates) in updates {
            let server = kind.as_server();
            let casemapping =
                clients.get_maybe_server_casemapping_or_default(server);
            let window = buffers.find_window_with(&kind);
            let notify = window.is_none() || *focused_window != window;
            let reference_types = server
                .map(|server| {
                    clients
                        .get_server_chathistory_message_reference_types(server)
                        .to_vec()
                })
                .unwrap_or_default();
            let monitored = buffers.is_open(&Kind::ChannelMonitor)
                && matches!(&kind, Kind::Channel(server, channel)
                    if config.channel_monitor.is_channel_included(server, channel, casemapping));
            let mut prepared = Vec::with_capacity(updates.len());
            for update in updates {
                prepared.push(match update {
                    Update::Message(_, message) => {
                        let (notification, reply_notification) = notifications(
                            &message,
                            server,
                            casemapping,
                            notify,
                            config,
                        );
                        Prepared::Message {
                            message,
                            notification,
                            reply_notification,
                        }
                    }
                    Update::Reaction(_, reaction) => {
                        Prepared::Reaction(reaction, notify)
                    }
                    Update::Redaction(_, redaction) => {
                        Prepared::Redaction(redaction)
                    }
                    Update::Remove(_, id, _) => Prepared::Remove(id),
                    Update::ShowPreview(_, id, _, url) => {
                        Prepared::Preview(id, url, true)
                    }
                    Update::HidePreview(_, id, _, url) => {
                        Prepared::Preview(id, url, false)
                    }
                    Update::ReadMarker(_, marker) => {
                        Prepared::ReadMarker(marker)
                    }
                    Update::ShowInSidebar(_, show) => Prepared::Sidebar(show),
                    Update::Broadcast(..) => continue,
                });
            }
            histories.push(HistoryBatch {
                kind,
                casemapping,
                reference_types,
                monitored,
                updates: prepared,
            });
        }
        Self {
            histories,
            filters: filters.to_vec(),
            buffer: config.buffer.clone(),
            captured_at: Utc::now(),
        }
    }

    pub(super) fn apply(
        &self,
        write: &mut database::Write<'_>,
    ) -> Result<Vec<Committed>, Error> {
        let mut committed = Vec::new();
        let mut highlights_changed = false;
        let mut monitor = None::<Committed>;
        for history in &self.histories {
            let kind = &history.kind;
            let mut result = Committed {
                change: Change::Replace,
                kind: kind.clone(),
                metadata: write.get_metadata(kind)?,
                display_read_marker: None,
                show_in_sidebar: None,
                admitted_latest: None,
                events: vec![],
            };
            let mut pending = IndexMap::<Id, Pending>::new();
            let mut received_marker = None;
            for update in &history.updates {
                match update {
                    Prepared::Message {
                        message,
                        notification,
                        reply_notification,
                    } => {
                        let reply_is_ours = if let Some(id) =
                            &message.inner.reply_to
                        {
                            write
                                .message_by_id(kind, id, message.inner.time)?
                                .is_some_and(|message| message.is_ours())
                        } else {
                            false
                        };
                        let (stored, replaced_sent) = write.insert_normalized(
                            kind,
                            message.clone(),
                            history.casemapping,
                        )?;
                        let mut notifications = vec![];
                        if let Some(notification) = if reply_is_ours {
                            reply_notification
                                .as_ref()
                                .or(notification.as_ref())
                        } else {
                            notification.as_ref()
                        } {
                            notifications.push(notification.clone());
                        }
                        let sent = stored.is_sent()
                            && self.buffer.mark_as_read.on_message_sent;
                        let echoed = replaced_sent
                            && self.buffer.mark_as_read.on_message_sent;
                        pending.insert(
                            stored.history_id,
                            Pending {
                                message: stored,
                                notifications,
                                highlight: message
                                    .highlight
                                    .as_ref()
                                    .map(|_| message.clone()),
                                sent,
                                echoed,
                            },
                        );
                    }
                    Prepared::Reaction(reaction, notify) => {
                        if let Some(stored) = write.reaction(
                            kind,
                            reaction.clone(),
                            history.casemapping,
                        )? {
                            let notification = (*notify
                                && reaction.notification_allowed
                                && stored.is_ours()
                                && !reaction.is_ours())
                            .then(|| Notification::Reaction {
                                casemapping: history.casemapping,
                                reaction: reaction.clone(),
                                message_text: stored.text().into_owned(),
                            });
                            if notification.is_some()
                                || pending.contains_key(&stored.history_id)
                            {
                                let entry = pending
                                    .entry(stored.history_id)
                                    .or_insert_with(|| {
                                        Pending::new(stored.clone())
                                    });
                                entry.message = stored;
                                entry.notifications.extend(notification);
                            }
                        }
                    }
                    Prepared::Redaction(redaction) => {
                        if let Some(stored) =
                            write.redaction(kind, redaction.clone())?
                        {
                            if stored.triggers_highlight() {
                                highlights_changed |= write.redact_highlight(
                                    kind,
                                    stored.history_id,
                                    redaction.clone(),
                                )?;
                            }
                            if stored.triggers_highlight()
                                || (history.monitored
                                    && stored.show_in_channel_monitor())
                                || pending.contains_key(&stored.history_id)
                            {
                                let entry = pending
                                    .entry(stored.history_id)
                                    .or_insert_with(|| {
                                        Pending::new(stored.clone())
                                    });
                                entry.message = stored;
                            }
                        }
                    }
                    Prepared::Remove(id) => {
                        write.remove(kind, *id)?;
                        pending.shift_remove(id);
                    }
                    Prepared::Preview(id, url, visible) => {
                        write.preview(kind, *id, url.clone(), *visible)?;
                    }
                    Prepared::ReadMarker(marker) => {
                        received_marker = received_marker.max(Some(*marker));
                    }
                    Prepared::Sidebar(show) => {
                        result.show_in_sidebar = Some(*show);
                    }
                }
            }
            let mut send_marker = None;
            for (_, pending) in pending {
                let Pending {
                    message: mut stored,
                    notifications,
                    highlight,
                    sent,
                    echoed,
                } = pending;
                stored.renormalize(history.casemapping);
                if !stored.is_rerouted()
                    && stored.can_reference(&history.reference_types)
                    && result
                        .metadata
                        .latest_chathistory_references
                        .as_ref()
                        .is_none_or(|(time, _)| *time < stored.time)
                {
                    result.metadata.latest_chathistory_references =
                        Some((stored.time, stored.references()));
                }
                if self.blocked(write, history, &stored)? && !stored.is_ours() {
                    continue;
                }
                if let Some(mut highlight) = highlight
                    && let Some(server) = kind.as_server()
                    && let Some(channel) = stored.target.as_channel()
                {
                    highlight.inner = stored.clone();
                    highlight.inner.history_id = Id::default();
                    highlight.inner.target = message::Target::Highlights {
                        server: server.clone(),
                        channel: channel.clone(),
                        history_id: stored.history_id,
                    };
                    highlight.highlight = None;
                    highlight.notification_allowed = false;
                    let (highlight, _) = write.insert_normalized(
                        &Kind::Highlights,
                        highlight,
                        history.casemapping,
                    )?;
                    let mut metadata = write.get_metadata(&Kind::Highlights)?;
                    update_metadata(&mut metadata, &highlight);
                    write.metadata(&Kind::Highlights, &metadata)?;
                    highlights_changed = true;
                }
                if history.monitored && stored.show_in_channel_monitor() {
                    if monitor.is_none() {
                        monitor = Some(Committed {
                            change: Change::Replace,
                            kind: Kind::ChannelMonitor,
                            metadata: write
                                .get_metadata(&Kind::ChannelMonitor)?,
                            display_read_marker: None,
                            show_in_sidebar: Some(true),
                            admitted_latest: None,
                            events: vec![],
                        });
                    }
                    if let Some(monitor) = &mut monitor {
                        update_metadata(&mut monitor.metadata, &stored);
                        monitor.admitted_latest =
                            monitor.admitted_latest.max(Some(stored.time.utc));
                        if sent || echoed {
                            monitor.display_read_marker = monitor
                                .display_read_marker
                                .max(Some(ReadMarker::from(&stored)));
                        }
                        if echoed {
                            monitor.metadata.read_marker = monitor
                                .metadata
                                .read_marker
                                .max(Some(ReadMarker::from(&stored)));
                        }
                    }
                }
                update_metadata(&mut result.metadata, &stored);
                result.admitted_latest =
                    result.admitted_latest.max(Some(stored.time.utc));
                result.show_in_sidebar = Some(true);
                if sent || echoed {
                    result.display_read_marker = result
                        .display_read_marker
                        .max(Some(ReadMarker::from(&stored)));
                }
                if echoed {
                    send_marker =
                        send_marker.max(Some(ReadMarker::from(&stored)));
                }
                if let Some(server) = kind.as_server() {
                    result.events.extend(notifications.into_iter().map(
                        |notification| {
                            Event::Notification(server.clone(), notification)
                        },
                    ));
                }
            }
            result.display_read_marker =
                result.display_read_marker.max(received_marker);
            let previous = result.metadata.read_marker;
            result.metadata.read_marker =
                previous.max(received_marker).max(send_marker);
            if send_marker > previous
                && send_marker >= received_marker
                && let Some(marker) = send_marker
                && let Some(server) = kind.as_server()
                && let Some(target) = kind.target()
            {
                result.events.push(Event::Client(
                    client::Message::SendMarkread(
                        server.clone(),
                        target,
                        marker,
                    ),
                ));
            }
            result.change = if history.updates.iter().all(|update| {
                matches!(update, Prepared::ReadMarker(_) | Prepared::Sidebar(_))
            }) {
                Change::Metadata
            } else if !matches!(kind, Kind::Highlights | Kind::ChannelMonitor)
                && history.updates.iter().all(|update| {
                    matches!(
                        update,
                        Prepared::Message { .. }
                            | Prepared::ReadMarker(_)
                            | Prepared::Sidebar(_)
                    )
                })
                && write.append_only(kind)
            {
                Change::Append
            } else {
                Change::Replace
            };
            write.metadata(kind, &result.metadata)?;
            committed.push(result);
        }
        if let Some(mut monitor) = monitor {
            let current = write.get_metadata(&Kind::ChannelMonitor)?;
            monitor.metadata.read_marker =
                monitor.metadata.read_marker.max(current.read_marker);
            monitor.metadata.latest =
                monitor.metadata.latest.max(current.latest);
            monitor.metadata.latest_triggers_unread = monitor
                .metadata
                .latest_triggers_unread
                .max(current.latest_triggers_unread);
            monitor.metadata.latest_triggers_highlight = monitor
                .metadata
                .latest_triggers_highlight
                .max(current.latest_triggers_highlight);
            write.metadata(&Kind::ChannelMonitor, &monitor.metadata)?;
            committed.push(monitor);
        }
        if highlights_changed {
            committed.push(Committed {
                change: Change::Replace,
                kind: Kind::Highlights,
                metadata: write.get_metadata(&Kind::Highlights)?,
                display_read_marker: None,
                show_in_sidebar: Some(true),
                admitted_latest: None,
                events: vec![],
            });
        }
        Ok(committed)
    }

    fn blocked(
        &self,
        write: &database::Write<'_>,
        history: &HistoryBatch,
        message: &message::Message,
    ) -> Result<bool, Error> {
        if FilterChain::borrow(&self.filters)
            .filter_message_of_kind(message, &history.kind)
        {
            return Ok(true);
        }
        if message.has_redaction()
            && !self.buffer.redaction.display.is_visible()
        {
            return Ok(true);
        }
        if let Source::Internal(message::source::Internal::Status(status)) =
            &message.source
        {
            if !self.buffer.internal_messages.enabled(status) {
                return Ok(true);
            }
            if let Some(seconds) = self.buffer.internal_messages.smart(status) {
                return Ok(crate::history::smart_filter_internal_message(
                    message,
                    &seconds,
                    &self.captured_at,
                ));
            }
        }
        if let Source::Server(source) = &message.source {
            if let Some(server) = history.kind.as_server()
                && let Some(target) = message.target.as_targetref()
                && !self.buffer.server_messages.should_show_message(
                    source.as_ref(),
                    target,
                    server,
                    history.casemapping,
                )
            {
                return Ok(true);
            }
            if let Some(source) = source
                && let Some(seconds) =
                    self.buffer.server_messages.smart(Some(source.kind))
                && let Some(nick) = &source.nick
            {
                let previous = write.smart_previous(
                    &history.kind,
                    message,
                    nick,
                    source.kind == message::Kind::Away,
                    seconds,
                    history.casemapping,
                )?;
                return Ok(if source.kind == message::Kind::Away {
                    smart_filter_repeat(message, &seconds, previous.as_ref())
                } else {
                    smart_filter_message(message, &seconds, previous.as_ref())
                });
            }
        }
        Ok(false)
    }
}

fn update_metadata(metadata: &mut Metadata, message: &message::Message) {
    metadata.latest = metadata.latest.max(Some(message.time.utc));
    if message.triggers_unread() {
        metadata.latest_triggers_unread =
            metadata.latest_triggers_unread.max(Some(message.time.utc));
    }
    if message.triggers_highlight() {
        metadata.latest_triggers_highlight = metadata
            .latest_triggers_highlight
            .max(Some(message.time.utc));
    }
}

fn notifications(
    message: &message::MessageWithContext,
    server: Option<&Server>,
    casemapping: isupport::CaseMap,
    notify: bool,
    config: &Config,
) -> (Option<Notification>, Option<Notification>) {
    let (Some(server), Some(user)) = (server, message.inner.user()) else {
        return (None, None);
    };
    if !notify || !message.notification_allowed {
        return (None, None);
    }
    let text = message.inner.text().into_owned();
    if let Some(channel) = message.inner.target.as_channel() {
        if let Some(highlight) = &message.highlight {
            let (description, sound) = match highlight {
                highlight::Kind::Nick => ("highlighted you".to_string(), None),
                highlight::Kind::Match { matching, sound } => {
                    (format!("matched highlight {matching}"), sound.clone())
                }
            };
            return (
                Some(Notification::Highlight {
                    user: user.clone(),
                    channel: channel.clone(),
                    casemapping,
                    message: text,
                    description,
                    sound,
                }),
                None,
            );
        }
        let reply =
            message
                .inner
                .reply_to
                .as_ref()
                .map(|_| Notification::Reply {
                    user: user.clone(),
                    channel: channel.clone(),
                    casemapping,
                    message: text.clone(),
                });
        let channel = config
            .notifications
            .channels
            .get(channel.as_str())
            .filter(|settings| {
                settings.should_notify(user, None, server, casemapping)
            })
            .map(|_| Notification::Channel {
                user: user.clone(),
                channel: channel.clone(),
                casemapping,
                message: text,
            });
        (channel, reply)
    } else if matches!(message.inner.target, message::Target::Query { .. }) {
        (
            Some(Notification::DirectMessage {
                user: user.clone(),
                casemapping,
                message: text,
            }),
            None,
        )
    } else {
        (None, None)
    }
}
