use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use irc::proto::command;

use crate::capabilities::{Capabilities, Capability};
use crate::isupport::{self, WhoToken, WhoXPollParameters};
use crate::rate_limit::{BackoffInterval, TokenPriority};
use crate::server::Server;
use crate::target::Channel;
use crate::{User, client, config, message};

#[derive(Debug, Clone)]
pub struct WhoPoll {
    pub channel: Channel,
    pub status: WhoStatus,
    pub source: WhoSource,
}

#[derive(Debug, Clone)]
pub enum WhoStatus {
    Requested {
        at: Instant,
        token: Option<WhoToken>,
    },
    Receiving {
        last_received: Instant,
        token: Option<WhoToken>,
    },
    Waiting {
        immediate: bool,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WhoSource {
    User,
    Join { priority: bool },
    Poll,
}

impl Ord for WhoSource {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else {
            match (self, other) {
                (WhoSource::User, _) => Ordering::Less,
                (WhoSource::Join { .. }, WhoSource::User) => Ordering::Greater,
                (WhoSource::Join { priority }, WhoSource::Join { .. }) => {
                    if *priority {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    }
                }
                (WhoSource::Join { .. }, _) => Ordering::Less,
                (WhoSource::Poll, _) => Ordering::Greater,
            }
        }
    }
}

impl PartialOrd for WhoSource {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct WhoQueue {
    queue: VecDeque<WhoPoll>,
    in_flight: Vec<WhoPoll>,
    in_flight_limit: usize,
    interval: BackoffInterval,
    reference_time: Option<Instant>,
}

impl WhoQueue {
    pub fn new(config: &Arc<config::Server>) -> Self {
        // Prevent the poll interval from being too short compared to the
        // anti-flood rate, to avoid overfilling the send queue with WHO polls.
        let interval = BackoffInterval::from(
            config
                .who_poll_interval
                .max(config.anti_flood.saturating_mul(2)),
        );

        Self {
            queue: VecDeque::new(),
            in_flight: Vec::new(),
            in_flight_limit: 1,
            interval,
            reference_time: None,
        }
    }

    pub fn update<'a>(
        &mut self,
        capabilities: &Capabilities,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
        config: &Arc<config::Server>,
        joined_channels: impl Iterator<Item = &'a Channel>,
    ) {
        if capabilities.acknowledged(Capability::NoImplicitNames) {
            self.in_flight_limit = 64;
        } else {
            self.in_flight_limit = 1;
        }

        if isupport.contains_key(&isupport::Kind::SAFELIST) {
            self.interval.set_min(config.who_poll_interval);
        } else {
            self.interval.set_min(
                config
                    .who_poll_interval
                    .max(config.anti_flood.saturating_mul(2)),
            );
        }

        if config.who_poll_enabled
            && !capabilities.acknowledged(Capability::AwayNotify)
        {
            for channel in joined_channels {
                if !self
                    .in_flight
                    .iter()
                    .chain(self.queue.iter())
                    .any(|who_poll| who_poll.channel == *channel)
                {
                    let who_poll = WhoPoll {
                        channel: channel.clone(),
                        source: WhoSource::Poll,
                        status: WhoStatus::Waiting { immediate: false },
                    };

                    self.insert_towards_back(who_poll);
                }
            }
        } else {
            self.queue
                .retain(|who_poll| !matches!(who_poll.source, WhoSource::Poll));
        }
    }

