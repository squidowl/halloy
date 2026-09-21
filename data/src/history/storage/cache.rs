use database::Extension;

use super::*;
use crate::history::database;

#[derive(Debug, Clone)]
pub struct Read {
    pub generation: u64,
    pub limit: message::Limit,
    pub clear: Option<DateTime<Utc>>,
    pub read_marker: Option<ReadMarker>,
    pub extension: Option<Extension>,
    tail_revision: u64,
}

#[derive(Debug, Default)]
pub(super) struct ReadCache {
    pub requested: Request,
    pub message_cache: MessageCache,
    // At most one read is in flight; invalidation makes its generation stale.
    generation: u64,
    pending: Option<u64>,
    // Dirty windows need replacement; append-only changes need just their tail.
    dirty: bool,
    failed: bool,
    // A tail response must not clear appends committed while it was in flight.
    tail_revision: u64,
    tail_pending: bool,
    pub(super) config: config::Buffer,
    // Trimming and renormalization can invalidate retained derived state even
    // when the database window itself remains useful for tail reads.
    pub(super) processed: bool,
}

#[derive(Debug, Default)]
pub(super) enum MessageCache {
    #[default]
    Unloaded,
    Loaded {
        has_more_older_messages: bool,
        has_more_newer_messages: bool,
        messages: Vec<message::MessageDisplay>,
        limit: message::Limit,
        clear: Option<DateTime<Utc>>,
    },
}

struct Selection {
    visible: usize,
    range: Range<usize>,
    older_needed: bool,
    newer_needed: bool,
}

fn cursor(message: &message::MessageDisplay) -> (i64, u64) {
    let Id::Determined(id) = message.inner.history_id else {
        unreachable!("cached messages have determined IDs");
    };
    (message.inner.time.utc.timestamp_micros(), id)
}

impl ReadCache {
    pub fn unrecorded(
        &mut self,
        mut message: message::Message,
        kind: &Kind,
        filters: FilterChain,
        clients: &dyn ClientsContext,
        config: &config::Buffer,
    ) {
        static NEXT: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(u64::MAX);
        let (limit, clear) = match self.requested {
            Request::Open { limit, clear } => (limit, clear),
            Request::Closed { .. } => (message::Limit::Bottom(50), None),
        };
        message.history_id = Id::Determined(
            NEXT.fetch_sub(1, std::sync::atomic::Ordering::Relaxed),
        );
        if matches!(self.message_cache, MessageCache::Unloaded) {
            self.message_cache = MessageCache::Loaded {
                messages: vec![],
                limit,
                clear,
                has_more_older_messages: false,
                has_more_newer_messages: false,
            };
        }
        self.config.clone_from(config);
        if let MessageCache::Loaded {
            messages,
            has_more_older_messages,
            ..
        } = &mut self.message_cache
        {
            messages.push(message::MessageDisplay::from(message));
            messages.sort_unstable_by_key(cursor);
            let keep = limit.count().saturating_mul(8).max(1);
            if messages.len() > keep {
                messages.drain(..messages.len() - keep);
                *has_more_older_messages = true;
            }
            renormalize_messages(kind, messages, clients);
            process_messages(kind, messages, filters, clients, config);
            self.processed = true;
        }
    }

    pub fn failed(&mut self) {
        self.failed = true;
        self.pending = None;
        self.dirty = false;
        self.tail_pending = false;
        // No reads can finish after terminal failure, including an initial read.
        if matches!(self.message_cache, MessageCache::Unloaded) {
            let (limit, clear) = match self.requested {
                Request::Open { limit, clear } => (limit, clear),
                Request::Closed { .. } => (message::Limit::Bottom(50), None),
            };
            self.message_cache = MessageCache::Loaded {
                messages: vec![],
                limit,
                clear,
                has_more_older_messages: false,
                has_more_newer_messages: false,
            };
        }
    }

