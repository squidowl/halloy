use std::ops::Range;
use std::sync::Arc;

use chrono::{DateTime, Local, Utc};
use hashbrown::HashSet;
use url::Url;

use super::{Content, Id, Message, Searchable, Source, Temporal, Time};
use crate::redaction::Redaction;
use crate::{User, config, history};

/// An IRC message with additional data used to display the message.  The
/// aditional data is not stored with the message, it is looked up or created as
/// needed for display based on the message contents, the message context (e.g.
/// whether it can be condensed into another message), and the user's
/// configuration.
#[derive(Debug, Clone)]
pub struct MessageDisplay {
    pub inner: Arc<Message>,
    pub blocked: bool,
    pub condensed: Option<Arc<MessageDisplay>>,
    pub expanded: bool, // Only relevant if message.can_condense() or message.redaction.is_some()
    pub reply_preview: Option<ReplyPreview>,
}

impl From<&Message> for MessageDisplay {
    fn from(message: &Message) -> Self {
        Self::from(message.clone())
    }
}

impl From<Message> for MessageDisplay {
    fn from(message: Message) -> Self {
        Self {
            inner: Arc::new(message),
            blocked: false,
            condensed: None,
            expanded: false,
            reply_preview: None,
        }
    }
}

impl MessageDisplay {
    pub fn displayed(
        &self,
        config: &config::Buffer,
        now: DateTime<Utc>,
    ) -> Option<&Self> {
        if self.blocked {
            return None;
        }
        if self.inner.can_condense(&config.server_messages.condense) {
            return if self.expanded {
                Some(self)
            } else {
                self.condensed.as_deref().filter(|summary| {
                    match &summary.inner.content {
                        Content::Plain(text) => !text.is_empty(),
                        Content::Fragments(fragments) => fragments
                            .iter()
                            .any(|fragment| !fragment.as_str().is_empty()),
                        Content::Log(record) => !record.message.is_empty(),
                    }
                })
            };
        }
        if let Source::Internal(super::source::Internal::Status(status)) =
            &self.inner.source
            && (!config.internal_messages.enabled(status)
                || config.internal_messages.smart(status).is_some_and(
                    |seconds| {
                        history::smart_filter_internal_message(
                            self.inner.as_ref(),
                            &seconds,
                            &now,
                        )
                    },
                ))
        {
            return None;
        }
        Some(self)
    }

    pub fn condensation_range(
        messages: &[Self],
        index: usize,
        config: &config::buffer::Condensation,
    ) -> Option<Range<usize>> {
        let source = messages.get(index)?;
        if source.blocked || !source.inner.can_condense(config) {
            return None;
        }
        let date = source.time().utc.with_timezone(&Local).date_naive();
        let same_group = |message: &Self| {
            message.inner.can_condense(config)
                && message.time().utc.with_timezone(&Local).date_naive() == date
        };
        let mut start = index;
        for (position, message) in messages[..index].iter().enumerate().rev() {
            if message.blocked {
                continue;
            }
            if !same_group(message) {
                break;
            }
            start = position;
        }
        let mut end = index + 1;
        for (position, message) in messages.iter().enumerate().skip(end) {
            if message.blocked {
                continue;
            }
            if !same_group(message) {
                break;
            }
            end = position + 1;
        }
        Some(start..end)
    }

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
            hidden_urls: self.inner.hidden_urls.clone(),
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
    pub hidden_urls: HashSet<Url>,
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
