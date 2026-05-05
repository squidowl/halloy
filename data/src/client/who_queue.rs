use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use indexmap::IndexMap;
use irc::proto::command;

use crate::capabilities::{Capabilities, Capability};
use crate::isupport::{self, WhoToken, WhoXPollParameters};
use crate::rate_limit::{BackoffInterval, TokenPriority};
use crate::target::Channel;
use crate::{User, client, config, message};

#[derive(Debug, Clone)]
pub struct WhoPoll {
    pub channel: Channel,
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

#[derive(Debug, Clone)]
pub struct WhoQueue {
    config: Arc<config::Server>,
    polls: VecDeque<WhoPoll>,
    interval: BackoffInterval,
}

impl WhoQueue {
    pub fn new(config: Arc<config::Server>) -> Self {
        let interval = BackoffInterval::from(
            config
                .who_poll_interval
                .min(config.anti_flood.saturating_mul(2)),
        );

        Self {
            config,
            polls: VecDeque::new(),
            interval,
        }
    }

    pub fn tick(
        &mut self,
        server: &client::Server,
        capabilities: &Capabilities,
        chanmap: &IndexMap<Channel, client::Channel>,
        config: &Arc<config::Server>,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
        now: Instant,
    ) -> Vec<(message::Encoded, TokenPriority)> {
        let mut messages: Vec<(message::Encoded, TokenPriority)> = vec![];
        if let Some(who_poll) = self.polls.front_mut() {
            #[derive(Debug)]
            enum Request {
                Poll,
                Retry,
            }

            let request = match &who_poll.status {
                WhoStatus::Joined => (capabilities
                    .acknowledged(Capability::NoImplicitNames)
                    || capabilities.acknowledged(Capability::AwayNotify)
                    || config.who_poll_enabled)
                    .then_some(Request::Poll),
                WhoStatus::Waiting(last) => {
                    if capabilities.acknowledged(Capability::AwayNotify) {
                        chanmap.get(&who_poll.channel).and_then(|channel| {
                            (!channel.who_init
                                && (now.duration_since(*last)
                                    >= self.interval.duration()))
                            .then_some(Request::Poll)
                        })
                    } else {
                        ((capabilities
                            .acknowledged(Capability::NoImplicitNames)
                            || config.who_poll_enabled)
                            && (now.duration_since(*last)
                                >= self.interval.duration()))
                        .then_some(Request::Poll)
                    }
                }
                WhoStatus::Requested(source, requested, _) => {
                    if matches!(source, WhoSource::Poll)
                        && !config.who_poll_enabled
                    {
                        None
                    } else {
                        (now.duration_since(*requested)
                            >= 5 * self.interval.duration())
                        .then_some(Request::Retry)
                    }
                }
                _ => None,
            };

            if let Some(request) = request {
                log::trace!(
                    "[{}] {} - WHO {}",
                    server,
                    who_poll.channel,
                    match request {
                        Request::Poll => "poll",
                        Request::Retry => "retry",
                    }
                );

                let message = if isupport.contains_key(&isupport::Kind::WHOX) {
                    let whox_params = if capabilities
                        .acknowledged(Capability::NoImplicitNames)
                        && chanmap
                            .get(&who_poll.channel)
                            .is_none_or(|channel| !channel.who_init)
                    {
                        WhoXPollParameters::InitialJoin
                    } else if capabilities
                        .acknowledged(Capability::AccountNotify)
                    {
                        WhoXPollParameters::WithAccountName
                    } else {
                        WhoXPollParameters::Default
                    };

                    who_poll.status = WhoStatus::Requested(
                        WhoSource::Poll,
                        Instant::now(),
                        Some(whox_params.token()),
                    );

                    command!(
                        "WHO",
                        who_poll.channel.to_string(),
                        whox_params.fields().to_string(),
                        whox_params.token().to_owned()
                    )
                } else {
                    who_poll.status = WhoStatus::Requested(
                        WhoSource::Poll,
                        Instant::now(),
                        None,
                    );

                    command!("WHO", who_poll.channel.to_string())
                };

                messages.push((message.into(), TokenPriority::Low));
            }
        }

        messages
    }

