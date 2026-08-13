use chrono::{DateTime, Utc};
use irc::proto::Command;
use serde::{Deserialize, Serialize};

use crate::isupport;
use crate::message::{Encoded, Id, Time};
use crate::target::Target;
use crate::user::{Nick, NickRef};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Redaction {
    pub from: Nick,
    #[serde(default)]
    pub reason: Option<String>,
}

impl Redaction {
    pub fn message(&self) -> String {
        match &self.reason {
            Some(reason) if !reason.is_empty() => {
                format!("Message redacted by {}: {reason}", self.from)
            }
            _ => format!("Message redacted by {}", self.from),
        }
    }

    pub fn received(
        message: Encoded,
        our_nick: NickRef<'_>,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Option<RedactionWithContext> {
        let user = message.user(casemapping)?;
        let id = message.message_id();
        let time = message.time();

        let Command::REDACT(target, msgid, reason) = message.0.command else {
            return None;
        };

        let target =
            if casemapping.normalize(&target) == our_nick.as_normalized_str() {
                Target::from(&user)
            } else {
                Target::parse(&target, chantypes, statusmsg, casemapping)
            };

        let redacts = Id::from(msgid.as_str());

        Some(RedactionWithContext {
            inner: Redaction {
                from: Nick::from(user),
                reason,
            },
            target,
            redacts,
            id,
            time,
        })
    }
}

#[derive(Debug)]
pub struct RedactionWithContext {
    pub inner: Redaction,
    pub target: Target,
    pub redacts: Id,
    pub id: Option<Id>,
    pub time: Time,
}

impl From<RedactionWithContext> for Redaction {
    fn from(redaction_with_context: RedactionWithContext) -> Self {
        redaction_with_context.inner
    }
}

impl RedactionWithContext {
    pub fn server_time(&self) -> Option<DateTime<Utc>> {
        self.time.try_into_server_time()
    }
}