    pub fn visible_through(&self, bound: ReadMarker) -> Option<ReadMarker> {
        let MessageCache::Loaded { messages, .. } = &self.message_cache else {
            return None;
        };
        messages
            .iter()
            .rev()
            .find(|message| {
                message.inner.time <= bound
                    && (!message.blocked || message.inner.is_ours())
            })
            .map(|message| ReadMarker::from(message.inner.as_ref()))
    }

    pub(super) fn invalidate_processing(&mut self) {
        self.processed = false;
    }

    pub fn changed(&mut self) {
        self.processed = false;
        self.dirty = true;
        self.tail_pending = false;
    }

    pub fn committed(&mut self, append_only: bool) {
        if append_only
            && !self.dirty
            && (self.tail_pending
                || matches!(
                    self.message_cache,
                    MessageCache::Loaded {
                        has_more_newer_messages: false,
                        ..
                    }
                ))
        {
            self.tail_revision = self.tail_revision.wrapping_add(1);
            self.tail_pending = true;
        } else {
            self.changed();
        }
    }

    pub fn invalidate(&mut self) {
        self.processed = false;
        self.generation = self.generation.wrapping_add(1);
        self.dirty = true;
        self.tail_pending = false;
    }

    fn useful(
        &self,
        requested: message::Limit,
        requested_clear: Option<DateTime<Utc>>,
        marker: Option<ReadMarker>,
    ) -> bool {
        let MessageCache::Loaded {
            messages,
            limit,
            clear,
            has_more_older_messages: older,
            has_more_newer_messages: newer,
        } = &self.message_cache
        else {
            return false;
        };
        if *clear != requested_clear {
            return false;
        }
        match requested {
            message::Limit::Top(_) => !older,
            message::Limit::Bottom(_) => !newer || self.tail_pending,
            message::Limit::Around(_, id) => {
                messages.iter().any(|m| m.inner.history_id == id)
                    || (matches!(limit, message::Limit::Around(_, loaded) if *loaded == id)
                        && !older
                        && !newer)
            }
            message::Limit::Backlog(_) => marker.map_or(!older, |marker| {
                (!older
                    || messages
                        .first()
                        .is_some_and(|message| message.inner.time <= marker))
                    && (!newer
                        || messages
                            .last()
                            .is_some_and(|message| message.inner.time > marker))
            }),
        }
    }

    fn selection(
        &self,
        requested: message::Limit,
        clear: Option<DateTime<Utc>>,
        marker: Option<ReadMarker>,
    ) -> Selection {
        let MessageCache::Loaded {
            messages,
            has_more_older_messages: older,
            has_more_newer_messages: newer,
            ..
        } = &self.message_cache
        else {
            return Selection {
                visible: 0,
                range: 0..0,
                older_needed: false,
                newer_needed: false,
            };
        };
        let now = Utc::now();
        let visible = || {
            messages
                .iter()
                .enumerate()
                .filter(|(_, message)| {
                    clear.is_none_or(|clear| message.inner.time.utc > clear)
                        && message.displayed(&self.config, now).is_some()
                })
                .map(|(index, _)| index)
        };
        let count = requested.count();
        let target = match requested {
            message::Limit::Top(_) => 0,
            message::Limit::Bottom(_) => messages.len(),
            message::Limit::Around(_, id) => messages
                .iter()
                .position(|message| message.inner.history_id == id)
                .unwrap_or(0),
            message::Limit::Backlog(_) => marker
                .and_then(|marker| {
                    messages
                        .iter()
                        .rposition(|message| message.inner.time <= marker)
                })
                .unwrap_or(0),
        };
        let target = message::MessageDisplay::condensation_range(
            messages,
            target,
            &self.config.server_messages.condense,
        )
        .map_or(target, |group| group.start);
        let (before, total) =
            visible().fold((0_usize, 0_usize), |(before, total), index| {
                (before + usize::from(index < target), total + 1)
            });
        let after = total.saturating_sub(before);
        let (mut want_before, mut want_after) = match requested {
            message::Limit::Top(_) => (0, count),
            message::Limit::Bottom(_) => (count, 0),
            _ => (count / 2, count - count / 2),
        };
        if !older && before < want_before {
            want_after += want_before - before;
            want_before = before;
        }
        if !newer && after < want_after {
            want_before += want_after - after;
            want_after = after;
        }
        let start_row = before.saturating_sub(want_before);
        let end_row = before.saturating_add(want_after).min(total);
        let mut range = if start_row < end_row {
            let mut rows = visible();
            let start = rows.nth(start_row).unwrap();
            let end = if end_row == start_row + 1 {
                start + 1
            } else {
                rows.nth(end_row - start_row - 2).unwrap() + 1
            };
            start..end
        } else {
            0..0
        };
        if !range.is_empty() {
            if let Some(group) = message::MessageDisplay::condensation_range(
                messages,
                range.start,
                &self.config.server_messages.condense,
            ) {
                range.start = group.start;
            }
            if let Some(group) = message::MessageDisplay::condensation_range(
                messages,
                range.end - 1,
                &self.config.server_messages.condense,
            ) {
                range.end = group.end;
            }
        }
        Selection {
            visible: total,
            range,
            older_needed: *older && before < want_before,
            newer_needed: *newer && after < want_after,
        }
    }

