use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{fmt, io};

use anyhow::{Context as ErrorContext, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use futures::channel::mpsc;
use futures::{Future, FutureExt};
use indexmap::IndexMap;
use irc::proto::{self, Command, command, tags};
use itertools::{Either, Itertools};
use log::error;
use tokio::fs;

pub use self::on_connect::on_connect;
use crate::environment::{SOURCE_WEBSITE, VERSION};
use crate::history::ReadMarker;
use crate::isupport::{
    ChatHistoryState, ChatHistorySubcommand, MessageReference, WhoToken,
    WhoXPollParameters,
};
use crate::message::{message_id, server_time, source};
use crate::target::{self, Target};
use crate::time::Posix;
use crate::user::{ChannelUsers, Nick, NickRef};
use crate::{
    Server, User, buffer, compression, config, ctcp, dcc, environment,
    file_transfer, isupport, message, mode, server,
};

pub mod on_connect;

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
pub enum Broadcast {
    Quit {
        user: User,
        comment: Option<String>,
        channels: Vec<target::Channel>,
        sent_time: DateTime<Utc>,
    },
    Nickname {
        old_user: User,
        new_nick: Nick,
        ourself: bool,
        channels: Vec<target::Channel>,
        sent_time: DateTime<Utc>,
    },
    Invite {
        inviter: User,
        channel: target::Channel,
        user_channels: Vec<target::Channel>,
        sent_time: DateTime<Utc>,
    },
    ChangeHost {
        old_user: User,
        new_username: String,
        new_hostname: String,
        ourself: bool,
        logged_in: bool,
        channels: Vec<target::Channel>,
        sent_time: DateTime<Utc>,
    },
}

#[derive(Debug)]
pub enum Message {
    ChatHistoryRequest(Server, ChatHistorySubcommand),
    ChatHistoryTargetsTimestampUpdated(
        Server,
        DateTime<Utc>,
        Result<(), Error>,
    ),
    RequestNewerChatHistory(Server, Target, DateTime<Utc>),
    RequestChatHistoryTargets(Server, Option<DateTime<Utc>>, DateTime<Utc>),
}

#[derive(Debug)]
pub enum Event {
    Single(message::Encoded, Nick),
    PrivOrNotice(message::Encoded, Nick, bool),
    WithTarget(message::Encoded, Nick, message::Target),
    Broadcast(Broadcast),
    FileTransferRequest(file_transfer::ReceiveRequest),
    UpdateReadMarker(Target, ReadMarker),
    JoinedChannel(target::Channel, DateTime<Utc>),
    LoggedIn(DateTime<Utc>),
    ChatHistoryTargetReceived(Target, DateTime<Utc>),
    ChatHistoryTargetsReceived(DateTime<Utc>),
    DirectMessage(message::Encoded, Nick, User),
    MonitoredOnline(Vec<User>),
    MonitoredOffline(Vec<Nick>),
    OnConnect(on_connect::Stream),
}

struct ChatHistoryRequest {
    subcommand: ChatHistorySubcommand,
    requested_at: Instant,
}

pub struct Client {
    server: Server,
    config: Arc<config::Server>,
    handle: server::Handle,
    alt_nick: Option<usize>,
    resolved_nick: Option<String>,
    chanmap: IndexMap<target::Channel, Channel>,
    resolved_queries: HashSet<target::Query>,
    labels: HashMap<String, Context>,
    batches: HashMap<Target, Batch>,
    reroute_responses_to: Option<buffer::Upstream>,
    logged_in: bool,
    registration_step: RegistrationStep,
    listed_caps: Vec<String>,
    supports_labels: bool,
    supports_away_notify: bool,
    supports_account_notify: bool,
    supports_extended_join: bool,
    supports_read_marker: bool,
    supports_chathistory: bool,
    chathistory_requests: HashMap<Target, ChatHistoryRequest>,
    chathistory_exhausted: HashMap<Target, bool>,
    chathistory_targets_request: Option<ChatHistoryRequest>,
    highlight_notification_blackout: HighlightNotificationBlackout,
    registration_required_channels: Vec<target::Channel>,
    isupport: HashMap<isupport::Kind, isupport::Parameter>,
    who_polls: VecDeque<WhoPoll>,
    who_poll_interval: BackoffInterval,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish()
    }
}

