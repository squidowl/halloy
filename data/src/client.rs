use std::collections::{BTreeMap, HashMap};

use irc::client::Client;
use itertools::Itertools;

use crate::time::Posix;
use crate::user::{Nick, NickRef};
use crate::{message, Buffer, Server, User};

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
    Single(message::Encoded, Nick),
    WithTarget(message::Encoded, Nick, message::Target),
    Brodcast(Brodcast),
}

#[derive(Debug)]
pub struct Connection {
    client: Client,
    resolved_nick: Option<String>,
    channels: Vec<String>,
    users: HashMap<String, Vec<User>>,
    labels: HashMap<String, Context>,
    batches: HashMap<String, Batch>,
    reroute_responses_to: Option<Buffer>,
    supports_labels: bool,
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
            reroute_responses_to: None,
            supports_labels: false,
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
        if self.supports_labels {
            use irc::proto::message::Tag;

            let label = generate_label();
            let context = Context::new(&message, buffer.clone());

            self.labels.insert(label.clone(), context);

            message.tags = Some(vec![Tag("label".to_string(), Some(label))]);
        }

        self.reroute_responses_to = start_reroute(&message.command).then(|| buffer.clone());

        if let Err(e) = self.client.send(message) {
            log::warn!("Error sending message: {e}");
        }
    }

    fn receive(&mut self, message: message::Encoded) -> Vec<Event> {
        log::trace!("Message received => {:?}", *message);

        let stop_reroute = stop_reroute(&message.command);

        let events = self.handle(message, None).unwrap_or_default();

        if stop_reroute {
            self.reroute_responses_to = None;
        }

        events
    }

    fn handle(
        &mut self,
        mut message: message::Encoded,
        parent_context: Option<Context>,
    ) -> Option<Vec<Event>> {
        use irc::proto::Response::*;
        use irc::proto::{CapSubCommand, Command};

        let label_tag = remove_tag("label", message.tags.as_mut());
        let batch_tag = remove_tag("batch", message.tags.as_mut());

        let context = parent_context.or_else(|| {
            label_tag
                // Remove context associated to label if we get resp for it
                .and_then(|label| self.labels.remove(&label))
                // Otherwise if we're in a batch, get it's context
                .or_else(|| {
                    batch_tag.as_ref().and_then(|batch| {
                        self.batches
                            .get(batch)
                            .and_then(|batch| batch.context.clone())
                    })
                })
        });

        match &message.command {
            Command::BATCH(batch, _, _) => {
                let mut chars = batch.chars();
                let symbol = chars.next()?;
                let reference = chars.collect::<String>();

                match symbol {
                    '+' => {
                        let batch = Batch::new(context);
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
                let events = self.handle(message, context)?;

                if let Some(batch) = self.batches.get_mut(&batch_tag.unwrap()) {
                    batch.events.extend(events);
                    return None;
                } else {
                    return Some(events);
                }
            }
            // Label context whois
            _ if context.as_ref().map(Context::is_whois).unwrap_or_default() => {
                if let Some(source) = context
                    .map(Context::buffer)
                    .map(|buffer| buffer.server_message_target(None))
                {
                    return Some(vec![Event::WithTarget(
                        message,
                        self.nickname().to_owned(),
                        source,
                    )]);
                }
            }
            // Reroute responses
            Command::Response(..) | Command::Raw(..) if self.reroute_responses_to.is_some() => {
                if let Some(source) = self
                    .reroute_responses_to
                    .clone()
                    .map(|buffer| buffer.server_message_target(None))
                {
                    return Some(vec![Event::WithTarget(
                        message,
                        self.nickname().to_owned(),
                        source,
                    )]);
                }
            }
            Command::CAP(_, CapSubCommand::ACK, a, b) => {
                let cap_str = if b.is_none() { a.as_ref() } else { b.as_ref() }?;
                let caps = cap_str.split(' ').collect::<Vec<_>>();

                if caps.contains(&"labeled-response") {
                    self.supports_labels = true;
                }
            }
            Command::PRIVMSG(_, _) | Command::NOTICE(_, _) => {
                if let Some(user) = message.user() {
                    // If we sent (echo) & context exists (we sent from this client), ignore
                    if user.nickname() == self.nickname() && context.is_some() {
                        return None;
                    }
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
            // QUIT
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

        Some(vec![Event::Single(message, self.nickname().to_owned())])
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

    pub fn get_channels<'a>(&'a self, server: &Server) -> &'a [String] {
        self.connection(server)
            .map(|connection| connection.channels())
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

#[derive(Debug, Clone)]
pub enum Context {
    Buffer(Buffer),
    Whois(Buffer),
}

impl Context {
    fn new(message: &message::Encoded, buffer: Buffer) -> Self {
        use irc::proto::Command;

        if let Command::WHOIS(_, _) = message.command {
            Self::Whois(buffer)
        } else {
            Self::Buffer(buffer)
        }
    }

    fn is_whois(&self) -> bool {
        matches!(self, Self::Whois(_))
    }

    fn buffer(self) -> Buffer {
        match self {
            Context::Buffer(buffer) => buffer,
            Context::Whois(buffer) => buffer,
        }
    }
}

#[derive(Debug)]
pub struct Batch {
    context: Option<Context>,
    events: Vec<Event>,
}

impl Batch {
    fn new(context: Option<Context>) -> Self {
        Self {
            context,
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

fn start_reroute(command: &irc::proto::Command) -> bool {
    use irc::proto::Command;

    matches!(
        command,
        Command::WHO(..) | Command::WHOIS(..) | Command::WHOWAS(..)
    )
}

fn stop_reroute(command: &irc::proto::Command) -> bool {
    use irc::proto::Command;
    use irc::proto::Response::*;

    matches!(
        command,
        Command::Response(
            RPL_ENDOFWHO
                | RPL_ENDOFWHOIS
                | RPL_ENDOFWHOWAS
                | ERR_NOSUCHNICK
                | ERR_NOSUCHSERVER
                | ERR_NONICKNAMEGIVEN
                | ERR_WASNOSUCHNICK
                | ERR_NEEDMOREPARAMS,
            _
        )
    )
}