    pub fn request(&mut self, read_marker: Option<ReadMarker>) -> Option<Read> {
        let Request::Open { limit, clear } = self.requested else {
            return None;
        };
        if self.failed || self.pending.is_some() {
            return None;
        }
        let useful = self.useful(limit, clear, read_marker);
        let selection = self.selection(limit, clear, read_marker);
        let extension = if !self.dirty && useful {
            let older = if self.tail_pending {
                false
            } else if selection.older_needed {
                true
            } else if selection.newer_needed {
                false
            } else {
                return None;
            };
            let MessageCache::Loaded { messages, .. } = &self.message_cache
            else {
                unreachable!()
            };
            let boundary = if older {
                messages.first()
            } else {
                messages.last()
            };
            boundary.map(|message| Extension {
                older,
                boundary: cursor(message),
            })
        } else {
            None
        };
        let mut count = limit.count().saturating_mul(8).max(1);
        if extension.is_some()
            && let MessageCache::Loaded { messages, .. } = &self.message_cache
        {
            if messages.len() < count {
                // Grow toward the initial raw reserve without re-reading its
                // cached portion.
                count -= messages.len();
            } else if selection.visible < limit.count() {
                // Sparse windows otherwise repeatedly process many small
                // prefixes. Grow missing chunks, with a fixed per-read cap.
                count = messages.len().min(count.saturating_mul(4));
            }
        }
        // Replacement mutations keep useful coverage instead of shrinking it.
        if extension.is_none()
            && useful
            && let MessageCache::Loaded { messages, .. } = &self.message_cache
        {
            count = count.max(messages.len());
        }
        let limit = match limit {
            message::Limit::Top(_) => message::Limit::Top(count),
            message::Limit::Bottom(_) => message::Limit::Bottom(count),
            message::Limit::Around(_, id) => message::Limit::Around(count, id),
            message::Limit::Backlog(_) => message::Limit::Backlog(count),
        };
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(self.generation);
        self.dirty = false;
        Some(Read {
            generation: self.generation,
            limit,
            clear,
            read_marker,
            extension,
            tail_revision: self.tail_revision,
        })
    }

