use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use itertools::Itertools;

use super::metadata::ReadMarker;
use super::{
    Id, Kind, KindRef, find_message_by_history_id, find_message_by_id,
    find_message_mut_by_history_id, position_message_by_history_id,
};
use crate::message::{self, ReplyPreview, Searchable, Temporal};
use crate::{Config, Server, config, target};

#[derive(Debug, Clone)]
pub enum Message {
    Update(Kind, Update),
    Remove(Kind),
    GoToMessage(Navigation),
}

#[derive(Debug)]
pub enum Event {
    Opened(Kind),
    GoToMessage(Navigation),
}

#[derive(Debug, Clone)]
pub struct Navigation {
    pub server: Server,
    pub channel: target::Channel,
    pub message: Option<Id>,
    pub buffer_action: crate::dashboard::BufferAction,
    pub token: std::sync::Weak<()>,
}

#[derive(Debug, Default)]
pub struct Manager {
    models: HashMap<Kind, Model>,
}

impl Manager {
    pub fn view(
        &self,
        kind_ref: KindRef,
        request_limit: &message::Limit,
        config: &Config,
    ) -> Option<View<'_>> {
        self.models
            .get(&kind_ref)
            .map(|model| model.view(request_limit, config))
    }

    /// Resolve a source message to its current row, including a condensed group.
    pub fn resolve_anchor(
        &self,
        kind: KindRef,
        id: &Id,
        time: &message::Time,
        config: &config::Buffer,
    ) -> Option<Id> {
        let model = self.models.get(&kind)?;
        let Pane::Open { messages, .. } = &model.pane else {
            return None;
        };
        resolve_anchor(messages, id, time, config)
    }

    pub fn can_mark_as_read(&self, kind: &Kind) -> bool {
        self.models.get(kind).is_some_and(Model::can_mark_as_read)
    }

    pub fn has_unread(&self, kind: &Kind) -> bool {
        self.models.get(kind).is_some_and(Model::has_unread)
    }

    pub fn has_highlight(&self, kind: &Kind) -> bool {
        self.models.get(kind).is_some_and(Model::has_highlight)
    }

    pub fn server_has_unread(&self, server: &Server) -> bool {
        self.models.iter().any(|(kind, model)| {
            if kind
                .as_server()
                .is_some_and(|model_server| *model_server == *server)
            {
                model.has_unread()
            } else {
                false
            }
        })
    }

    pub fn visible_server_queries(
        &self,
        server: &Server,
    ) -> Vec<&target::Query> {
        self.models
            .iter()
            .filter_map(|(kind, model)| {
                if let Kind::Query(query_server, query_target) = &kind
                    && query_server == server
                    && model.show_in_sidebar
                {
                    Some(query_target)
                } else {
                    None
                }
            })
            .sorted_by(Ord::cmp)
            .collect()
    }

    pub fn has_more_messages(&self, kind_ref: KindRef) -> bool {
        self.models
            .get(&kind_ref)
            .is_some_and(Model::has_more_messages)
    }

    #[must_use]
    pub fn update(
        &mut self,
        message: Message,
        config: &config::buffer::Condensation,
    ) -> Option<Event> {
        match message {
            Message::GoToMessage(navigation) => {
                Some(Event::GoToMessage(navigation))
            }
            Message::Update(kind, update) => {
                if let Some(model) = self.models.get_mut(&kind) {
                    model.update(kind, update, config)
                } else {
                    let mut model = Model::default();

                    let event = model.update(kind.clone(), update, config);

                    self.models.insert(kind, model);

                    event
                }
            }
            Message::Remove(kind) => {
                self.models.remove(&kind);

                None
            }
        }
    }

    pub fn expand_message(
        &mut self,
        kind: &Kind,
        history_id: &Id,
        time: &message::Time,
        config: &config::buffer::Condensation,
    ) {
        if let Some(model) = self.models.get_mut(kind) {
            model.expand_message(history_id, time, config);
        }
    }

    pub fn contract_message(
        &mut self,
        kind: &Kind,
        history_id: &Id,
        time: &message::Time,
        config: &config::buffer::Condensation,
    ) {
        if let Some(model) = self.models.get_mut(kind) {
            model.contract_message(history_id, time, config);
        }
    }

    pub fn generate_reply_preview(
        &self,
        kind: &Kind,
        id: &message::Id,
        time: &message::Time,
    ) -> Option<ReplyPreview> {
        self.models
            .get(kind)
            .and_then(|model| model.generate_reply_preview(id, time))
    }

    pub fn find_message_by_history_id(
        &self,
        history_id: &Id,
        kind_ref: KindRef,
        time: &message::Time,
    ) -> Option<&message::MessageDisplay> {
        self.models.get(&kind_ref).and_then(|model| {
            model.find_message_by_history_id(history_id, time)
        })
    }

    pub fn find_message_by_id(
        &self,
        id: &message::Id,
        kind_ref: KindRef,
        time: &message::Time,
    ) -> Option<&message::MessageDisplay> {
        self.models
            .get(&kind_ref)
            .and_then(|model| model.find_message_by_id(id, time))
    }
}

