use std::collections::BTreeMap;
use std::fmt;

use irc::client::Client;
use irc::proto;

use crate::{message, Command, Message, Server, User};

#[derive(Debug)]
pub enum State {
    Disconnected,
    Ready(Connection),
}

#[derive(Debug)]
pub struct Connection {
    client: Client,
    messages: Vec<Message>,
    // TODO: Is there a better way to handle this?
    nick_change: Option<String>,
}

impl Connection {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            messages: vec![],
            nick_change: None,
        }
    }

    fn send_channel_message(&mut self, channel: String, text: impl fmt::Display) {
        let text = text.to_string();

        let command = proto::Command::PRIVMSG(channel.clone(), text.clone());
        let proto_message = irc::proto::Message::from(command);
        // TODO: Handle error
        if let Err(e) = self.client.send(proto_message) {
            dbg!(&e);
        }

        self.messages.push(Message {
            timestamp: 0,
            direction: message::Direction::Sent,
            source: message::Source::Channel(channel, User::new(self.nickname(), None, None)),
            text,
        });
    }

    fn send_user_message(&mut self, user: User, text: impl fmt::Display) {
        let text = text.to_string();

        let target = user
            .hostname()
            .unwrap_or_else(|| user.nickname())
            .to_string();
        let command = proto::Command::PRIVMSG(target, text.clone());
        let proto_message = irc::proto::Message::from(command);
        // TODO: Handle error
        if let Err(e) = self.client.send(proto_message) {
            dbg!(&e);
        }

        self.messages.push(Message {
            timestamp: 0,
            direction: message::Direction::Sent,
            source: message::Source::Private(user),
            text,
        });
    }

    fn send_command(&mut self, command: Command) {
        self.handle_command(&command);

        let Ok(command) = proto::Command::try_from(command) else {
            return;
        };
        let proto_message = irc::proto::Message::from(command);

        // TODO: Handle error
        if let Err(e) = self.client.send(proto_message) {
            dbg!(&e);
        }
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

    fn nickname(&self) -> &str {
        self.nick_change
            .as_deref()
            .unwrap_or_else(|| self.client.current_nickname())
    }

    fn handle_command(&mut self, command: &Command) {
        match command {
            Command::Nick(nick) => {
                self.nick_change = Some(nick.clone());
            }
            Command::Join(..) | Command::Motd(..) | Command::Quit(..) | Command::Unknown(..) => {}
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

    fn connection(&self, server: &Server) -> Option<&Connection> {
        if let Some(State::Ready(client)) = self.0.get(server) {
            Some(client)
        } else {
            None
        }
    }

    fn connection_mut(&mut self, server: &Server) -> Option<&mut Connection> {
        if let Some(State::Ready(client)) = self.0.get_mut(server) {
            Some(client)
        } else {
            None
        }
    }

    pub fn add_message(
        &mut self,
        server: &Server,
        message: irc::proto::Message,
    ) -> Option<message::Source> {
        if let Some(State::Ready(connection)) = self.0.get_mut(server) {
            if let Some(message) = Message::received(message) {
                let source = message.source.clone();

                connection.messages.push(message);

                return Some(source);
            }
        }

        None
    }

    pub fn send_privmsg(&mut self, server: &Server, channel: &str, text: impl fmt::Display) {
        if let Some(connection) = self.connection_mut(server) {
            connection.send_channel_message(channel.to_string(), text);
        }
    }

    pub fn send_command(&mut self, server: &Server, command: Command) {
        if let Some(connection) = self.connection_mut(server) {
            connection.send_command(command);
        }
    }

    pub fn get_channel_users(&self, server: &Server, channel: &str) -> Vec<User> {
        self.connection(server)
            .map(|connection| connection.users(channel))
            .unwrap_or_default()
    }

    pub fn get_channels(&self) -> BTreeMap<Server, Vec<String>> {
        let mut map = BTreeMap::new();

        for (server, _) in self.0.iter() {
            map.insert(
                server.clone(),
                self.connection(server)
                    .map(|connection| connection.channels())
                    .unwrap_or_default(),
            );
        }

        map
    }

    pub fn get_channel_messages(&self, server: &Server, channel: &str) -> Vec<&Message> {
        self.connection(server)
            .map(|connection| {
                connection
                    .messages
                    .iter()
                    .filter(|message| message.channel() == Some(channel))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_server_messages(&self, server: &Server) -> Vec<&Message> {
        self.connection(server)
            .map(|connection| {
                connection
                    .messages
                    .iter()
                    .filter(|message| message.is_server())
                    .collect()
            })
            .unwrap_or_default()
    }
}
