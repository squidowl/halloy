use super::{Direction, Id, Message, Searchable, Temporal, Time, highlight};
use crate::capabilities::LabeledResponseContext;
use crate::client::Destination;
use crate::history;

#[derive(Debug, Clone)]
pub struct MessageWithContext {
    pub inner: Message,
    pub highlight: Option<highlight::Kind>,
    pub labeled_response_context: Option<LabeledResponseContext>,
    pub historical: bool, // i.e. from chathistory or ZNC-playback (needs deduplication)
    pub notification_allowed: bool,
}

impl From<MessageWithContext> for Message {
    fn from(message_with_context: MessageWithContext) -> Self {
        message_with_context.inner
    }
}

impl MessageWithContext {
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
        if matches!(self.inner.direction, Direction::Sent { .. }) {
            Self {
                inner: self
                    .inner
                    .with_labeled_response_context(labeled_response_context),
                ..self
            }
        } else {
            Self {
                labeled_response_context,
                ..self
            }
        }
    }

    pub fn with_notification_prohibited(self) -> Self {
        Self {
            notification_allowed: false,
            ..self
        }
    }

    pub fn with_target(self, target: Destination) -> Self {
        Self {
            inner: self.inner.with_target(target),
            ..self
        }
    }
}

impl Temporal for MessageWithContext {
    fn time(&self) -> &Time {
        self.inner.time()
    }
}

impl Searchable for MessageWithContext {
    fn history_id(&self) -> &history::Id {
        self.inner.history_id()
    }

    fn id(&self) -> &Option<Id> {
        self.inner.id()
    }
}
