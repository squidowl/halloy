use std::borrow::Cow;

use chrono::{DateTime, Utc};

use super::{Direction, Id, Message, Source, Target, Time, highlight};
use crate::capabilities::LabeledResponseContext;
use crate::client::Destination;
use crate::isupport;
use crate::time::Posix;

#[derive(Debug, Clone)]
pub struct MessageWithContext {
    pub inner: Message,
    pub received_at: Posix,
    pub highlight: Option<highlight::Kind>,
    pub labeled_response_context: Option<LabeledResponseContext>,
    pub historical: bool, // i.e. from chathistory or ZNC-playback (needs deduplication)
    pub notification_allowed: bool,
    pub casemapping: isupport::CaseMap,
}

impl MessageWithContext {
    pub fn is_ours(&self) -> bool {
        self.inner.is_ours()
    }

    pub fn id(&self) -> &Option<Id> {
        &self.inner.id
    }

    pub fn server_time(&self) -> Option<DateTime<Utc>> {
        self.inner.server_time()
    }

    pub fn time(&self) -> &Time {
        &self.inner.time
    }

    pub fn source(&self) -> &Source {
        &self.inner.source
    }

    pub fn target(&self) -> &Target {
        &self.inner.target
    }

    pub fn text(&self) -> Cow<'_, str> {
        self.inner.text()
    }

    pub fn triggers_unread(&self) -> bool {
        self.inner.triggers_unread()
    }

    pub fn triggers_highlight(&self) -> bool {
        self.inner.triggers_highlight()
    }

    pub fn with_target(self, target: Destination) -> Self {
        Self {
            inner: self.inner.with_target(target),
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

    pub fn into_historical(self) -> Self {
        Self {
            historical: true,
            notification_allowed: false,
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