#[derive(Debug, Clone)]
pub struct Update {
    pub show_in_sidebar: bool,
    pub read_marker: Option<ReadMarker>,
    pub display_read_marker: Option<ReadMarker>,
    pub latest: Option<DateTime<Utc>>,
    pub latest_triggers_unread: Option<DateTime<Utc>>,
    pub latest_triggers_highlight: Option<DateTime<Utc>>,
    pub pane: Pane,
}

#[derive(Debug, Default, Clone)]
pub struct Model {
    show_in_sidebar: bool,
    read_marker: Option<ReadMarker>,
    display_read_marker: Option<ReadMarker>,
    latest: Option<DateTime<Utc>>,
    latest_triggers_unread: Option<DateTime<Utc>>,
    latest_triggers_highlight: Option<DateTime<Utc>>,
    pane: Pane,
}

#[derive(Debug, Default, Clone)]
pub enum Pane {
    Open {
        has_more_older_messages: bool,
        has_more_newer_messages: bool,
        messages: Vec<message::MessageDisplay>, // ordered by (time, history ID)
        limit: message::Limit,
        clear: Option<DateTime<Utc>>,
        loading: bool,
        read_marker: Option<ReadMarker>,
    },
    Loading,
    Migrating,
    #[default]
    Closed,
}

impl Model {
    fn view(
        &self,
        request_limit: &message::Limit,
        config: &Config,
    ) -> View<'_> {
        match &self.pane {
            Pane::Open {
                has_more_older_messages,
                has_more_newer_messages,
                messages,
                limit,
                clear,
                loading,
                read_marker,
            } => {
                let mut processed = process_messages(messages, config);

                let split_at =
                    self.display_read_marker.map_or(0, |display_read_marker| {
                        processed
                            .iter()
                            .rev()
                            .position(|message| {
                                *message.time() <= display_read_marker
                            })
                            .map_or_else(
                                || 0, // Backlog is before current, limited view of messages
                                |position| processed.len() - position,
                            )
                    });

                let new_messages = processed.split_off(split_at);

                View {
                    old_messages: processed,
                    new_messages,
                    has_more_older_messages: *has_more_older_messages,
                    has_more_newer_messages: *has_more_newer_messages,
                    loading: *loading
                        || request_limit != limit
                        || (matches!(limit, message::Limit::Backlog(_))
                            && *read_marker != self.display_read_marker),
                    migrating: false,
                    cleared: clear.is_some(),
                }
            }
            Pane::Loading | Pane::Migrating => View {
                loading: true,
                migrating: matches!(self.pane, Pane::Migrating),
                ..View::default()
            },
            Pane::Closed => View::default(),
        }
    }

    #[must_use]
    fn update(
        &mut self,
        kind: Kind,
        mut update: Update,
        config: &config::buffer::Condensation,
    ) -> Option<Event> {
        self.show_in_sidebar = update.show_in_sidebar;
        self.read_marker = update.read_marker;
        self.display_read_marker = update.display_read_marker;
        self.latest = update.latest;
        self.latest_triggers_unread = update.latest_triggers_unread;
        self.latest_triggers_highlight = update.latest_triggers_highlight;

        if let (
            Pane::Open { messages, .. },
            Pane::Open {
                messages: update_messages,
                ..
            },
        ) = (&self.pane, &mut update.pane)
        {
            let mut has_expanded_group = false;
            for message in messages {
                if message.expanded
                    && (message.inner.can_condense(config)
                        || message.inner.has_redaction())
                    && let Some(update_message) = find_message_mut_by_history_id(
                        update_messages,
                        message.history_id(),
                        message.time(),
                    )
                {
                    update_message.expanded = true;
                    has_expanded_group |=
                        update_message.inner.can_condense(config);
                }
            }
            // Extending an expanded group must also expand its new members.
            let mut index = 0;
            while has_expanded_group && index < update_messages.len() {
                if let Some(range) = message::MessageDisplay::condensation_range(
                    update_messages,
                    index,
                    config,
                ) {
                    let expanded = update_messages[range.clone()]
                        .iter()
                        .any(|message| !message.blocked && message.expanded);
                    if expanded {
                        for message in &mut update_messages[range.clone()] {
                            if !message.blocked {
                                message.expanded = true;
                            }
                        }
                    }
                    index = range.end;
                } else {
                    index += 1;
                }
            }
        }

        let event = if matches!(self.pane, Pane::Closed)
            && matches!(update.pane, Pane::Open { .. })
        {
            Some(Event::Opened(kind))
        } else {
            None
        };

        self.pane = update.pane;

        event
    }

    fn can_mark_as_read(&self) -> bool {
        // Read marker is prior to last known message
        if let Some(read_marker) = self.read_marker {
            self.latest.is_some_and(|latest| read_marker < latest)
        // Default state == unread if there's a message
        } else {
            self.latest.is_some()
        }
    }

    fn has_unread(&self) -> bool {
        // Read marker is prior to last known message which triggers unread
        if let Some(read_marker) = self.read_marker {
            self.latest_triggers_unread
                .is_some_and(|latest| read_marker < latest)
        // Default state == unread if there's a message that triggers unread
        } else {
            self.latest_triggers_unread.is_some()
        }
    }

    fn has_highlight(&self) -> bool {
        // Read marker is prior to last known message which triggers highlight
        if let Some(read_marker) = self.read_marker {
            self.latest_triggers_highlight
                .is_some_and(|latest| read_marker < latest)
        // Default state == highlight if there's a message that triggers highlight
        } else {
            self.latest_triggers_highlight.is_some()
        }
    }

    fn has_more_messages(&self) -> bool {
        match &self.pane {
            Pane::Open {
                has_more_older_messages,
                has_more_newer_messages,
                ..
            } => *has_more_older_messages || *has_more_newer_messages,
            Pane::Loading | Pane::Migrating | Pane::Closed => true,
        }
    }

    fn expand_message(
        &mut self,
        history_id: &Id,
        time: &message::Time,
        config: &config::buffer::Condensation,
    ) {
        self.get_expansion_messages(history_id, time, config)
            .iter_mut()
            .filter(|message| !message.blocked)
            .for_each(|message| {
                message.expanded = true;
            });
    }

    fn contract_message(
        &mut self,
        history_id: &Id,
        time: &message::Time,
        config: &config::buffer::Condensation,
    ) {
        self.get_expansion_messages(history_id, time, config)
            .iter_mut()
            .filter(|message| !message.blocked)
            .for_each(|message| {
                message.expanded = false;
            });
    }

    /// For a condensation-expansion, find the first message in the
    /// condensation, then return all messages in the condensation.  Or, for a
    /// redaction-expansion, return the redacted message.
    fn get_expansion_messages(
        &mut self,
        history_id: &Id,
        time: &message::Time,
        config: &config::buffer::Condensation,
    ) -> &mut [message::MessageDisplay] {
        let Pane::Open { messages, .. } = &mut self.pane else {
            return &mut [];
        };

        let Some(index) =
            position_message_by_history_id(messages, history_id, time)
        else {
            return &mut [];
        };
        if messages[index].inner.redaction.is_some() {
            return &mut messages[index..=index];
        }
        let Some(range) = message::MessageDisplay::condensation_range(
            messages, index, config,
        ) else {
            return &mut [];
        };
        &mut messages[range]
    }

    fn generate_reply_preview(
        &self,
        id: &message::Id,
        time: &message::Time,
    ) -> Option<ReplyPreview> {
        let Pane::Open { messages, .. } = &self.pane else {
            return None;
        };

        find_message_by_id(messages, id, time)
            .map(message::MessageDisplay::as_reply_preview)
    }

    fn find_message_by_history_id(
        &self,
        history_id: &Id,
        time: &message::Time,
    ) -> Option<&message::MessageDisplay> {
        let Pane::Open { messages, .. } = &self.pane else {
            return None;
        };

        find_message_by_history_id(messages, history_id, time)
    }

    fn find_message_by_id(
        &self,
        id: &message::Id,
        time: &message::Time,
    ) -> Option<&message::MessageDisplay> {
        let Pane::Open { messages, .. } = &self.pane else {
            return None;
        };

        find_message_by_id(messages, id, time)
    }
}

