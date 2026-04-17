use chrono::{DateTime, Utc};
use irc::proto::Command;

use crate::isupport;
use crate::message::{Encoded, Id};
use crate::target::Target;
use crate::user::Nick;

#[derive(Debug, Clone)]
pub struct Redaction {
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
    ) -> Option<Self> {
        let user = message.user(casemapping)?;
        let server_time = message.server_time_or_now();

        let Command::REDACT(target, msgid, _) = message.0.command else {
            return None;
        };

        let target =
            if casemapping.normalize(&target) == our_nick.as_normalized_str() {
                Target::from(&user)
            } else {
                Target::parse(&target, chantypes, statusmsg, casemapping)
            };

        let id = Id::from(msgid.as_str());

        Some(Self {
            target,
            id,
            server_time,
        })
    }
}

#[derive(Debug)]
pub struct Pending {
    pub server_time: DateTime<Utc>,
}

impl Pending {
    pub fn new(server_time: DateTime<Utc>) -> Self {
        Self { server_time }
    }
}
