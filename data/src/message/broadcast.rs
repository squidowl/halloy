//! Generate messages that can be broadcast into every buffer
use std::collections::HashSet;

use super::{
    Content, Direction, Message, Source, kick_text, nickname_text,
    parse_fragments_with_user, plain, quit_text, source,
};
use crate::config::buffer::UsernameFormat;
use crate::user::Nick;
use crate::{Config, User, history, isupport, message, target};

#[derive(Debug, Clone)]
pub enum Broadcast {
    Disconnected {
        error: Option<String>,
    },
    Reconnected,
    Quit {
        user: User,
        comment: Option<String>,
    },
    ChangeNickname {
        old_nick: Nick,
        new_nick: Nick,
        ourself: bool,
    },
    ChangeHost {
        old_user: User,
        new_username: String,
        new_hostname: String,
        ourself: bool,
        logged_in: bool,
    },
    Kick {
        kicker: User,
        victim: User,
        reason: Option<String>,
    },
}

impl Broadcast {
    fn source(&self) -> message::Source {
        match self {
            Broadcast::Disconnected { .. } => Source::Internal(
                source::Internal::Status(source::Status::Error),
            ),
            Broadcast::Reconnected => Source::Internal(
                source::Internal::Status(source::Status::Success),
            ),
            Broadcast::Quit { user, .. } => {
                Source::Server(Some(source::Server::new(
                    source::server::Kind::Quit,
                    Some(user.nickname().clone()),
                    None,
                )))
            }
            Broadcast::ChangeNickname {
                old_nick, new_nick, ..
            } => Source::Server(Some(source::Server::new(
                source::server::Kind::ChangeNick,
                Some(old_nick.clone()),
                Some(source::server::Change::Nick(new_nick.clone())),
            ))),
            Broadcast::ChangeHost {
                old_user,
                new_hostname,
                ..
            } => Source::Server(Some(source::Server::new(
                source::server::Kind::ChangeHost,
                Some(old_user.nickname().clone()),
                old_user.hostname().map(|old_hostname| {
                    source::server::Change::Host(
                        old_hostname.to_string(),
                        new_hostname.to_string(),
                    )
                }),
            ))),
            Broadcast::Kick { kicker, .. } => {
                Source::Server(Some(source::Server::new(
                    source::server::Kind::Kick,
                    Some(kicker.nickname().clone()),
                    None,
                )))
            }
        }
    }

    fn content(
        &self,
        target: &message::Target,
        casemapping: isupport::CaseMap,
        config: &Config,
    ) -> Content {
        match self {
            Broadcast::Disconnected { error } => {
                let error = error
                    .as_ref()
                    .map(|error| format!(" ({error})"))
                    .unwrap_or_default();

                plain(format!("Connection to server lost{error}"))
            }
            Broadcast::Reconnected => {
                plain("Connection to server restored".into())
            }
            Broadcast::Quit { user, comment } => {
                quit_text(user, comment, config, casemapping)
            }
            Broadcast::ChangeNickname {
                old_nick,
                new_nick,
                ourself,
            } => nickname_text(
                old_nick.into(),
                new_nick.into(),
                *ourself,
                casemapping,
            ),
            Broadcast::ChangeHost {
                old_user,
                new_username,
                new_hostname,
                ourself,
                ..
            } => {
                if *ourself {
                    plain(format!(
                        "You've changed host to {new_username}@{new_hostname}",
                    ))
                } else {
                    parse_fragments_with_user(
                        format!(
                            "{} changed host to {new_username}@{new_hostname}",
                            old_user.formatted(UsernameFormat::Full)
                        ),
                        old_user,
                        casemapping,
                    )
                }
            }
            Broadcast::Kick {
                kicker,
                victim,
                reason,
            } => {
                kick_text(
                    kicker.clone(),
                    victim.clone(),
                    true, // Broadcast of KICK is always ourself
                    reason,
                    target.as_channel().cloned(),
                    &config.display.direction_arrows,
                    casemapping,
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct BroadcastWithContext {
    pub inner: Broadcast,
    pub in_channels: Vec<target::Channel>,
    pub in_queries: Queries,
    pub in_server: bool,
    pub time: message::Time,
}

impl BroadcastWithContext {
    pub fn into_messages(
        self,
        targets: Vec<message::Target>,
        casemapping: isupport::CaseMap,
        config: &Config,
    ) -> Vec<message::Message> {
        targets
            .into_iter()
            .map(|target| {
                let content = self.inner.content(&target, casemapping, config);

                Message {
                    history_id: history::Id::default(),
                    time: self.time,
                    direction: Direction::Received { is_echo: false },
                    source: self.inner.source(),
                    target,
                    content,
                    id: None,
                    reply_to: None,
                    relayed_by: None,
                    hidden_urls: HashSet::default(),
                    reactions: vec![],
                    rerouted_from: None,
                    redaction: None,
                }
            })
            .collect()
    }
}

#[derive(Debug, Default)]
pub enum Queries {
    All,
    WithNick(Nick),
    #[default]
    None,
}
