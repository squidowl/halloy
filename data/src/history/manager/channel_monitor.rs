use std::collections::HashMap;

use futures::FutureExt;
use futures::future::BoxFuture;

use super::{Data, Event, FilterChain, History, Manager, Message};
use crate::capabilities::LabeledResponseContext;
use crate::{client, config, history, message};

mod load;

#[derive(Debug, Default)]
pub(super) struct ChannelMonitor {
    /// Used to ignore old history loads.
    generation: u64,
    active: bool,
}

impl ChannelMonitor {
    pub(super) fn open(
        &mut self,
        data: &Data,
        clients: &client::Map,
        config: &config::ChannelMonitor,
    ) -> BoxFuture<'static, Message> {
        let history = load::all(data, clients, config);

        self.generation = self.generation.wrapping_add(1);
        self.active = true;

        task(self.generation, history)
    }

    pub(super) fn close(
        &mut self,
        data: &mut Data,
    ) -> Option<BoxFuture<'static, Result<(), history::Error>>> {
        self.active = false;
        data.map
            .get_mut(&history::Kind::ChannelMonitor)
            .and_then(History::make_partial)
    }

    pub(super) fn reload(
        &mut self,
        data: &mut Data,
        clients: &client::Map,
        config: &config::ChannelMonitor,
    ) -> Option<BoxFuture<'static, Message>> {
        if !self.active {
            return None;
        }

        data.map.remove(&history::Kind::ChannelMonitor);
        Some(self.open(data, clients, config))
    }

    pub(super) fn clear(&mut self, data: &mut Data) {
        self.invalidate();
        data.map.insert(
            history::Kind::ChannelMonitor,
            History::Full {
                kind: history::Kind::ChannelMonitor,
                messages: vec![],
                last_updated_at: None,
                read_marker: None,
                display_read_marker: None,
                chathistory_references: None,
                last_seen: HashMap::default(),
                cleared: true,
                last_flushed_at: 0,
            },
        );
    }

    pub(super) fn load_channel(
        &mut self,
        data: &Data,
        server: &crate::Server,
        channel: &crate::target::Channel,
        clients: &client::Map,
        config: &config::ChannelMonitor,
    ) -> Option<BoxFuture<'static, Message>> {
        if !self.active {
            return None;
        }

        let kind = history::Kind::Channel(server.clone(), channel.clone());
        let history = load::channel(data, &kind, clients, config)?;
        Some(task(self.generation, history))
    }

    pub(super) fn finish_load(
        &self,
        generation: u64,
        loaded: history::Loaded,
        data: &mut Data,
        filter_chain: FilterChain,
        clients: &client::Map,
        buffer_config: &config::Buffer,
    ) -> Option<Event> {
        if !self.accepts(generation) {
            return None;
        }

        let len = loaded.messages.len();
        let initialized = !matches!(
            data.map.get(&history::Kind::ChannelMonitor),
            Some(History::Full { .. })
        );

        apply(data, loaded, filter_chain, clients, buffer_config);
        log::debug!("loaded channel monitor history: {len} messages");

        initialized.then_some(Event::Loaded(history::Kind::ChannelMonitor))
    }

    pub(super) fn record(
        &self,
        data: &mut Data,
        filters: &[super::Filter],
        server: &crate::Server,
        casemapping: crate::isupport::CaseMap,
        message: &crate::Message,
        labeled_response_context: Option<&LabeledResponseContext>,
        config: &config::ChannelMonitor,
    ) -> Option<BoxFuture<'static, Message>> {
        if !is_channel_message(message) {
            return None;
        }

        let crate::message::Target::Channel { channel } = &message.target
        else {
            return None;
        };

        if !config.is_channel_included(server, channel, casemapping) {
            return None;
        }
        let mut message = crate::Message {
            target: crate::message::Target::ChannelMonitor {
                server: server.clone(),
                channel: channel.clone(),
            },
            ..message.clone()
        };

        FilterChain::borrow(filters).filter_message_of_kind(
            &mut message,
            &history::Kind::ChannelMonitor,
        );
        let task = data.add_message(
            history::Kind::ChannelMonitor,
            message,
            labeled_response_context.cloned(),
        );
        truncate(data);

        task.map(FutureExt::boxed)
    }

    pub(super) fn redact(
        &self,
        data: &mut Data,
        server: &crate::Server,
        channel: &crate::target::Channel,
        id: &crate::message::Id,
        redaction: &crate::redaction::Redaction,
        server_time: chrono::DateTime<chrono::Utc>,
        display_redacted: bool,
    ) {
        if !self.is_active() {
            return;
        }

        let Some(history) = data.map.get_mut(&history::Kind::ChannelMonitor)
        else {
            return;
        };

        let History::Full {
            messages,
            last_updated_at,
            ..
        } = history
        else {
            history.redact_message(
                id.clone(),
                redaction.clone(),
                server_time,
                display_redacted,
            );
            return;
        };

        let matches_origin = |message: &crate::Message| {
            matches!(
                &message.target,
                crate::message::Target::ChannelMonitor {
                    server: message_server,
                    channel: message_channel,
                    ..
                } if message_server == server && message_channel == channel
            )
        };
        let position =
            history::position_message_by_id(messages, id, &server_time)
                .filter(|position| matches_origin(&messages[*position]))
                .or_else(|| {
                    messages.iter().rposition(|message| {
                        message.id.as_deref() == Some(&**id)
                            && matches_origin(message)
                    })
                });
        let Some(position) = position else {
            return;
        };

        messages[position].redaction = Some(redaction.clone());
        if !display_redacted {
            messages[position].blocked = true;
        }

        let updated_reply = messages[position].as_reply_preview();
        for message in messages.iter_mut().filter(|message| {
            matches_origin(message)
                && message.reply_to.as_deref() == Some(&**id)
        }) {
            message.reply_preview = Some(updated_reply.clone());
        }

        *last_updated_at = Some(tokio::time::Instant::now());
    }

    fn invalidate(&mut self) {
        if self.is_active() {
            self.generation = self.generation.wrapping_add(1);
        }
    }

    fn accepts(&self, generation: u64) -> bool {
        self.is_active() && self.generation == generation
    }

    pub(super) fn is_active(&self) -> bool {
        self.active
    }
}

