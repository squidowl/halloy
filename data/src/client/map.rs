use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context as ErrorContext, Result, anyhow};

use super::{CLIENT_CHATHISTORY_LIMIT, Client, Event, Topic};
use crate::buffer::{self, BuffersContext};
use crate::capabilities::{
    self, Capabilities, Capability, LabeledResponseContext, MultilineLimits,
};
use crate::config::server::filehost;
use crate::features::{self, Features};
use crate::history::{ReadMarker, storage};
use crate::isupport::{self, ChathistoryState, ChathistorySubcommand};
use crate::rate_limit::TokenPriority;
use crate::server::{self, Server};
use crate::target::{self, Target};
use crate::user::{ChannelUsers, Nick, User};
use crate::{channel_discovery, config, fileupload, message, metadata};

#[derive(Debug)]
pub enum State {
    Disconnected { autoconnect: bool, connecting: bool },
    Ready(Client),
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Unavailable,
    Connected,
    Disconnected,
}

impl Status {
    pub fn connected(&self) -> bool {
        matches!(self, Status::Connected)
    }
}

#[derive(Debug, Default)]
pub struct Map(BTreeMap<Server, State>);

impl Map {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn disconnected(&mut self, server: Server, autoconnect: bool) {
        self.0.insert(
            server,
            State::Disconnected {
                autoconnect,
                connecting: false,
            },
        );
    }

    pub fn connection_failed(&mut self, server: &Server, autoconnect: bool) {
        if let Some(State::Disconnected {
            autoconnect: server_autoconnect,
            connecting,
        }) = self.0.get_mut(server)
        {
            *server_autoconnect = autoconnect;
            *connecting = false;
        } else {
            self.disconnected(server.clone(), autoconnect);
        }
    }

    pub fn autoconnect_disabled(&mut self, server: &Server) {
        if let Some(State::Disconnected { autoconnect, .. }) =
            self.0.get_mut(server)
        {
            *autoconnect = false;
        }
    }

    pub fn connecting(&mut self, server: &Server) {
        if let Some(State::Disconnected { connecting, .. }) =
            self.0.get_mut(server)
        {
            *connecting = true;
        }
    }

    pub fn ready(&mut self, server: Server, client: Client) {
        self.0.insert(server, State::Ready(client));
    }

    pub fn update_config(
        &mut self,
        server: &Server,
        config: Arc<config::Server>,
        from_modal: bool,
    ) -> Vec<Event> {
        if let Some(State::Ready(client)) = self.0.get_mut(server) {
            client.update_config(config, from_modal)
        } else {
            vec![]
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn remove(&mut self, server: &Server) -> Option<Client> {
        self.0.remove(server).and_then(|state| match state {
            State::Disconnected { .. } => None,
            State::Ready(client) => Some(client),
        })
    }

    pub fn client(&self, server: &Server) -> Option<&Client> {
        if let Some(State::Ready(client)) = self.0.get(server) {
            Some(client)
        } else {
            None
        }
    }

    pub fn client_mut(&mut self, server: &Server) -> Option<&mut Client> {
        if let Some(State::Ready(client)) = self.0.get_mut(server) {
            Some(client)
        } else {
            None
        }
    }

    pub fn nickname<'a>(&'a self, server: &Server) -> Option<&'a Nick> {
        self.client(server).map(Client::nickname)
    }

    pub fn receive(
        &mut self,
        server: &Server,
        message: message::Encoded,
        history: &mut storage::Manager,
        buffers_context: &dyn BuffersContext,
        config: &config::Config,
    ) -> Result<Vec<Event>> {
        if let Some(client) = self.client_mut(server) {
            client.receive(message, history, buffers_context, config)
        } else {
            Ok(Vec::default())
        }
    }

    pub fn send(
        &mut self,
        buffer: &buffer::Upstream,
        message: message::Encoded,
        priority: TokenPriority,
    ) -> Option<LabeledResponseContext> {
        self.client_mut(buffer.server())
            .and_then(|client| client.send(Some(buffer), message, priority))
    }

    pub fn send_multiline_batch(
        &mut self,
        buffer: &buffer::Upstream,
        messages: Vec<message::Encoded>,
        priority: TokenPriority,
        reply_id: Option<&message::Id>,
    ) -> Option<LabeledResponseContext> {
        self.client_mut(buffer.server()).and_then(|client| {
            client.send_multiline_batch(buffer, messages, priority, reply_id)
        })
    }

