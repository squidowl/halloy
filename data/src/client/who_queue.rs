use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use indexmap::IndexMap;
use irc::proto::{Message, command};

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
}

#[derive(Debug, Clone)]
pub enum WhoStatus {
    Requested(WhoSource, Instant, Option<WhoToken>),
    Receiving(WhoSource, Option<WhoToken>),
    Received,
    Waiting(Instant),
    Joined,
    PrioritizedJoin,
}

#[derive(Debug, Clone)]
pub enum WhoSource {
    User,
    Poll,
}

#[derive(Debug, Clone)]
pub struct WhoQueue {
    config: Arc<config::Server>,
    queue: VecDeque<WhoPoll>,
    inflight: Vec<WhoPoll>,
    interval: BackoffInterval,
    last_throttled: Option<Instant>,
}
#[derive(Debug)]
enum WhoRequest {
    Poll,
    Retry,
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
            queue: VecDeque::new(),
            inflight: Vec::new(),
            interval,
            last_throttled: None,
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

        if let Some(mut who_poll) = self.queue.pop_front() {
            let request = match &who_poll.status {
                WhoStatus::Joined | WhoStatus::PrioritizedJoin => (capabilities
                    .acknowledged(Capability::NoImplicitNames)
                    || capabilities.acknowledged(Capability::AwayNotify)
                    || config.who_poll_enabled)
                    .then_some(WhoRequest::Poll),
                WhoStatus::Waiting(last) => ((capabilities
                    .acknowledged(Capability::NoImplicitNames)
                    || config.who_poll_enabled)
                    && (now.duration_since(*last) >= self.interval.duration()))
                .then_some(WhoRequest::Poll),
                _ => None,
            };

            if request.is_some() {
                log::trace!("[{}] {} - WHO poll", server, who_poll.channel);

                let message = tick_who_poll_request(
                    &mut who_poll,
                    capabilities,
                    chanmap,
                    isupport,
                );
                self.inflight.push(who_poll);

                messages.push((message.into(), TokenPriority::Low));
            } else {
                self.queue.push_back(who_poll);
            }
        }

