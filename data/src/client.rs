use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::time::{Duration, Instant};

use futures::channel::mpsc;
use irc::proto::{self, command, Command};
use itertools::Itertools;

use crate::time::Posix;
use crate::user::{Nick, NickRef};
use crate::{config, message, mode, Buffer, Server, User};

const WHO_POLL_INTERVAL: Duration = Duration::from_secs(60);
const WHO_RETRY_INTERVAL: Duration = Duration::from_secs(10);

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
    Ready(Client),
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

pub struct Client {
    server: Server,
    config: config::Server,
    sender: mpsc::Sender<proto::Message>,
    alt_nick: Option<usize>,
    resolved_nick: Option<String>,
    chanmap: BTreeMap<String, Channel>,
    channels: Vec<String>,
    users: HashMap<String, Vec<User>>,
    labels: HashMap<String, Context>,
    batches: HashMap<String, Batch>,
    reroute_responses_to: Option<Buffer>,
    registration_step: RegistrationStep,
    listed_caps: Vec<String>,
    supports_labels: bool,
    supports_away_notify: bool,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish()
    }
}

impl Client {
    pub fn new(
        server: Server,
        config: config::Server,
        mut sender: mpsc::Sender<proto::Message>,
    ) -> Self {
        // Begin registration
        let _ = sender.try_send(command!("CAP", "LS", "302"));
        let registration_step = RegistrationStep::List;

        // Identify
        {
            let nick = &config.nickname;
            let user = config.username.as_ref().unwrap_or(nick);
            let real = config.realname.as_ref().unwrap_or(nick);

            if let Some(pass) = config.password.as_ref() {
                let _ = sender.try_send(command!("PASS", pass));
            }
            let _ = sender.try_send(command!("NICK", nick));
            let _ = sender.try_send(command!("USER", user, real));
        }

        Self {
            server,
            config,
            sender,
            resolved_nick: None,
            alt_nick: None,
            chanmap: BTreeMap::default(),
            channels: vec![],
            users: HashMap::new(),
            labels: HashMap::new(),
            batches: HashMap::new(),
            reroute_responses_to: None,
            registration_step,
            listed_caps: vec![],
            supports_labels: false,
            supports_away_notify: false,
        }
    }

    pub async fn quit(mut self) {
        use tokio::time;

        let _ = self.sender.try_send(command!("QUIT"));

        // Ensure message is sent before dropping
        time::sleep(Duration::from_secs(1)).await;
    }