    pub fn send_markread(
        &mut self,
        server: &Server,
        target: Target,
        read_marker: ReadMarker,
        priority: TokenPriority,
    ) {
        if let Some(client) = self.client_mut(server) {
            client.send_markread(target, read_marker, priority);
        }
    }

    pub fn join(&mut self, server: &Server, channels: &[target::Channel]) {
        if let Some(client) = self.client_mut(server) {
            client.join(channels);
        }
    }

    pub fn quit(&mut self, server: &Server, reason: Option<String>) {
        if let Some(client) = self.client_mut(server) {
            client.quit(reason);
        }
    }

    pub fn prioritize_who_poll(
        &mut self,
        server: &Server,
        channel: &target::Channel,
    ) {
        if let Some(client) = self.client_mut(server) {
            client.prioritize_who_poll(channel);
        }
    }

    pub fn deprioritize_who_poll(
        &mut self,
        server: &Server,
        channel: &target::Channel,
    ) {
        if let Some(client) = self.client_mut(server) {
            client.deprioritize_who_poll(channel);
        }
    }

    pub fn add_monitored_user_automated(
        &mut self,
        server: &Server,
        user: &User,
    ) {
        if let Some(client) = self.client_mut(server) {
            client.add_monitored_user_automated(user);
        }
    }

