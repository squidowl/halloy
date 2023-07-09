use std::collections::{BTreeMap, HashMap};

use irc::client::Client;
use itertools::Itertools;

use crate::time::Posix;
use crate::user::{Nick, NickRef};
use crate::{message, Buffer, Message, Server, User};

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
pub enum Brodcast {
    Quit {
        user: User,
        comment: Option<String>,
        channels: Vec<String>,
    },
    Nickname {
        old_user: User,
        new_nick: Nick,
        ourself: bool,
        channels: Vec<String>,
    },
}

#[derive(Debug)]
pub enum Event {
    Single(Message),
    Brodcast(Brodcast),
    Whois(Message, Option<Buffer>),
}

#[derive(Debug)]
pub struct Connection {
    client: Client,
    resolved_nick: Option<String>,
    channels: Vec<String>,
    users: HashMap<String, Vec<User>>,
    labels: HashMap<String, Buffer>,
    batches: HashMap<String, Batch>,
}

impl Connection {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            resolved_nick: None,
            channels: vec![],
            users: HashMap::new(),
            labels: HashMap::new(),
            batches: HashMap::new(),
        }
    }

    pub async fn quit(self) {
        use std::time::Duration;

        use tokio::time;

        let _ = self.client.send_quit("");

        // Ensure message is sent before dropping
        time::sleep(Duration::from_secs(1)).await;
    }

    fn send(&mut self, buffer: &Buffer, mut message: message::Encoded) {
        // Add message label
        {
            use irc::proto::message::Tag;

            let label = generate_label();

            self.labels.insert(label.clone(), buffer.clone());

            message.tags = Some(vec![Tag("label".to_string(), Some(label))]);
        }

        if let Err(e) = self.client.send(message) {
            log::warn!("Error sending message: {e}");
        }
    }

    fn receive(&mut self, message: message::Encoded) -> Vec<Event> {
        log::trace!("Message received => {:?}", *message);

        self.handle(message).unwrap_or_default()
    }

    fn handle(&mut self, mut message: message::Encoded) -> Option<Vec<Event>> {
        use irc::proto::Command;
        use irc::proto::Response::*;

        let label_tag = remove_tag("label", message.tags.as_mut());
        let batch_tag = remove_tag("batch", message.tags.as_mut());

        let buffer = label_tag
            // Remove label if we get resp for it
            .and_then(|label| self.labels.remove(&label))
            .or_else(|| {
                batch_tag.as_ref().and_then(|batch| {
                    self.batches
                        .get(batch)
                        .and_then(|batch| batch.buffer.clone())
                })
            });

        match &message.command {
            Command::BATCH(batch, _, _) => {
                let mut chars = batch.chars();
                let symbol = chars.next()?;
                let reference = chars.collect::<String>();

                match symbol {
                    '+' => {
                        let batch = Batch::new(buffer);
                        self.batches.insert(reference, batch);
                    }
                    '-' => {
                        if let Some(finished) = self.batches.remove(&reference) {
                            // If nested, extend events into parent batch
                            if let Some(parent) = batch_tag
                                .as_ref()
                                .and_then(|batch| self.batches.get_mut(batch))
                            {
                                parent.events.extend(finished.events);
                            } else {
                                return Some(finished.events);
                            }
                        }
                    }
                    _ => {}
                }

                return None;
            }
            _ if batch_tag.is_some() => {
                let events = self.handle(message)?;

                if let Some(batch) = self.batches.get_mut(&batch_tag.unwrap()) {
                    batch.events.extend(events);
                    return None;
                } else {
                    return Some(events);
                }
            }
            Command::PRIVMSG(target, _) => {
                // If we sent (buffer exists) & we're target (echo), ignore
                if target == self.nickname().as_ref() && buffer.is_some() {
                    return None;
                }
            }
            Command::NICK(nick) => {
                let old_user = message.user()?;
                let ourself = self.nickname() == old_user.nickname();

                if ourself {
                    self.resolved_nick = Some(nick.clone());
                }

                let channels = self.user_channels(old_user.nickname());

                return Some(vec![Event::Brodcast(Brodcast::Nickname {
                    old_user,
                    new_nick: Nick::from(nick.as_str()),
                    ourself,
                    channels,
                })]);
            }
            Command::Response(RPL_WELCOME, args) => {
                if let Some(nick) = args.first() {
                    self.resolved_nick = Some(nick.to_string());
                }
            }
            Command::Response(
                RPL_WHOISCERTFP | RPL_WHOISCHANNELS | RPL_WHOISIDLE | RPL_WHOISKEYVALUE
                | RPL_WHOISOPERATOR | RPL_WHOISSERVER | RPL_WHOISUSER | RPL_ENDOFWHOIS,
                _,
            ) => {
                return Some(vec![Event::Whois(
                    Message::received(message, self.nickname())?,
                    buffer,
                )])
            }
            Command::QUIT(comment) => {
                let user = message.user()?;

                let channels = self.user_channels(user.nickname());

                return Some(vec![Event::Brodcast(Brodcast::Quit {
                    user,
                    comment: comment.clone(),
                    channels,
                })]);
            }
            _ => {}
        }

        Some(vec![Event::Single(Message::received(
            message,
            self.nickname(),
        )?)])
    }

    fn sync(&mut self) {
        self.channels = self
            .client
            .list_channels()
            .unwrap_or_default()
            .into_iter()
            .sorted()
            .collect();

        self.users = self
            .channels
            .iter()
            .map(|channel| {
                (
                    channel.clone(),
                    self.client
                        .list_users(channel)
                        .unwrap_or_default()
                        .into_iter()
                        .map(User::from)
                        .sorted()
                        .collect(),
                )
            })
            .collect();
    }

    pub fn channels(&self) -> &[String] {
        &self.channels
    }

    fn users<'a>(&'a self, channel: &str) -> &'a [User] {
        self.users
            .get(channel)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn user_channels(&self, nick: NickRef) -> Vec<String> {
        self.channels()
            .iter()
            .filter(|channel| {
                self.users(channel)
                    .iter()
                    .any(|user| user.nickname() == nick)
            })
            .cloned()
            .collect()
    }

    pub fn nickname(&self) -> NickRef {
        NickRef::from(
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

    pub fn nickname(&self, server: &Server) -> Option<NickRef> {
        self.connection(server).map(Connection::nickname)
    }

    pub fn receive(&mut self, server: &Server, message: message::Encoded) -> Vec<Event> {
        self.connection_mut(server)
            .map(|connection| connection.receive(message))
            .unwrap_or_default()
    }

    pub fn sync(&mut self, server: &Server) {
        if let Some(State::Ready(connection)) = self.0.get_mut(server) {
            connection.sync();
        }
    }

    pub fn send(&mut self, buffer: &Buffer, message: message::Encoded) {
        if let Some(connection) = self.connection_mut(buffer.server()) {
            connection.send(buffer, message);
        }
    }

    pub fn get_channel_users<'a>(&'a self, server: &Server, channel: &str) -> &'a [User] {
        self.connection(server)
            .map(|connection| connection.users(channel))
            .unwrap_or_default()
    }

    pub fn get_user_channels(&self, server: &Server, nick: NickRef) -> Vec<String> {
        self.connection(server)
            .map(|connection| connection.user_channels(nick))
            .unwrap_or_default()
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

#[derive(Debug)]
pub struct Batch {
    buffer: Option<Buffer>,
    events: Vec<Event>,
}

impl Batch {
    fn new(buffer: Option<Buffer>) -> Self {
        Self {
            buffer,
            events: vec![],
        }
    }
}

fn generate_label() -> String {
    Posix::now().as_nanos().to_string()
}

fn remove_tag(key: &str, tags: Option<&mut Vec<irc::proto::message::Tag>>) -> Option<String> {
    let tags = tags?;

    tags.remove(tags.iter().position(|tag| tag.0 == key)?).1
}