    pub fn update_config(
        &mut self,
        chanmap: &IndexMap<Channel, client::Channel>,
        config: Arc<config::Server>,
    ) {
        if self.config.who_poll_enabled != config.who_poll_enabled {
            if config.who_poll_enabled {
                for channel in chanmap.keys() {
                    self.polls.push_back(WhoPoll {
                        channel: channel.clone(),
                        status: WhoStatus::Joined,
                    });
                }
            } else {
                self.polls.retain(|who_poll| {
                    matches!(
                        who_poll.status,
                        WhoStatus::Requested(_, _, _)
                            | WhoStatus::Receiving(_, _)
                    )
                });
            }
        }
        self.config = config;
    }

    pub fn quit(&mut self) {
        self.polls.retain(|who_poll| {
            matches!(
                who_poll.status,
                WhoStatus::Requested(_, _, _) | WhoStatus::Receiving(_, _)
            )
        });
    }

    pub fn any_matching<F>(&self, f: F) -> bool
    where
        F: Fn(&WhoPoll) -> bool,
    {
        self.polls.iter().any(f)
    }

    pub fn pos_matching<F>(&self, f: F) -> Option<usize>
    where
        F: Fn(&WhoPoll) -> bool,
    {
        self.polls.iter().position(f)
    }

    pub fn remove_matching<F>(&mut self, f: F) -> bool
    where
        F: Fn(&WhoPoll) -> bool,
    {
        if let Some(pos) = self.pos_matching(f) {
            self.polls.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn requested_channel_poll(
        &mut self,
        params: Vec<String>,
        channel: Channel,
    ) {
        // Record user WHO request(s) for reply filtering
        let status = WhoStatus::Requested(
            WhoSource::User,
            Instant::now(),
            params
                .get(2)
                .and_then(|token| token.parse::<WhoToken>().ok()),
        );

        if let Some(who_poll) = self
            .polls
            .iter_mut()
            .find(|who_poll| who_poll.channel == channel)
        {
            who_poll.status = status;
        } else {
            self.polls.push_front(WhoPoll { channel, status });
        }
    }

    pub fn queue_channel_poll(&mut self, channel: &Channel) {
        if !self
            .polls
            .iter()
            .any(|who_poll| who_poll.channel == *channel)
        {
            self.polls.push_front(WhoPoll {
                channel: channel.to_owned(),
                status: WhoStatus::Joined,
            });
        }
    }

    pub fn receiving_channel_poll(
        &mut self,
        server: &client::Server,
        channel: &Channel,
    ) {
        if let Some(who_poll) = self
            .polls
            .iter_mut()
            .find(|who_poll| who_poll.channel == *channel)
            && let WhoStatus::Requested(source, _, None) = &who_poll.status
        {
            who_poll.status = WhoStatus::Receiving(source.clone(), None);
            log::trace!("[{server}] {channel} - WHO receiving...");
        }
    }

    pub fn handle_whox_reply(
        &mut self,
        casemapping: isupport::CaseMap,
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

        let bot_mode_char = isupport::get_bot_mode_char(isupport);

        if let Some(who_poll) = self
            .polls
            .iter_mut()
            .find(|who_poll| who_poll.channel == *target_channel)
        {
            match &who_poll.status {
                WhoStatus::Requested(source, _, Some(request_token))
                    if matches!(source, WhoSource::Poll) =>
                {
                    if let Ok(token) = ok!(args.get(1)).parse::<WhoToken>()
                        && *request_token == token
                    {
                        who_poll.status = WhoStatus::Receiving(
                            source.clone(),
                            Some(*request_token),
                        );
                        log::trace!(
                            "[{server}] {target_channel} - WHO receiving...",
                        );
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

                    log::trace!(
                        "[{server}] {target_channel} - WHO receiving...",
                    );
                }
                _ => (),
            }

            if let WhoStatus::Receiving(WhoSource::Poll, Some(_)) =
                &who_poll.status
            {
                // Check token to ~ensure reply is to poll request
                if let Ok(token) = ok!(args.get(1)).parse::<WhoToken>() {
                    if token == WhoXPollParameters::Default.token() {
                        client_channel.update_user_status(
                            ok!(args.get(3)),
                            ok!(args.get(4)),
                            casemapping,
                            bot_mode_char,
                        );
                    } else if token
                        == WhoXPollParameters::WithAccountName.token()
                    {
                        let nick = ok!(args.get(3));

                        client_channel.update_user_status(
                            nick,
                            ok!(args.get(4)),
                            casemapping,
                            bot_mode_char,
                        );

                        client_channel.update_user_accountname(
                            nick,
                            ok!(args.get(5)),
                            casemapping,
                        );
                    } else if token == WhoXPollParameters::InitialJoin.token()
                        && let flags = ok!(args.get(6))
                        && let Ok(mut user) = User::parse_from_whoreply(
                            ok!(args.get(5)),
                            flags,
                            ok!(args.get(3)),
                            ok!(args.get(4)),
                            casemapping,
                            isupport::get_prefix(isupport),
                        )
                        && client_channel.users.resolve(&user).is_none()
                    {
                        if let Some(account) = args.get(7) {
                            user = user.with_accountname(account);
                        }
                        if flags.starts_with('G') {
                            user.update_away(true);
                        }
                        if let Some(bot_char) = bot_mode_char {
                            user.update_bot(flags.contains(bot_char));
                        }
                        client_channel.users.insert(user);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn handle_end_who_reply(
        &mut self,
        chanmap: &IndexMap<Channel, client::Channel>,
        config: &Arc<config::Server>,
        channel: &Channel,
    ) {
        if let Some(pos) = self
            .polls
            .iter()
            .position(|who_poll| who_poll.channel == *channel)
        {
            self.polls[pos].status = WhoStatus::Received;

            if let Some(who_poll) = self.polls.remove(pos)
                && chanmap.contains_key(channel)
                && config.who_poll_enabled
            {
                self.polls.push_back(who_poll);
            }

            // Prioritize WHO requests due to joining a channel
            if let Some(pos) = self
                .polls
                .iter()
                .position(|who_poll| {
                    matches!(who_poll.status, WhoStatus::Joined)
                })
                .or(self.polls.iter().position(|who_poll| {
                    matches!(who_poll.status, WhoStatus::Received)
                }))
            {
                self.polls[pos].status = WhoStatus::Waiting(Instant::now());

                if pos != 0
                    && let Some(who_poll) = self.polls.remove(pos)
                {
                    self.polls.push_front(who_poll);
                }
            }
        }
    }

    pub fn interval_long_enough(&mut self) {
        self.interval.long_enough();
    }

    pub fn interval_set_min(&mut self, interval: Duration) {
        self.interval.set_min(interval);
    }

    pub fn interval_too_short(&mut self) {
        self.interval.too_short();
    }

    pub fn interval_duration(&self) -> Duration {
        self.interval.duration()
    }

    // User did not request, treat as part of rate-limiting response
    // (in conjunction with RPL_TRYAGAIN) and don't save to history.
    pub fn handle_who_rate_limit(&mut self) {
        if let Some(who_poll) = self.polls.front_mut() {
            who_poll.status = WhoStatus::Waiting(Instant::now());
        }

        self.polls
            .iter_mut()
            .skip(1)
            .for_each(|who_poll| who_poll.status = WhoStatus::Received);
    }

    pub fn prioritize_joined_who_poll(&mut self, channel: Channel) {
        if let Some(pos) = self.polls.iter().position(|who_poll| {
            who_poll.channel == channel
                && matches!(
                    who_poll.status,
                    WhoStatus::Joined | WhoStatus::Waiting(_)
                )
        }) && pos != 0
            && let Some(who_poll) = self.polls.remove(pos)
        {
            log::debug!("prioritizing who poll for: {channel:?}");
            self.polls.push_front(who_poll);
        }
    }
}