    pub fn resolve_user_attributes<'a>(
        &'a self,
        server: &Server,
        channel: &target::Channel,
        user: &User,
    ) -> Option<&'a User> {
        self.client(server)
            .and_then(|client| client.resolve_user_attributes(channel, user))
    }

    pub fn get_channel_discovery_manager(
        &self,
        server: &Server,
    ) -> Option<&channel_discovery::Manager> {
        self.client(server)
            .map(|client| &client.channel_discovery_manager)
    }

    pub fn get_channel_discovery_manager_mut(
        &mut self,
        server: &Server,
    ) -> Option<&mut channel_discovery::Manager> {
        self.client_mut(server)
            .map(|client| &mut client.channel_discovery_manager)
    }

    pub fn get_channel_users(
        &self,
        server: &Server,
        channel: &target::Channel,
    ) -> Option<&ChannelUsers> {
        self.client(server).and_then(|client| client.users(channel))
    }

    pub fn get_user_channels(
        &self,
        server: &Server,
        nick: &Nick,
    ) -> Vec<target::Channel> {
        self.client(server)
            .map(|client| client.user_channels(nick))
            .unwrap_or_default()
    }

    pub fn get_channel_topic<'a>(
        &'a self,
        server: &Server,
        channel: &target::Channel,
    ) -> Option<&'a Topic> {
        self.client(server)
            .map(|client| client.topic(channel))
            .unwrap_or_default()
    }

    pub fn get_channel_mode<'a>(
        &'a self,
        server: &Server,
        channel: &target::Channel,
    ) -> Option<&'a String> {
        self.client(server)
            .map(|client| client.mode(channel))
            .unwrap_or_default()
    }

    pub fn get_channels<'a>(
        &'a self,
        server: &Server,
    ) -> impl Iterator<Item = &'a target::Channel> {
        self.client(server)
            .map(Client::channels)
            .into_iter()
            .flatten()
    }

    pub fn contains_channel(
        &self,
        server: &Server,
        chan: &target::Channel,
    ) -> bool {
        self.client(server)
            .is_some_and(|c| c.chanmap.contains_key(chan))
    }

    pub fn resolve_query<'a>(
        &'a self,
        server: &Server,
        query: &target::Query,
    ) -> Option<&'a target::Query> {
        self.client(server)
            .and_then(|client| client.resolve_query(query))
    }

    pub fn get_isupport_ref(
        &self,
        server: &Server,
    ) -> &HashMap<isupport::Kind, isupport::Parameter> {
        self.client(server)
            .map_or(&isupport::DEFAULT, |client| &client.isupport)
    }

    pub fn get_capabilities_ref(&self, server: &Server) -> &Capabilities {
        self.client(server)
            .map_or(&capabilities::DEFAULT, |client| &client.capabilities)
    }

    pub fn get_filehost<'a>(&'a self, server: &Server) -> Option<&'a str> {
        let client = self.client(server)?;

        if !client.config.filehost.enabled {
            return None;
        }

        client.config.filehost.override_url.as_deref().or(
            if server.is_bouncer_network() {
                server.parent().as_ref().and_then(|p| self.get_filehost(p))
            } else {
                isupport::get_filehost(&client.isupport)
            },
        )
    }

    pub fn get_icon_url<'a>(&'a self, server: &Server) -> Option<&'a str> {
        self.client(server).and_then(|client| {
            if client.config.icon.enabled {
                client
                    .config
                    .icon
                    .override_url
                    .as_deref()
                    .or(isupport::get_icon_url(&client.isupport))
            } else {
                None
            }
        })
    }

    pub fn get_filehost_auth(
        &self,
        server: &Server,
    ) -> Option<fileupload::Auth> {
        let client = self.client(server)?;

        if !client.config.filehost.enabled {
            return None;
        }

        match &client.config.filehost.credentials {
            filehost::Credentials::Server => {
                if server.is_bouncer_network() {
                    return server
                        .parent()
                        .as_ref()
                        .and_then(|parent| self.get_filehost_auth(parent));
                }

                fileupload::Auth::from_sasl(
                    client.config.sasl.as_ref()?,
                    &client.config.nickname,
                )
                .ok()
            }
            filehost::Credentials::Sasl(credentials) => {
                fileupload::Auth::from_sasl(
                    credentials,
                    &client.config.nickname,
                )
                .ok()
            }
            filehost::Credentials::None => None,
        }
    }

    pub fn get_use_tls(&self, server: &Server) -> bool {
        self.client(server).is_none_or(|c| c.config.use_tls)
    }

    pub fn get_filehost_is_override(&self, server: &Server) -> bool {
        let Some(client) = self.client(server) else {
            return false;
        };

        if !client.config.filehost.enabled {
            return false;
        }

        if client.config.filehost.override_url.is_some() {
            return true;
        }

        if server.is_bouncer_network() {
            server
                .parent()
                .as_ref()
                .is_some_and(|p| self.get_filehost_is_override(p))
        } else {
            false
        }
    }

    pub fn get_features_ref(&self, server: &Server) -> &Features {
        self.client(server)
            .map_or(&features::DEFAULT, |client| &client.features)
    }

    pub fn get_server_chanmodes_or_default<'a>(
        &'a self,
        server: &Server,
    ) -> &'a [isupport::ModeKind] {
        self.client(server)
            .map(Client::chanmodes)
            .unwrap_or_default()
    }

    pub fn get_server_chantypes_or_default<'a>(
        &'a self,
        server: &Server,
    ) -> &'a [char] {
        self.get_maybe_server_chantypes_or_default(Some(server))
    }

    pub fn get_maybe_server_chantypes_or_default<'a>(
        &'a self,
        server: Option<&Server>,
    ) -> &'a [char] {
        server
            .and_then(|server| self.client(server).map(Client::chantypes))
            .unwrap_or(isupport::DEFAULT_CHANTYPES)
    }

    pub fn get_server_prefix_or_default<'a>(
        &'a self,
        server: &Server,
    ) -> &'a [isupport::PrefixMap] {
        self.client(server).map(Client::prefix).unwrap_or_default()
    }

    pub fn get_server_statusmsg_or_default<'a>(
        &'a self,
        server: &Server,
    ) -> &'a [char] {
        self.get_maybe_server_statusmsg_or_default(Some(server))
    }

    pub fn get_maybe_server_statusmsg_or_default<'a>(
        &'a self,
        server: Option<&Server>,
    ) -> &'a [char] {
        server
            .and_then(|server| self.client(server).map(Client::statusmsg))
            .unwrap_or(isupport::DEFAULT_STATUSMSG)
    }

    // The default value is chosen to be a reasonable, conservative
    // estimate when no client is available
    pub fn get_relay_bytes(&self, server: &Server) -> usize {
        self.client(server).map_or(144, Client::relay_bytes)
    }

    pub fn get_multiline_limits(
        &self,
        server: &Server,
    ) -> Option<MultilineLimits> {
        self.client(server).and_then(Client::multiline_limits)
    }

    pub fn get_server_supports_multiline(&self, server: &Server) -> bool {
        self.client(server).is_some_and(|client| {
            client.capabilities.acknowledged(Capability::Multiline)
        })
    }

    pub fn get_server_supports_echoes(&self, server: &Server) -> bool {
        self.client(server).is_some_and(|client| {
            client.capabilities.acknowledged(Capability::EchoMessage)
        })
    }

    /// The config used by this server connection.
    ///
    /// Unlike `config.servers`, this distinguishes networks on the same bouncer.
    pub fn get_server_config(
        &self,
        server: &Server,
    ) -> Option<&config::Server> {
        self.client(server).map(|client| client.config.as_ref())
    }

    pub fn get_server_supports_typing(&self, server: &Server) -> bool {
        self.client(server).is_some_and(Client::supports_typing)
    }

    pub fn get_server_can_send_reactions(&self, server: &Server) -> bool {
        self.client(server).is_some_and(Client::can_send_reactions)
    }

    pub fn get_server_can_send_unreactions(&self, server: &Server) -> bool {
        self.client(server)
            .is_some_and(Client::can_send_unreactions)
    }

    pub fn get_server_can_redact(&self, server: &Server) -> bool {
        self.client(server).is_some_and(Client::can_redact)
    }

    pub fn get_server_can_send_replies(&self, server: &Server) -> bool {
        self.client(server).is_some_and(Client::can_send_replies)
    }

    pub fn get_server_can_send_typing(&self, server: &Server) -> bool {
        self.client(server).is_some_and(Client::can_send_typing)
    }

    pub fn get_server_show_typing(&self, server: &Server) -> bool {
        self.client(server).is_some_and(Client::show_typing)
    }

    pub fn get_server_share_typing(&self, server: &Server) -> bool {
        self.client(server).is_some_and(Client::share_typing)
    }

    pub fn get_channel_typing_users(
        &self,
        server: &Server,
        channel: &target::Channel,
    ) -> Vec<String> {
        self.client(server)
            .map(|client| client.channel_typing_users(channel))
            .unwrap_or_default()
    }

    pub fn get_query_typing_users(
        &self,
        server: &Server,
        query: &target::Query,
    ) -> Vec<String> {
        self.client(server)
            .map(|client| client.query_typing_users(query))
            .unwrap_or_default()
    }

    pub fn has_channel_typing_users(
        &self,
        server: &Server,
        channel: &target::Channel,
    ) -> bool {
        self.client(server)
            .is_some_and(|client| client.has_channel_typing_users(channel))
    }

    pub fn has_query_typing_users(
        &self,
        server: &Server,
        query: &target::Query,
    ) -> bool {
        self.client(server)
            .is_some_and(|client| client.has_query_typing_users(query))
    }

    pub fn get_server_chathistory_message_reference_types(
        &self,
        server: &Server,
    ) -> Vec<isupport::MessageReferenceType> {
        self.client(server)
            .map(Client::chathistory_message_reference_types)
            .unwrap_or_default()
    }

    pub fn get_server_chathistory_limit(&self, server: &Server) -> u16 {
        self.client(server)
            .map_or(CLIENT_CHATHISTORY_LIMIT, |client| {
                client.chathistory_limit()
            })
    }

    pub fn get_server_supports_chathistory(&self, server: &Server) -> bool {
        self.client(server).is_some_and(|client| {
            client.capabilities.acknowledged(Capability::Chathistory)
        })
    }

    pub fn get_chathistory_request(
        &self,
        server: &Server,
        target: &Target,
    ) -> Option<ChathistorySubcommand> {
        self.client(server)
            .and_then(|client| client.chathistory_request(target))
    }

    pub fn send_chathistory_request(
        &mut self,
        server: &Server,
        subcommand: ChathistorySubcommand,
        priority: TokenPriority,
    ) {
        if let Some(client) = self.client_mut(server) {
            client.send_chathistory_request(subcommand, priority);
        }
    }

    pub fn clear_chathistory_request(
        &mut self,
        server: &Server,
        target: Option<&Target>,
    ) {
        if let Some(client) = self.client_mut(server) {
            client.clear_chathistory_request(target);
        }
    }

    pub fn get_chathistory_exhausted(
        &self,
        server: &Server,
        target: &Target,
    ) -> bool {
        self.client(server)
            .is_some_and(|client| client.chathistory_exhausted(target))
    }

    pub fn get_chathistory_state(
        &self,
        server: &Server,
        target: &Target,
    ) -> Option<ChathistoryState> {
        self.client(server).and_then(|client| {
            if client.capabilities.acknowledged(Capability::Chathistory) {
                if client.chathistory_request(target).is_some() {
                    Some(ChathistoryState::PendingRequest)
                } else if client.chathistory_exhausted(target) {
                    Some(ChathistoryState::Exhausted)
                } else {
                    Some(ChathistoryState::Ready)
                }
            } else {
                None
            }
        })
    }

    pub fn get_server_supports_detach(&self, server: &Server) -> bool {
        self.client(server)
            .is_some_and(|client| client.features.detach)
    }

    pub fn get_server_supports_list(&self, server: &Server) -> bool {
        self.client(server).is_some_and(Client::safelist)
    }

    pub fn get_server_is_connected(&self, server: &Server) -> bool {
        self.client(server).is_some()
    }

    pub fn get_server_http_client(
        &self,
        server: &Server,
    ) -> Option<Arc<reqwest::Client>> {
        self.client(server)
            .and_then(|client| client.http_client.clone())
    }

    pub fn get_server_proxy_config(
        &self,
        server: &Server,
    ) -> Option<&config::Proxy> {
        self.client(server)
            .as_ref()
            .and_then(|client| client.config.proxy.as_ref())
    }

    pub fn context_snapshot(&self) -> ClientsContextSnapshot {
        ClientsContextSnapshot {
            casemappings: self
                .0
                .iter()
                .filter_map(|(server, state)| {
                    if let State::Ready(client) = &state {
                        Some((server.clone(), client.casemapping()))
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    pub fn get_server_handle(
        &self,
        server: &Server,
    ) -> Option<&server::Handle> {
        self.client(server).map(|client| &client.handle)
    }

    pub fn connected_servers(&self) -> impl Iterator<Item = &Server> {
        self.0.iter().filter_map(|(server, state)| {
            if let State::Ready(_) = state {
                Some(server)
            } else {
                None
            }
        })
    }

    pub fn servers(&self) -> impl Iterator<Item = &Server> {
        self.0.keys()
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, Server, State> {
        self.0.iter()
    }

    pub fn status(&self, server: &Server) -> Status {
        self.0.get(server).map_or(Status::Unavailable, |s| match s {
            State::Disconnected { .. } => Status::Disconnected,
            State::Ready(_) => Status::Connected,
        })
    }

    pub fn state(&self, server: &Server) -> Option<&State> {
        self.0.get(server)
    }

    pub fn tick(&mut self, now: Instant) -> Result<()> {
        for client in self.0.values_mut() {
            if let State::Ready(client) = client {
                client.tick(now).with_context(|| {
                    anyhow!("[{}] tick failed", client.server)
                })?;
            }
        }
        Ok(())
    }

    pub fn get_registry(&self, server: &Server) -> &dyn metadata::Registry {
        self.0
            .get(server)
            .and_then::<&dyn metadata::Registry, _>(|state| match state {
                State::Disconnected { .. } => None,
                State::Ready(client) => Some(&client.registry),
            })
            .unwrap_or(metadata::EMPTY)
    }
}

impl ClientsContext for Map {
    fn get_maybe_server_casemapping_or_default(
        &self,
        server: Option<&Server>,
    ) -> isupport::CaseMap {
        server
            .and_then(|server| self.client(server).map(Client::casemapping))
            .unwrap_or_default()
    }
}

pub struct ClientsContextSnapshot {
    casemappings: HashMap<Server, isupport::CaseMap>,
}

impl ClientsContext for ClientsContextSnapshot {
    fn get_maybe_server_casemapping_or_default(
        &self,
        server: Option<&Server>,
    ) -> isupport::CaseMap {
        server
            .and_then(|server| self.casemappings.get(server))
            .copied()
            .unwrap_or_default()
    }
}

pub trait ClientsContext {
    fn get_server_casemapping_or_default(
        &self,
        server: &Server,
    ) -> isupport::CaseMap {
        self.get_maybe_server_casemapping_or_default(Some(server))
    }

    fn get_maybe_server_casemapping_or_default(
        &self,
        server: Option<&Server>,
    ) -> isupport::CaseMap;
}
