use std::collections::BTreeMap;
use std::fmt;

use irc::client::Client;
use irc::proto::Command;

use crate::message;
use crate::message::Message;
use crate::server::Server;
use crate::user::User;

#[derive(Debug)]
pub enum State {
    Disconnected,
    Ready(Connection),
}

#[derive(Debug)]
pub struct Connection {
    client: Client,
    messages: Vec<Message>,
}

impl Connection {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            messages: vec![],
        }
    }

    fn send_privmsg(&mut self, target: String, text: impl fmt::Display) {
        let command = Command::PRIVMSG(target, text.to_string());

        let proto_message = irc::proto::Message::from(command);

        let message = Message::Sent {
            nickname: self.client.current_nickname().to_string(),
            message: proto_message.clone(),
        };

        // TODO: Handle error
        if let Err(e) = self.client.send(proto_message) {
            dbg!(&e);
        }

        self.messages.push(message);
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

    pub fn add_message(&mut self, server: &Server, message: Message) -> Option<message::Source> {
        if let Some(State::Ready(connection)) = self.0.get_mut(server) {
            let source = message.source(&connection.channels());

            connection.messages.push(message);

            source
        } else {
            None
        }
    }

    pub fn send_privmsg(&mut self, server: &Server, channel: &str, text: impl fmt::Display) {
        if let Some(connection) = self.connection_mut(server) {
            connection.send_privmsg(channel.to_string(), text);
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
                    .filter(|message| message.is_for_channel(channel))
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
                    .filter(|message| message.is_server_message(&connection.channels()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
