use chrono::{DateTime, Utc};
use irc::proto::Command;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use crate::capabilities::LabeledResponseContext;
use crate::isupport;
use crate::message::{Encoded, Id};
use crate::serde::{deserialize_date_time_utc_or_epoch, fail_as_none};
use crate::target::Target;
use crate::user::Nick;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reaction {
    pub sender: Nick,
    pub text: String,
    pub unreact: bool,
    #[serde(default, deserialize_with = "fail_as_none")]
    pub id: Option<Id>,
    #[serde(default, deserialize_with = "deserialize_date_time_utc_or_epoch")]
    pub server_time: DateTime<Utc>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Context {
    pub inner: Reaction,
    pub target: Target,
    pub in_reply_to: Id,
    pub is_echo: bool,
    pub deduplicate: bool,
}

impl Reaction {
    pub fn received(
        message: Encoded,
        our_nick: Nick,
        deduplicate: bool,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
        max_reaction_chars: u32,
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
        let server_time = message.server_time_or_now();

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

        let is_echo = sender == our_nick;

        Some(Context {
            inner: Reaction {
                sender,
                text,
                unreact,
                id,
                server_time,
            },
            in_reply_to,
            target,
            is_echo,
            deduplicate,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Pending {
    pub reactions: Vec<PendingReaction>,
}

impl Pending {
    pub fn server_time(&self) -> Option<DateTime<Utc>> {
        self.reactions
            .iter()
            .fold(None, |server_time, pending_reaction| {
                Some(if let Some(server_time) = server_time {
                    server_time.min(pending_reaction.reaction.server_time)
                } else {
                    pending_reaction.reaction.server_time
                })
            })
    }
}

#[derive(Debug, Clone)]
pub struct PendingReaction {
    pub reaction: Reaction,
    pub is_echo: bool,
    pub deduplicate: bool,
    pub labeled_response_context: Option<LabeledResponseContext>,
    pub notification_enabled: bool,
}
