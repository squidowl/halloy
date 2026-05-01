use chrono::{DateTime, Utc};
use irc::proto::Command;
use serde::{Deserialize, Serialize};

use crate::isupport;
use crate::message::{Encoded, Id};
use crate::target::Target;
use crate::user::Nick;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Redaction {
    pub from: Nick,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub struct Context {
    pub inner: Redaction,
    pub target: Target,
    pub id: Id,
    pub server_time: DateTime<Utc>,
}

impl Redaction {
    pub fn received(
        message: Encoded,
        our_nick: Nick,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Option<Context> {
        let user = message.user(casemapping)?;
        let server_time = message.server_time_or_now();

        let Command::REDACT(target, msgid, reason) = message.0.command else {
            return None;
        };

        let target =
            if casemapping.normalize(&target) == our_nick.as_normalized_str() {
                Target::from(&user)
            } else {
                Target::parse(&target, chantypes, statusmsg, casemapping)
            };

        let id = Id::from(msgid.as_str());

        Some(Context {
            inner: Redaction {
                from: Nick::from(user),
                reason,
            },
            target,
            id,
            server_time,
        })
    }
}

#[derive(Debug)]
pub struct Pending {
    pub redaction: Redaction,
    pub server_time: DateTime<Utc>,
}

impl Pending {
    pub fn new(redaction: Redaction, server_time: DateTime<Utc>) -> Self {
        Self {
            redaction,
            server_time,
        }
    }
}
