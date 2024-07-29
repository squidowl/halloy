use chrono::{DateTime, Utc};
use futures::channel::mpsc;
use irc::proto::{self, command, Command};
use itertools::{Either, Itertools};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::time::{Duration, Instant};

use crate::isupport::{ChatHistorySubcommand, MessageReference};
use crate::message::{message_id, server_time, source};
use crate::time::Posix;
use crate::user::{Nick, NickRef};
use crate::{config, ctcp, dcc, isupport, message, mode, Buffer, Server, User};
use crate::{file_transfer, server};

const HIGHLIGHT_BLACKOUT_INTERVAL: Duration = Duration::from_secs(5);

const CLIENT_CHATHISTORY_LIMIT: u16 = 500;
const CHATHISTORY_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

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
pub enum Notification {
    DirectMessage(User),
    Highlight(User, String),
    MonitoredOnline(Vec<User>),
    MonitoredOffline(Vec<Nick>),
}

#[derive(Debug)]
pub enum Broadcast {
    Quit {
        user: User,
        comment: Option<String>,
        channels: Vec<String>,
        sent_time: DateTime<Utc>,
    },
    Nickname {
        old_user: User,
        new_nick: Nick,
        ourself: bool,
        channels: Vec<String>,
        sent_time: DateTime<Utc>,
    },
    Invite {
        inviter: User,
        channel: String,
        user_channels: Vec<String>,
        sent_time: DateTime<Utc>,
    },
    ChangeHost {
        old_user: User,
        new_username: String,
        new_hostname: String,
        ourself: bool,
        channels: Vec<String>,
        sent_time: DateTime<Utc>,
    },
}

#[derive(Debug, Clone)]
pub enum HistoryRequest {
    Targets,
    Queries(Vec<isupport::MessageReferenceType>, DateTime<Utc>),
    Recent(String, Vec<isupport::MessageReferenceType>, DateTime<Utc>),
    Older(String, Vec<isupport::MessageReferenceType>),
}

#[derive(Debug)]
pub enum Event {
    Single(message::Encoded, Nick),
    WithTarget(message::Encoded, Nick, message::Target),
    Broadcast(Broadcast),
    Notification(message::Encoded, Nick, Notification),
    FileTransferRequest(file_transfer::ReceiveRequest),
    ChatHistoryRequest(ChatHistorySubcommand),
    ChatHistoryRequestFromHistory(HistoryRequest),
    ChatHistoryRequestReceived(ChatHistorySubcommand, usize),
    ChatHistoryTargetsReceived(DateTime<Utc>),
    LoadReadMarker(String),
}

struct ChatHistoryRequest {
    subcommand: ChatHistorySubcommand,
    requested_at: Instant,
}

