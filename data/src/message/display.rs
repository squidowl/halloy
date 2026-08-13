use std::sync::Arc;

use super::{Content, Id, Message, Searchable, Source, Temporal, Time};
use crate::redaction::Redaction;
use crate::{User, config, history};

#[derive(Debug, Clone)]
pub struct MessageDisplay {
    pub inner: Message,
    pub blocked: bool,
    pub condensed: Option<Arc<MessageDisplay>>,
    pub expanded: bool, // Only relevant if message.can_condense() or message.redaction.is_some()
    pub reply_preview: Option<ReplyPreview>,
}

impl From<&Message> for MessageDisplay {
    fn from(message: &Message) -> Self {
        Self {
            inner: message.clone(),
            blocked: false,
            condensed: None,
            expanded: false,
            reply_preview: None,
        }
    }
}

impl MessageDisplay {
    pub fn redaction_expanded(
        &self,
        config: &config::buffer::Redaction,
    ) -> Option<bool> {
        (self.inner.redaction.is_some() && config.display.is_redacted())
            .then_some(self.expanded)
    }

    pub fn as_reply_preview(&self) -> ReplyPreview {
        ReplyPreview {
            history_id: *self.history_id(),
            time: *self.time(),
            user: self.inner.user().cloned(),
            content: self.inner.content.clone(),
            in_reply_to: self.reply_preview.clone().map(Box::new),
            redaction: self.inner.redaction.clone(),
            blocked: self.blocked,
            is_action: matches!(self.inner.source, Source::Action(_)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplyPreview {
    pub history_id: history::Id,
    pub time: Time,
    pub user: Option<User>,
    pub content: Content,
    pub in_reply_to: Option<Box<ReplyPreview>>,
    pub redaction: Option<Redaction>,
    pub blocked: bool,
    pub is_action: bool,
}

impl ReplyPreview {
    pub fn preview_text(&self) -> String {
        match self {
            Self { blocked: true, .. } => {
                "Message blocked by Halloy configuration".to_string()
            }
            Self {
                redaction: Some(r), ..
            } => r.message(),
            Self {
                is_action: true,
                user: Some(user),
                ..
            } => action_preview_text(&self.content, user),
            _ => self.content.preview_text(),
        }
    }
}

/// in preview contexts the nick is added on the side as a `UserDisplay`
pub fn action_preview_text(content: &Content, user: &User) -> String {
    let text = content.preview_text();
    let prefix = format!("{} ", user.nickname());
    text.strip_prefix(&prefix).unwrap_or(&text).to_string()
}

impl Temporal for MessageDisplay {
    fn time(&self) -> &Time {
        self.inner.time()
    }
}

impl Searchable for MessageDisplay {
    fn history_id(&self) -> &history::Id {
        self.inner.history_id()
    }

    fn id(&self) -> &Option<Id> {
        self.inner.id()
    }
}