impl Client {
    pub fn new(
        server: Server,
        config: Arc<config::Server>,
        sender: mpsc::Sender<proto::Message>,
    ) -> Self {
        Self {
            server,
            handle: sender,
            resolved_nick: None,
            alt_nick: None,
            chanmap: IndexMap::default(),
            resolved_queries: HashSet::new(),
            labels: HashMap::new(),
            batches: HashMap::new(),
            reroute_responses_to: None,
            logged_in: false,
            registration_step: RegistrationStep::Start,
            listed_caps: vec![],
            supports_labels: false,
            supports_away_notify: false,
            supports_account_notify: false,
            supports_extended_join: false,
            supports_read_marker: false,
            supports_chathistory: false,
            chathistory_requests: HashMap::new(),
            chathistory_exhausted: HashMap::new(),
            chathistory_targets_request: None,
            highlight_notification_blackout:
                HighlightNotificationBlackout::Blackout(Instant::now()),
            registration_required_channels: vec![],
            isupport: HashMap::new(),
            who_polls: VecDeque::new(),
            who_poll_interval: BackoffInterval::from_duration(
                config.who_poll_interval,
            ),
            config,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        // Begin registration
        self.handle.try_send(command!("CAP", "LS", "302"))?;

        // Identify
        let nick = &self.config.nickname;
        let user = self.config.username.as_ref().unwrap_or(nick);
        let real = self.config.realname.as_ref().unwrap_or(nick);

        if let Some(pass) = self.config.password.as_ref() {
            self.handle.try_send(command!("PASS", pass))?;
        }
        self.handle.try_send(command!("NICK", nick))?;
        self.handle.try_send(command!("USER", user, real))?;
        self.registration_step = RegistrationStep::List;
        Ok(())
    }

    fn quit(&mut self, reason: Option<String>) {
        self.who_polls.retain(|who_poll| {
            matches!(
                who_poll.status,
                WhoStatus::Requested(_, _, _) | WhoStatus::Receiving(_, _)
            )
        });

        if let Err(e) = if let Some(reason) = reason {
            self.handle.try_send(command!("QUIT", reason))
        } else {
            self.handle.try_send(command!("QUIT"))
        } {
            log::warn!("Error sending quit: {e}");
        }
    }

    fn join(&mut self, channels: &[target::Channel]) {
        let keys = HashMap::new();

        let messages = group_joins(channels, &keys);

        for message in messages {
            if let Err(e) = self.handle.try_send(message) {
                log::warn!("Error sending join: {e}");
            }
        }
    }

    fn start_reroute(&self, command: &Command) -> bool {
        use Command::*;

        if let MODE(target, _, _) = command {
            !self.is_channel(target)
        } else {
            matches!(command, WHO(..) | WHOIS(..) | WHOWAS(..))
        }
    }

    fn stop_reroute(&self, command: &Command) -> bool {
        use command::Numeric::*;

        match &command {
            Command::Numeric(RPL_ENDOFWHO, args) => {
                let mask = args.get(1).cloned().unwrap_or_default();

                if let Ok(target_channel) = target::Channel::parse(
                    &mask,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ) {
                    self.user_who_request(&target_channel)
                    // Some servers respond with the mask * instead of the requested
                    // channel name when rate-limiting WHO requests
                } else if mask == "*" {
                    // Either the user requested the mask, in which case the request
                    // should be in who_polls and rerouting should be stopped, or it
                    // is treated as part of a rate-limiting response and if there
                    // are any outstanding user requests rerouting should be stopped.
                    self.who_polls.iter().any(|who_poll| {
                        matches!(
                            who_poll.status,
                            WhoStatus::Requested(WhoSource::User, _, _)
                                | WhoStatus::Receiving(WhoSource::User, _)
                        )
                    })
                } else {
                    let target_channel =
                        target::Channel::from_str(&mask, self.casemapping());

                    self.user_who_request(&target_channel)
                }
            }
            _ => matches!(
                command,
                Command::Numeric(
                    RPL_ENDOFWHOIS
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
            ),
        }
    }

    fn send(
        &mut self,
        buffer: &buffer::Upstream,
        mut message: message::Encoded,
    ) {
        if self.supports_labels {
            let label = generate_label();
            let context = Context::new(&message, buffer.clone());

            self.labels.insert(label.clone(), context);

            // IRC: Encode tags
            message.tags = tags!["label" => label];
        }

        self.reroute_responses_to =
            self.start_reroute(&message.command).then(|| buffer.clone());

        if matches!(message.command, Command::WHO(..)) {
            let params = message.command.clone().parameters();

            if let Some(mask) = params.first() {
                let channel = if let Ok(channel) = target::Channel::parse(
                    mask,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ) {
                    Some(channel)
                } else if mask == "*" {
                    Some(target::Channel::from_str(mask, self.casemapping()))
                } else {
                    None
                };

                if let Some(channel) = channel {
                    // Record user WHO request(s) for reply filtering
                    let status = WhoStatus::Requested(
                        WhoSource::User,
                        Instant::now(),
                        params
                            .get(2)
                            .and_then(|token| token.parse::<WhoToken>().ok()),
                    );

                    if let Some(who_poll) = self
                        .who_polls
                        .iter_mut()
                        .find(|who_poll| who_poll.channel == channel)
                    {
                        who_poll.status = status;
                    } else {
                        self.who_polls.push_front(WhoPoll { channel, status });
                    }
                }
            }
        }

        if let Err(e) = self.handle.try_send(message.into()) {
            log::warn!("Error sending message: {e}");
        }
    }

    fn receive(
        &mut self,
        message: message::Encoded,
        ctcp_config: &config::Ctcp,
    ) -> Result<Vec<Event>> {
        log::trace!("Message received => {:?}", *message);

        let stop_reroute = self.stop_reroute(&message.command);

        let events = self.handle(message, None, ctcp_config)?;

        if stop_reroute {
            self.reroute_responses_to = None;
        }

        Ok(events)
    }

    fn handle(
        &mut self,
        mut message: message::Encoded,
        parent_context: Option<Context>,
        ctcp_config: &config::Ctcp,
    ) -> Result<Vec<Event>> {
        use irc::proto::command::Numeric::*;

        let label_tag = message.tags.remove("label");
        let batch_tag = message.tags.remove("batch");

        let context = parent_context.or_else(|| {
            label_tag
                // Remove context associated to label if we get resp for it
                .and_then(|label| self.labels.remove(&label))
                // Otherwise if we're in a batch, get it's context
                .or_else(|| {
                    batch_tag.as_ref().and_then(|batch| {
                        self.batches
                            .get(&Target::parse(
                                batch,
                                self.chantypes(),
                                self.statusmsg(),
                                self.casemapping(),
                            ))
                            .and_then(|batch| batch.context.clone())
                    })
                })
        });

        macro_rules! ok {
            ($option:expr) => {
                $option.ok_or_else(|| {
                    anyhow!(
                        "[{}] Malformed command: {:?}",
                        self.server,
                        message.command
                    )
                })?
            };
        }

        macro_rules! context {
            ($result:expr) => {
                $result.with_context(|| {
                    anyhow!(
                        "[{}] Malformed command: {:?}",
                        self.server,
                        message.command
                    )
                })?
            };
        }

        match &message.command {
            Command::BATCH(batch, params) => {
                let mut chars = batch.chars();
                let symbol = ok!(chars.next());
                let reference = chars.collect::<String>();

                match symbol {
                    '+' => {
                        let mut batch = Batch::new(context);

                        batch.chathistory =
                            match params.first().map(String::as_str) {
                                Some("chathistory") => {
                                    params.get(1).map(|target| {
                                        ChatHistoryBatch::Target(Target::parse(
                                            target,
                                            self.chantypes(),
                                            self.statusmsg(),
                                            self.casemapping(),
                                        ))
                                    })
                                }
                                Some("draft/chathistory-targets") => {
                                    Some(ChatHistoryBatch::Targets)
                                }
                                _ => None,
                            };

                        self.batches.insert(
                            Target::parse(
                                &reference,
                                self.chantypes(),
                                self.statusmsg(),
                                self.casemapping(),
                            ),
                            batch,
                        );
                    }
                    '-' => {
                        if let Some(mut finished) =
                            self.batches.remove(&Target::parse(
                                &reference,
                                self.chantypes(),
                                self.statusmsg(),
                                self.casemapping(),
                            ))
                        {
                            // If nested, extend events into parent batch
                            if let Some(parent) =
                                batch_tag.as_ref().and_then(|batch| {
                                    self.batches.get_mut(&Target::parse(
                                        batch,
                                        self.chantypes(),
                                        self.statusmsg(),
                                        self.casemapping(),
                                    ))
                                })
                            {
                                parent.events.extend(finished.events);
                            } else {
                                match &finished.chathistory {
                                    Some(ChatHistoryBatch::Target(
                                        batch_target,
                                    )) => {
                                        let continuation_subcommand =
                                            if let Some(ChatHistoryRequest {
                                                subcommand,
                                                ..
                                            }) = self
                                                .chathistory_requests
                                                .get(batch_target)
                                            {
                                                if let ChatHistorySubcommand::Before(_, _, limit)
                                            | ChatHistorySubcommand::Latest(
                                                _,
                                                MessageReference::None,
                                                limit,
                                            ) = subcommand
                                            {
                                                self.chathistory_exhausted.insert(
                                                    batch_target.clone(),
                                                    finished.events.len() < *limit as usize,
                                                );
                                            }

                                                match subcommand {
                                                ChatHistorySubcommand::Latest(
                                                    target,
                                                    message_reference,
                                                    limit,
                                                ) => {
                                                    log::debug!(
                                                        "[{}] received latest {} messages in {} since {}",
                                                        self.server,
                                                        finished.events.len(),
                                                        target,
                                                        message_reference,
                                                    );

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
                                                ChatHistorySubcommand::Before(
                                                    target,
                                                    message_reference,
                                                    _,
                                                ) => {
                                                    log::debug!(
                                                        "[{}] received {} messages in {} before {}",
                                                        self.server,
                                                        finished.events.len(),
                                                        target,
                                                        message_reference,
                                                    );

                                                    None
                                                }
                                                ChatHistorySubcommand::Between(
                                                    target,
                                                    start_message_reference,
                                                    end_message_reference,
                                                    limit,
                                                ) => {
                                                    log::debug!(
                                                        "[{}] received {} messages in {} between {} and {}",
                                                        self.server,
                                                        finished.events.len(),
                                                        target,
                                                        start_message_reference,
                                                        end_message_reference,
                                                    );

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
                                                ChatHistorySubcommand::Targets(_, _, _) => {
                                                    log::debug!(
                                                        "[{}] chathistory batch received for TARGETS request (draft/chathistory-targets batch expected)",
                                                        self.server
                                                    );

                                                    None
                                                }
                                            }
                                            } else {
                                                None
                                            };

                                        self.clear_chathistory_request(Some(
                                            batch_target,
                                        ));

                                        if let Some(continuation_subcommand) =
                                            continuation_subcommand
                                        {
                                            self.send_chathistory_request(
                                                continuation_subcommand,
                                            );
                                        }
                                    }
                                    Some(ChatHistoryBatch::Targets) => {
                                        if let Some(ChatHistoryRequest {
                                            subcommand,
                                            ..
                                        }) =
                                            &self.chathistory_targets_request
                                        {
                                            if let ChatHistorySubcommand::Targets(
                                                start_message_reference,
                                                end_message_reference,
                                                _,
                                            ) = subcommand
                                            {
                                                log::debug!(
                                                    "[{}] received {} targets between {} and {}",
                                                    self.server,
                                                    finished.events.len(),
                                                    start_message_reference,
                                                    end_message_reference,
                                                );
                                            }

                                            finished.events.push(
                                                Event::ChatHistoryTargetsReceived(server_time(
                                                    &message,
                                                )),
                                            );
                                        }

                                        self.clear_chathistory_request(None);
                                    }
                                    _ => (),
                                }

                                return Ok(finished.events);
                            }
                        }
                    }
                    _ => {}
                }

                return Ok(vec![]);
            }
            _ if batch_tag.is_some() => {
                let events = if let Some(batch_target) = batch_tag
                    .as_ref()
                    .and_then(|batch| {
                        self.batches.get(&Target::parse(
                            batch,
                            self.chantypes(),
                            self.statusmsg(),
                            self.casemapping(),
                        ))
                    })
                    .and_then(|batch| {
                        batch
                            .chathistory
                            .as_ref()
                            .and_then(ChatHistoryBatch::target)
                    })
                    .and_then(|target| {
                        match self.chathistory_requests.contains_key(&target) {
                            true => Some(target),
                            false => None,
                        }
                    }) {
                    if Some(User::from(Nick::from("HistServ")))
                        == message.user()
                    {
                        // HistServ provides event-playback without event-playback
                        // which would require client-side parsing to map appropriately.
                        // Avoid that complexity by only providing that functionality
                        // via event-playback.
                        vec![]
                    } else {
                        match &message.command {
                            Command::NICK(_) => batch_target
                                .as_channel()
                                .map(|channel| {
                                    let target = message::Target::Channel {
                                        channel: channel.clone(),
                                        source: source::Source::Server(None),
                                    };

                                    vec![Event::WithTarget(
                                        message,
                                        self.nickname().to_owned(),
                                        target,
                                    )]
                                })
                                .unwrap_or_default(),
                            Command::QUIT(_) => batch_target
                                .as_channel()
                                .map(|channel| {
                                    let target = message::Target::Channel {
                                        channel: channel.clone(),
                                        source: source::Source::Server(Some(
                                            source::Server::new(
                                                source::server::Kind::Quit,
                                                message.user().map(|user| {
                                                    Nick::from(
                                                        user.nickname()
                                                            .as_ref(),
                                                    )
                                                }),
                                            ),
                                        )),
                                    };

                                    vec![Event::WithTarget(
                                        message,
                                        self.nickname().to_owned(),
                                        target,
                                    )]
                                })
                                .unwrap_or_default(),
                            Command::PRIVMSG(target, text)
                            | Command::NOTICE(target, text) => {
                                if ctcp::is_query(text)
                                    && !message::is_action(text)
                                {
                                    // Ignore historical CTCP queries/responses except for ACTIONs
                                    vec![]
                                } else {
                                    if let Some(user) = message.user() {
                                        // If direct message, update resolved queries with user
                                        if target
                                            == &self.nickname().to_string()
                                        {
                                            self.resolved_queries.replace(
                                                target::Query::from_user(
                                                    &user,
                                                    self.casemapping(),
                                                ),
                                            );
                                        }
                                    }

                                    vec![Event::PrivOrNotice(
                                        message,
                                        self.nickname().to_owned(),
                                        // Don't allow highlight notifications from history
                                        false,
                                    )]
                                }
                            }
                            _ => vec![Event::Single(
                                message,
                                self.nickname().to_owned(),
                            )],
                        }
                    }
                } else {
                    self.handle(message, context, ctcp_config)?
                };

                if let Some(batch) = self.batches.get_mut(&Target::parse(
                    &batch_tag.unwrap(),
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                )) {
                    batch.events.extend(events);
                    return Ok(vec![]);
                } else {
                    return Ok(events);
                }
            }
            // Label context whois
            _ if context.as_ref().is_some_and(Context::is_whois) => {
                if let Some(source) = context
                    .map(Context::buffer)
                    .map(|buffer| buffer.server_message_target(None))
                {
                    return Ok(vec![Event::WithTarget(
                        message,
                        self.nickname().to_owned(),
                        source,
                    )]);
                }
            }
            // Reroute whois, whowas, and user mode responses
            Command::Numeric(
                RPL_WHOISCERTFP | RPL_WHOISREGNICK | RPL_WHOISUSER
                | RPL_WHOISSERVER | RPL_WHOISOPERATOR | RPL_WHOISIDLE
                | RPL_WHOISCHANNELS | RPL_WHOISSPECIAL | RPL_WHOISACCOUNT
                | RPL_WHOISACTUALLY | RPL_WHOISHOST | RPL_WHOISMODES
                | RPL_WHOISSECURE | RPL_AWAY | RPL_ENDOFWHOIS | RPL_WHOWASUSER
                | RPL_ENDOFWHOWAS | RPL_UMODEIS | ERR_NOSUCHNICK
                | ERR_NOSUCHSERVER | ERR_NONICKNAMEGIVEN | ERR_WASNOSUCHNICK
                | ERR_NEEDMOREPARAMS | ERR_USERSDONTMATCH
                | ERR_UMODEUNKNOWNFLAG,
                _,
            ) if self.reroute_responses_to.is_some() => {
                if let Some(source) = self
                    .reroute_responses_to
                    .clone()
                    .map(|buffer| buffer.server_message_target(None))
                {
                    return Ok(vec![Event::WithTarget(
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
                    (None, None) | (None, Some(_)) => return Ok(vec![]),
                };

                self.listed_caps.extend(caps.split(' ').map(String::from));

                // Finished
                if asterisk.is_none() {
                    let mut requested = vec![];

                    let contains =
                        |s| self.listed_caps.iter().any(|cap| cap == s);

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
                        if contains("draft/chathistory") {
                            requested.push("draft/chathistory");

                            if contains("draft/event-playback") {
                                requested.push("draft/event-playback");
                            }
                        }
                    }
                    if contains("labeled-response") {
                        requested.push("labeled-response");
                    }
                    if contains("echo-message") {
                        requested.push("echo-message");
                    }
                    if self
                        .listed_caps
                        .iter()
                        .any(|cap| cap.starts_with("sasl"))
                    {
                        requested.push("sasl");
                    }
                    if contains("multi-prefix") {
                        requested.push("multi-prefix");
                    }
                    if contains("draft/read-marker") {
                        requested.push("draft/read-marker");
                    }
                    if contains("setname") {
                        requested.push("setname");
                    }

                    if !requested.is_empty() {
                        // Request
                        self.registration_step = RegistrationStep::Req;

                        for message in group_capability_requests(&requested) {
                            self.handle.try_send(message)?;
                        }
                    } else {
                        // If none requested, end negotiation
                        self.registration_step = RegistrationStep::End;
                        self.handle.try_send(command!("CAP", "END"))?;
                    }
                }
            }
            Command::CAP(_, sub, a, b) if sub == "ACK" => {
                // TODO this code is duplicated several times. Fix in `Command`.
                let caps = ok!(b.as_ref().or(a.as_ref()));

                log::info!(
                    "[{}] capabilities acknowledged: {caps}",
                    self.server
                );

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
                if caps.contains(&"draft/read-marker") {
                    self.supports_read_marker = true;
                }

                let supports_sasl = caps.iter().any(|cap| cap.contains("sasl"));

                if let Some(sasl) =
                    self.config.sasl.as_ref().filter(|_| supports_sasl)
                {
                    self.registration_step = RegistrationStep::Sasl;
                    self.handle
                        .try_send(command!("AUTHENTICATE", sasl.command()))?;
                } else {
                    self.registration_step = RegistrationStep::End;
                    self.handle.try_send(command!("CAP", "END"))?;
                }

                if caps.contains(&"draft/chathistory")
                    && self.config.chathistory
                {
                    self.supports_chathistory = true;
                }
            }
            Command::CAP(_, sub, a, b) if sub == "NAK" => {
                let caps = ok!(b.as_ref().or(a.as_ref()));

                log::warn!(
                    "[{}] capabilities not acknowledged: {caps}",
                    self.server
                );

                // End if we didn't move to sasl or already ended
                if self.registration_step < RegistrationStep::Sasl {
                    self.registration_step = RegistrationStep::End;
                    self.handle.try_send(command!("CAP", "END"))?;
                }
            }
            Command::CAP(_, sub, a, b) if sub == "NEW" => {
                let caps = ok!(b.as_ref().or(a.as_ref()));

                let new_caps =
                    caps.split(' ').map(String::from).collect::<Vec<String>>();

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
                if contains("account-notify")
                    || newly_contains("account-notify")
                {
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
                    if newly_contains("draft/chathistory")
                        && self.config.chathistory
                    {
                        requested.push("draft/chathistory");

                        if newly_contains("draft/event-playback") {
                            requested.push("draft/event-playback");
                        }
                    }
                }
                if newly_contains("labeled-response") {
                    requested.push("labeled-response");
                }
                if newly_contains("echo-message") {
                    requested.push("echo-message");
                }
                if newly_contains("multi-prefix") {
                    requested.push("multi-prefix");
                }
                if newly_contains("draft/read-marker") {
                    requested.push("draft/read-marker");
                }
                if newly_contains("setname") {
                    requested.push("setname");
                }

                if !requested.is_empty() {
                    for message in group_capability_requests(&requested) {
                        self.handle.try_send(message)?;
                    }
                }

                self.listed_caps.extend(new_caps);
            }
            Command::CAP(_, sub, a, b) if sub == "DEL" => {
                let caps = ok!(b.as_ref().or(a.as_ref()));

                log::info!(
                    "[{}] capabilities no longer supported: {caps}",
                    self.server
                );

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
                if del_caps.contains(&"draft/read-marker") {
                    self.supports_read_marker = false;
                }
                if del_caps.contains(&"draft/chathistory") {
                    self.supports_chathistory = false;
                }

                self.listed_caps.retain(|cap| {
                    !del_caps.iter().any(|del_cap| del_cap == cap)
                });
            }
            Command::AUTHENTICATE(param) if param == "+" => {
                if let Some(sasl) = self.config.sasl.as_ref() {
                    log::info!(
                        "[{}] sasl auth: {}",
                        self.server,
                        sasl.command()
                    );

                    for param in sasl.params() {
                        self.handle
                            .try_send(command!("AUTHENTICATE", param))?;
                    }
                }
            }
            Command::Numeric(RPL_LOGGEDIN, args) => {
                log::info!("[{}] logged in", self.server);

                self.logged_in = true;

                if !self.registration_required_channels.is_empty() {
                    for message in group_joins(
                        &self.registration_required_channels,
                        &self.config.channel_keys,
                    ) {
                        self.handle.try_send(message)?;
                    }

                    self.registration_required_channels.clear();
                }

                if !self.supports_account_notify {
                    let accountname = ok!(args.get(2));

                    let old_user = User::from(self.nickname().to_owned());

                    self.chanmap.values_mut().for_each(|channel| {
                        if let Some(user) = channel.users.take(&old_user) {
                            channel
                                .users
                                .insert(user.with_accountname(accountname));
                        }
                    });
                }

                return Ok(vec![Event::LoggedIn(server_time(&message))]);
            }
            Command::Numeric(RPL_LOGGEDOUT, _) => {
                log::info!("[{}] logged out", self.server);

                self.logged_in = false;

                if !self.supports_account_notify {
                    let old_user = User::from(self.nickname().to_owned());

                    self.chanmap.values_mut().for_each(|channel| {
                        if let Some(user) = channel.users.take(&old_user) {
                            channel.users.insert(user.with_accountname("*"));
                        }
                    });
                }
            }
            Command::PRIVMSG(target, text) | Command::NOTICE(target, text) => {
                if let Some(user) = message.user() {
                    let is_echo = user.nickname() == self.nickname();

                    let dcc_command = dcc::decode(text);
                    let ctcp_query = ctcp::parse_query(text);

                    // DCC Handling
                    if let Some(command) = dcc_command {
                        // Ignore echoed DCC messages
                        if is_echo {
                            return Ok(vec![]);
                        }

                        match command {
                            dcc::Command::Send(request) => {
                                log::trace!("DCC Send => {request:?}");
                                return Ok(vec![Event::FileTransferRequest(
                                    file_transfer::ReceiveRequest {
                                        from: user,
                                        dcc_send: request,
                                        server: self.server.clone(),
                                        server_handle: self.handle.clone(),
                                    },
                                )]);
                            }
                            dcc::Command::Unsupported(command) => {
                                bail!("Unsupported DCC command: {command}",);
                            }
                        }
                    };

                    // CTCP Handling
                    if let Some(query) = ctcp_query {
                        let is_action = message::is_action(text);

                        // Ignore CTCP Action queries.
                        if !is_action && !is_echo {
                            // Response to us sending a CTCP request to another client
                            if matches!(&message.command, Command::NOTICE(_, _))
                            {
                                let event = Event::PrivOrNotice(
                                    message,
                                    self.nickname().to_owned(),
                                    self.highlight_notification_blackout
                                        .allowed(),
                                );

                                return Ok(vec![event]);
                            }

                            // Response to a client sending us a CTCP request
                            if matches!(
                                &message.command,
                                Command::PRIVMSG(_, _)
                            ) {
                                match query.command {
                                    ctcp::Command::Action => (),
                                    ctcp::Command::ClientInfo => {
                                        self.handle.try_send(
                                            ctcp::response_message(
                                                &query.command,
                                                user.nickname().to_string(),
                                                Some(ctcp_config.client_info()),
                                            ),
                                        )?;
                                    }
                                    ctcp::Command::DCC => (),
                                    ctcp::Command::Ping => {
                                        if ctcp_config.ping {
                                            self.handle.try_send(
                                                ctcp::response_message(
                                                    &query.command,
                                                    user.nickname().to_string(),
                                                    query.params,
                                                ),
                                            )?;
                                        }
                                    }
                                    ctcp::Command::Source => {
                                        if ctcp_config.source {
                                            self.handle.try_send(
                                                ctcp::response_message(
                                                    &query.command,
                                                    user.nickname().to_string(),
                                                    Some(SOURCE_WEBSITE),
                                                ),
                                            )?;
                                        }
                                    }
                                    ctcp::Command::Version => {
                                        if ctcp_config.version {
                                            self.handle.try_send(
                                                ctcp::response_message(
                                                    &query.command,
                                                    user.nickname().to_string(),
                                                    Some(format!(
                                                        "Halloy {VERSION}"
                                                    )),
                                                ),
                                            )?;
                                        }
                                    }
                                    ctcp::Command::Time => {
                                        if ctcp_config.time {
                                            let utc_time = Utc::now();
                                            let formatted = utc_time
                                                .to_rfc3339_opts(
                                                chrono::SecondsFormat::Millis,
                                                true,
                                            );

                                            self.handle.try_send(
                                                ctcp::response_message(
                                                    &query.command,
                                                    user.nickname().to_string(),
                                                    Some(formatted),
                                                ),
                                            )?;
                                        }
                                    }
                                    ctcp::Command::Unknown(command) => {
                                        log::debug!(
                                            "Ignoring CTCP command {command}: Unknown command"
                                        );
                                    }
                                }
                            }

                            return Ok(vec![]);
                        }
                    }

                    // use `target` to confirm the direct message
                    let direct_message = target == &self.nickname().to_string();

                    if direct_message {
                        self.resolved_queries.replace(
                            target::Query::from_user(&user, self.casemapping()),
                        );
                    }

                    let event = Event::PrivOrNotice(
                        message.clone(),
                        self.nickname().to_owned(),
                        self.highlight_notification_blackout.allowed(),
                    );

                    if direct_message {
                        return Ok(vec![
                            event,
                            Event::DirectMessage(
                                message,
                                self.nickname().to_owned(),
                                user,
                            ),
                        ]);
                    } else {
                        return Ok(vec![event]);
                    }
                }
            }
            Command::INVITE(user, channel) => {
                let user = User::from(Nick::from(user.as_str()));
                let channel = context!(target::Channel::parse(
                    channel,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ));
                let inviter = ok!(message.user());
                let user_channels = self.user_channels(user.nickname());

                return Ok(vec![Event::Broadcast(Broadcast::Invite {
                    inviter,
                    channel,
                    user_channels,
                    sent_time: server_time(&message),
                })]);
            }
            Command::NICK(nick) => {
                let old_user = ok!(message.user());
                let ourself = self.nickname() == old_user.nickname();

                if ourself {
                    self.resolved_nick = Some(nick.to_string());
                }

                let new_nick = Nick::from(nick.as_str());

                self.chanmap.values_mut().for_each(|channel| {
                    if let Some(user) = channel.users.take(&old_user) {
                        channel
                            .users
                            .insert(user.with_nickname(new_nick.clone()));
                    }
                });

                let channels = self.user_channels(old_user.nickname());

                return Ok(vec![Event::Broadcast(Broadcast::Nickname {
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
                    None if !self.config.alt_nicks.is_empty() => {
                        self.alt_nick = Some(0);
                    }
                    None => {}
                }

                if let Some(nick) =
                    self.alt_nick.and_then(|i| self.config.alt_nicks.get(i))
                {
                    self.handle.try_send(command!("NICK", nick))?;
                }
            }
            Command::Numeric(RPL_WELCOME, args) => {
                // Updated actual nick
                let nick = ok!(args.first());
                self.resolved_nick = Some(nick.to_string());
            }
            // QUIT
            Command::QUIT(comment) => {
                let user = ok!(message.user());

                self.chanmap.values_mut().for_each(|channel| {
                    channel.users.remove(&user);
                });

                let channels = self.user_channels(user.nickname());

                return Ok(vec![Event::Broadcast(Broadcast::Quit {
                    user,
                    comment: comment.clone(),
                    channels,
                    sent_time: server_time(&message),
                })]);
            }
            Command::PART(channel, _) => {
                let user = ok!(message.user());

                if user.nickname() == self.nickname() {
                    self.chanmap.shift_remove(&context!(
                        target::Channel::parse(
                            channel,
                            self.chantypes(),
                            self.statusmsg(),
                            self.casemapping(),
                        )
                    ));
                } else if let Some(channel) =
                    self.chanmap.get_mut(&context!(target::Channel::parse(
                        channel,
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    )))
                {
                    channel.users.remove(&user);
                }
            }
            Command::JOIN(channel, accountname) => {
                let user = ok!(message.user());

                let target_channel = context!(target::Channel::parse(
                    channel,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ));

                if user.nickname() == self.nickname() {
                    // TODO(pounce) change to `insert_sorted_by` when merged
                    let (Ok(i) | Err(i)) =
                        self.chanmap.binary_search_by(|c, _| {
                            self.compare_channels(
                                c.as_normalized_str(),
                                target_channel.as_normalized_str(),
                            )
                        });
                    self.chanmap.insert_before(
                        i,
                        target_channel.clone(),
                        Channel::default(),
                    );

                    // Add channel to WHO poll queue
                    if !self
                        .who_polls
                        .iter()
                        .any(|who_poll| who_poll.channel == target_channel)
                    {
                        self.who_polls.push_back(WhoPoll {
                            channel: target_channel.clone(),
                            status: WhoStatus::Joined,
                        });
                    }

                    return Ok(vec![Event::JoinedChannel(
                        target_channel,
                        server_time(&message),
                    )]);
                } else if let Some(channel) =
                    self.chanmap.get_mut(&target_channel)
                {
                    let user = if self.supports_extended_join {
                        accountname
                            .as_ref()
                            .map_or(user.clone(), |accountname| {
                                user.with_accountname(accountname)
                            })
                    } else {
                        user
                    };

                    channel.users.insert(user);
                }
            }
            Command::KICK(channel, victim, _) => {
                if victim == self.nickname().as_ref() {
                    self.chanmap.shift_remove(&context!(
                        target::Channel::parse(
                            channel,
                            self.chantypes(),
                            self.statusmsg(),
                            self.casemapping(),
                        )
                    ));
                } else if let Some(channel) =
                    self.chanmap.get_mut(&context!(target::Channel::parse(
                        channel,
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    )))
                {
                    channel
                        .users
                        .remove(&User::from(Nick::from(victim.as_str())));
                }
            }
            Command::Numeric(RPL_WHOREPLY, args) => {
                let channel = ok!(args.get(1));

                if let Ok(target_channel) = target::Channel::parse(
                    channel,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ) {
                    let user_request = self.user_who_request(&target_channel);

                    if let Some(client_channel) =
                        self.chanmap.get_mut(&target_channel)
                    {
                        client_channel.update_user_away(
                            ok!(args.get(5)),
                            ok!(args.get(6)),
                        );

                        if let Some(who_poll) = self
                            .who_polls
                            .iter_mut()
                            .find(|who_poll| who_poll.channel == target_channel)
                        {
                            if let WhoStatus::Requested(source, _, None) =
                                &who_poll.status
                            {
                                who_poll.status =
                                    WhoStatus::Receiving(source.clone(), None);
                                log::debug!(
                                    "[{}] {channel} - WHO receiving...",
                                    self.server
                                );
                            }
                        }
                    }

                    if !user_request {
                        // User did not request, don't save to history
                        return Ok(vec![]);
                    // Reroute who responses
                    } else if let Some(source) = self
                        .reroute_responses_to
                        .clone()
                        .map(|buffer| buffer.server_message_target(None))
                    {
                        return Ok(vec![Event::WithTarget(
                            message,
                            self.nickname().to_owned(),
                            source,
                        )]);
                    }
                }
            }
            Command::Numeric(RPL_WHOSPCRPL, args) => {
                let channel = ok!(args.get(2));

                if let Ok(target_channel) = target::Channel::parse(
                    channel,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ) {
                    let user_request = self.user_who_request(&target_channel);

                    if let Some(client_channel) =
                        self.chanmap.get_mut(&target_channel)
                    {
                        if let Some(who_poll) = self
                            .who_polls
                            .iter_mut()
                            .find(|who_poll| who_poll.channel == target_channel)
                        {
                            match &who_poll.status {
                                WhoStatus::Requested(
                                    source,
                                    _,
                                    Some(request_token),
                                ) if matches!(source, WhoSource::Poll) => {
                                    if let Ok(token) =
                                        ok!(args.get(1)).parse::<WhoToken>()
                                    {
                                        if *request_token == token {
                                            who_poll.status =
                                                WhoStatus::Receiving(
                                                    source.clone(),
                                                    Some(*request_token),
                                                );
                                            log::debug!(
                                                "[{}] {channel} - WHO receiving...",
                                                self.server
                                            );
                                        }
                                    }
                                }
                                WhoStatus::Requested(
                                    WhoSource::User,
                                    _,
                                    Some(request_token),
                                ) => {
                                    who_poll.status = WhoStatus::Receiving(
                                        WhoSource::User,
                                        Some(*request_token),
                                    );

                                    log::debug!(
                                        "[{}] {channel} - WHO receiving...",
                                        self.server
                                    );
                                }
                                _ => (),
                            }

                            if let WhoStatus::Receiving(
                                WhoSource::Poll,
                                Some(_),
                            ) = &who_poll.status
                            {
                                // Check token to ~ensure reply is to poll request
                                if let Ok(token) =
                                    ok!(args.get(1)).parse::<WhoToken>()
                                {
                                    if token
                                        == WhoXPollParameters::Default.token()
                                    {
                                        client_channel.update_user_away(
                                            ok!(args.get(3)),
                                            ok!(args.get(4)),
                                        );
                                    } else if token
                                        == WhoXPollParameters::WithAccountName
                                            .token()
                                    {
                                        let user = ok!(args.get(3));

                                        client_channel.update_user_away(
                                            user,
                                            ok!(args.get(4)),
                                        );

                                        client_channel.update_user_accountname(
                                            user,
                                            ok!(args.get(5)),
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if !user_request {
                        // User did not request, don't save to history
                        return Ok(vec![]);
                    // Reroute who responses
                    } else if let Some(source) = self
                        .reroute_responses_to
                        .clone()
                        .map(|buffer| buffer.server_message_target(None))
                    {
                        return Ok(vec![Event::WithTarget(
                            message,
                            self.nickname().to_owned(),
                            source,
                        )]);
                    }
                }
            }
            Command::Numeric(RPL_ENDOFWHO, args) => {
                let mask = ok!(args.get(1));

                if let Ok(target_channel) = target::Channel::parse(
                    mask,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ) {
                    let user_request = self.user_who_request(&target_channel);

                    if let Some(pos) = self
                        .who_polls
                        .iter()
                        .position(|who_poll| who_poll.channel == target_channel)
                    {
                        self.who_polls[pos].status = WhoStatus::Received;

                        if let Some(who_poll) = self.who_polls.remove(pos) {
                            if self.chanmap.contains_key(&target_channel)
                                && self.config.who_poll_enabled
                            {
                                self.who_polls.push_back(who_poll);
                            }
                        }

                        // Prioritize WHO requests due to joining a channel
                        if let Some(pos) = self
                            .who_polls
                            .iter()
                            .position(|who_poll| {
                                matches!(who_poll.status, WhoStatus::Joined)
                            })
                            .or(self.who_polls.iter().position(|who_poll| {
                                matches!(who_poll.status, WhoStatus::Received)
                            }))
                        {
                            self.who_polls[pos].status =
                                WhoStatus::Waiting(Instant::now());

                            if pos != 0 {
                                if let Some(who_poll) =
                                    self.who_polls.remove(pos)
                                {
                                    self.who_polls.push_front(who_poll);
                                }
                            }
                        }
                    }

                    log::debug!("[{}] {mask} - WHO done", self.server);

                    if let Some(client_channel) =
                        self.chanmap.get_mut(&target_channel)
                    {
                        client_channel.who_init = true;
                    }

                    if !user_request {
                        self.who_poll_interval.long_enough();

                        // User did not request, don't save to history
                        return Ok(vec![]);
                    // Reroute who responses
                    } else if let Some(source) = self
                        .reroute_responses_to
                        .clone()
                        .map(|buffer| buffer.server_message_target(None))
                    {
                        return Ok(vec![Event::WithTarget(
                            message,
                            self.nickname().to_owned(),
                            source,
                        )]);
                    }
                } else if mask == "*" {
                    // Some servers respond with the mask * instead of the requested
                    // channel name when rate-limiting WHO requests
                    let target_channel =
                        target::Channel::from_str(mask, self.casemapping());

                    if let Some(pos) = self
                        .who_polls
                        .iter()
                        .position(|who_poll| who_poll.channel == target_channel)
                    {
                        self.who_polls.remove(pos);
                    } else {
                        // User did not request, treat as part of rate-limiting response
                        // (in conjunction with RPL_TRYAGAIN) and don't save to history.
                        if let Some(who_poll) = self.who_polls.front_mut() {
                            who_poll.status =
                                WhoStatus::Waiting(Instant::now());
                        }

                        self.who_polls.iter_mut().skip(1).for_each(
                            |who_poll| who_poll.status = WhoStatus::Received,
                        );

                        return Ok(vec![]);
                    }
                }
            }
            Command::AWAY(args) => {
                let away = args.is_some();
                let user = ok!(message.user());

                for channel in self.chanmap.values_mut() {
                    if let Some(mut user) = channel.users.take(&user) {
                        user.update_away(away);
                        channel.users.insert(user);
                    }
                }
            }
            // RPL_UNAWAY is a reply to "/AWAY" from the server
            // for the client/user itself.
            Command::Numeric(RPL_UNAWAY, _) => {
                let user = User::from(self.nickname().to_owned());

                for channel in self.chanmap.values_mut() {
                    if let Some(mut user) = channel.users.take(&user) {
                        user.update_away(false);
                        channel.users.insert(user);
                    }
                }
            }
            // RPL_UNAWAY is a reply to "/AWAY <msg>" from the server
            // for the client/user itself.
            Command::Numeric(RPL_NOWAWAY, _) => {
                let user = User::from(self.nickname().to_owned());

                for channel in self.chanmap.values_mut() {
                    if let Some(mut user) = channel.users.take(&user) {
                        user.update_away(true);
                        channel.users.insert(user);
                    }
                }
            }
            Command::MODE(target, Some(modes), Some(args)) => {
                if let Ok(channel) = target::Channel::parse(
                    target,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ) {
                    let modes = mode::parse::<mode::Channel>(
                        modes,
                        args,
                        self.chanmodes(),
                        self.prefix(),
                    );

                    if let Some(channel) = self.chanmap.get_mut(&channel) {
                        for mode in modes {
                            if let Some((op, lookup)) = mode.operation().zip(
                                mode.arg()
                                    .map(|nick| User::from(Nick::from(nick))),
                            ) {
                                if let Some(mut user) =
                                    channel.users.take(&lookup)
                                {
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
                        let modes = mode::parse::<mode::User>(
                            modes,
                            args,
                            self.chanmodes(),
                            self.prefix(),
                        );

                        if modes.into_iter().any(|mode| {
                            matches!(
                                mode,
                                mode::Mode::Add(mode::User::Registered, None)
                            )
                        }) {
                            for message in group_joins(
                                &self.registration_required_channels,
                                &self.config.channel_keys,
                            ) {
                                self.handle.try_send(message)?;
                            }

                            self.registration_required_channels.clear();
                        }
                    }
                }
            }
            Command::Numeric(RPL_NAMREPLY, args) if args.len() > 3 => {
                let channel = ok!(args.get(2));

                if let Some(channel) =
                    self.chanmap.get_mut(&context!(target::Channel::parse(
                        channel,
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    )))
                {
                    let prefix = isupport::get_prefix(&self.isupport);
                    for user in args[3].split(' ') {
                        if let Ok(user) = User::parse(user, prefix) {
                            channel.users.insert(user);
                        }
                    }

                    // Don't save to history if names list was triggered by JOIN
                    if !channel.names_init {
                        return Ok(vec![]);
                    }
                }
            }
            Command::Numeric(RPL_ENDOFNAMES, args) => {
                let target = ok!(args.get(1));

                let target_channel = context!(target::Channel::parse(
                    target,
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ));

                if let Some(channel) = self.chanmap.get_mut(&target_channel) {
                    if !channel.names_init {
                        channel.names_init = true;

                        return Ok(vec![]);
                    }
                }
            }
            Command::TOPIC(channel, topic) => {
                if let Some(channel) =
                    self.chanmap.get_mut(&context!(target::Channel::parse(
                        channel,
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    )))
                {
                    if let Some(text) = topic {
                        channel.topic.content =
                            Some(message::parse_fragments(text.clone()));
                    }

                    channel.topic.who =
                        message.user().map(|user| user.nickname().to_owned());
                    channel.topic.time = Some(server_time(&message));
                }
            }
            Command::Numeric(RPL_TOPIC, args) => {
                let channel = ok!(args.get(1));

                if let Some(channel) =
                    self.chanmap.get_mut(&context!(target::Channel::parse(
                        channel,
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    )))
                {
                    channel.topic.content = Some(message::parse_fragments(
                        ok!(args.get(2)).to_owned(),
                    ));
                }
                // Exclude topic message from history to prevent spam during dev
                #[cfg(feature = "dev")]
                return Ok(vec![]);
            }
            Command::Numeric(RPL_TOPICWHOTIME, args) => {
                let channel = ok!(args.get(1));

                if let Some(channel) =
                    self.chanmap.get_mut(&context!(target::Channel::parse(
                        channel,
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    )))
                {
                    channel.topic.who =
                        Some(Nick::from(ok!(args.get(2)).as_str()));
                    let timestamp =
                        Posix::from_seconds(ok!(args.get(3)).parse::<u64>()?);
                    channel.topic.time =
                        Some(timestamp.datetime().ok_or_else(|| {
                            anyhow!(
                                "Unable to parse timestamp: {:?}",
                                timestamp
                            )
                        })?);
                }
                // Exclude topic message from history to prevent spam during dev
                #[cfg(feature = "dev")]
                return Ok(vec![]);
            }
            Command::Numeric(RPL_CHANNELMODEIS, args) => {
                let channel = ok!(args.get(1));

                if let Some(channel) =
                    self.chanmap.get_mut(&context!(target::Channel::parse(
                        channel,
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    )))
                {
                    channel.mode = args.get(2).cloned();
                }
            }
            Command::Numeric(ERR_NOCHANMODES, args) => {
                let channel = context!(target::Channel::parse(
                    ok!(args.get(1)),
                    self.chantypes(),
                    self.statusmsg(),
                    self.casemapping(),
                ));

                // If the channel has not been joined but is in the configured channels,
                // then interpret this numeric as ERR_NEEDREGGEDNICK (which has the
                // same number as ERR_NOCHANMODES)
                if !self.chanmap.contains_key(&channel)
                    && self.config.channels.iter().any(|config_channel| {
                        config_channel == channel.as_str()
                    })
                {
                    self.registration_required_channels.push(channel.clone());
                }
            }
            Command::Numeric(RPL_ISUPPORT, args) => {
                let args_len = args.len();
                for (index, arg) in args.iter().enumerate().skip(1) {
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

                                        self.isupport.insert(
                                            kind.clone(),
                                            parameter.clone(),
                                        );

                                        if let isupport::Parameter::MONITOR(
                                            target_limit,
                                        ) = parameter
                                        {
                                            let messages = group_monitors(
                                                &self.config.monitor,
                                                target_limit,
                                            );

                                            for message in messages {
                                                self.handle
                                                    .try_send(message)?;
                                            }
                                        }
                                    } else {
                                        log::info!(
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
                                log::warn!(
                                    "[{}] unable to parse ISUPPORT parameter: {} ({})",
                                    self.server,
                                    arg,
                                    error
                                );
                            }
                        }
                    }
                }

                return Ok(vec![]);
            }
            Command::TAGMSG(_) => {
                return Ok(vec![]);
            }
            Command::ACCOUNT(accountname) => {
                let old_user = ok!(message.user());

                self.chanmap.values_mut().for_each(|channel| {
                    if let Some(user) = channel.users.take(&old_user) {
                        channel
                            .users
                            .insert(user.with_accountname(accountname));
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
                        self.handle.try_send(message)?;
                    }

                    self.registration_required_channels.clear();
                }
            }
            Command::CHGHOST(new_username, new_hostname) => {
                let old_user = ok!(message.user());

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

                return Ok(vec![Event::Broadcast(Broadcast::ChangeHost {
                    old_user,
                    new_username: new_username.clone(),
                    new_hostname: new_hostname.clone(),
                    ourself,
                    logged_in: self.logged_in,
                    channels,
                    sent_time: server_time(&message),
                })]);
            }
            Command::Numeric(RPL_MONONLINE, args) => {
                let targets = ok!(args.get(1))
                    .split(',')
                    .map(|target| User::from(Nick::from(target)))
                    .collect::<Vec<_>>();

                return Ok(vec![
                    Event::Single(message.clone(), self.nickname().to_owned()),
                    Event::MonitoredOnline(targets),
                ]);
            }
            Command::Numeric(RPL_MONOFFLINE, args) => {
                let targets = ok!(args.get(1))
                    .split(',')
                    .map(Nick::from)
                    .collect::<Vec<_>>();

                return Ok(vec![
                    Event::Single(message.clone(), self.nickname().to_owned()),
                    Event::MonitoredOffline(targets),
                ]);
            }
            Command::Numeric(RPL_ENDOFMONLIST, _) => {
                return Ok(vec![]);
            }
            Command::MARKREAD(target, Some(timestamp)) => {
                if let Some(read_marker) = timestamp
                    .strip_prefix("timestamp=")
                    .and_then(|timestamp| timestamp.parse::<ReadMarker>().ok())
                {
                    return Ok(vec![Event::UpdateReadMarker(
                        Target::parse(
                            target,
                            self.chantypes(),
                            self.statusmsg(),
                            self.casemapping(),
                        ),
                        read_marker,
                    )]);
                }
            }
            Command::CHATHISTORY(sub, args) => {
                let mut events = vec![];

                if sub == "TARGETS" {
                    let target = Target::parse(
                        ok!(args.first()),
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    );

                    match target {
                        Target::Channel(ref channel) => {
                            if !channel.prefixes().is_empty()
                                && self.chanmap.contains_key(channel)
                            {
                                events.push(Event::ChatHistoryTargetReceived(
                                    target,
                                    server_time(&message),
                                ));
                            }
                        }
                        Target::Query(_) => {
                            events.push(Event::ChatHistoryTargetReceived(
                                target,
                                server_time(&message),
                            ));
                        }
                    }

                    if self.chathistory_targets_request.is_none() {
                        // User requested, save to history
                        events.push(Event::Single(
                            message.clone(),
                            self.nickname().to_owned(),
                        ));
                    }
                }

                return Ok(events);
            }
            Command::Numeric(RPL_SASLSUCCESS, _) => {
                self.registration_step = RegistrationStep::End;
                self.handle.try_send(command!("CAP", "END"))?;
            }
            Command::Numeric(ERR_SASLFAIL | ERR_SASLTOOLONG, _) => {
                log::debug!("[{}] sasl auth failed", self.server);

                self.registration_step = RegistrationStep::End;
                self.handle.try_send(command!("CAP", "END"))?;
            }
            Command::Numeric(RPL_TRYAGAIN, args) => {
                let command = ok!(args.get(1));

                if command == "WHO" && self.config.who_poll_enabled {
                    self.who_poll_interval.too_short();
                    log::debug!(
                        "self.who_poll_interval is too short → duration = {:?}",
                        self.who_poll_interval.duration
                    );

                    if !self.who_polls.iter().any(|who_poll| {
                        matches!(
                            who_poll.status,
                            WhoStatus::Requested(WhoSource::User, _, _)
                        )
                    }) {
                        // No user request, rate-limited due to WHO polling
                        return Ok(vec![]);
                    }
                }
            }
            Command::Numeric(RPL_ENDOFMOTD | ERR_NOMOTD, _) => {
                // MOTD (or ERR_NOMOTD) is the last required message in the numerics
                // sent on successfully completing the registration process (after
                // RPL_ISUPPORT message(s) are sent).
                // https://modern.ircdocs.horse/#connection-registration
                if self.registration_step == RegistrationStep::End {
                    self.registration_step = RegistrationStep::Complete;

                    // Send nick password & ghost
                    if let Some(nick_pass) = self.config.nick_password.as_ref()
                    {
                        // Try ghost recovery if we couldn't claim our nick
                        if self.config.should_ghost
                            && self.resolved_nick
                                == Some(self.config.nickname.clone())
                        {
                            for sequence in &self.config.ghost_sequence {
                                self.handle.try_send(command!(
                                    "PRIVMSG",
                                    "NickServ",
                                    format!(
                                        "{sequence} {} {nick_pass}",
                                        &self.config.nickname
                                    )
                                ))?;
                            }
                        }

                        if let Some(identify_syntax) =
                            &self.config.nick_identify_syntax
                        {
                            match identify_syntax {
                                config::server::IdentifySyntax::PasswordNick => {
                                    self.handle.try_send(command!(
                                        "PRIVMSG",
                                        "NickServ",
                                        format!("IDENTIFY {nick_pass} {}", &self.config.nickname)
                                    ))?;
                                }
                                config::server::IdentifySyntax::NickPassword => {
                                    self.handle.try_send(command!(
                                        "PRIVMSG",
                                        "NickServ",
                                        format!("IDENTIFY {} {nick_pass}", &self.config.nickname)
                                    ))?;
                                }
                            }
                        } else if self.resolved_nick
                            == Some(self.config.nickname.clone())
                        {
                            // Use nickname-less identification if possible, since it has
                            // no possible argument order issues.
                            self.handle.try_send(command!(
                                "PRIVMSG",
                                "NickServ",
                                format!("IDENTIFY {nick_pass}")
                            ))?;
                        } else {
                            // Default to most common syntax if unknown
                            self.handle.try_send(command!(
                                "PRIVMSG",
                                "NickServ",
                                format!(
                                    "IDENTIFY {} {nick_pass}",
                                    &self.config.nickname
                                )
                            ))?;
                        }
                    }

                    // Send user modestring
                    if let (Some(nick), Some(modestring)) = (
                        self.resolved_nick.clone(),
                        self.config.umodes.as_ref(),
                    ) {
                        self.handle
                            .try_send(command!("MODE", nick, modestring))?;
                    }

                    let channels = self
                        .config
                        .channels
                        .iter()
                        .filter_map(|channel| {
                            target::Channel::parse(
                                channel,
                                self.chantypes(),
                                self.statusmsg(),
                                self.casemapping(),
                            )
                            .ok()
                        })
                        .collect::<Vec<_>>();

                    // Send JOIN
                    for message in
                        group_joins(&channels, &self.config.channel_keys)
                    {
                        self.handle.try_send(message)?;
                    }

                    return Ok(vec![Event::OnConnect(on_connect(
                        self.handle.clone(),
                        self.config.clone(),
                        &self.isupport,
                    ))]);
                }
            }
            _ => {}
        }

        Ok(vec![Event::Single(message, self.nickname().to_owned())])
    }

    pub fn send_markread(&mut self, target: Target, read_marker: ReadMarker) {
        if self.supports_read_marker {
            if let Err(e) = self.handle.try_send(command!(
                "MARKREAD",
                target.as_str().to_string(),
                format!("timestamp={read_marker}"),
            )) {
                log::warn!("Error sending markread: {e}");
            }
        }
    }

    fn user_who_request(&self, channel: &target::Channel) -> bool {
        if let Some(who_poll) = self
            .who_polls
            .iter()
            .find(|who_poll| who_poll.channel == *channel)
        {
            matches!(
                who_poll.status,
                WhoStatus::Requested(WhoSource::User, _, _)
                    | WhoStatus::Receiving(WhoSource::User, _)
            )
        } else {
            false
        }
    }

    // TODO allow configuring the "sorting method"
    // this function sorts channels together which have similar names when the chantype prefix
    // (sometimes multiplied) is removed
    // e.g. '#chat', '##chat-offtopic' and '&chat-local' all get sorted together instead of in
    // wildly different places.
    fn compare_channels(&self, a: &str, b: &str) -> Ordering {
        let (Some(a_chantype), Some(b_chantype)) =
            (a.chars().nth(0), b.chars().nth(0))
        else {
            return a.cmp(b);
        };

        if [a_chantype, b_chantype]
            .iter()
            .all(|c| self.chantypes().contains(c))
        {
            let ord = a
                .trim_start_matches(a_chantype)
                .cmp(b.trim_start_matches(b_chantype));
            if ord != Ordering::Equal {
                return ord;
            }
        }
        a.cmp(b)
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

    pub fn chathistory_message_reference_types(
        &self,
    ) -> Vec<isupport::MessageReferenceType> {
        if let Some(isupport::Parameter::MSGREFTYPES(message_reference_types)) =
            self.isupport.get(&isupport::Kind::MSGREFTYPES)
        {
            message_reference_types.clone()
        } else {
            vec![]
        }
    }

    pub fn chathistory_request(
        &self,
        target: &Target,
    ) -> Option<ChatHistorySubcommand> {
        self.chathistory_requests
            .get(target)
            .map(|request| request.subcommand.clone())
    }

    pub fn send_chathistory_request(
        &mut self,
        subcommand: ChatHistorySubcommand,
    ) {
        use std::collections::hash_map;

        if self.supports_chathistory {
            if let Some(target) = subcommand.target() {
                if let hash_map::Entry::Vacant(entry) =
                    self.chathistory_requests.entry(Target::parse(
                        target,
                        self.chantypes(),
                        self.statusmsg(),
                        self.casemapping(),
                    ))
                {
                    entry.insert(ChatHistoryRequest {
                        subcommand: subcommand.clone(),
                        requested_at: Instant::now(),
                    });
                } else {
                    return;
                }
            } else if self.chathistory_targets_request.is_some() {
                return;
            } else {
                self.chathistory_targets_request = Some(ChatHistoryRequest {
                    subcommand: subcommand.clone(),
                    requested_at: Instant::now(),
                });
            }

            match subcommand {
                ChatHistorySubcommand::Latest(
                    target,
                    message_reference,
                    limit,
                ) => {
                    let command_message_reference =
                        isupport::fuzz_start_message_reference(
                            message_reference,
                        );

                    log::debug!(
                        "[{}] requesting {limit} latest messages in {target} since {}",
                        self.server,
                        command_message_reference,
                    );

                    let _ = self.handle.try_send(command!(
                        "CHATHISTORY",
                        "LATEST",
                        target.to_string(),
                        command_message_reference.to_string(),
                        limit.to_string(),
                    ));
                }
                ChatHistorySubcommand::Before(
                    target,
                    message_reference,
                    limit,
                ) => {
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
                        target.to_string(),
                        command_message_reference.to_string(),
                        limit.to_string(),
                    ));
                }
                ChatHistorySubcommand::Between(
                    target,
                    start_message_reference,
                    end_message_reference,
                    limit,
                ) => {
                    let (
                        command_start_message_reference,
                        command_end_message_reference,
                    ) = isupport::fuzz_message_reference_range(
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
                        target.to_string(),
                        command_start_message_reference.to_string(),
                        command_end_message_reference.to_string(),
                        limit.to_string(),
                    ));
                }
                ChatHistorySubcommand::Targets(
                    start_message_reference,
                    end_message_reference,
                    limit,
                ) => {
                    let command_start_message_reference =
                        match start_message_reference {
                            isupport::MessageReference::Timestamp(_) => {
                                start_message_reference
                            }
                            _ => isupport::MessageReference::Timestamp(
                                DateTime::UNIX_EPOCH,
                            ),
                        };

                    let command_end_message_reference =
                        match end_message_reference {
                            isupport::MessageReference::Timestamp(_) => {
                                end_message_reference
                            }
                            _ => isupport::MessageReference::Timestamp(
                                chrono::offset::Utc::now(),
                            ),
                        };

                    let (
                        command_start_message_reference,
                        command_end_message_reference,
                    ) = isupport::fuzz_message_reference_range(
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
                        limit.to_string(),
                    ));
                }
            }
        }
    }

    pub fn clear_chathistory_request(&mut self, target: Option<&Target>) {
        if let Some(target) = target {
            self.chathistory_requests.remove(target);
        } else {
            self.chathistory_targets_request = None;
        }
    }

    pub fn chathistory_exhausted(&self, target: &Target) -> bool {
        self.chathistory_exhausted
            .get(target)
            .copied()
            .unwrap_or_default()
    }

    pub fn load_chathistory_targets_timestamp(
        &self,
        server_time: DateTime<Utc>,
    ) -> impl Future<Output = Message> + use<> {
        let server = self.server.clone();

        let limit = self.chathistory_limit();

        async move {
            let timestamp = load_chathistory_targets_timestamp(server.clone())
                .await
                .ok()
                .flatten();

            let start_message_reference = timestamp
                .map_or(MessageReference::None, |timestamp| {
                    MessageReference::Timestamp(timestamp)
                });

            let end_message_reference =
                MessageReference::Timestamp(server_time);

            Message::ChatHistoryRequest(
                server,
                ChatHistorySubcommand::Targets(
                    start_message_reference,
                    end_message_reference,
                    limit,
                ),
            )
        }
        .boxed()
    }

    pub fn overwrite_chathistory_targets_timestamp(
        &self,
        timestamp: DateTime<Utc>,
    ) -> impl Future<Output = Message> + use<> {
        let server = self.server.clone();

        async move {
            let result = overwrite_chathistory_targets_timestamp(
                server.clone(),
                timestamp,
            )
            .await;

            Message::ChatHistoryTargetsTimestampUpdated(
                server, timestamp, result,
            )
        }
        .boxed()
    }

    pub fn channels(&self) -> impl Iterator<Item = &target::Channel> {
        self.chanmap.keys()
    }

    fn topic<'a>(&'a self, channel: &target::Channel) -> Option<&'a Topic> {
        self.chanmap.get(channel).map(|channel| &channel.topic)
    }

    fn mode<'a>(&'a self, channel: &target::Channel) -> Option<&'a String> {
        self.chanmap
            .get(channel)
            .and_then(|channel| channel.mode.as_ref())
    }

    fn resolve_user_attributes<'a>(
        &'a self,
        channel: &target::Channel,
        user: &User,
    ) -> Option<&'a User> {
        self.chanmap
            .get(channel)
            .and_then(|channel| channel.users.resolve(user))
    }

    pub fn users<'a>(
        &'a self,
        channel: &target::Channel,
    ) -> Option<&'a ChannelUsers> {
        self.chanmap.get(channel).map(|chanimpl| &chanimpl.users)
    }

    fn user_channels(&self, nick: NickRef) -> Vec<target::Channel> {
        self.chanmap
            .iter()
            .filter(|(_, chan)| chan.users.get_by_nick(nick).is_some())
            .map(|(t, _)| t)
            .cloned()
            .collect()
    }

    fn resolve_query<'a>(
        &'a self,
        query: &target::Query,
    ) -> Option<&'a target::Query> {
        self.resolved_queries.get(query)
    }

    pub fn nickname(&self) -> NickRef {
        // TODO: Fallback nicks
        NickRef::from(
            self.resolved_nick
                .as_deref()
                .unwrap_or(&self.config.nickname),
        )
    }

    pub fn tick(&mut self, now: Instant) -> Result<()> {
        match self.highlight_notification_blackout {
            HighlightNotificationBlackout::Blackout(instant) => {
                if now.duration_since(instant) >= HIGHLIGHT_BLACKOUT_INTERVAL {
                    self.highlight_notification_blackout =
                        HighlightNotificationBlackout::Receiving;
                }
            }
            HighlightNotificationBlackout::Receiving => {}
        }

        if let Some(who_poll) = self.who_polls.front_mut() {
            #[derive(Debug)]
            enum Request {
                Poll,
                Retry,
            }

            let request = match &who_poll.status {
                WhoStatus::Joined => (self.supports_away_notify
                    || self.config.who_poll_enabled)
                    .then_some(Request::Poll),
                WhoStatus::Waiting(last) => {
                    if self.supports_away_notify {
                        self.chanmap.get(&who_poll.channel).and_then(
                            |channel| {
                                (!channel.who_init
                                    && (now.duration_since(*last)
                                        >= self.who_poll_interval.duration))
                                    .then_some(Request::Poll)
                            },
                        )
                    } else {
                        (self.config.who_poll_enabled
                            && (now.duration_since(*last)
                                >= self.who_poll_interval.duration))
                            .then_some(Request::Poll)
                    }
                }
                WhoStatus::Requested(source, requested, _) => {
                    if matches!(source, WhoSource::Poll)
                        && !self.config.who_poll_enabled
                    {
                        None
                    } else {
                        (now.duration_since(*requested)
                            >= 5 * self.who_poll_interval.duration)
                            .then_some(Request::Retry)
                    }
                }
                _ => None,
            };

            if let Some(request) = request {
                if self.isupport.contains_key(&isupport::Kind::WHOX) {
                    let whox_params = if self.supports_account_notify {
                        WhoXPollParameters::WithAccountName
                    } else {
                        WhoXPollParameters::Default
                    };

                    who_poll.status = WhoStatus::Requested(
                        WhoSource::Poll,
                        Instant::now(),
                        Some(whox_params.token()),
                    );

                    self.handle.try_send(command!(
                        "WHO",
                        who_poll.channel.to_string(),
                        whox_params.fields().to_string(),
                        whox_params.token().to_owned()
                    ))?;
                } else {
                    who_poll.status = WhoStatus::Requested(
                        WhoSource::Poll,
                        Instant::now(),
                        None,
                    );

                    self.handle.try_send(command!(
                        "WHO",
                        who_poll.channel.to_string()
                    ))?;
                }

                log::debug!(
                    "[{}] {} - WHO {}",
                    self.server,
                    who_poll.channel,
                    match request {
                        Request::Poll => "poll",
                        Request::Retry => "retry",
                    }
                );
            }
        }

        self.chathistory_requests.retain(|_, chathistory_request| {
            now.duration_since(chathistory_request.requested_at)
                < CHATHISTORY_REQUEST_TIMEOUT
        });

        Ok(())
    }

    pub fn casemapping(&self) -> isupport::CaseMap {
        isupport::get_casemapping_or_default(&self.isupport)
    }

    pub fn chanmodes(&self) -> &[isupport::ModeKind] {
        isupport::get_chanmodes_or_default(&self.isupport)
    }

    pub fn chantypes(&self) -> &[char] {
        isupport::get_chantypes_or_default(&self.isupport)
    }

    pub fn prefix(&self) -> &[isupport::PrefixMap] {
        isupport::get_prefix_or_default(&self.isupport)
    }

    pub fn statusmsg(&self) -> &[char] {
        isupport::get_statusmsg_or_default(&self.isupport)
    }

    pub fn is_channel(&self, target: &str) -> bool {
        proto::is_channel(target, self.chantypes())
    }
}

fn continue_chathistory_between(
    target: &Target,
    events: &[Event],
    end_message_reference: &MessageReference,
    limit: u16,
) -> Option<ChatHistorySubcommand> {
    let start_message_reference =
        events.first().and_then(|first_event| match first_event {
            Event::Single(message, _) | Event::WithTarget(message, _, _) => {
                match end_message_reference {
                    MessageReference::MessageId(_) => {
                        message_id(message).map(MessageReference::MessageId)
                    }
                    MessageReference::Timestamp(_) => {
                        Some(MessageReference::Timestamp(server_time(message)))
                    }
                    MessageReference::None => None,
                }
            }
            _ => None,
        });

    start_message_reference.map(|start_message_reference| {
        ChatHistorySubcommand::Between(
            target.clone(),
            start_message_reference,
            end_message_reference.clone(),
            limit,
        )
    })
}

async fn chathistory_targets_path(server: &Server) -> Result<PathBuf, Error> {
    let data_dir = environment::data_dir();

    let targets_dir = data_dir.join("targets");

    if !targets_dir.exists() {
        fs::create_dir_all(&targets_dir).await?;
    }

    let hashed_server = seahash::hash(format!("{server}").as_bytes());

    Ok(targets_dir.join(format!("{hashed_server}.json")))
}

pub async fn load_chathistory_targets_timestamp(
    server: Server,
) -> Result<Option<DateTime<Utc>>, Error> {
    let path = chathistory_targets_path(&server).await?;

    if let Ok(bytes) = fs::read(path).await {
        Ok(serde_json::from_slice(&bytes).unwrap_or_default())
    } else {
        Ok(None)
    }
}

pub async fn overwrite_chathistory_targets_timestamp(
    server: Server,
    timestamp: DateTime<Utc>,
) -> Result<(), Error> {
    let bytes = serde_json::to_vec(&Some(timestamp))?;

    let path = chathistory_targets_path(&server).await?;

    fs::write(path, &bytes).await?;

    Ok(())
}

#[derive(Debug)]
enum HighlightNotificationBlackout {
    Blackout(Instant),
    Receiving,
}

impl HighlightNotificationBlackout {
    fn allowed(&self) -> bool {
        match self {
            HighlightNotificationBlackout::Blackout(_) => false,
            HighlightNotificationBlackout::Receiving => true,
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

    pub fn receive(
        &mut self,
        server: &Server,
        message: message::Encoded,
        ctcp_config: &config::Ctcp,
    ) -> Result<Vec<Event>> {
        if let Some(client) = self.client_mut(server) {
            client.receive(message, ctcp_config)
        } else {
            Ok(Vec::default())
        }
    }

    pub fn send(
        &mut self,
        buffer: &buffer::Upstream,
        message: message::Encoded,
    ) {
        if let Some(client) = self.client_mut(buffer.server()) {
            client.send(buffer, message);
        }
    }

    pub fn send_markread(
        &mut self,
        server: &Server,
        target: Target,
        read_marker: ReadMarker,
    ) {
        if let Some(client) = self.client_mut(server) {
            client.send_markread(target, read_marker);
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

    pub fn exit(&mut self) -> HashSet<Server> {
        self.0
            .iter_mut()
            .filter_map(|(server, state)| {
                if let State::Ready(client) = state {
                    client.quit(None);
                    Some(server.clone())
                } else {
                    None
                }
            })
            .collect()
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
        nick: NickRef,
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

    pub fn get_isupport(
        &self,
        server: &Server,
    ) -> HashMap<isupport::Kind, isupport::Parameter> {
        self.client(server)
            .map(|client| client.isupport.clone())
            .unwrap_or_default()
    }

    pub fn get_casemapping(&self, server: &Server) -> isupport::CaseMap {
        self.client(server)
            .map(Client::casemapping)
            .unwrap_or_default()
    }

    pub fn get_chanmodes<'a>(
        &'a self,
        server: &Server,
    ) -> &'a [isupport::ModeKind] {
        self.client(server)
            .map(Client::chanmodes)
            .unwrap_or_default()
    }

    pub fn get_chantypes<'a>(&'a self, server: &Server) -> &'a [char] {
        self.client(server)
            .map(Client::chantypes)
            .unwrap_or_default()
    }

    pub fn get_prefix<'a>(
        &'a self,
        server: &Server,
    ) -> &'a [isupport::PrefixMap] {
        self.client(server).map(Client::prefix).unwrap_or_default()
    }

    pub fn get_statusmsg<'a>(&'a self, server: &Server) -> &'a [char] {
        self.client(server)
            .map(Client::statusmsg)
            .unwrap_or_default()
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
        self.client(server)
            .is_some_and(|client| client.supports_chathistory)
    }

    pub fn get_chathistory_request(
        &self,
        server: &Server,
        target: &Target,
    ) -> Option<ChatHistorySubcommand> {
        self.client(server)
            .and_then(|client| client.chathistory_request(target))
    }

    pub fn send_chathistory_request(
        &mut self,
        server: &Server,
        subcommand: ChatHistorySubcommand,
    ) {
        if let Some(client) = self.client_mut(server) {
            client.send_chathistory_request(subcommand);
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
    ) -> Option<ChatHistoryState> {
        self.client(server).and_then(|client| {
            if client.supports_chathistory {
                if client.chathistory_request(target).is_some() {
                    Some(ChatHistoryState::PendingRequest)
                } else if client.chathistory_exhausted(target) {
                    Some(ChatHistoryState::Exhausted)
                } else {
                    Some(ChatHistoryState::Ready)
                }
            } else {
                None
            }
        })
    }

    pub fn load_chathistory_targets_timestamp(
        &self,
        server: &Server,
        server_time: DateTime<Utc>,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.client(server).map(|client| {
            client.load_chathistory_targets_timestamp(server_time)
        })
    }

    pub fn overwrite_chathistory_targets_timestamp(
        &self,
        server: &Server,
        server_time: DateTime<Utc>,
    ) -> Option<impl Future<Output = Message> + use<>> {
        self.client(server).map(|client| {
            client.overwrite_chathistory_targets_timestamp(server_time)
        })
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

    pub fn iter(&self) -> std::collections::btree_map::Iter<Server, State> {
        self.0.iter()
    }

    pub fn status(&self, server: &Server) -> Status {
        self.0.get(server).map_or(Status::Unavailable, |s| match s {
            State::Disconnected => Status::Disconnected,
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
}

#[derive(Debug, Clone)]
pub enum Context {
    Buffer(buffer::Upstream),
    Whois(buffer::Upstream),
}

impl Context {
    fn new(message: &message::Encoded, buffer: buffer::Upstream) -> Self {
        if let Command::WHOIS(_, _) = message.command {
            Self::Whois(buffer)
        } else {
            Self::Buffer(buffer)
        }
    }

    fn is_whois(&self) -> bool {
        matches!(self, Self::Whois(_))
    }

    fn buffer(self) -> buffer::Upstream {
        match self {
            Context::Buffer(buffer) => buffer,
            Context::Whois(buffer) => buffer,
        }
    }
}

#[derive(Debug)]
pub enum ChatHistoryBatch {
    Target(Target),
    Targets,
}

impl ChatHistoryBatch {
    pub fn target(&self) -> Option<Target> {
        match self {
            ChatHistoryBatch::Target(batch_target) => {
                Some(batch_target.clone())
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RegistrationStep {
    Start,
    List,
    Req,
    Sasl,
    End,
    Complete,
}

#[derive(Debug, Default)]
pub struct Channel {
    pub users: ChannelUsers,
    pub topic: Topic,
    pub names_init: bool,
    pub who_init: bool,
    pub mode: Option<String>,
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
    pub who: Option<Nick>,
    pub time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct WhoPoll {
    pub channel: target::Channel,
    pub status: WhoStatus,
}

#[derive(Debug, Clone)]
pub enum WhoStatus {
    Requested(WhoSource, Instant, Option<WhoToken>),
    Receiving(WhoSource, Option<WhoToken>),
    Received,
    Waiting(Instant),
    Joined,
}

#[derive(Debug, Clone)]
pub enum WhoSource {
    User,
    Poll,
}

pub struct BackoffInterval {
    duration: Duration,
    previous: Duration,
    original: Duration,
    mode: BackoffMode,
}

pub enum BackoffMode {
    Set,
    BackingOff(usize),
    EasingOn,
}

impl BackoffInterval {
    pub fn from_duration(duration: Duration) -> Self {
        BackoffInterval {
            duration,
            previous: duration,
            original: duration,
            mode: BackoffMode::Set,
        }
    }

    pub fn long_enough(&mut self) {
        match &mut self.mode {
            BackoffMode::Set => (),
            BackoffMode::EasingOn => {
                self.previous = self.duration;
                self.duration =
                    std::cmp::max(self.duration.mul_f64(0.9), self.original);
            }
            BackoffMode::BackingOff(count) => {
                *count += 1;

                if *count > 8 {
                    self.mode = BackoffMode::EasingOn;
                }
            }
        }
    }

    pub fn too_short(&mut self) {
        match &mut self.mode {
            BackoffMode::EasingOn => {
                self.duration = self.previous;
                self.mode = BackoffMode::Set;
            }
            _ => {
                self.mode = BackoffMode::BackingOff(0);
                self.duration = std::cmp::min(
                    self.duration.mul_f64(2.0),
                    256 * self.original,
                );
            }
        }
    }
}

fn group_capability_requests<'a>(
    capabilities: &'a [&'a str],
) -> impl Iterator<Item = proto::Message> + 'a {
    const MAX_LEN: usize = proto::format::BYTE_LIMIT - b"CAP REQ :\r\n".len();

    capabilities
        .iter()
        .scan(0, |count, capability| {
            // Capability + a space
            *count += capability.len() + 1;

            let chunk = *count / MAX_LEN;

            Some((chunk, capability))
        })
        .into_group_map()
        .into_values()
        .map(|capabilities| {
            command!("CAP", "REQ", capabilities.into_iter().join(" "))
        })
}

/// Group channels together into as few JOIN messages as possible
fn group_joins<'a>(
    channels: &'a [target::Channel],
    keys: &'a HashMap<String, String>,
) -> impl Iterator<Item = proto::Message> + 'a {
    const MAX_LEN: usize = proto::format::BYTE_LIMIT - b"JOIN \r\n".len();

    let (without_keys, with_keys): (Vec<_>, Vec<_>) =
        channels.iter().partition_map(|channel| {
            keys.get(channel.as_str())
                .map_or(Either::Left(channel), |key| {
                    Either::Right((channel, key))
                })
        });

    let joins_without_keys = without_keys
        .into_iter()
        .scan(0, |count, channel| {
            // Channel + a comma
            *count += channel.as_str().len() + 1;

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
            *count += channel.as_str().len() + key.len() + 2;

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
    .map(|targets| command!("MONITOR", "+", targets.into_iter().join(",")))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Compression(#[from] compression::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Target(#[from] target::ParseError),
}