    pub fn loaded(
        &mut self,
        read: &Read,
        window: database::Window,
        kind: &Kind,
        filter_chain: FilterChain,
        clients: &dyn ClientsContext,
        config: &config::Buffer,
    ) -> bool {
        if self.pending != Some(read.generation) {
            return false;
        }
        self.pending = None;
        // Replacement snapshots can still make progress during live traffic.
        // Extensions must never merge across a replacement mutation.
        if read.generation != self.generation
            || (read.extension.is_some() && self.dirty)
            || !matches!(self.requested, Request::Open { .. })
        {
            return false;
        }
        self.config.clone_from(config);
        if let Some(extension) = read.extension {
            let MessageCache::Loaded {
                messages,
                has_more_older_messages,
                has_more_newer_messages,
                ..
            } = &mut self.message_cache
            else {
                return false;
            };
            let boundary = if extension.older {
                messages.first()
            } else {
                messages.last()
            };
            if boundary.map(cursor) != Some(extension.boundary) {
                self.invalidate();
                return false;
            }
            let old_len = messages.len();
            messages.extend(
                window
                    .messages
                    .into_iter()
                    .map(message::MessageDisplay::from),
            );
            renormalize_messages(kind, &mut messages[old_len..], clients);
            if extension.older {
                let added = messages.len() - old_len;
                messages.rotate_right(added);
                *has_more_older_messages = window.has_more_older;
            } else {
                *has_more_newer_messages = window.has_more_newer;
                self.tail_pending = window.has_more_newer
                    || read.tail_revision != self.tail_revision;
            }
            if !extension.older && self.processed {
                process_appended_messages(
                    kind,
                    messages,
                    old_len,
                    filter_chain,
                    clients,
                    config,
                );
            } else {
                process_messages(kind, messages, filter_chain, clients, config);
            }
        } else {
            let mut messages = match std::mem::take(&mut self.message_cache) {
                MessageCache::Loaded { mut messages, .. } => {
                    messages.clear();
                    messages
                }
                MessageCache::Unloaded => Vec::new(),
            };
            messages.extend(
                window
                    .messages
                    .into_iter()
                    .map(message::MessageDisplay::from),
            );
            renormalize_messages(kind, &mut messages, clients);
            process_messages(
                kind,
                &mut messages,
                filter_chain,
                clients,
                config,
            );
            self.message_cache = MessageCache::Loaded {
                messages,
                has_more_older_messages: window.has_more_older,
                has_more_newer_messages: window.has_more_newer,
                limit: read.limit,
                clear: read.clear,
            };
            self.tail_pending = read.tail_revision != self.tail_revision;
        }
        self.processed = true;
        self.trim(read.read_marker);
        true
    }

    fn trim(&mut self, marker: Option<ReadMarker>) {
        let Request::Open { limit, clear } = self.requested else {
            return;
        };
        let selection = self.selection(limit, clear, marker);
        if selection.older_needed
            || selection.newer_needed
            || self.tail_pending
            || selection.range.is_empty()
        {
            return;
        }
        let MessageCache::Loaded {
            messages,
            has_more_older_messages,
            has_more_newer_messages,
            ..
        } = &mut self.message_cache
        else {
            return;
        };
        // Selection already includes complete groups. Keep raw neighboring
        // context without retaining unrelated groups beyond that reserve.
        let margin = limit.count().saturating_mul(4).max(1);
        let mut start = selection.range.start.saturating_sub(margin);
        let mut end = selection
            .range
            .end
            .saturating_add(margin)
            .min(messages.len());
        if matches!(limit, message::Limit::Top(_)) {
            start = 0;
        }
        if matches!(limit, message::Limit::Bottom(_)) {
            end = messages.len();
        }
        if end < messages.len() {
            self.processed &= messages[..end]
                .iter()
                .rfind(|message| !message.blocked)
                .is_none_or(|message| {
                    !message
                        .inner
                        .can_condense(&self.config.server_messages.condense)
                });
            messages.truncate(end);
            *has_more_newer_messages = true;
        }
        if start > 0 {
            self.processed &= messages[start..]
                .iter()
                .find(|message| !message.blocked)
                .is_none_or(|message| {
                    !message
                        .inner
                        .can_condense(&self.config.server_messages.condense)
                });
            messages.drain(..start);
            *has_more_older_messages = true;
        }

        let useful_capacity = messages.len().saturating_add(margin);
        if messages.capacity() > useful_capacity.saturating_mul(2) {
            messages.shrink_to(useful_capacity);
        }
    }

