use chrono::{DateTime, Utc};
use irc::proto::Command;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use crate::capabilities::LabeledResponseContext;
use crate::isupport;
use crate::message::{Direction, Encoded, Id, Time};
use crate::target::Target;
use crate::user::{Nick, NickRef};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reaction {
    pub sender: Nick,
    pub text: String,
    pub unreact: bool,
    #[serde(default)]
    pub id: Option<Id>,
    pub time: Time,
}

impl Reaction {
    pub fn received(
        message: Encoded,
        our_nick: NickRef<'_>,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
        max_reaction_chars: u32,
    ) -> Option<ReactionWithContext> {
        let user = message.user(casemapping)?;
        let (text, unreact) = match (
            message.tags.get("+draft/react"),
            message.tags.get("+draft/unreact"),
        ) {
            (Some(s), None) => (s.clone(), false),
            (None, Some(s)) => (s.clone(), true),
            _ => return None,
        };
        // Drop reactions above the maximum rather than truncate, to avoid
        // potentially creating a new, separate reaction when interacting with
        // it (from the perspective of other clients)
        if UnicodeSegmentation::graphemes(text.as_str(), true).count()
            > max_reaction_chars as usize
        {
            return None;
        }
        let in_reply_to = message.in_reply_to()?;
        let id = message.message_id();
        let time = message.time();
        let is_echo = our_nick == *user.nickname();

        let (Command::PRIVMSG(target, _) | Command::TAGMSG(target)) =
            message.0.command
        else {
            return None;
        };

        let target =
            if casemapping.normalize(&target) == our_nick.as_normalized_str() {
                Target::from(&user)
            } else {
                Target::parse(&target, chantypes, statusmsg, casemapping)
            };

        let sender = Nick::from(user);

        Some(ReactionWithContext {
            inner: Reaction {
                sender,
                text,
                unreact,
                id,
                time,
            },
            in_reply_to,
            target,
            direction: Direction::Received { is_echo },
            labeled_response_context: None,
            historical: false,
            notification_allowed: true,
        })
    }

    pub fn server_time(&self) -> Option<DateTime<Utc>> {
        self.time.try_into_server_time()
    }
}

#[derive(Debug, Clone)]
pub struct ReactionWithContext {
    pub inner: Reaction,
    pub target: Target,
    pub in_reply_to: Id,
    pub direction: Direction,
    pub labeled_response_context: Option<LabeledResponseContext>,
    pub historical: bool,
    pub notification_allowed: bool,
}

impl From<ReactionWithContext> for Reaction {
    fn from(reaction_with_context: ReactionWithContext) -> Self {
        reaction_with_context.inner
    }
}

impl ReactionWithContext {
    pub fn is_echo(&self) -> bool {
        matches!(self.direction, Direction::Received { is_echo } if is_echo)
    }

    pub fn is_ours(&self) -> bool {
        self.is_sent() || self.is_echo()
    }

    pub fn is_sent(&self) -> bool {
        matches!(self.direction, Direction::Sent { .. })
    }

    pub fn into_historical(self) -> Self {
        Self {
            historical: true,
            notification_allowed: false,
            ..self
        }
    }

    pub fn with_labeled_response_context(
        self,
        labeled_response_context: Option<LabeledResponseContext>,
    ) -> Self {
        Self {
            labeled_response_context,
            ..self
        }
    }

    pub fn with_notification_prohibited(self) -> Self {
        Self {
            notification_allowed: false,
            ..self
        }
    }
}