    fn send(&mut self, buffer: &Buffer, mut message: message::Encoded) {
        if self.supports_labels {
            use proto::Tag;

            let label = generate_label();
            let context = Context::new(&message, buffer.clone());

            self.labels.insert(label.clone(), context);

            // IRC: Encode tags
            message.tags = vec![Tag {
                key: "label".to_string(),
                value: Some(label),
            }];
        }

        self.reroute_responses_to = start_reroute(&message.command).then(|| buffer.clone());

        if let Err(e) = self.sender.try_send(message.into()) {
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
        use irc::proto::command::Numeric::*;

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
            Command::BATCH(batch, ..) => {
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
            Command::Numeric(..) | Command::Unknown(..) if self.reroute_responses_to.is_some() => {
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
            Command::CAP(_, sub, a, b) if sub == "LS" => {
                let (caps, asterisk) = match (a, b) {
                    (Some(caps), None) => (caps, None),
                    (Some(asterisk), Some(caps)) => (caps, Some(asterisk)),
                    // Unreachable
                    (None, None) | (None, Some(_)) => return None,
                };

                self.listed_caps.extend(caps.split(' ').map(String::from));

                // Finished
                if asterisk.is_none() {
                    let mut requested = vec![];

                    let contains = |s| self.listed_caps.iter().any(|cap| cap == s);

                    if contains("away-notify") {
                        requested.push("away-notify");
                    }
                    if contains("server-time") {
                        requested.push("server-time");
                    }
                    if contains("batch") {
                        requested.push("batch");
                    }
                    if contains("labeled-response") {
                        requested.push("labeled-response");

                        // We require labeled-response so we can properly tag echo-messages
                        if contains("echo-message") {
                            requested.push("echo-message");
                        }
                    }
                    if self.listed_caps.iter().any(|cap| cap.starts_with("sasl")) {
                        requested.push("sasl");
                    }

                    if !requested.is_empty() {
                        // Request
                        self.registration_step = RegistrationStep::Req;
                        let _ = self
                            .sender
                            .try_send(command!("CAP", "REQ", requested.join(" ")));
                    } else {
                        // If none requested, end negotiation
                        self.registration_step = RegistrationStep::End;
                        let _ = self.sender.try_send(command!("CAP", "END"));
                    }
                }
            }
            Command::CAP(_, sub, a, b) if sub == "ACK" => {
                let caps = if b.is_none() { a.as_ref() } else { b.as_ref() }?;
                log::info!("[{}] capabilities acknowledged: {caps}", self.server);

                let caps = caps.split(' ').collect::<Vec<_>>();

                if caps.contains(&"labeled-response") {
                    self.supports_labels = true;
                }
                if caps.contains(&"away-notify") {
                    self.supports_away_notify = true;
                }

                let supports_sasl = caps.iter().any(|cap| cap.contains("sasl"));

                if let Some(sasl) = self.config.sasl.as_ref().filter(|_| supports_sasl) {
                    self.registration_step = RegistrationStep::Sasl;
                    let _ = self
                        .sender
                        .try_send(command!("AUTHENTICATE", sasl.command()));
                } else {
                    self.registration_step = RegistrationStep::End;
                    let _ = self.sender.try_send(command!("CAP", "END"));
                }
            }
            Command::CAP(_, sub, a, b) if sub == "NAK" => {
                let caps = if b.is_none() { a.as_ref() } else { b.as_ref() }?;
                log::warn!("[{}] capabilities not acknowledged: {caps}", self.server);

                // End we didn't move to sasl or already ended
                if self.registration_step < RegistrationStep::Sasl {
                    self.registration_step = RegistrationStep::End;
                    let _ = self.sender.try_send(command!("CAP", "END"));
                }
            }
            Command::AUTHENTICATE(param) if param == "+" => {
                if let Some(sasl) = self.config.sasl.as_ref() {
                    log::info!("[{}] sasl auth: {}", self.server, sasl.command());

                    let _ = self.sender.try_send(command!("AUTHENTICATE", sasl.param()));
                    self.registration_step = RegistrationStep::End;
                    let _ = self.sender.try_send(command!("CAP", "END"));
                }
            }
            Command::Numeric(RPL_LOGGEDIN, _) => {
                log::info!("[{}] logged in", self.server);
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

                let new_nick = Nick::from(nick.as_str());

                self.chanmap.values_mut().for_each(|channel| {
                    if let Some(user) = channel.users.take(&old_user) {
                        channel.users.insert(user.with_nickname(new_nick.clone()));
                    }
                });

                let channels = self.user_channels(old_user.nickname());

                return Some(vec![Event::Brodcast(Brodcast::Nickname {
                    old_user,
                    new_nick,
                    ourself,
                    channels,
                })]);
            }
            Command::Numeric(ERR_NICKNAMEINUSE | ERR_ERRONEUSNICKNAME, _)
                if self.resolved_nick.is_none() =>
            {
                // Try alt nicks
                match &mut self.alt_nick {
                    Some(index) => {
                        if *index == self.config.alt_nicks.len() - 1 {
                            self.alt_nick = None;
                        } else {
                            *index += 1;
                        }
                    }
                    None if !self.config.alt_nicks.is_empty() => self.alt_nick = Some(0),
                    None => {}
                }

                if let Some(nick) = self.alt_nick.and_then(|i| self.config.alt_nicks.get(i)) {
                    let _ = self.sender.try_send(command!("NICK", nick));
                }
            }
            Command::Numeric(RPL_WELCOME, args) => {
                // Updated actual nick
                let nick = args.first()?;
                self.resolved_nick = Some(nick.to_string());

                // Send nick password & ghost
                if let Some(nick_pass) = self.config.nick_password.as_ref() {
                    // Try ghost recovery if we couldn't claim our nick
                    if self.config.should_ghost && nick != &self.config.nickname {
                        for sequence in &self.config.ghost_sequence {
                            let _ = self.sender.try_send(command!(
                                "PRIVMSG",
                                "NickServ",
                                format!("{sequence} {} {nick_pass}", &self.config.nickname)
                            ));
                        }
                    }

                    let _ = self.sender.try_send(command!(
                        "PRIVMSG",
                        "NickServ",
                        format!("IDENTIFY {} {nick_pass}", &self.config.nickname)
                    ));
                }

                // Send user modestring
                if let Some(modestring) = self.config.umodes.as_ref() {
                    let _ = self.sender.try_send(command!("MODE", nick, modestring));
                }

                // Loop on connect commands
                for command in self.config.on_connect.iter() {
                    if let Ok(cmd) = crate::command::parse(command, None) {
                        if let Ok(command) = proto::Command::try_from(cmd) {
                            let _ = self.sender.try_send(command.into());
                        };
                    };
                }

                // Send JOIN
                for channel in &self.config.channels {
                    if let Some(keys) = self.config.channel_keys.get(channel) {
                        let _ = self.sender.try_send(command!("JOIN", channel, keys));
                    } else {
                        let _ = self.sender.try_send(command!("JOIN", channel));
                    }
                }
            }
            // QUIT
            Command::QUIT(comment) => {
                let user = message.user()?;

                self.chanmap.values_mut().for_each(|channel| {
                    channel.users.remove(&user);
                });

                let channels = self.user_channels(user.nickname());

                return Some(vec![Event::Brodcast(Brodcast::Quit {
                    user,
                    comment: comment.clone(),
                    channels,
                })]);
            }
            Command::PART(channel, _) => {
                let user = message.user()?;

                if user.nickname() == self.nickname() {
                    self.chanmap.remove(channel);
                } else if let Some(channel) = self.chanmap.get_mut(channel) {
                    channel.users.remove(&user);
                }
            }
            Command::JOIN(channel, _) => {
                let user = message.user()?;

                if user.nickname() == self.nickname() {
                    self.chanmap.insert(channel.clone(), Channel::default());

                    if let Some(state) = self.chanmap.get_mut(channel) {
                        // Sends WHO to get away state on users.
                        let _ = self.sender.try_send(command!("WHO", channel));
                        state.last_who = Some(WhoStatus::Requested(Instant::now()));
                        log::debug!("[{}] {channel} - WHO requested", self.server);
                    }
                } else if let Some(channel) = self.chanmap.get_mut(channel) {
                    channel.users.insert(user);
                }
            }
            Command::Numeric(RPL_WHOREPLY, args) => {
                let target = args.get(1)?;

                if proto::is_channel(target) {
                    if let Some(channel) = self.chanmap.get_mut(target) {
                        if matches!(channel.last_who, Some(WhoStatus::Requested(_)) | None) {
                            channel.last_who = Some(WhoStatus::Receiving);
                            log::debug!("[{}] {target} - WHO receiving...", self.server);
                        }

                        // H = Here, G = gone (away)
                        let flags = args.get(6)?.chars().collect::<Vec<char>>();
                        let away = *(flags.first()?) == 'G';

                        let lookup = User::from(Nick::from(args[5].clone()));

                        if let Some(mut user) = channel.users.take(&lookup) {
                            user.update_away(away);
                            channel.users.insert(user);
                        }

                        // We requested, don't save to history
                        if matches!(channel.last_who, Some(WhoStatus::Receiving)) {
                            return None;
                        }
                    }
                }
            }
            Command::Numeric(RPL_ENDOFWHO, args) => {
                let target = args.get(1)?;

                if proto::is_channel(target) {
                    if let Some(channel) = self.chanmap.get_mut(target) {
                        if matches!(channel.last_who, Some(WhoStatus::Receiving)) {
                            channel.last_who = Some(WhoStatus::Done(Instant::now()));
                            log::debug!("[{}] {target} - WHO done", self.server);
                            return None;
                        }
                    }
                }
            }
            Command::AWAY(args) => {
                let away = args.is_some();
                let user = message.user()?;

                for channel in self.chanmap.values_mut() {
                    if let Some(mut user) = channel.users.take(&user) {
                        user.update_away(away);
                        channel.users.insert(user);
                    }
                }
            }
            Command::Numeric(RPL_UNAWAY, args) => {
                let nick = args.get(0)?.as_str();
                let user = User::try_from(nick).ok()?;

                if user.nickname() == self.nickname() {
                    for channel in self.chanmap.values_mut() {
                        if let Some(mut user) = channel.users.take(&user) {
                            user.update_away(false);
                            channel.users.insert(user);
                        }
                    }
                }
            }
            Command::Numeric(RPL_NOWAWAY, args) => {
                let nick = args.get(0)?.as_str();
                let user = User::try_from(nick).ok()?;

                if user.nickname() == self.nickname() {
                    for channel in self.chanmap.values_mut() {
                        if let Some(mut user) = channel.users.take(&user) {
                            user.update_away(true);
                            channel.users.insert(user);
                        }
                    }
                }
            }
            Command::MODE(target, Some(modes), args) if proto::is_channel(target) => {
                let modes = mode::parse::<mode::Channel>(modes, args);

                if let Some(channel) = self.chanmap.get_mut(target) {
                    for mode in modes {
                        if let Some((op, lookup)) = mode
                            .operation()
                            .zip(mode.arg().map(|nick| User::from(Nick::from(nick))))
                        {
                            if let Some(mut user) = channel.users.take(&lookup) {
                                user.update_access_level(op, *mode.value());
                                channel.users.insert(user);
                            }
                        }
                    }
                }
            }
            Command::Numeric(RPL_NAMREPLY, args) if args.len() > 3 => {
                if let Some(channel) = self.chanmap.get_mut(&args[2]) {
                    for user in args[3].split(' ') {
                        if let Ok(user) = User::try_from(user) {
                            channel.users.insert(user);
                        }
                    }
                }
            }
            #[cfg(feature = "dev")]
            // Suppress topic during development to prevent history spam
            Command::Numeric(RPL_TOPIC | RPL_TOPICWHOTIME, _) => return None,
            _ => {}
        }

        Some(vec![Event::Single(message, self.nickname().to_owned())])
    }

    fn sync(&mut self) {
        self.channels = self.chanmap.keys().cloned().collect();
        self.users = self
            .chanmap
            .iter()
            .map(|(channel, state)| {
                (
                    channel.clone(),
                    state.users.iter().sorted().cloned().collect(),
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
        // TODO: Fallback nicks
        NickRef::from(
            self.resolved_nick
                .as_deref()
                .unwrap_or(&self.config.nickname),
        )
    }

    pub fn tick(&mut self, now: Instant) {
        for (channel, state) in self.chanmap.iter_mut() {
            enum Request {
                Poll,
                Retry,
            }

            let request = match state.last_who {
                Some(WhoStatus::Done(last)) if !self.supports_away_notify => {
                    (now.duration_since(last) >= WHO_POLL_INTERVAL).then_some(Request::Poll)
                }
                Some(WhoStatus::Requested(requested)) => {
                    (now.duration_since(requested) >= WHO_RETRY_INTERVAL).then_some(Request::Retry)
                }
                _ => None,
            };

            if let Some(request) = request {
                let _ = self.sender.try_send(command!("WHO", channel));
                state.last_who = Some(WhoStatus::Requested(Instant::now()));
                log::debug!(
                    "[{}] {channel} - WHO {}",
                    self.server,
                    match request {
                        Request::Poll => "poll",
                        Request::Retry => "retry",
                    }
                );
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Map(BTreeMap<Server, State>);

impl Map {
    pub fn disconnected(&mut self, server: Server) {
        self.0.insert(server, State::Disconnected);
    }

    pub fn ready(&mut self, server: Server, client: Client) {
        self.0.insert(server, State::Ready(client));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn remove(&mut self, server: &Server) -> Option<Client> {
        self.0.remove(server).and_then(|state| match state {
            State::Disconnected => None,
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

    pub fn nickname(&self, server: &Server) -> Option<NickRef> {
        self.client(server).map(Client::nickname)
    }

    pub fn receive(&mut self, server: &Server, message: message::Encoded) -> Vec<Event> {
        self.client_mut(server)
            .map(|client| client.receive(message))
            .unwrap_or_default()
    }

    pub fn sync(&mut self, server: &Server) {
        if let Some(State::Ready(client)) = self.0.get_mut(server) {
            client.sync();
        }
    }

    pub fn send(&mut self, buffer: &Buffer, message: message::Encoded) {
        if let Some(client) = self.client_mut(buffer.server()) {
            client.send(buffer, message);
        }
    }

    pub fn get_channel_users<'a>(&'a self, server: &Server, channel: &str) -> &'a [User] {
        self.client(server)
            .map(|client| client.users(channel))
            .unwrap_or_default()
    }

    pub fn get_user_channels(&self, server: &Server, nick: NickRef) -> Vec<String> {
        self.client(server)
            .map(|client| client.user_channels(nick))
            .unwrap_or_default()
    }

    pub fn get_channels<'a>(&'a self, server: &Server) -> &'a [String] {
        self.client(server)
            .map(|client| client.channels())
            .unwrap_or_default()
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

    pub fn tick(&mut self, now: Instant) {
        self.0.values_mut().for_each(|client| {
            if let State::Ready(client) = client {
                client.tick(now);
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum Context {
    Buffer(Buffer),
    Whois(Buffer),
}

impl Context {
    fn new(message: &message::Encoded, buffer: Buffer) -> Self {
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

fn remove_tag(key: &str, tags: &mut Vec<irc::proto::Tag>) -> Option<String> {
    tags.remove(tags.iter().position(|tag| tag.key == key)?)
        .value
}

fn start_reroute(command: &Command) -> bool {
    use Command::*;

    matches!(command, WHO(..) | WHOIS(..) | WHOWAS(..))
}

fn stop_reroute(command: &Command) -> bool {
    use command::Numeric::*;

    matches!(
        command,
        Command::Numeric(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RegistrationStep {
    List,
    Req,
    Sasl,
    End,
}

#[derive(Debug, Default)]
pub struct Channel {
    pub users: HashSet<User>,
    pub last_who: Option<WhoStatus>,
}

#[derive(Debug, Clone, Copy)]
pub enum WhoStatus {
    Requested(Instant),
    Receiving,
    Done(Instant),
}