    pub fn set_model_limit(&mut self, new_limit: message::Limit) {
        if let Request::Open { limit, .. } = &mut self.requested {
            if *limit == new_limit {
                return;
            }
            let same_anchor = match (*limit, new_limit) {
                (message::Limit::Top(_), message::Limit::Top(_))
                | (message::Limit::Bottom(_), message::Limit::Bottom(_))
                | (message::Limit::Backlog(_), message::Limit::Backlog(_)) => {
                    true
                }
                (
                    message::Limit::Around(_, previous),
                    message::Limit::Around(_, next),
                ) => previous == next,
                _ => false,
            };
            *limit = new_limit;
            if self.pending.is_some() && !same_anchor {
                self.invalidate();
            }
        } else {
            self.requested = Request::Open {
                limit: new_limit,
                clear: None,
            };
            self.invalidate();
        }
    }

    pub fn clear_model(&mut self) {
        if let Request::Open { clear, .. } = &mut self.requested {
            *clear = Some(Utc::now());
        }
        self.invalidate();
    }

    pub fn close_model(&mut self) {
        self.requested = Request::Closed {
            at: Some(Instant::now()),
        };
        self.invalidate();
    }

    pub fn clear(&mut self) {
        self.message_cache = MessageCache::Unloaded;
        if self.failed {
            self.failed();
        }
        if let Request::Closed { at } = &mut self.requested {
            *at = None;
        }
    }

    pub(super) fn model_update(
        &self,
        display_read_marker: &Option<ReadMarker>,
    ) -> model::Pane {
        let Request::Open { limit, clear } = self.requested else {
            return model::Pane::Closed;
        };
        let MessageCache::Loaded {
            messages,
            has_more_older_messages,
            has_more_newer_messages,
            ..
        } = &self.message_cache
        else {
            return model::Pane::Loading;
        };
        let selection = self.selection(limit, clear, *display_read_marker);
        let loading = !self.failed
            && (self.pending.is_some()
                || self.dirty
                || self.tail_pending
                || !self.useful(limit, clear, *display_read_marker)
                || selection.older_needed
                || selection.newer_needed);
        let now = Utc::now();
        let visible_outside = |range: Range<usize>| {
            messages[range]
                .iter()
                .any(|message| message.displayed(&self.config, now).is_some())
        };
        model::Pane::Open {
            has_more_older_messages: (!self.failed && *has_more_older_messages)
                || visible_outside(0..selection.range.start),
            has_more_newer_messages: (!self.failed && *has_more_newer_messages)
                || (!selection.range.is_empty()
                    && visible_outside(selection.range.end..messages.len())),
            messages: messages[selection.range].to_vec(),
            limit,
            clear,
            read_marker: *display_read_marker,
            loading,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_failure_stops_loading_even_without_requested_coverage() {
        let mut cache = ReadCache::default();
        cache.set_model_limit(message::Limit::Around(50, Id::Determined(42)));
        let read = cache.request(None).unwrap();
        cache.changed();
        cache.failed();
        assert!(cache.pending.is_none());
        assert!(cache.request(None).is_none());
        assert!(matches!(
            cache.model_update(&None),
            model::Pane::Open { loading: false, .. }
        ));

        // An incomplete old window also cannot promise further database reads.
        if let MessageCache::Loaded {
            has_more_older_messages,
            has_more_newer_messages,
            ..
        } = &mut cache.message_cache
        {
            *has_more_older_messages = true;
            *has_more_newer_messages = true;
        }
        assert!(matches!(
            cache.model_update(&None),
            model::Pane::Open {
                loading: false,
                has_more_older_messages: false,
                has_more_newer_messages: false,
                ..
            }
        ));
        assert_ne!(cache.pending, Some(read.generation));

        cache.close_model();
        cache.clear();
        cache.set_model_limit(message::Limit::Bottom(50));
        assert!(cache.request(None).is_none());
        assert!(matches!(
            cache.model_update(&None),
            model::Pane::Open { loading: false, .. }
        ));
    }
}