fn task(
    generation: u64,
    history: BoxFuture<'static, history::Loaded>,
) -> BoxFuture<'static, Message> {
    history
        .map(move |result| Message::LoadChannelMonitor(generation, result))
        .boxed()
}

fn is_channel_message(message: &Message) -> bool {
    matches!(&message.target, message::Target::Channel { .. })
        && matches!(
            &message.source,
            message::Source::User(_) | message::Source::Action(_)
        )
}

fn apply(
    data: &mut Data,
    loaded: history::Loaded,
    filter_chain: FilterChain,
    clients: &client::Map,
    buffer_config: &config::Buffer,
) {
    let kind = history::Kind::ChannelMonitor;

    let Some(History::Full {
        messages,
        last_seen,
        last_flushed_at,
        ..
    }) = data.map.get_mut(&kind)
    else {
        data.load_full(kind, loaded, filter_chain, clients, buffer_config);
        truncate(data);
        return;
    };

    for message in loaded.messages {
        history::insert_message(messages, message, None);
    }

    history::truncate_messages(messages);
    Manager::process_messages(
        &kind,
        messages,
        filter_chain,
        clients,
        buffer_config,
    );

    *last_seen = history::get_last_seen(messages);
    *last_flushed_at = messages.len();
}

fn truncate(data: &mut Data) {
    if let Some(History::Full { messages, .. }) =
        data.map.get_mut(&history::Kind::ChannelMonitor)
    {
        history::truncate_messages(messages);
    }
}