    pub fn tick(
        &mut self,
        server: &client::Server,
        capabilities: &Capabilities,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
        now: &Instant,
    ) -> Vec<(message::Encoded, TokenPriority)> {
        let interval_duration = self.interval.duration();
        let reference_time = self.reference_time;

        // Find any timed out polls and return them in the queue.

        let timed_out_polls: Vec<_> = self
            .in_flight
            .extract_if(.., |who_poll| {
                can_send_who_poll(
                    false,
                    &interval_duration,
                    &reference_time,
                    now,
                    who_poll,
                )
            })
            .collect();

        for who_poll in timed_out_polls.into_iter() {
            self.insert_towards_front(who_poll);
        }

        // Prepare polls to be sent during this tick.

        let mut who_polls_to_send = vec![];

        for _ in 0..self.in_flight_limit.saturating_sub(self.in_flight.len()) {
            if let Some(who_poll) = self.queue.pop_front_if(|who_poll| {
                can_send_who_poll(
                    self.in_flight.is_empty() && who_polls_to_send.is_empty(),
                    &interval_duration,
                    &reference_time,
                    now,
                    who_poll,
                )
            }) {
                who_polls_to_send.push(who_poll);
            } else {
                break;
            }
        }

        // If any polls will be sent, then update the reference time used for
        // determining if polls can be sent (and thus also for throttling).
        if !who_polls_to_send.is_empty() {
            self.reference_time = Some(*now);
        }

        let number_of_who_polls = who_polls_to_send.len();

        who_polls_to_send
            .into_iter()
            .enumerate()
            .map(|(request_number, who_poll)| {
                log::trace!(
                    "[{server}] {} - WHO {}{}",
                    who_poll.channel,
                    if matches!(who_poll.status, WhoStatus::Waiting { .. }) {
                        "request"
                    } else {
                        "retry"
                    },
                    if number_of_who_polls > 1 {
                        format!(" ({request_number} of {number_of_who_polls})")
                    } else {
                        String::new()
                    }
                );

                self.send_who_poll(capabilities, isupport, who_poll)
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn is_user_who_request(&self, channel: &Channel) -> bool {
        self.in_flight.iter().any(|who_poll| {
            who_poll.channel == *channel
                && matches!(who_poll.source, WhoSource::User)
        })
    }

    pub fn any_user_who_request(&self) -> bool {
        self.in_flight
            .iter()
            .any(|who_poll| matches!(who_poll.source, WhoSource::User))
    }

    pub fn parted_channel(&mut self, channel: &Channel) {
        if let Some(pos) = self
            .queue
            .iter()
            .position(|who_poll| who_poll.channel == *channel)
        {
            self.queue.remove(pos);
        }
    }

    // Record user WHO request(s) for reply filtering.
    pub fn user_requested_who_poll(
        &mut self,
        channel: Channel,
        token: Option<WhoToken>,
    ) {
        let who_poll = WhoPoll {
            channel,
            status: WhoStatus::Requested {
                at: Instant::now(),
                token,
            },
            source: WhoSource::User,
        };

        if let Some(pos) = self
            .queue
            .iter()
            .position(|queued_poll| queued_poll.channel == who_poll.channel)
        {
            self.queue.remove(pos);
        }

        self.in_flight.push(who_poll);
    }

    pub fn queue_join_who_request(
        &mut self,
        capabilities: &Capabilities,
        channel: &Channel,
    ) {
        let who_poll = if let Some(pos) = self
            .queue
            .iter()
            .position(|who_poll| who_poll.channel == *channel)
            && let Some(who_poll) = self.queue.remove(pos)
        {
            who_poll
        } else {
            WhoPoll {
                channel: channel.clone(),
                source: WhoSource::Join { priority: false },
                status: WhoStatus::Waiting {
                    immediate: capabilities
                        .acknowledged(Capability::NoImplicitNames),
                },
            }
        };

        self.insert_towards_back(who_poll);
    }

    pub fn receiving_who_poll(
        &mut self,
        server: &client::Server,
        channel: &Channel,
    ) {
        if let Some(who_poll) = self
            .in_flight
            .iter_mut()
            .find(|who_poll| who_poll.channel == *channel)
            && let WhoStatus::Requested { token, .. } = who_poll.status
        {
            who_poll.status = WhoStatus::Receiving {
                last_received: Instant::now(),
                token,
            };

            log::trace!("[{server}] {channel} - WHO receiving...");
        }
    }

    pub fn handle_who_reply(
        &mut self,
        server: &client::Server,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
        target_channel: &Channel,
        client_channel: &mut client::Channel,
        args: &[String],
    ) -> anyhow::Result<()> {
        macro_rules! ok {
            ($option:expr) => {
                $option.ok_or_else(|| {
                    anyhow!("[{}] Malformed WHO reply", server)
                })?
            };
        }

        if let Some(who_poll) = self
            .in_flight
            .iter_mut()
            .find(|who_poll| who_poll.channel == *target_channel)
        {
            match &mut who_poll.status {
                WhoStatus::Receiving { last_received, .. } => {
                    *last_received = Instant::now();
                }
                WhoStatus::Requested { token: None, .. } => {
                    who_poll.status = WhoStatus::Receiving {
                        last_received: Instant::now(),
                        token: None,
                    };

                    log::trace!("[{server}] {target_channel} - receiving WHO",);
                }
                WhoStatus::Requested { token: Some(_), .. }
                | WhoStatus::Waiting { .. } => {
                    log::debug!(
                        "[{server}] {target_channel} - receiving unexpected WHO",
                    );
                }
            }
        }

        let casemapping = isupport::get_casemapping_or_default(isupport);
        let bot_mode_char = isupport::get_bot_mode_char(isupport);

        let nick = ok!(args.get(5));
        let flags = ok!(args.get(6));
        let username = ok!(args.get(2));
        let hostname = ok!(args.get(3));

        let user = User::from_whoreply(
            nick,
            flags,
            username,
            hostname,
            None,
            casemapping,
            bot_mode_char,
        );

        if client_channel.users.contains(&user) {
            client_channel.update_user_status(
                nick,
                flags,
                casemapping,
                bot_mode_char,
            );
        } else {
            client_channel.users.insert(user);
        }

        Ok(())
    }

    pub fn handle_whox_reply(
        &mut self,
        server: &client::Server,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
        target_channel: &Channel,
        client_channel: &mut client::Channel,
        args: &[String],
    ) -> anyhow::Result<()> {
        macro_rules! ok {
            ($option:expr) => {
                $option.ok_or_else(|| {
                    anyhow!("[{}] Malformed WHOX reply", server)
                })?
            };
        }

        if let Some(who_poll) = self
            .in_flight
            .iter_mut()
            .find(|who_poll| who_poll.channel == *target_channel)
        {
            match &mut who_poll.status {
                WhoStatus::Receiving { last_received, .. } => {
                    *last_received = Instant::now();
                }
                WhoStatus::Requested {
                    token: Some(request_token),
                    ..
                } => {
                    if matches!(who_poll.source, WhoSource::User) {
                        who_poll.status = WhoStatus::Receiving {
                            last_received: Instant::now(),
                            token: Some(*request_token),
                        };
                    } else if let Ok(token) =
                        ok!(args.get(1)).parse::<WhoToken>()
                        && *request_token == token
                    {
                        who_poll.status = WhoStatus::Receiving {
                            last_received: Instant::now(),
                            token: Some(*request_token),
                        };

                        log::trace!(
                            "[{server}] {target_channel} - receiving WHO",
                        );
                    }
                }
                WhoStatus::Requested { token: None, .. }
                | WhoStatus::Waiting { .. } => {
                    log::debug!(
                        "[{server}] {target_channel} - receiving unexpected WHO",
                    );
                }
            }

            // Don't bother trying to parse user-initiated WHO requests since we
            // do not currently track user WHO poll request parameters.
            if matches!(
                who_poll.source,
                WhoSource::Poll | WhoSource::Join { .. }
            ) {
                let casemapping =
                    isupport::get_casemapping_or_default(isupport);
                let bot_mode_char = isupport::get_bot_mode_char(isupport);

                // Check token to ~ensure reply is to poll request
                if let Ok(token) = ok!(args.get(1)).parse::<WhoToken>() {
                    if token == WhoXPollParameters::Default.token() {
                        let nick = ok!(args.get(3));
                        let flags = ok!(args.get(4));

                        client_channel.update_user_status(
                            nick,
                            flags,
                            casemapping,
                            bot_mode_char,
                        );
                    } else if token
                        == WhoXPollParameters::WithAccountName.token()
                    {
                        let nick = ok!(args.get(3));
                        let flags = ok!(args.get(4));

                        client_channel.update_user_status(
                            nick,
                            flags,
                            casemapping,
                            bot_mode_char,
                        );

                        let accountname = ok!(args.get(5));

                        client_channel.update_user_accountname(
                            nick,
                            accountname,
                            casemapping,
                        );
                    } else if token == WhoXPollParameters::InitialJoin.token() {
                        let flags = ok!(args.get(6));
                        let nick = ok!(args.get(5));
                        let username = ok!(args.get(3));
                        let hostname = ok!(args.get(4));
                        let accountname = ok!(args.get(7));

                        let user = User::from_whoreply(
                            nick,
                            flags,
                            username,
                            hostname,
                            Some(accountname),
                            casemapping,
                            isupport::get_bot_mode_char(isupport),
                        );

                        if client_channel.users.contains(&user) {
                            client_channel.update_user_status(
                                nick,
                                flags,
                                casemapping,
                                bot_mode_char,
                            );

                            client_channel.update_user_accountname(
                                nick,
                                accountname,
                                casemapping,
                            );
                        } else {
                            client_channel.users.insert(user);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn handle_end_who_reply(
        &mut self,
        channel: &Channel,
        is_joined: bool,
        capabilities: &Capabilities,
        config: &Arc<config::Server>,
    ) {
        if let Some(pos) = self
            .in_flight
            .iter()
            .position(|who_poll| who_poll.channel == *channel)
        {
            let who_poll = self.in_flight.remove(pos);

            let user_request = matches!(who_poll.source, WhoSource::User);

            // Check whether the WHO polling is enabled & needed (for updating
            // away state of a joined channel).
            if config.who_poll_enabled
                && !capabilities.acknowledged(Capability::AwayNotify)
                && is_joined
                && !self
                    .queue
                    .iter()
                    .any(|queued_poll| queued_poll.channel == who_poll.channel)
            {
                self.insert_towards_back(WhoPoll {
                    status: WhoStatus::Waiting { immediate: false },
                    source: WhoSource::Poll,
                    ..who_poll
                });
            }

            if !user_request {
                self.interval.long_enough();
            }
        }
    }

    // User did not request, treat as part of rate-limiting response
    // (in conjunction with RPL_TRYAGAIN) and don't save to history.
    pub fn handle_who_rate_limited(&mut self, server: &client::Server) {
        self.interval.too_short();

        self.reference_time = Some(Instant::now());

        for who_poll in self.queue.iter_mut() {
            who_poll.status = WhoStatus::Waiting { immediate: false };
        }

        log::debug!(
            "[{server}] WHO poll interval is too short → duration = {:?}",
            self.interval.duration()
        );
    }

    pub fn prioritize_who_poll(&mut self, server: &Server, channel: &Channel) {
        if let Some(pos) = self.queue.iter().position(|who_poll| {
            who_poll.channel == *channel
                && matches!(who_poll.source, WhoSource::Join { .. })
        }) && let Some(mut who_poll) = self.queue.remove(pos)
        {
            log::trace!("[{server}] {channel} - prioritizing WHO poll",);

            who_poll.source = WhoSource::Join { priority: true };

            self.insert_towards_front(who_poll);
        }
    }

    pub fn deprioritize_who_poll(
        &mut self,
        server: &Server,
        channel: &Channel,
    ) {
        if let Some(pos) = self.queue.iter().position(|who_poll| {
            who_poll.channel == *channel
                && matches!(who_poll.source, WhoSource::Join { priority: true })
        }) && let Some(mut who_poll) = self.queue.remove(pos)
        {
            log::trace!("[{server}] {channel} - deprioritizing WHO poll",);

            who_poll.source = WhoSource::Join { priority: false };

            self.insert_towards_front(who_poll);
        }
    }

    fn send_who_poll(
        &mut self,
        capabilities: &Capabilities,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
        mut who_poll: WhoPoll,
    ) -> (message::Encoded, TokenPriority) {
        let message = if isupport.contains_key(&isupport::Kind::WHOX) {
            let whox_params = if capabilities
                .acknowledged(Capability::NoImplicitNames)
                && matches!(who_poll.source, WhoSource::Join { .. })
            {
                WhoXPollParameters::InitialJoin
            } else if capabilities.acknowledged(Capability::AccountNotify) {
                WhoXPollParameters::WithAccountName
            } else {
                WhoXPollParameters::Default
            };

            who_poll.status = WhoStatus::Requested {
                at: Instant::now(),
                token: Some(whox_params.token()),
            };

            command!(
                "WHO",
                who_poll.channel.to_string(),
                whox_params.fields().to_string(),
                whox_params.token().to_owned()
            )
        } else {
            who_poll.status = WhoStatus::Requested {
                at: Instant::now(),
                token: None,
            };

            command!("WHO", who_poll.channel.to_string())
        };

        self.in_flight.push(who_poll);

        (message.into(), TokenPriority::Low)
    }

    // Insert poll based on sort order, at the back of its sort category.
    fn insert_towards_back(&mut self, who_poll: WhoPoll) {
        let at = self.queue.partition_point(|queued_poll| {
            who_poll.source <= queued_poll.source
        });
        self.queue.insert(at, who_poll);
    }

    // Insert poll based on sort order, at the front of its sort category.
    fn insert_towards_front(&mut self, who_poll: WhoPoll) {
        let at = self.queue.partition_point(|queued_poll| {
            who_poll.source < queued_poll.source
        });
        self.queue.insert(at, who_poll);
    }
}

fn can_send_who_poll(
    no_polls_in_flight_or_to_send: bool,
    interval_duration: &Duration,
    reference_time: &Option<Instant>,
    now: &Instant,
    who_poll: &WhoPoll,
) -> bool {
    if matches!(who_poll.source, WhoSource::Poll)
        && matches!(who_poll.status, WhoStatus::Waiting { .. })
        && !no_polls_in_flight_or_to_send
    {
        return false;
    }

    match who_poll.status {
        WhoStatus::Waiting { immediate } => {
            if immediate {
                true
            } else if let Some(reference_time) = reference_time {
                now.duration_since(*reference_time) > *interval_duration
            } else {
                true
            }
        }
        WhoStatus::Requested { at, .. } => {
            now.duration_since(at) > 5 * *interval_duration
        }
        WhoStatus::Receiving { last_received, .. } => {
            now.duration_since(last_received) > 5 * *interval_duration
        }
    }
}