        if let Some(who_poll) = self.inflight.first_mut() {
            let request = match &who_poll.status {
                WhoStatus::Requested(source, requested, _) => {
                    if matches!(source, WhoSource::Poll)
                        && !config.who_poll_enabled
                    {
                        None
                    } else {
                        (now.duration_since(*requested)
                            >= 5 * self.interval.duration())
                        .then_some(WhoRequest::Retry)
                    }
                }
                _ => None,
            };

            if request.is_some() {
                log::trace!("[{}] {} - WHO retry", server, who_poll.channel);

                let message = tick_who_poll_request(
                    who_poll,
                    capabilities,
                    chanmap,
                    isupport,
                );

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
                    self.queue.push_back(WhoPoll {
                        channel: channel.clone(),
                        status: WhoStatus::Joined,
                    });
                }
            } else {
                self.queue.truncate(0);
            }
        }
        self.config = config;
    }

    pub fn quit(&mut self) {
        self.queue.truncate(0);
    }

    pub fn any_matching_polls<F>(&self, f: F) -> bool
    where
        F: Fn(&WhoPoll) -> bool,
    {
        self.queue.iter().any(f)
    }

    pub fn any_matching_inflight<F>(&self, f: F) -> bool
    where
        F: Fn(&WhoPoll) -> bool,
    {
        self.queue.iter().any(f)
    }

    pub fn has_inflight(&self) -> bool {
        !self.inflight.is_empty()
    }

    pub fn pos_matching_polls<F>(&self, f: F) -> Option<usize>
    where
        F: Fn(&WhoPoll) -> bool,
    {
        self.queue.iter().position(f)
    }

    pub fn remove_matching_polls<F>(&mut self, f: F) -> bool
    where
        F: Fn(&WhoPoll) -> bool,
    {
        if let Some(pos) = self.pos_matching_polls(f) {
            self.queue.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn pos_matching_inflight<F>(&self, f: F) -> Option<usize>
    where
        F: Fn(&WhoPoll) -> bool,
    {
        self.queue.iter().position(f)
    }

    pub fn remove_matching_inflight<F>(&mut self, f: F) -> bool
    where
        F: Fn(&WhoPoll) -> bool,
    {
        if let Some(pos) = self.pos_matching_inflight(f) {
            self.queue.remove(pos);
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

        if let Some(pos) = self
            .queue
            .iter()
            .position(|who_poll| who_poll.channel == channel)
            && let Some(mut who_poll) = self.queue.remove(pos)
        {
            who_poll.status = status;
            self.inflight.push(who_poll);
        } else {
            self.inflight.push(WhoPoll { channel, status });
        }
    }

    pub fn queue_channel_poll(&mut self, channel: &Channel) {
        if !self
            .queue
            .iter()
            .any(|who_poll| who_poll.channel == *channel)
            && !self
                .inflight
                .iter()
                .any(|who_poll| who_poll.channel == *channel)
        {
            self.queue.push_front(WhoPoll {
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
            .inflight
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
            .inflight
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
                        && let nick = ok!(args.get(5))
                        && let Ok(user) = User::parse_from_whoreply(
                            nick,
                            flags,
                            ok!(args.get(3)),
                            ok!(args.get(4)),
                            casemapping,
                            isupport::get_prefix(isupport),
                        )
                        && client_channel.users.resolve(&user).is_none()
                    {
                        client_channel.users.insert(user);

                        client_channel.update_user_status(
                            nick,
                            flags,
                            casemapping,
                            bot_mode_char,
                        );

                        client_channel.update_user_accountname(
                            nick,
                            ok!(args.get(7)),
                            casemapping,
                        );
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
        capabilities: &Capabilities,
        channel: &Channel,
    ) {
        if let Some(pos) = self
            .inflight
            .iter()
            .position(|who_poll| who_poll.channel == *channel)
        {
            let mut who_poll = self.inflight.remove(pos);
            if chanmap.contains_key(channel)
                && config.who_poll_enabled
                && !capabilities.acknowledged(Capability::AwayNotify)
            {
                who_poll.status = WhoStatus::Received;
                self.queue.push_back(who_poll);
            }

            // Prioritize next WHO request due to joining a channel
            if let Some(pos) = self
                .queue
                .iter()
                .position(|who_poll| {
                    matches!(who_poll.status, WhoStatus::PrioritizedJoin)
                })
                .or(self.queue.iter().position(|who_poll| {
                    matches!(who_poll.status, WhoStatus::Joined)
                }))
                .or(self.queue.iter().position(|who_poll| {
                    matches!(who_poll.status, WhoStatus::Received)
                }))
            {
                self.queue[pos].status = WhoStatus::Waiting(Instant::now());

                if pos != 0
                    && let Some(who_poll) = self.queue.remove(pos)
                {
                    self.queue.push_front(who_poll);
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

    pub fn who_reply_throttled(&mut self) {
        self.last_throttled = Some(Instant::now());

        self.interval.too_short();
    }

    pub fn interval_duration(&self) -> Duration {
        self.interval.duration()
    }

    // User did not request, treat as part of rate-limiting response
    // (in conjunction with RPL_TRYAGAIN) and don't save to history.
    pub fn handle_who_rate_limited(&mut self) {
        if let Some(who_poll) = self.queue.front_mut() {
            who_poll.status = WhoStatus::Waiting(Instant::now());
        }

        self.queue
            .iter_mut()
            .skip(1)
            .for_each(|who_poll| who_poll.status = WhoStatus::Received);
    }

    pub fn prioritize_joined_who_poll(
        &mut self,
        server: &Server,
        channel: &Channel,
    ) {
        if let Some(pos) = self
            .queue
            .iter()
            .position(|who_poll| &who_poll.channel == channel)
            && pos != 0
            && let Some(mut who_poll) = self.queue.remove(pos)
        {
            if matches!(who_poll.status, WhoStatus::Joined) {
                who_poll.status = WhoStatus::PrioritizedJoin;
            }

            log::trace!(
                "[{}] {} - WHO poll prioritizing join",
                server,
                channel.as_normalized_str(),
            );
            self.queue.push_front(who_poll);
        }
    }

    pub fn deprioritize_joined_who_poll(
        &mut self,
        server: &Server,
        channel: Channel,
    ) {
        if let Some(pos) = self.queue.iter().position(|who_poll| {
            who_poll.channel == channel
                && matches!(who_poll.status, WhoStatus::PrioritizedJoin)
        }) {
            log::trace!(
                "[{}] {} - WHO poll deprioritizing join",
                server,
                channel.as_normalized_str()
            );
            self.queue[pos].status = WhoStatus::Joined;
        }
    }

    pub fn reprioritize_joined_who_polls(
        &mut self,
        server: &Server,
        opened_channels: &Vec<(&Server, &Channel)>,
        exclude_channel: &Channel,
    ) {
        let channels_to_deprioritize: Vec<Channel> = self
            .queue
            .iter()
            .filter(|who_poll| {
                matches!(who_poll.status, WhoStatus::PrioritizedJoin)
                    && who_poll.channel != *exclude_channel
                    && !opened_channels
                        .iter()
                        .any(|(s, c)| *s == server && *c == &who_poll.channel)
            })
            .map(|who_poll| who_poll.channel.to_owned())
            .collect();

        for channel in channels_to_deprioritize {
            self.deprioritize_joined_who_poll(server, channel);
        }
    }

    pub fn process_next_prioritized_join(
        &mut self,
        server: &client::Server,
        capabilities: &Capabilities,
        chanmap: &IndexMap<Channel, client::Channel>,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
    ) -> Vec<(message::Encoded, TokenPriority)> {
        let mut messages: Vec<(message::Encoded, TokenPriority)> = vec![];

        let now = Instant::now();
        if let Some(last_throttled) = self.last_throttled
            && now.duration_since(last_throttled) < self.interval.duration()
        {
            log::trace!(
                "[{server}] - WHO poll throttled, end next prioritizing"
            );
            return messages;
        }

        if let Some(pos) = self.queue.iter().position(|who_poll| {
            matches!(who_poll.status, WhoStatus::PrioritizedJoin)
        }) && let Some(mut who_poll) = self.queue.remove(pos)
        {
            log::trace!(
                "[{}] {} - WHO poll processing next prioritized join",
                server,
                who_poll.channel
            );

            let message = tick_who_poll_request(
                &mut who_poll,
                capabilities,
                chanmap,
                isupport,
            );
            self.inflight.push(who_poll);

            messages.push((message.into(), TokenPriority::Low));
        }
        messages
    }
}

fn tick_who_poll_request(
    who_poll: &mut WhoPoll,
    capabilities: &Capabilities,
    chanmap: &IndexMap<Channel, client::Channel>,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
) -> Message {
    if isupport.contains_key(&isupport::Kind::WHOX) {
        let whox_params = if capabilities
            .acknowledged(Capability::NoImplicitNames)
            && chanmap
                .get(&who_poll.channel)
                .is_none_or(|channel| !channel.who_init)
        {
            WhoXPollParameters::InitialJoin
        } else if capabilities.acknowledged(Capability::AccountNotify) {
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
        who_poll.status =
            WhoStatus::Requested(WhoSource::Poll, Instant::now(), None);

        command!("WHO", who_poll.channel.to_string())
    }
}
