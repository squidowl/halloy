use std::collections::BTreeMap;
use std::fmt;

use chrono::Utc;
use irc::client::Client;
use irc::proto;
use irc::proto::ChannelExt;

use crate::user::Nick;
use crate::{message, Command, Message, Server, User};

#[derive(Debug)]
pub enum State {
    Disconnected,
    Ready(Connection),
}

#[derive(Debug)]
pub struct Connection {
    client: Client,
    resolved_nick: Option<String>,
}

impl Connection {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            resolved_nick: None,
        }
    }

    fn send_channel_message(&mut self, channel: String, text: impl fmt::Display) -> Message {
        let text = text.to_string();
        let user = User::new(self.nickname(), None, None);

        let command = proto::Command::PRIVMSG(channel.clone(), text.clone());
        let proto_message = irc::proto::Message::from(command);
        // TODO: Handle error
        if let Err(e) = self.client.send(proto_message) {
            dbg!(&e);
        }

        Message {
            datetime: Utc::now(),
            direction: message::Direction::Sent,
            source: message::Source::Channel(channel, message::ChannelSender::User(user)),
            text,
        }
    }

    fn send_user_message(&mut self, nick: &Nick, text: impl fmt::Display) -> Message {
        let text = text.to_string();

        let target = nick.to_string();
        let command = proto::Command::PRIVMSG(target, text.clone());
        let proto_message = irc::proto::Message::from(command);
        // TODO: Handle error
        if let Err(e) = self.client.send(proto_message) {
            dbg!(&e);
        }

        Message {
            datetime: Utc::now(),
            direction: message::Direction::Sent,
            source: message::Source::Query(nick.clone(), User::new(self.nickname(), None, None)),
            text,
        }
    }

    fn send_command(&mut self, command: Command) -> Option<Message> {
        if let Command::Msg(target, message) = &command {
            if target.is_channel_name() {
                return Some(self.send_channel_message(target.clone(), message));
            } else if let Ok(user) = User::try_from(target.clone()) {
                return Some(self.send_user_message(&user.nickname(), message));
            }
        }

        let Ok(command) = proto::Command::try_from(command) else {
            return None;
        };
        let proto_message = irc::proto::Message::from(command);

        // TODO: Handle error
        if let Err(e) = self.client.send(proto_message) {
            dbg!(&e);
        }

        None
    }

    fn channels(&self) -> Vec<String> {
        self.client.list_channels().unwrap_or_default()
    }

    fn users(&self, channel: &str) -> Vec<User> {
        self.client
            .list_users(channel)
            .unwrap_or_default()
            .into_iter()
            .map(User::from)
            .collect()
    }

    pub fn nickname(&self) -> Nick {
        Nick::from(
            self.resolved_nick
                .as_deref()
                .unwrap_or_else(|| self.client.current_nickname()),
        )
    }

    pub fn handle_message(&mut self, message: &irc::proto::Message) {
        use irc::proto::{Command, Response};

        match &message.command {
            Command::NICK(nick) => self.resolved_nick = Some(nick.to_string()),
            Command::Response(response, args) => match response {
                Response::RPL_WELCOME => {
                    if let Some(nick) = args.first() {
                        self.resolved_nick = Some(nick.to_string());
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

#[derive(Debug, Default)]
pub struct Map(BTreeMap<Server, State>);

impl Map {
    pub fn disconnected(&mut self, server: Server) {
        self.0.insert(server, State::Disconnected);
    }

    pub fn ready(&mut self, server: Server, client: Connection) {
        self.0.insert(server, State::Ready(client));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn connection(&self, server: &Server) -> Option<&Connection> {
        if let Some(State::Ready(client)) = self.0.get(server) {
            Some(client)
        } else {
            None
        }
    }

    pub fn connection_mut(&mut self, server: &Server) -> Option<&mut Connection> {
        if let Some(State::Ready(client)) = self.0.get_mut(server) {
            Some(client)
        } else {
            None
        }
    }

    pub fn send_channel_message(
        &mut self,
        server: &Server,
        channel: &str,
        text: impl fmt::Display,
    ) -> Option<Message> {
        self.connection_mut(server)
            .map(|connection| connection.send_channel_message(channel.to_string(), text))
    }

    pub fn send_user_message(
        &mut self,
        server: &Server,
        nick: &Nick,
        text: impl fmt::Display,
    ) -> Option<Message> {
        self.connection_mut(server)
            .map(|connection| connection.send_user_message(nick, text))
    }

    pub fn send_command(&mut self, server: &Server, command: Command) -> Option<Message> {
        if let Some(connection) = self.connection_mut(server) {
            connection.send_command(command)
        } else {
            None
        }
    }

    pub fn get_channel_users(&self, server: &Server, channel: &str) -> Vec<User> {
        let mut users = self
            .connection(server)
            .map(|connection| connection.users(channel))
            .unwrap_or_default();
        users.sort();

        users
    }

    pub fn get_channels(&self) -> BTreeMap<Server, Vec<String>> {
        let mut servers = Vec::from_iter(self.0.iter());
        servers.sort_by(|(s1, _), (s2, _)| s2.name.cmp(&s1.name));

        let mut map = BTreeMap::new();

        for (server, _) in servers.into_iter() {
            let mut channels = self
                .connection(server)
                .map(|connection| connection.channels())
                .unwrap_or_default();
            channels.sort();

            map.insert(server.clone(), channels);
        }

        map
    }
}
