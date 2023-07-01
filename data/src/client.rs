use std::collections::BTreeMap;

use irc::client::Client;

use crate::user::Nick;
use crate::{message, Message, Server, User};

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

    pub async fn quit(self) {
        use std::time::Duration;

        use tokio::time;

        let _ = self.client.send_quit("");

        // Ensure message is sent before dropping
        time::sleep(Duration::from_secs(1)).await;
    }

    fn send(&mut self, message: message::Encoded) {
        if let Err(e) = self.client.send(message) {
            log::warn!("Error sending message: {e}");
        }
    }

    fn receive(&mut self, message: message::Encoded) -> Option<Message> {
        log::trace!("Message received => {:?}", *message);

        self.handle(&message);

        Message::received(message, &self.nickname())
    }

    fn handle(&mut self, message: &message::Encoded) {
        use irc::proto::{Command, Response};

        match &message.command {
            Command::NICK(nick) => {
                let Some(old_nick) = message.prefix.as_ref().and_then(|prefix| match prefix {
                    irc::proto::Prefix::ServerName(_) => None,
                    irc::proto::Prefix::Nickname(nick, _, _) => Some(nick),
                }) else {
                    return;
                };

                if self.resolved_nick.as_ref() == Some(old_nick) {
                    self.resolved_nick = Some(nick.clone())
                }
            }
            Command::Response(Response::RPL_WELCOME, args) => {
                if let Some(nick) = args.first() {
                    self.resolved_nick = Some(nick.to_string());
                }
            }
            _ => {}
        }
    }

    pub fn channels(&self) -> Vec<String> {
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

    pub fn remove(&mut self, server: &Server) -> Option<Connection> {
        self.0.remove(server).and_then(|state| match state {
            State::Disconnected => None,
            State::Ready(connection) => Some(connection),
        })
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

    pub fn nickname(&self, server: &Server) -> Option<Nick> {
        self.connection(server).map(Connection::nickname)
    }

    pub fn receive(&mut self, server: &Server, message: message::Encoded) -> Option<Message> {
        self.connection_mut(server)
            .and_then(|connection| connection.receive(message))
    }

    pub fn send(&mut self, server: &Server, message: message::Encoded) {
        if let Some(connection) = self.connection_mut(server) {
            connection.send(message);
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
        self.0
            .iter()
            .filter_map(|(server, state)| match state {
                State::Disconnected => None,
                State::Ready(connection) => Some((server.clone(), connection.channels())),
            })
            .collect()
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<Server, State> {
        self.0.iter()
    }

    pub fn status(&self, server: &Server) -> Status {
        self.0
            .get(server)
            .map(|s| match s {
                State::Disconnected => Status::Disconnected,
                State::Ready(_) => Status::Connected,
            })
            .unwrap_or(Status::Unavailable)
    }
}
