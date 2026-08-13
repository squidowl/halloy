use std::collections::HashMap;

use chrono::{DateTime, Utc};
use itertools::Itertools;

use super::metadata::ReadMarker;
use super::{
    Id, Kind, find_message_by_id, find_message_mut_by_history_id,
    smart_filter_internal_message,
};
use crate::message::{self, ReplyPreview, Searchable, Temporal};
use crate::{Config, Server, config, target};

#[derive(Debug, Clone)]
pub enum Message {
    Update(Kind, Update),
    Remove(Kind),
}

#[derive(Debug)]
pub struct Manager {
    models: HashMap<Kind, Model>,
}

impl Manager {
    pub fn view(
        &self,
        kind: &Kind,
        request_limit: &message::Limit,
        config: &Config,
    ) -> Option<View<'_>> {
        self.models
            .get(kind)
            .map(|model| model.view(request_limit, config))
    }

    pub fn can_mark_read(&self, kind: &Kind) -> bool {
        self.models.get(kind).is_some_and(Model::can_mark_read)
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

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update(kind, update) => {
                if let Some(model) = self.models.get_mut(&kind) {
                    model.update(update);
                } else {
                    self.models.insert(kind, Model::default().with(update));
                }
            }
            Message::Remove(kind) => {
                self.models.remove(&kind);
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
        messages: Vec<message::MessageDisplay>, // sorted by MessageDisplay.message.server_time
        limit: message::Limit,
        clear: Option<DateTime<Utc>>,
    },
    Loading,
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
            } => {
                let processed = process_messages(messages, config);

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

                let (old, new) = processed.split_at(split_at);

                View {
                    old_messages: old.to_vec(),
                    new_messages: new.to_vec(),
                    has_more_older_messages: *has_more_older_messages,
                    has_more_newer_messages: *has_more_newer_messages,
                    loading: request_limit != limit,
                    cleared: clear.is_some(),
                }
            }
            Pane::Loading => View {
                loading: true,
                ..View::default()
            },
            Pane::Closed => View::default(),
        }
    }

    fn update(&mut self, mut update: Update) {
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
            for message in messages {
                if (message.condensed.is_some()
                    || message.inner.has_redaction())
                    && let Some(update_message) = find_message_mut_by_history_id(
                        update_messages,
                        message.history_id(),
                        message.time(),
                    )
                {
                    update_message.expanded = message.expanded;
                }
            }
        }

        self.pane = update.pane;
    }

    fn with(mut self, update: Update) -> Self {
        self.update(update);
        self
    }

    fn can_mark_read(&self) -> bool {
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
            self.latest_triggers_unread.is_some()
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
    ) -> Vec<&mut message::MessageDisplay> {
        let Pane::Open { messages, .. } = &mut self.pane else {
            return vec![];
        };

        let fuzz_seconds = chrono::Duration::seconds(1);

        let start = time.utc - fuzz_seconds;
        let end = time.utc + fuzz_seconds;

        let start_index = match messages
            .binary_search_by(|stored| stored.time().utc.cmp(&start))
        {
            Ok(match_index) => match_index,
            Err(sorted_insert_index) => sorted_insert_index,
        };
        let end_index = match messages
            .binary_search_by(|stored| stored.time().utc.cmp(&end))
        {
            Ok(match_index) => match_index,
            Err(sorted_insert_index) => sorted_insert_index,
        };

        if let Some(index) = messages[start_index..end_index]
            .iter()
            .enumerate()
            .find_map(|(slice_index, message)| {
                (message.history_id() == history_id)
                    .then_some(start_index + slice_index)
            })
        {
            if messages[index].inner.redaction.is_some() {
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
                            (message.inner.can_condense(config)
                                && message.condensed.is_none())
                            .then_some(message)
                        }
                    })
                    .collect();
            }
        }

        vec![]
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
}

#[derive(Debug, Default)]
pub struct View<'a> {
    pub old_messages: Vec<&'a message::MessageDisplay>,
    pub new_messages: Vec<&'a message::MessageDisplay>,
    pub has_more_older_messages: bool,
    pub has_more_newer_messages: bool,
    pub loading: bool,
    pub cleared: bool,
}

/// MessageDisplay processing that must happen at view-time (e.g. filtering
/// messages based on how long ago they were received) is done here.  All
/// other processing is done by Storage.
fn process_messages<'a>(
    messages: &'a [message::MessageDisplay],
    config: &Config,
) -> Vec<&'a message::MessageDisplay> {
    let current_time = Utc::now();

    messages
        .iter()
        .flat_map(|message| {
            if message.blocked {
                None
            } else if message
                .inner
                .can_condense(&config.buffer.server_messages.condense)
            {
                if message.expanded {
                    Some(message)
                } else {
                    message.condensed.as_ref().and_then(|condensed_message| {
                        (!condensed_message.inner.text().is_empty())
                            .then_some(condensed_message.as_ref())
                    })
                }
            } else {
                match &message.inner.source {
                    message::Source::Internal(
                        message::source::Internal::Status(status),
                    ) => {
                        if !config.buffer.internal_messages.enabled(status) {
                            return None;
                        } else if let Some(seconds) =
                            config.buffer.internal_messages.smart(status)
                            && smart_filter_internal_message(
                                &message.inner,
                                &seconds,
                                &current_time,
                            )
                        {
                            return None;
                        }

                        Some(message)
                    }
                    _ => Some(message),
                }
            }
        })
        .collect::<Vec<_>>()
}