#[derive(Debug, Default)]
pub struct View<'a> {
    pub old_messages: Vec<&'a message::MessageDisplay>,
    pub new_messages: Vec<&'a message::MessageDisplay>,
    pub has_more_older_messages: bool,
    pub has_more_newer_messages: bool,
    pub loading: bool,
    pub migrating: bool,
    pub cleared: bool,
}

/// MessageDisplay processing that must happen at view-time (e.g. filtering
/// messages based on how long ago they were received) is done here.  All
/// other processing is done by Storage.
fn process_messages<'a>(
    messages: &'a [message::MessageDisplay],
    config: &Config,
) -> Vec<&'a message::MessageDisplay> {
    let now = Utc::now();
    messages
        .iter()
        .filter_map(|message| message.displayed(&config.buffer, now))
        .collect()
}

fn resolve_anchor(
    messages: &[message::MessageDisplay],
    id: &Id,
    time: &message::Time,
    config: &config::Buffer,
) -> Option<Id> {
    let now = Utc::now();
    let index = position_message_by_history_id(messages, id, time);
    if let Some(index) = index {
        if let Some(row) = messages[index].displayed(config, now) {
            return Some(*row.history_id());
        }
        if let Some(range) = message::MessageDisplay::condensation_range(
            messages,
            index,
            &config.server_messages.condense,
        ) && let Some(row) = messages[range.start].displayed(config, now)
        {
            return Some(*row.history_id());
        }
    }

    let next = index.map_or_else(
        || messages.partition_point(|message| {
            message.time() < time || (message.time() == time && matches!(
                (message.history_id(), id), (Id::Determined(candidate), Id::Determined(anchor)) if candidate < anchor
            ))
        }),
        |index| index + 1,
    );
    messages[next..]
        .iter()
        .filter_map(|message| message.displayed(config, now))
        .next()
        .or_else(|| {
            messages[..next]
                .iter()
                .rev()
                .find_map(|message| message.displayed(config, now))
        })
        .map(|message| *message.history_id())
}