pub struct Client {
    server: Server,
    config: config::Server,
    handle: server::Handle,
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
    supports_account_notify: bool,
    supports_extended_join: bool,
    supports_chathistory: bool,
    chathistory_requests: HashMap<String, ChatHistoryRequest>,
    chathistory_exhausted: HashMap<String, bool>,
    highlight_blackout: HighlightBlackout,
    registration_required_channels: Vec<String>,
    isupport: HashMap<isupport::Kind, isupport::Parameter>,
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
            handle: sender,
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
            supports_account_notify: false,
            supports_extended_join: false,
            supports_chathistory: false,
            chathistory_requests: HashMap::new(),
            chathistory_exhausted: HashMap::new(),
            highlight_blackout: HighlightBlackout::Blackout(Instant::now()),
            registration_required_channels: vec![],
            isupport: HashMap::new(),
        }
    }

    fn quit(&mut self, reason: Option<String>) {
        if let Err(e) = if let Some(reason) = reason {
            self.handle.try_send(command!("QUIT", reason))
        } else {
            self.handle.try_send(command!("QUIT"))
        } {
            log::warn!("Error sending quit: {e}");
        }
    }

    fn join(&mut self, channels: &[String]) {
        let keys = HashMap::new();

        let messages = group_joins(channels, &keys);

        for message in messages {
            if let Err(e) = self.handle.try_send(message) {
                log::warn!("Error sending join: {e}");
            }
        }
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

        if let Err(e) = self.handle.try_send(message.into()) {
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
            Command::BATCH(batch, params) => {
                let mut chars = batch.chars();
                let symbol = chars.next()?;
                let reference = chars.collect::<String>();

                match symbol {
                    '+' => {
                        let mut batch = Batch::new(context);

                        batch.chathistory = match params.first().map(|x| x.as_str()) {
                            Some("chathistory") => params
                                .get(1)
                                .map(|target| ChatHistoryBatch::Target(target.clone())),
                            Some("chathistory-targets") | Some("draft/chathistory-targets") => {
                                Some(ChatHistoryBatch::Targets)
                            }
                            _ => None,
                        };

                        self.batches.insert(reference, batch);
                    }
                    '-' => {
                        if let Some(mut finished) = self.batches.remove(&reference) {
                            // If nested, extend events into parent batch
                            if let Some(parent) = batch_tag
                                .as_ref()
                                .and_then(|batch| self.batches.get_mut(batch))
                            {
                                parent.events.extend(finished.events);
                            } else {
                                match &finished.chathistory {
                                    Some(ChatHistoryBatch::Target(batch_target)) => {
                                        if let Some(ChatHistoryRequest { subcommand, .. }) =
                                            self.chathistory_requests.get(batch_target)
                                        {
                                            if let ChatHistorySubcommand::Before(_, _, limit) =
                                                subcommand
                                            {
                                                self.chathistory_exhausted.insert(
                                                    batch_target.to_string(),
                                                    finished.events.len() < *limit as usize,
                                                );
                                            }

                                            let continue_chathistory = match subcommand {
                                                ChatHistorySubcommand::Latest(
                                                    target,
                                                    message_reference,
                                                    limit,
                                                ) => {
                                                    if matches!(
                                                        message_reference,
                                                        MessageReference::None
                                                    ) {
                                                        None
                                                    } else if finished.events.len()
                                                        == *limit as usize
                                                    {
                                                        continue_chathistory_between(
                                                            target,
                                                            &finished.events,
                                                            message_reference,
                                                            self.chathistory_limit(),
                                                        )
                                                    } else {
                                                        None
                                                    }
                                                }
                                                ChatHistorySubcommand::Before(_, _, _) => None,
                                                ChatHistorySubcommand::Between(
                                                    target,
                                                    _,
                                                    end_message_reference,
                                                    limit,
                                                ) => {
                                                    if finished.events.len() == *limit as usize {
                                                        continue_chathistory_between(
                                                            target,
                                                            &finished.events,
                                                            end_message_reference,
                                                            self.chathistory_limit(),
                                                        )
                                                    } else {
                                                        None
                                                    }
                                                }
                                                ChatHistorySubcommand::Targets(_, _, _) => None,
                                            };

                                            finished.events.push(
                                                Event::ChatHistoryRequestReceived(
                                                    subcommand.clone(),
                                                    finished.events.len(),
                                                ),
                                            );

                                            if let Some(subcommand) = continue_chathistory {
                                                finished
                                                    .events
                                                    .push(Event::ChatHistoryRequest(subcommand));
                                            }
                                        }
                                    }
                                    Some(ChatHistoryBatch::Targets) => {
                                        finished.events.push(Event::ChatHistoryTargetsReceived(
                                            server_time(&message),
                                        ));
                                    }
                                    _ => (),
                                }

                                return Some(finished.events);
                            }
                        }
                    }
                    _ => {}
                }

                return None;
            }
            _ if batch_tag.is_some() => {
                let events = if let Some(target) = batch_tag
                    .as_ref()
                    .and_then(|batch| self.batches.get(batch))
                    .and_then(|batch| {
                        batch
                            .chathistory
                            .as_ref()
                            .and_then(|chathistory| chathistory.target())
                    })
                    .and_then(|target| {
                        if self.chathistory_requests.contains_key(&target) {
                            Some(target)
                        } else {
                            None
                        }
                    }) {
                    if Some(User::from(Nick::from("HistServ"))) == message.user() {
                        // HistServ provides event-playback without event-playback
                        // which would require client-side parsing to map appropriately.
                        // Avoid that complexity by only providing that functionality
                        // via event-playback.
                        vec![]
                    } else {
                        match &message.command {
                            Command::NICK(_) => {
                                let target = message::Target::Channel {
                                    channel: target,
                                    source: source::Source::Server(None),
                                };

                                vec![Event::WithTarget(
                                    message,
                                    self.nickname().to_owned(),
                                    target,
                                )]
                            }
                            Command::QUIT(_) => {
                                let target = message::Target::Channel {
                                    channel: target,
                                    source: source::Source::Server(Some(source::Server::new(
                                        source::server::Kind::Quit,
                                        message
                                            .user()
                                            .map(|user| Nick::from(user.nickname().as_ref())),
                                    ))),
                                };

                                vec![Event::WithTarget(
                                    message,
                                    self.nickname().to_owned(),
                                    target,
                                )]
                            }
                            Command::PRIVMSG(_, text) | Command::NOTICE(_, text) => {
                                if ctcp::is_query(text) && !message::is_action(text) {
                                    // Ignore historical CTCP queries/responses except for ACTIONs
                                    vec![]
                                } else {
                                    vec![Event::Single(message, self.nickname().to_owned())]
                                }
                            }
                            _ => vec![Event::Single(message, self.nickname().to_owned())],
                        }
                    }
                } else {
                    self.handle(message, context)?
                };

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

                    let contains_starting_with =
                        |s| self.listed_caps.iter().any(|cap| cap.starts_with(s));

                    if contains("invite-notify") {
                        requested.push("invite-notify");
                    }
                    if contains("userhost-in-names") {
                        requested.push("userhost-in-names");
                    }
                    if contains("away-notify") {
                        requested.push("away-notify");
                    }
                    if contains("message-tags") {
                        requested.push("message-tags");
                    }
                    if contains("server-time") {
                        requested.push("server-time");
                    }
                    if contains("chghost") {
                        requested.push("chghost");
                    }
                    if contains("extended-monitor") {
                        requested.push("extended-monitor");
                    }
                    if contains("account-notify") {
                        requested.push("account-notify");

                        if contains("extended-join") {
                            requested.push("extended-join");
                        }
                    }
                    if contains("batch") {
                        requested.push("batch");

                        // We require batch for our chathistory support
                        let requesting_chathistory = if contains("chathistory") {
                            requested.push("chathistory");
                            true
                        } else if contains("draft/chathistory") {
                            requested.push("draft/chathistory");
                            true
                        } else {
                            false
                        };

                        if requesting_chathistory {
                            if contains("event-playback") {
                                requested.push("event-playback");
                            } else if contains("draft/event-playback") {
                                requested.push("draft/event-playback");
                            }
                        }
                    }
                    if contains("labeled-response") {
                        requested.push("labeled-response");

                        // We require labeled-response so we can properly tag echo-messages
                        if contains("echo-message") {
                            requested.push("echo-message");
                        }
                    }
                    if contains_starting_with("sasl") {
                        requested.push("sasl");
                    }
                    if contains("multi-prefix") {
                        requested.push("multi-prefix");
                    }

                    if !requested.is_empty() {
                        // Request
                        self.registration_step = RegistrationStep::Req;
                        let _ = self
                            .handle
                            .try_send(command!("CAP", "REQ", requested.join(" ")));
                    } else {
                        // If none requested, end negotiation
                        self.registration_step = RegistrationStep::End;
                        let _ = self.handle.try_send(command!("CAP", "END"));
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
                if caps.contains(&"account-notify") {
                    self.supports_account_notify = true;
                }
                if caps.contains(&"extended-join") {
                    self.supports_extended_join = true;
                }

                let supports_sasl = caps.iter().any(|cap| cap.contains("sasl"));

                if let Some(sasl) = self.config.sasl.as_ref().filter(|_| supports_sasl) {
                    self.registration_step = RegistrationStep::Sasl;
                    let _ = self
                        .handle
                        .try_send(command!("AUTHENTICATE", sasl.command()));
                } else {
                    self.registration_step = RegistrationStep::End;
                    let _ = self.handle.try_send(command!("CAP", "END"));
                }

                if caps.contains(&"chathistory") || caps.contains(&"draft/chathistory") {
                    self.supports_chathistory = true;

                    let mut commands = vec![Event::ChatHistoryRequestFromHistory(
                        HistoryRequest::Targets,
                    )];

                    if self.isupport.contains_key(&isupport::Kind::MSGREFTYPES) {
                        commands.push(Event::ChatHistoryRequestFromHistory(
                            HistoryRequest::Queries(
                                self.chathistory_message_reference_types(),
                                server_time(&message),
                            ),
                        ));
                    }

                    return Some(commands);
                }
            }
            Command::CAP(_, sub, a, b) if sub == "NAK" => {
                let caps = if b.is_none() { a.as_ref() } else { b.as_ref() }?;
                log::warn!("[{}] capabilities not acknowledged: {caps}", self.server);

                // End we didn't move to sasl or already ended
                if self.registration_step < RegistrationStep::Sasl {
                    self.registration_step = RegistrationStep::End;
                    let _ = self.handle.try_send(command!("CAP", "END"));
                }
            }
            Command::CAP(_, sub, a, b) if sub == "NEW" => {
                let caps = if b.is_none() { a.as_ref() } else { b.as_ref() }?;

                let new_caps = caps.split(' ').map(String::from).collect::<Vec<String>>();

                let mut requested = vec![];

                let newly_contains = |s| new_caps.iter().any(|cap| cap == s);

                let contains = |s| self.listed_caps.iter().any(|cap| cap == s);

                if newly_contains("invite-notify") {
                    requested.push("invite-notify");
                }
                if newly_contains("userhost-in-names") {
                    requested.push("userhost-in-names");
                }
                if newly_contains("away-notify") {
                    requested.push("away-notify");
                }
                if newly_contains("message-tags") {
                    requested.push("message-tags");
                }
                if newly_contains("server-time") {
                    requested.push("server-time");
                }
                if newly_contains("chghost") {
                    requested.push("chghost");
                }
                if newly_contains("extended-monitor") {
                    requested.push("extended-monitor");
                }
                if contains("account-notify") || newly_contains("account-notify") {
                    if newly_contains("account-notify") {
                        requested.push("account-notify");
                    }

                    if newly_contains("extended-join") {
                        requested.push("extended-join");
                    }
                }
                if contains("batch") || newly_contains("batch") {
                    if newly_contains("batch") {
                        requested.push("batch");
                    }

                    // We require batch for our chathistory support
                    let requesting_chathistory = if newly_contains("chathistory") {
                        requested.push("chathistory");
                        true
                    } else if newly_contains("draft/chathistory") {
                        requested.push("draft/chathistory");
                        true
                    } else {
                        false
                    };

                    if self.supports_chathistory || requesting_chathistory {
                        if newly_contains("event-playback") {
                            requested.push("event-playback");
                        } else if newly_contains("draft/event-playback") {
                            requested.push("draft/event-playback");
                        }
                    }
                }
                if contains("labeled-response") || newly_contains("labeled-response") {
                    if newly_contains("labeled-response") {
                        requested.push("labeled-response");
                    }

                    // We require labeled-response so we can properly tag echo-messages
                    if newly_contains("echo-message") {
                        requested.push("echo-message");
                    }
                }
                if newly_contains("multi-prefix") {
                    requested.push("multi-prefix");
                }

                if !requested.is_empty() {
                    // Request
                    let _ = self
                        .handle
                        .try_send(command!("CAP", "REQ", requested.join(" ")));
                }

                self.listed_caps.extend(new_caps);
            }
            Command::CAP(_, sub, a, b) if sub == "DEL" => {
                let caps = if b.is_none() { a.as_ref() } else { b.as_ref() }?;

                let del_caps = caps.split(' ').collect::<Vec<_>>();

                if del_caps.contains(&"labeled-response") {
                    self.supports_labels = false;
                }
                if del_caps.contains(&"away-notify") {
                    self.supports_away_notify = false;
                }
                if del_caps.contains(&"account-notify") {
                    self.supports_account_notify = false;
                }
                if del_caps.contains(&"extended-join") {
                    self.supports_extended_join = false;
                }
                if del_caps.contains(&"chathistory") || del_caps.contains(&"draft/chathistory") {
                    self.supports_chathistory = false;
                }

                self.listed_caps
                    .retain(|cap| !del_caps.iter().any(|del_cap| del_cap == cap));
            }
            Command::AUTHENTICATE(param) if param == "+" => {
                if let Some(sasl) = self.config.sasl.as_ref() {
                    log::info!("[{}] sasl auth: {}", self.server, sasl.command());

                    let _ = self.handle.try_send(command!("AUTHENTICATE", sasl.param()));
                    self.registration_step = RegistrationStep::End;
                    let _ = self.handle.try_send(command!("CAP", "END"));
                }
            }
            Command::Numeric(RPL_LOGGEDIN, args) => {
                log::info!("[{}] logged in", self.server);

                if !self.registration_required_channels.is_empty() {
                    for message in group_joins(
                        &self.registration_required_channels,
                        &self.config.channel_keys,
                    ) {
                        let _ = self.handle.try_send(message);
                    }

                    self.registration_required_channels.clear();
                }

                if !self.supports_account_notify {
                    let accountname = args.first()?;

                    let old_user = User::from(self.nickname().to_owned());

                    self.chanmap.values_mut().for_each(|channel| {
                        if let Some(user) = channel.users.take(&old_user) {
                            channel.users.insert(user.with_accountname(accountname));
                        }
                    });
                }
            }
            Command::Numeric(RPL_LOGGEDOUT, _) => {
                log::info!("[{}] logged out", self.server);

                if !self.supports_account_notify {
                    let old_user = User::from(self.nickname().to_owned());

                    self.chanmap.values_mut().for_each(|channel| {
                        if let Some(user) = channel.users.take(&old_user) {
                            channel.users.insert(user.with_accountname("*"));
                        }
                    });
                }
            }
            Command::PRIVMSG(channel, text) | Command::NOTICE(channel, text) => {
                if let Some(user) = message.user() {
                    if let Some(command) = dcc::decode(text) {
                        match command {
                            dcc::Command::Send(request) => {
                                log::trace!("DCC Send => {request:?}");
                                return Some(vec![Event::FileTransferRequest(
                                    file_transfer::ReceiveRequest {
                                        from: user.nickname().to_owned(),
                                        dcc_send: request,
                                        server: self.server.clone(),
                                        server_handle: self.handle.clone(),
                                    },
                                )]);
                            }
                            dcc::Command::Unsupported(command) => {
                                log::debug!("Unsupported DCC command: {command}",);
                                return None;
                            }
                        }
                    } else {
                        // Handle CTCP queries except ACTION and DCC
                        if user.nickname() != self.nickname()
                            && ctcp::is_query(text)
                            && !message::is_action(text)
                        {
                            if let Some(query) = ctcp::parse_query(text) {
                                if matches!(&message.command, Command::PRIVMSG(_, _)) {
                                    match query.command {
                                        ctcp::Command::Action => (),
                                        ctcp::Command::ClientInfo => {
                                            let _ = self.handle.try_send(ctcp::response_message(
                                                &query.command,
                                                user.nickname().to_string(),
                                                Some("ACTION CLIENTINFO DCC PING SOURCE VERSION"),
                                            ));
                                        }
                                        ctcp::Command::DCC => (),
                                        ctcp::Command::Ping => {
                                            let _ = self.handle.try_send(ctcp::response_message(
                                                &query.command,
                                                user.nickname().to_string(),
                                                query.params,
                                            ));
                                        }
                                        ctcp::Command::Source => {
                                            let _ = self.handle.try_send(ctcp::response_message(
                                                &query.command,
                                                user.nickname().to_string(),
                                                Some(crate::environment::SOURCE_WEBSITE),
                                            ));
                                        }
                                        ctcp::Command::Version => {
                                            let _ = self.handle.try_send(ctcp::response_message(
                                                &query.command,
                                                user.nickname().to_string(),
                                                Some(format!(
                                                    "Halloy {}",
                                                    crate::environment::VERSION
                                                )),
                                            ));
                                        }
                                        ctcp::Command::Unknown(command) => {
                                            log::debug!(
                                                "Ignorning CTCP command {command}: Unknown command"
                                            )
                                        }
                                    }
                                }

                                return None;
                            }
                        }

                        // Highlight notification
                        if message::reference_user_text(user.nickname(), self.nickname(), text)
                            && self.highlight_blackout.allow_highlights()
                        {
                            return Some(vec![Event::Notification(
                                message.clone(),
                                self.nickname().to_owned(),
                                Notification::Highlight(user, channel.clone()),
                            )]);
                        } else if user.nickname() == self.nickname()
                            && context.is_some()
                            && message_id(&message).is_none()
                        {
                            // If we sent (echo) & context exists (we sent from this client),
                            // then ignore unless it has a message id (in which case the local
                            // copy should be updated with the message id)
                            return None;
                        }

                        // use `channel` to confirm the direct message, then send notification
                        if channel == &self.nickname().to_string() {
                            return Some(vec![Event::Notification(
                                message.clone(),
                                self.nickname().to_owned(),
                                Notification::DirectMessage(user),
                            )]);
                        }
                    }
                }
            }
            Command::INVITE(user, channel) => {
                let user = User::from(Nick::from(user.as_str()));
                let inviter = message.user()?;
                let user_channels = self.user_channels(user.nickname());

                return Some(vec![Event::Broadcast(Broadcast::Invite {
                    inviter,
                    channel: channel.clone(),
                    user_channels,
                    sent_time: server_time(&message),
                })]);
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

                return Some(vec![Event::Broadcast(Broadcast::Nickname {
                    old_user,
                    new_nick,
                    ourself,
                    channels,
                    sent_time: server_time(&message),
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
                    let _ = self.handle.try_send(command!("NICK", nick));
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
                            let _ = self.handle.try_send(command!(
                                "PRIVMSG",
                                "NickServ",
                                format!("{sequence} {} {nick_pass}", &self.config.nickname)
                            ));
                        }
                    }

                    let _ = if let Some(identify_syntax) = &self.config.nick_identify_syntax {
                        match identify_syntax {
                            config::server::IdentifySyntax::PasswordNick => {
                                self.handle.try_send(command!(
                                    "PRIVMSG",
                                    "NickServ",
                                    format!("IDENTIFY {nick_pass} {}", &self.config.nickname)
                                ))
                            }
                            config::server::IdentifySyntax::NickPassword => {
                                self.handle.try_send(command!(
                                    "PRIVMSG",
                                    "NickServ",
                                    format!("IDENTIFY {} {nick_pass}", &self.config.nickname)
                                ))
                            }
                        }
                    } else if self.resolved_nick == Some(self.config.nickname.clone()) {
                        // Use nickname-less identification if possible, since it has
                        // no possible argument order issues.
                        self.handle.try_send(command!(
                            "PRIVMSG",
                            "NickServ",
                            format!("IDENTIFY {nick_pass}")
                        ))
                    } else {
                        // Default to most common syntax if unknown
                        self.handle.try_send(command!(
                            "PRIVMSG",
                            "NickServ",
                            format!("IDENTIFY {} {nick_pass}", &self.config.nickname)
                        ))
                    };
                }

                // Send user modestring
                if let Some(modestring) = self.config.umodes.as_ref() {
                    let _ = self.handle.try_send(command!("MODE", nick, modestring));
                }

                // Loop on connect commands
                for command in self.config.on_connect.iter() {
                    if let Ok(cmd) = crate::command::parse(command, None) {
                        if let Ok(command) = proto::Command::try_from(cmd) {
                            let _ = self.handle.try_send(command.into());
                        };
                    };
                }

                // Send JOIN
                for message in group_joins(&self.config.channels, &self.config.channel_keys) {
                    let _ = self.handle.try_send(message);
                }
            }
            // QUIT
            Command::QUIT(comment) => {
                let user = message.user()?;

                self.chanmap.values_mut().for_each(|channel| {
                    channel.users.remove(&user);
                });

                let channels = self.user_channels(user.nickname());

                return Some(vec![Event::Broadcast(Broadcast::Quit {
                    user,
                    comment: comment.clone(),
                    channels,
                    sent_time: server_time(&message),
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
            Command::JOIN(channel, accountname) => {
                let mut events = vec![];

                let user = message.user()?;

                if user.nickname() == self.nickname() {
                    self.chanmap.insert(channel.clone(), Channel::default());

                    events.push(Event::LoadReadMarker(channel.clone()));

                    if let Some(state) = self.chanmap.get_mut(channel) {
                        // Sends WHO to get away state on users.
                        if self.isupport.contains_key(&isupport::Kind::WHOX) {
                            let fields = if self.supports_account_notify {
                                "tcnfa"
                            } else {
                                "tcnf"
                            };

                            let _ = self.handle.try_send(command!(
                                "WHO",
                                channel,
                                fields,
                                isupport::WHO_POLL_TOKEN.to_owned()
                            ));

                            state.last_who = Some(WhoStatus::Requested(
                                Instant::now(),
                                Some(isupport::WHO_POLL_TOKEN),
                            ));
                        } else {
                            let _ = self.handle.try_send(command!("WHO", channel));
                            state.last_who = Some(WhoStatus::Requested(Instant::now(), None));
                        }
                        log::debug!("[{}] {channel} - WHO requested", self.server);

                        if self.supports_chathistory
                            && self.isupport.contains_key(&isupport::Kind::MSGREFTYPES)
                        {
                            events.push(Event::ChatHistoryRequestFromHistory(
                                HistoryRequest::Recent(
                                    channel.clone(),
                                    self.chathistory_message_reference_types(),
                                    server_time(&message),
                                ),
                            ));
                        }
                    }
                } else if let Some(channel) = self.chanmap.get_mut(channel) {
                    let user = if self.supports_extended_join {
                        accountname.as_ref().map_or(user.clone(), |accountname| {
                            user.with_accountname(accountname)
                        })
                    } else {
                        user
                    };

                    channel.users.insert(user);
                }

                events.push(Event::Single(message, self.nickname().to_owned()));

                return Some(events);
            }
            Command::KICK(channel, victim, _) => {
                if victim == self.nickname().as_ref() {
                    self.chanmap.remove(channel);
                } else if let Some(channel) = self.chanmap.get_mut(channel) {
                    channel
                        .users
                        .remove(&User::from(Nick::from(victim.as_str())));
                }
            }
            Command::Numeric(RPL_WHOREPLY, args) => {
                let target = args.get(1)?;

                if proto::is_channel(target) {
                    if let Some(channel) = self.chanmap.get_mut(target) {
                        channel.update_user_away(args.get(5)?, args.get(6)?);

                        if matches!(channel.last_who, Some(WhoStatus::Requested(_, None)) | None) {
                            channel.last_who = Some(WhoStatus::Receiving(None));
                            log::debug!("[{}] {target} - WHO receiving...", self.server);
                        }

                        if matches!(channel.last_who, Some(WhoStatus::Receiving(_))) {
                            // We requested, don't save to history
                            return None;
                        }
                    }
                }
            }
            Command::Numeric(RPL_WHOSPCRPL, args) => {
                let target = args.get(2)?;

                if proto::is_channel(target) {
                    if let Some(channel) = self.chanmap.get_mut(target) {
                        channel.update_user_away(args.get(3)?, args.get(4)?);

                        if self.supports_account_notify {
                            if let (Some(user), Some(accountname)) = (args.get(3), args.get(5)) {
                                channel.update_user_accountname(user, accountname);
                            }
                        }

                        if let Ok(token) = args.get(1)?.parse::<isupport::WhoToken>() {
                            if let Some(WhoStatus::Requested(_, Some(request_token))) =
                                channel.last_who
                            {
                                if request_token == token {
                                    channel.last_who =
                                        Some(WhoStatus::Receiving(Some(request_token)));
                                    log::debug!("[{}] {target} - WHO receiving...", self.server);
                                }
                            }
                        }

                        if matches!(channel.last_who, Some(WhoStatus::Receiving(_))) {
                            // We requested, don't save to history
                            return None;
                        }
                    }
                }
            }
            Command::Numeric(RPL_ENDOFWHO, args) => {
                let target = args.get(1)?;

                if proto::is_channel(target) {
                    if let Some(channel) = self.chanmap.get_mut(target) {
                        if matches!(channel.last_who, Some(WhoStatus::Receiving(_))) {
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
                let nick = args.first()?.as_str();
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
                let nick = args.first()?.as_str();
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
            Command::MODE(target, Some(modes), Some(args)) => {
                if proto::is_channel(target) {
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
                } else {
                    // Only check for being logged in via mode if account-notify is not available,
                    // since it is not standardized across networks.

                    if target == self.nickname().as_ref()
                        && !self.supports_account_notify
                        && !self.registration_required_channels.is_empty()
                    {
                        let modes = mode::parse::<mode::User>(modes, args);

                        if modes.into_iter().any(|mode| {
                            matches!(mode, mode::Mode::Add(mode::User::Registered, None))
                        }) {
                            for message in group_joins(
                                &self.registration_required_channels,
                                &self.config.channel_keys,
                            ) {
                                let _ = self.handle.try_send(message);
                            }

                            self.registration_required_channels.clear();
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

                    // Don't save to history if names list was triggered by JOIN
                    if !channel.names_init {
                        return None;
                    }
                }
            }
            Command::Numeric(RPL_ENDOFNAMES, args) => {
                let target = args.get(1)?;

                if proto::is_channel(target) {
                    if let Some(channel) = self.chanmap.get_mut(target) {
                        if !channel.names_init {
                            channel.names_init = true;

                            return None;
                        }
                    }
                }
            }
            Command::TOPIC(channel, topic) => {
                if let Some(channel) = self.chanmap.get_mut(channel) {
                    if let Some(text) = topic {
                        channel.topic.content = Some(message::parse_fragments(text.clone()));
                    }

                    channel.topic.who = message
                        .user()
                        .map(|user| user.nickname().to_string());
                    channel.topic.time = Some(server_time(&message));
                }
            }
            Command::Numeric(RPL_TOPIC, args) => {
                if let Some(channel) = self.chanmap.get_mut(&args[1]) {
                    channel.topic.content = Some(message::parse_fragments(args.get(2)?.to_owned()));
                }
                // Exclude topic message from history to prevent spam during dev
                #[cfg(feature = "dev")]
                return None;
            }
            Command::Numeric(RPL_TOPICWHOTIME, args) => {
                if let Some(channel) = self.chanmap.get_mut(&args[1]) {
                    channel.topic.who = Some(args.get(2)?.to_string());
                    channel.topic.time = Some(
                        args.get(3)?
                            .parse::<u64>()
                            .ok()
                            .map(Posix::from_seconds)?
                            .datetime()?,
                    );
                }
                // Exclude topic message from history to prevent spam during dev
                #[cfg(feature = "dev")]
                return None;
            }
            Command::Numeric(ERR_NOCHANMODES, args) => {
                let channel = args.get(1)?;

                // If the channel has not been joined but is in the configured channels,
                // then interpret this numeric as ERR_NEEDREGGEDNICK (which has the
                // same number as ERR_NOCHANMODES)
                if !self.chanmap.contains_key(channel)
                    && self
                        .config
                        .channels
                        .iter()
                        .any(|config_channel| config_channel == channel)
                {
                    self.registration_required_channels.push(channel.clone());
                }
            }
            Command::Numeric(RPL_ISUPPORT, args) => {
                let mut events = vec![];

                let args_len = args.len();
                args.iter().enumerate().skip(1).for_each(|(index, arg)| {
                    let operation = arg.parse::<isupport::Operation>();

                    match operation {
                        Ok(operation) => {
                            match operation {
                                isupport::Operation::Add(parameter) => {
                                    if let Some(kind) = parameter.kind() {
                                        log::info!(
                                            "[{}] adding ISUPPORT parameter: {:?}",
                                            self.server,
                                            parameter
                                        );

                                        self.isupport.insert(kind.clone(), parameter);

                                        if kind == isupport::Kind::MSGREFTYPES
                                            && self.supports_chathistory
                                        {
                                            events.push(Event::ChatHistoryRequestFromHistory(
                                                HistoryRequest::Queries(
                                                    self.chathistory_message_reference_types(),
                                                    server_time(&message),
                                                ),
                                            ));
                                        } else if let isupport::Parameter::MONITOR(target_limit) =
                                            parameter
                                        {
                                            let messages =
                                                group_monitors(&self.config.monitor, target_limit);

                                            for message in messages {
                                                let _ = self.handle.try_send(message);
                                            }
                                        }
                                    } else {
                                        log::debug!(
                                            "[{}] ignoring ISUPPORT parameter: {:?}",
                                            self.server,
                                            parameter
                                        );
                                    }
                                }
                                isupport::Operation::Remove(_) => {
                                    if let Some(kind) = operation.kind() {
                                        log::info!(
                                            "[{}] removing ISUPPORT parameter: {:?}",
                                            self.server,
                                            kind
                                        );

                                        self.isupport.remove(&kind);
                                    }
                                }
                            };
                        }
                        Err(error) => {
                            if index != args_len - 1 {
                                log::debug!(
                                    "[{}] unable to parse ISUPPORT parameter: {} ({})",
                                    self.server,
                                    arg,
                                    error
                                )
                            }
                        }
                    }
                });

                if !events.is_empty() {
                    return Some(events);
                } else {
                    return None;
                }
            }
            Command::TAGMSG(_) => {
                return None;
            }
            Command::ACCOUNT(accountname) => {
                let old_user = message.user()?;

                self.chanmap.values_mut().for_each(|channel| {
                    if let Some(user) = channel.users.take(&old_user) {
                        channel.users.insert(user.with_accountname(accountname));
                    }
                });

                if old_user.nickname() == self.nickname()
                    && accountname != "*"
                    && !self.registration_required_channels.is_empty()
                {
                    for message in group_joins(
                        &self.registration_required_channels,
                        &self.config.channel_keys,
                    ) {
                        let _ = self.handle.try_send(message);
                    }

                    self.registration_required_channels.clear();
                }
            }
            Command::CHGHOST(new_username, new_hostname) => {
                let old_user = message.user()?;

                let ourself = old_user.nickname() == self.nickname();

                self.chanmap.values_mut().for_each(|channel| {
                    if let Some(user) = channel.users.take(&old_user) {
                        channel.users.insert(user.with_username_and_hostname(
                            new_username.clone(),
                            new_hostname.clone(),
                        ));
                    }
                });

                let channels = self.user_channels(old_user.nickname());

                return Some(vec![Event::Broadcast(Broadcast::ChangeHost {
                    old_user,
                    new_username: new_username.clone(),
                    new_hostname: new_hostname.clone(),
                    ourself,
                    channels,
                    sent_time: server_time(&message),
                })]);
            }
            Command::Numeric(RPL_MONONLINE, args) => {
                let targets = args
                    .get(1)?
                    .split(',')
                    .filter_map(|target| User::try_from(target).ok())
                    .collect::<Vec<_>>();

                return Some(vec![Event::Notification(
                    message.clone(),
                    self.nickname().to_owned(),
                    Notification::MonitoredOnline(targets),
                )]);
            }
            Command::Numeric(RPL_MONOFFLINE, args) => {
                let targets = args.get(1)?.split(',').map(Nick::from).collect::<Vec<_>>();

                return Some(vec![Event::Notification(
                    message.clone(),
                    self.nickname().to_owned(),
                    Notification::MonitoredOffline(targets),
                )]);
            }
            Command::Numeric(RPL_ENDOFMONLIST, _) => {
                return None;
            }
            Command::CHATHISTORY(sub, args) => {
                if sub == "TARGETS" {
                    let target = args.first()?;

                    if !proto::is_channel(target) {
                        return Some(vec![Event::ChatHistoryRequestFromHistory(
                            HistoryRequest::Recent(
                                target.clone(),
                                self.chathistory_message_reference_types(),
                                server_time(&message),
                            ),
                        )]);
                    }
                }

                return None;
            }
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

    pub fn chathistory_limit(&self) -> u16 {
        if let Some(isupport::Parameter::CHATHISTORY(server_limit)) =
            self.isupport.get(&isupport::Kind::CHATHISTORY)
        {
            if *server_limit != 0 {
                return std::cmp::min(*server_limit, CLIENT_CHATHISTORY_LIMIT);
            }
        }

        CLIENT_CHATHISTORY_LIMIT
    }

    pub fn chathistory_message_reference_types(&self) -> Vec<isupport::MessageReferenceType> {
        if let Some(isupport::Parameter::MSGREFTYPES(message_reference_types)) =
            self.isupport.get(&isupport::Kind::MSGREFTYPES)
        {
            message_reference_types.clone()
        } else {
            vec![]
        }
    }

    pub fn chathistory_request(&self, target: &str) -> Option<ChatHistorySubcommand> {
        self.chathistory_requests
            .get(target)
            .map(|request| request.subcommand.clone())
    }

    pub fn send_chathistory_request(&mut self, subcommand: ChatHistorySubcommand) {
        if self.supports_chathistory {
            if let Some(target) = subcommand.target() {
                if self.chathistory_requests.contains_key(target) {
                    return;
                } else {
                    self.chathistory_requests.insert(
                        target.to_string(),
                        ChatHistoryRequest {
                            subcommand: subcommand.clone(),
                            requested_at: Instant::now(),
                        },
                    );
                }
            }

            match subcommand {
                ChatHistorySubcommand::Latest(target, message_reference, limit) => {
                    let command_message_reference =
                        isupport::fuzz_start_message_reference(message_reference);

                    log::debug!(
                        "[{}] requesting {limit} latest messages in {target} since {}",
                        self.server,
                        command_message_reference,
                    );

                    let _ = self.handle.try_send(command!(
                        "CHATHISTORY",
                        "LATEST",
                        target,
                        command_message_reference.to_string(),
                        limit.to_string()
                    ));
                }
                ChatHistorySubcommand::Before(target, message_reference, limit) => {
                    let command_message_reference =
                        isupport::fuzz_end_message_reference(message_reference);

                    log::debug!(
                        "[{}] requesting {limit} messages in {target} before {}",
                        self.server,
                        command_message_reference,
                    );

                    let _ = self.handle.try_send(command!(
                        "CHATHISTORY",
                        "BEFORE",
                        target,
                        command_message_reference.to_string(),
                        limit.to_string()
                    ));
                }
                ChatHistorySubcommand::Between(
                    target,
                    start_message_reference,
                    end_message_reference,
                    limit,
                ) => {
                    let (command_start_message_reference, command_end_message_reference) =
                        isupport::fuzz_message_reference_range(
                            start_message_reference,
                            end_message_reference,
                        );

                    log::debug!(
                        "[{}] requesting {limit} messages in {target} between {} and {}",
                        self.server,
                        command_start_message_reference,
                        command_end_message_reference,
                    );

                    let _ = self.handle.try_send(command!(
                        "CHATHISTORY",
                        "BETWEEN",
                        target,
                        command_start_message_reference.to_string(),
                        command_end_message_reference.to_string(),
                        limit.to_string()
                    ));
                }
                ChatHistorySubcommand::Targets(
                    start_message_reference,
                    end_message_reference,
                    limit,
                ) => {
                    let command_start_message_reference = match start_message_reference {
                        isupport::MessageReference::Timestamp(_) => start_message_reference,
                        _ => isupport::MessageReference::Timestamp(DateTime::UNIX_EPOCH),
                    };

                    let command_end_message_reference = match end_message_reference {
                        isupport::MessageReference::Timestamp(_) => end_message_reference,
                        _ => isupport::MessageReference::Timestamp(chrono::offset::Utc::now()),
                    };

                    let (command_start_message_reference, command_end_message_reference) =
                        isupport::fuzz_message_reference_range(
                            command_start_message_reference,
                            command_end_message_reference,
                        );

                    log::debug!(
                        "[{}] requesting {limit} targets between {} and {}",
                        self.server,
                        command_start_message_reference,
                        command_end_message_reference,
                    );

                    let _ = self.handle.try_send(command!(
                        "CHATHISTORY",
                        "TARGETS",
                        command_start_message_reference.to_string(),
                        command_end_message_reference.to_string(),
                        limit.to_string()
                    ));
                }
            }
        }
    }

    pub fn clear_chathistory_request(&mut self, target: &str) {
        self.chathistory_requests.remove(target);
    }

    pub fn chathistory_exhausted(&self, target: &str) -> bool {
        self.chathistory_exhausted
            .get(target)
            .cloned()
            .unwrap_or_default()
    }

    fn topic<'a>(&'a self, channel: &str) -> Option<&'a Topic> {
        self.chanmap.get(channel).map(|channel| &channel.topic)
    }

    fn resolve_user_attributes<'a>(&'a self, channel: &str, user: &User) -> Option<&'a User> {
        self.chanmap
            .get(channel)
            .and_then(|channel| channel.users.get(user))
    }

    pub fn users<'a>(&'a self, channel: &str) -> &'a [User] {
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
        match self.highlight_blackout {
            HighlightBlackout::Blackout(instant) => {
                if now.duration_since(instant) >= HIGHLIGHT_BLACKOUT_INTERVAL {
                    self.highlight_blackout = HighlightBlackout::Receiving;
                }
            }
            HighlightBlackout::Receiving => {}
        }

        for (channel, state) in self.chanmap.iter_mut() {
            enum Request {
                Poll,
                Retry,
            }

            let request = match state.last_who {
                Some(WhoStatus::Done(last)) if !self.supports_away_notify => {
                    (now.duration_since(last) >= self.config.who_poll_interval)
                        .then_some(Request::Poll)
                }
                Some(WhoStatus::Requested(requested, _)) => (now.duration_since(requested)
                    >= self.config.who_retry_interval)
                    .then_some(Request::Retry),
                _ => None,
            };

            if let Some(request) = request {
                if self.isupport.contains_key(&isupport::Kind::WHOX) {
                    let fields = if self.supports_account_notify {
                        "tcnfa"
                    } else {
                        "tcnf"
                    };

                    let _ = self.handle.try_send(command!(
                        "WHO",
                        channel,
                        fields,
                        isupport::WHO_POLL_TOKEN.to_owned()
                    ));

                    state.last_who = Some(WhoStatus::Requested(
                        Instant::now(),
                        Some(isupport::WHO_POLL_TOKEN),
                    ));
                } else {
                    let _ = self.handle.try_send(command!("WHO", channel));
                    state.last_who = Some(WhoStatus::Requested(Instant::now(), None));
                }
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

        self.chathistory_requests.retain(|_, chathistory_request| {
            now.duration_since(chathistory_request.requested_at) < CHATHISTORY_REQUEST_TIMEOUT
        });
    }
}

fn continue_chathistory_between(
    target: &str,
    events: &[Event],
    end_message_reference: &MessageReference,
    limit: u16,
) -> Option<ChatHistorySubcommand> {
    let start_message_reference = events.first().and_then(|first_event| match first_event {
        Event::Single(message, _) | Event::WithTarget(message, _, _) => match end_message_reference
        {
            MessageReference::MessageId(_) => message_id(message).map(MessageReference::MessageId),
            MessageReference::Timestamp(_) => {
                Some(MessageReference::Timestamp(server_time(message)))
            }
            MessageReference::None => None,
        },
        _ => None,
    });

    start_message_reference.map(|start_message_reference| {
        ChatHistorySubcommand::Between(
            target.to_string(),
            start_message_reference,
            end_message_reference.clone(),
            limit,
        )
    })
}

#[derive(Debug)]
enum HighlightBlackout {
    Blackout(Instant),
    Receiving,
}

impl HighlightBlackout {
    fn allow_highlights(&self) -> bool {
        match self {
            HighlightBlackout::Blackout(_) => false,
            HighlightBlackout::Receiving => true,
        }
    }
}

#[derive(Debug, Default)]
pub struct Map(BTreeMap<Server, State>);

impl Map {
    pub fn len(&self) -> usize {
        self.0.len()
    }

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

    pub fn nickname<'a>(&'a self, server: &Server) -> Option<NickRef<'a>> {
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

    pub fn join(&mut self, server: &Server, channels: &[String]) {
        if let Some(client) = self.client_mut(server) {
            client.join(channels);
        }
    }

    pub fn quit(&mut self, server: &Server, reason: Option<String>) {
        if let Some(client) = self.client_mut(server) {
            client.quit(reason);
        }
    }

    pub fn resolve_user_attributes<'a>(
        &'a self,
        server: &Server,
        channel: &str,
        user: &User,
    ) -> Option<&'a User> {
        self.client(server)
            .and_then(|client| client.resolve_user_attributes(channel, user))
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

    pub fn get_channel_topic<'a>(&'a self, server: &Server, channel: &str) -> Option<&'a Topic> {
        self.client(server)
            .map(|client| client.topic(channel))
            .unwrap_or_default()
    }

    pub fn get_server_chathistory_message_reference_types(
        &self,
        server: &Server,
    ) -> Vec<isupport::MessageReferenceType> {
        self.client(server)
            .map(|client| client.chathistory_message_reference_types())
            .unwrap_or_default()
    }

    pub fn get_server_chathistory_limit(&self, server: &Server) -> u16 {
        self.client(server)
            .map(|client| client.chathistory_limit())
            .unwrap_or(CLIENT_CHATHISTORY_LIMIT)
    }

    pub fn get_server_supports_chathistory(&self, server: &Server) -> bool {
        self.client(server)
            .map(|client| client.supports_chathistory)
            .unwrap_or_default()
    }

    pub fn get_chathistory_request(
        &self,
        server: &Server,
        target: &str,
    ) -> Option<ChatHistorySubcommand> {
        self.client(server)
            .and_then(|client| client.chathistory_request(target))
    }

    pub fn send_chathistory_request(&mut self, server: &Server, subcommand: ChatHistorySubcommand) {
        if let Some(client) = self.client_mut(server) {
            client.send_chathistory_request(subcommand);
        }
    }

    pub fn clear_chathistory_request(&mut self, server: &Server, target: &str) {
        if let Some(client) = self.client_mut(server) {
            client.clear_chathistory_request(target);
        }
    }

    pub fn get_chathistory_exhausted(&self, server: &Server, target: &str) -> bool {
        self.client(server)
            .map(|client| client.chathistory_exhausted(target))
            .unwrap_or_default()
    }

    pub fn get_channels<'a>(&'a self, server: &Server) -> &'a [String] {
        self.client(server)
            .map(|client| client.channels())
            .unwrap_or_default()
    }

    pub fn get_isupport(&self, server: &Server) -> HashMap<isupport::Kind, isupport::Parameter> {
        self.client(server)
            .map(|client| client.isupport.clone())
            .unwrap_or_default()
    }

    pub fn get_server_handle(&self, server: &Server) -> Option<&server::Handle> {
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
pub enum ChatHistoryBatch {
    Target(String),
    Targets,
}

impl ChatHistoryBatch {
    pub fn target(&self) -> Option<String> {
        match self {
            ChatHistoryBatch::Target(batch_target) => Some(batch_target.clone()),
            ChatHistoryBatch::Targets => None,
        }
    }
}

#[derive(Debug)]
pub struct Batch {
    context: Option<Context>,
    events: Vec<Event>,
    chathistory: Option<ChatHistoryBatch>,
}

impl Batch {
    fn new(context: Option<Context>) -> Self {
        Self {
            context,
            events: vec![],
            chathistory: None,
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

    if let MODE(target, _, _) = command {
        !proto::is_channel(target)
    } else {
        matches!(command, WHO(..) | WHOIS(..) | WHOWAS(..))
    }
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
                | ERR_NEEDMOREPARAMS
                | ERR_USERSDONTMATCH
                | RPL_UMODEIS
                | ERR_UMODEUNKNOWNFLAG,
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
    pub topic: Topic,
    pub names_init: bool,
}

impl Channel {
    pub fn update_user_away(&mut self, user: &str, flags: &str) {
        let user = User::from(Nick::from(user));

        if let Some(away_flag) = flags.chars().next() {
            // H = Here, G = gone (away)
            let away = match away_flag {
                'G' => true,
                'H' => false,
                _ => return,
            };

            if let Some(mut user) = self.users.take(&user) {
                user.update_away(away);
                self.users.insert(user);
            }
        }
    }

    pub fn update_user_accountname(&mut self, user: &str, accountname: &str) {
        let user = User::from(Nick::from(user));

        if let Some(user) = self.users.take(&user) {
            self.users.insert(user.with_accountname(accountname));
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Topic {
    pub content: Option<message::Content>,
    pub who: Option<String>,
    pub time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum WhoStatus {
    Requested(Instant, Option<isupport::WhoToken>),
    Receiving(Option<isupport::WhoToken>),
    Done(Instant),
}

/// Group channels together into as few JOIN messages as possible
fn group_joins<'a>(
    channels: &'a [String],
    keys: &'a HashMap<String, String>,
) -> impl Iterator<Item = proto::Message> + 'a {
    const MAX_LEN: usize = proto::format::BYTE_LIMIT - b"JOIN \r\n".len();

    let (without_keys, with_keys): (Vec<_>, Vec<_>) = channels.iter().partition_map(|channel| {
        keys.get(channel)
            .map(|key| Either::Right((channel, key)))
            .unwrap_or(Either::Left(channel))
    });

    let joins_without_keys = without_keys
        .into_iter()
        .scan(0, |count, channel| {
            // Channel + a comma
            *count += channel.len() + 1;

            let chunk = *count / MAX_LEN;

            Some((chunk, channel))
        })
        .into_group_map()
        .into_values()
        .map(|channels| command!("JOIN", channels.into_iter().join(",")));

    let joins_with_keys = with_keys
        .into_iter()
        .scan(0, |count, (channel, key)| {
            // Channel + key + a comma for each
            *count += channel.len() + key.len() + 2;

            let chunk = *count / MAX_LEN;

            Some((chunk, (channel, key)))
        })
        .into_group_map()
        .into_values()
        .map(|values| {
            command!(
                "JOIN",
                values.iter().map(|(c, _)| c).join(","),
                values.iter().map(|(_, k)| k).join(",")
            )
        });

    joins_without_keys.chain(joins_with_keys)
}

fn group_monitors(
    targets: &[String],
    target_limit: Option<u16>,
) -> impl Iterator<Item = proto::Message> + '_ {
    const MAX_LEN: usize = proto::format::BYTE_LIMIT - b"MONITOR + \r\n".len();

    if let Some(target_limit) = target_limit.map(usize::from) {
        &targets[0..std::cmp::min(target_limit, targets.len())]
    } else {
        targets
    }
    .iter()
    .scan(0, |count, target| {
        // Target + a comma
        *count += target.len() + 1;

        let chunk = *count / MAX_LEN;

        Some((chunk, target))
    })
    .into_group_map()
    .into_values()
    .map(|targets| command!("MONITOR", "+", targets.into_iter().join(","),))
}
