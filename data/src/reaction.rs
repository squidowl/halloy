use chrono::{DateTime, Utc};
use irc::proto::Command;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

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
        let text = truncate_text(&text, max_reaction_chars as usize);
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

pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if UnicodeSegmentation::graphemes(text, true).count() <= max_chars {
        return text.to_string();
    }

    let mut truncated = UnicodeSegmentation::graphemes(text, true)
        .take(max_chars)
        .collect::<String>();
    truncated.push_str("...");
    truncated
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

#[cfg(test)]
mod tests {
    use super::truncate_text;

    #[test]
    fn keeps_short_reaction_text() {
        assert_eq!(truncate_text("hello", 5), "hello");
    }

    #[test]
    fn truncates_ascii_to_limit() {
        assert_eq!(truncate_text("hello world", 5), "hello...");
    }

    #[test]
    fn truncates_unicode_graphemes() {
        assert_eq!(truncate_text("cafe\u{301}", 4), "cafe\u{301}");
    }

    #[test]
    fn limit_one_keeps_first_grapheme_when_truncated() {
        assert_eq!(truncate_text("👍🏽👍🏽", 1), "👍🏽...");
    }

    #[test]
    fn does_not_split_zwj_emoji_clusters() {
        assert_eq!(truncate_text("👨‍👩‍👧‍👦x", 1), "👨‍👩‍👧‍👦...");
    }

    #[test]
    fn does_not_split_combining_mark_clusters() {
        assert_eq!(truncate_text("a\u{0301}b", 1), "a\u{0301}...");
    }
}
