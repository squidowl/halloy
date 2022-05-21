use std::{collections::HashMap, fmt};

use crate::message::Command;
use crate::{
    message::{Channel, Message, MsgTarget},
    server::Server,
};

#[derive(Debug)]
pub enum State {
    Disconnected,
    Ready(Connection),
}

#[derive(Debug)]
pub struct Connection {
    client: ClientTwo,
    messages: Vec<Message>,
}

impl Connection {
    pub fn new(client: ClientTwo) -> Self {
        Self {
            client,
            messages: vec![],
        }
    }
}

#[derive(Debug)]
pub struct ClientTwo(irc::client::Client);

impl ClientTwo {
    pub fn sender(&self) -> Sender {
        let a = self.0.sender();
        a.into()
    }
}

impl From<irc::client::Client> for ClientTwo {
    fn from(client: irc::client::Client) -> Self {
        Self(client)
    }
}

#[derive(Debug, Clone)]
pub struct Sender(irc::client::Sender);

impl Sender {
    pub fn send_privmsg(
        &self,
        nickname: Option<String>,
        msg_target: MsgTarget,
        text: impl fmt::Display,
    ) -> Message {
        let command = Command::PrivMsg {
            msg_target,
            text: text.to_string(),
        };

        let proto_command = irc::proto::Command::from(command.clone());
        let proto_message = irc::proto::Message::from(proto_command);

        let mut message = Message {
            raw: proto_message.clone(),
            command,
        };

        // TODO: Handle error
        if let Err(e) = self.0.send(proto_message) {
            dbg!(&e);
        }

        if let Some(nick) = nickname {
            message.raw.prefix = Some(irc::proto::Prefix::Nickname(
                nick,
                String::new(),
                String::new(),
            ));
        }

        message
    }
}

impl From<irc::client::Sender> for Sender {
    fn from(sender: irc::client::Sender) -> Self {
        Self(sender)
    }
}

#[derive(Debug, Default)]
pub struct Map(HashMap<Server, State>);

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

    pub fn add_message(&mut self, server: &Server, message: Message) {
        println!("{:?}", message.command());
        if let Some(State::Ready(client)) = self.0.get_mut(server) {
            /* match message.command() {
                Command::Nick(nickname) => client.nickname = Some(nickname.clone()),
                _ => {}
            } */

            client.messages.push(message);
        }
    }

    pub fn send_message(&mut self, server: &Server, channel: &Channel, text: impl fmt::Display) {
        if let Some(connection) = self.connection_mut(server) {
            let message = connection.client.sender().send_privmsg(
                None,
                MsgTarget::Channel(channel.clone()),
                text,
            );
            self.add_message(server, message);
        }
    }

    pub fn get_messages(&self, server: &Server, channel: &Channel) -> Vec<&Message> {
        self.connection(server)
            .map(|client| {
                client
                    .messages
                    .iter()
                    .filter(|m| m.is_for_channel(channel))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_messages_for_server(&self) -> Vec<&Message> {
        let mut messages: Vec<&Message> = vec![];

        for server in self.0.keys() {
            let client = self.connection(server);
            messages.append(
                &mut client
                    .map(|client| {
                        client
                            .messages
                            .iter()
                            .filter(|m| m.is_for_server())
                            .collect()
                    })
                    .unwrap_or_default(),
            );
        }

        messages
    }
}
