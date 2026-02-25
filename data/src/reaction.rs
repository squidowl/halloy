use chrono::{DateTime, Utc};
use irc::proto::Command;
use serde::{Deserialize, Serialize};

use crate::isupport;
use crate::message::{Encoded, Id};
use crate::target::Target;
use crate::user::Nick;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reaction {
    pub sender: Nick,
    pub text: String,
    pub unreact: bool,
}

#[derive(Debug)]
pub struct Context {
    pub inner: Reaction,
    pub target: Target,
    pub in_reply_to: Id,
    pub server_time: DateTime<Utc>,
}

impl Reaction {
    pub fn received(
        message: Encoded,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Option<Context> {
        let user = message.user(casemapping)?;
        let (text, unreact) = match (
            message.tags.get("+draft/react"),
            message.tags.get("+draft/unreact"),
        ) {
            (Some(s), None) => (s.clone(), false),
            (None, Some(s)) => (s.clone(), true),
            _ => return None,
        };
        let in_reply_to = message.in_reply_to()?;
        let server_time = message.server_time();

        let (Command::PRIVMSG(target, _) | Command::TAGMSG(target)) =
            message.0.command
        else {
            return None;
        };

        Some(Context {
            inner: Reaction {
                sender: Nick::from(user),
                text,
                unreact,
            },
            in_reply_to,
            target: Target::parse(&target, chantypes, statusmsg, casemapping),
            server_time,
        })
    }
}

#[derive(Debug)]
pub struct Pending {
    pub reactions: Vec<Reaction>,
    pub server_time: DateTime<Utc>,
}

impl Pending {
    pub fn new(server_time: DateTime<Utc>) -> Self {
        Self {
            reactions: vec![],
            server_time,
        }
    }
}
