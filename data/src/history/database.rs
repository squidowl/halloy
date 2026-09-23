use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::ops::Bound;
use std::time::Duration;

use chrono::{DateTime, SubsecRound, Utc};
use hashbrown::{HashMap, HashSet};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use url::Url;

use super::{Id, Kind, Metadata, ReadMarker, search, storage};
use crate::{isupport, message, reaction, redaction, user};

pub(super) const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const FILE_NAME: &str = "history.sqlite3";

const UNLABELED_ECHO_CANDIDATES: &str = "SELECT id, time, time_from_server, msgid, content FROM message
                 WHERE history = ?1 AND time >= ?2 AND time < ?3
                   AND (msgid = ?4 OR ((?4 IS NULL OR msgid IS NULL)
                     AND (json_type(CAST(content AS TEXT), '$.direction.Sent') IS NOT NULL
                       OR (?5 AND time = ?6))))
                 ORDER BY time, id";

const REFERENCE_TYPES: &[isupport::MessageReferenceType] = &[
    isupport::MessageReferenceType::MessageId,
    isupport::MessageReferenceType::Timestamp,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extension {
    pub older: bool,
    pub boundary: (i64, u64),
}

#[derive(Debug)]
pub struct Window {
    pub messages: Vec<message::Message>,
    pub has_more_older: bool,
    pub has_more_newer: bool,
}

enum WindowSource {
    History(i64),
    Monitor(Vec<i64>),
}

impl WindowSource {
    fn single_history(&self) -> Option<(i64, bool)> {
        match self {
            Self::History(id) => Some((*id, false)),
            Self::Monitor(histories) => match histories.as_slice() {
                [id] => Some((*id, true)),
                _ => None,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct WindowQuery {
    clear: Option<i64>,
    boundary: Bound<(i64, i64)>,
    descending: bool,
    count: usize,
}

impl WindowQuery {
    fn order(self) -> &'static str {
        if self.descending {
            "time DESC, id DESC"
        } else {
            "time, id"
        }
    }

    fn sql(self, columns: &str, history: &str, monitor: bool) -> String {
        let index = if monitor {
            " INDEXED BY message_monitor"
        } else {
            ""
        };
        let eligibility = if monitor {
            " AND in_channel_monitor = 1"
        } else {
            ""
        };
        // A nullable OR prevents SQLite from seeking directly past cleared rows.
        let clear = if self.clear.is_some() {
            " AND time > ?2"
        } else {
            ""
        };
        let select = format!(
            "SELECT {columns} FROM message{index} WHERE history = {history}{eligibility}{clear}"
        );
        let order = self.order();
        if matches!(self.boundary, Bound::Unbounded) {
            return format!("{select} ORDER BY {order} LIMIT ?5");
        }
        let comparison = if self.descending { "<" } else { ">" };
        let inclusive = if matches!(self.boundary, Bound::Included(_)) {
            "="
        } else {
            ""
        };
        // SQLite's tuple comparison can rescan every equal-timestamp row on
        // each seek. Separate timestamp ties so both branches use index bounds.
        format!(
            "{select} AND time = ?3 AND id {comparison}{inclusive} ?4
                 UNION ALL {select} AND time {comparison} ?3
                 ORDER BY {order} LIMIT ?5"
        )
    }

    fn params(
        self,
        history: rusqlite::types::Value,
    ) -> [rusqlite::types::Value; 5] {
        let (time, id) = match self.boundary {
            Bound::Included(key) | Bound::Excluded(key) => key,
            Bound::Unbounded => (0, 0),
        };
        [
            history,
            self.clear.into(),
            time.into(),
            id.into(),
            i64::try_from(self.count).unwrap_or(i64::MAX).into(),
        ]
    }
}

#[derive(Eq, PartialEq)]
struct MonitorHead {
    key: (i64, i64),
    history: i64,
    descending: bool,
}

impl Ord for MonitorHead {
    fn cmp(&self, other: &Self) -> Ordering {
        self.descending
            .cmp(&other.descending)
            .then_with(|| {
                if self.descending {
                    self.key.cmp(&other.key)
                } else {
                    other.key.cmp(&self.key)
                }
            })
            .then(self.history.cmp(&other.history))
    }
}

impl PartialOrd for MonitorHead {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) struct Database {
    connection: Connection,
    indexed: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("unsupported history schema version {0}")]
    Schema(i64),
}

#[derive(Serialize, Deserialize)]
struct StoredMessage {
    direction: message::Direction,
    source: message::Source,
    target: message::Target,
    content: message::Content,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    hidden_urls: HashSet<Url>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    reactions: Vec<reaction::Reaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relayed_by: Option<user::Nick>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rerouted_from: Option<message::Target>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    redaction: Option<redaction::Redaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reply_to: Option<message::Id>,
}

#[derive(Deserialize)]
struct StoredSource {
    source: message::Source,
}

#[derive(Deserialize)]
struct SearchableMessage {
    source: message::Source,
    content: message::Content,
    #[serde(default)]
    redaction: Option<redaction::Redaction>,
}

impl Database {
    pub fn new(mut connection: Connection) -> Result<Self, Error> {
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.set_prepared_statement_cache_capacity(128);
        connection.execute_batch(
            "PRAGMA auto_vacuum = INCREMENTAL;
             PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        let version: i64 =
            connection
                .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        match version {
            0 => {
                let transaction = connection.transaction()?;
                transaction.execute_batch(
                    "CREATE TABLE history (
                        id INTEGER PRIMARY KEY,
                        kind TEXT NOT NULL UNIQUE,
                        server TEXT,
                        target TEXT,
                        read_marker INTEGER,
                        latest INTEGER,
                        latest_triggers_unread INTEGER,
                        latest_triggers_highlight INTEGER,
                        chathistory_references TEXT,
                        imported_at INTEGER
                    );
                    CREATE TABLE message (
                        -- Do not reuse deleted IDs: stale references can outlive even the highest row.
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        history INTEGER NOT NULL REFERENCES history(id) ON DELETE CASCADE,
                        time INTEGER NOT NULL,
                        time_from_server INTEGER NOT NULL,
                        msgid TEXT,
                        referenceable INTEGER NOT NULL,
                        in_channel_monitor INTEGER NOT NULL,
                        content BLOB NOT NULL
                    );
                    CREATE INDEX message_history_time ON message(history, time);
                    CREATE INDEX message_msgid ON message(history, msgid) WHERE msgid IS NOT NULL;
                    CREATE INDEX message_referenceable ON message(history, time) WHERE referenceable = 1;
                    CREATE INDEX message_monitor ON message(history, time) WHERE in_channel_monitor = 1;
                    CREATE VIRTUAL TABLE message_search USING fts5(
                        nick, text, message UNINDEXED,
                        content='', contentless_delete=1, contentless_unindexed=1,
                        tokenize='unicode61 remove_diacritics 2'
                    );
                    CREATE TABLE search_state (indexed INTEGER NOT NULL);
                    INSERT INTO search_state VALUES (0);
                    CREATE TABLE search_pending (key INTEGER PRIMARY KEY, message INTEGER NOT NULL);
                    PRAGMA user_version = 3;",
                )?;
                transaction.commit()?;
            }
            3 => (),
            other => return Err(Error::Schema(other)),
        }
        let indexed = connection.query_row(
            "SELECT indexed FROM search_state",
            [],
            |row| row.get(0),
        )?;
        Ok(Self {
            connection,
            indexed,
        })
    }

    pub fn is_imported(&self, kind: &Kind) -> Result<bool, Error> {
        is_imported(&self.connection, kind)
    }

    pub fn shutdown(&self) -> Result<(), Error> {
        self.connection.execute_batch(
            "PRAGMA incremental_vacuum(256); PRAGMA wal_checkpoint(PASSIVE);",
        )?;
        Ok(())
    }

    pub fn transaction(&mut self) -> Result<Write<'_>, Error> {
        Ok(Write {
            transaction: self.connection.transaction()?,
            histories: HashMap::new(),
            searchable: HashSet::new(),
            indexed: self.indexed,
            appends: HashMap::new(),
            serialization_buffer: Vec::new(),
        })
    }

    pub fn index(&mut self, limit: usize) -> Result<bool, Error> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let transaction = self.connection.transaction()?;
        let pending = transaction
            .prepare_cached("SELECT key, message FROM search_pending LIMIT ?1")?
            .query_map([limit], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut rows = vec![];
        for (key, message) in &pending {
            transaction
                .prepare_cached("DELETE FROM message_search WHERE rowid = ?1")?
                .execute([key])?;
            transaction
                .prepare_cached("DELETE FROM search_pending WHERE key = ?1")?
                .execute([key])?;
            rows.extend(
                transaction
                    .prepare_cached(
                        "SELECT m.id, m.time, m.content, h.server IS NOT NULL FROM message m
                         JOIN history h ON h.id = m.history WHERE m.id = ?1",
                    )?
                    .query_row([message], indexable_row)
                    .optional()?,
            );
        }
        let remaining = limit - pending.len() as i64;
        let new = transaction
            .prepare_cached(
                "SELECT m.id, m.time, m.content, h.server IS NOT NULL FROM message m
                 JOIN history h ON h.id = m.history
                 WHERE m.id > ?1 ORDER BY m.id LIMIT ?2",
            )?
            .query_map(params![self.indexed, remaining], indexable_row)?
            .collect::<Result<Vec<_>, _>>()?;
        if pending.is_empty() && new.is_empty() {
            return Ok(false);
        }
        let more = remaining == 0 || new.len() as i64 == remaining;
        let indexed = new.last().map_or(self.indexed, |(id, ..)| *id);
        rows.extend(new);
        {
            let mut insert = transaction.prepare_cached(
                "INSERT OR REPLACE INTO message_search(rowid, nick, text, message) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (id, time, content, searchable) in &rows {
                if !searchable {
                    continue;
                }
                let stored: SearchableMessage =
                    serde_json::from_slice(content)?;
                if stored.redaction.is_some() {
                    continue;
                }
                let (Some(user), Some(key)) =
                    (stored.source.user(), search_key(*time, *id))
                else {
                    continue;
                };
                insert.execute(params![
                    key,
                    search::nick_token(user.nickname().as_str()),
                    stored.content.text(),
                    id,
                ])?;
            }
        }
        transaction
            .execute("UPDATE search_state SET indexed = ?1", [indexed])?;
        transaction.commit()?;
        self.indexed = indexed;
        Ok(more)
    }

    pub fn window(
        &self,
        kind: &Kind,
        limit: message::Limit,
        clear: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
    ) -> Result<Window, Error> {
        self.read_window(kind, &[], limit, clear, read_marker, None)
    }

    pub fn monitor_window(
        &self,
        kinds: &[Kind],
        limit: message::Limit,
        clear: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
    ) -> Result<Window, Error> {
        self.read_window(
            &Kind::ChannelMonitor,
            kinds,
            limit,
            clear,
            read_marker,
            None,
        )
    }

    pub fn read_window(
        &self,
        kind: &Kind,
        monitored: &[Kind],
        limit: message::Limit,
        clear: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
        extension: Option<Extension>,
    ) -> Result<Window, Error> {
        let history_id = |kind: &Kind| -> Result<Option<i64>, Error> {
            Ok(self
                .connection
                .prepare_cached("SELECT id FROM history WHERE kind = ?1")?
                .query_row([history_key(kind)?], |row| row.get(0))
                .optional()?)
        };
        if *kind != Kind::ChannelMonitor {
            return self.window_filtered(
                WindowSource::History(history_id(kind)?.unwrap_or(-1)),
                &[],
                limit,
                clear,
                read_marker,
                extension,
            );
        }
        let mut histories = Vec::new();
        for kind in monitored
            .iter()
            .filter(|kind| matches!(kind, Kind::Channel(..)))
        {
            if let Some(id) = history_id(kind)? {
                histories.push((id, kind));
            }
        }
        self.window_filtered(
            WindowSource::Monitor(
                histories.iter().map(|(id, _)| *id).collect(),
            ),
            &histories,
            limit,
            clear,
            read_marker,
            extension,
        )
    }

    fn window_filtered(
        &self,
        source: WindowSource,
        monitored: &[(i64, &Kind)],
        limit: message::Limit,
        clear: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
        extension: Option<Extension>,
    ) -> Result<Window, Error> {
        let clear = clear.map(|time| time.timestamp_micros());
        let count = limit.count();
        if count == 0 {
            return Ok(Window {
                messages: vec![],
                has_more_older: false,
                has_more_newer: false,
            });
        }
        if let Some(extension) = extension {
            let mut messages = self.select_window(
                &source,
                WindowQuery {
                    clear,
                    boundary: Bound::Excluded((
                        extension.boundary.0,
                        sql_id(extension.boundary.1)?,
                    )),
                    descending: extension.older,
                    count: count.saturating_add(1),
                },
                monitored,
            )?;
            let more = messages.len() > count;
            messages.truncate(count);
            if extension.older {
                messages.reverse();
            }
            return Ok(Window {
                messages,
                has_more_older: extension.older && more,
                has_more_newer: !extension.older && more,
            });
        }
        let anchor = match limit {
            message::Limit::Around(_, Id::Determined(id)) => {
                let (history, predicate) = match &source {
                    WindowSource::History(id) => {
                        (rusqlite::types::Value::Integer(*id), "history = ?1")
                    }
                    WindowSource::Monitor(histories) => (
                        rusqlite::types::Value::Text(serde_json::to_string(
                            histories,
                        )?),
                        "history IN (SELECT value FROM json_each(?1)) AND in_channel_monitor = 1",
                    ),
                };
                self.connection.query_row(
                &format!("SELECT time, id FROM message WHERE {predicate} AND id = ?2 AND (?3 IS NULL OR time > ?3)"),
                params![history, sql_id(id)?, clear], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            ).optional()?
            }
            message::Limit::Backlog(_) if read_marker.is_some() => self
                .window_keys(
                    &source,
                    WindowQuery {
                        clear,
                        boundary: Bound::Included((
                            read_marker
                                .unwrap()
                                .as_date_time()
                                .timestamp_micros(),
                            i64::MAX,
                        )),
                        descending: true,
                        count: 1,
                    },
                )?
                .into_iter()
                .next(),
            _ => None,
        };
        if let Some((time, id)) = anchor {
            let select =
                |descending: bool, count: usize, time: i64, id: i64| {
                    self.select_window(
                        &source,
                        WindowQuery {
                            clear,
                            boundary: if descending {
                                Bound::Excluded((time, id))
                            } else {
                                Bound::Included((time, id))
                            },
                            descending,
                            count,
                        },
                        monitored,
                    )
                };

            let half = count / 2;
            let mut before = select(true, half + 1, time, id)?;
            let after_count = if before.len() < half {
                count.saturating_sub(before.len()).saturating_add(1)
            } else {
                half + 1
            };
            let after = select(false, after_count, time, id)?;
            if after.len() < half && before.len() > half {
                let oldest = before.last().expect("older sentinel exists");
                let Id::Determined(oldest_id) = oldest.history_id else {
                    unreachable!("stored messages have determined IDs");
                };
                let remaining = count
                    .saturating_sub(after.len())
                    .saturating_add(1)
                    .saturating_sub(before.len());
                before.extend(select(
                    true,
                    remaining,
                    oldest.time.utc.timestamp_micros(),
                    sql_id(oldest_id)?,
                )?);
            }
            before.reverse();
            let position = before.len();
            before.extend(after);
            let (older, newer) =
                storage::get_before_and_after_count_from_position(
                    count,
                    position,
                    before.len(),
                );
            let start = position.saturating_sub(older);
            let end = position.saturating_add(newer).min(before.len());
            return Ok(Window {
                has_more_older: start != 0,
                has_more_newer: end != before.len(),
                messages: before.drain(start..end).collect(),
            });
        }
        if matches!(limit, message::Limit::Around(..)) {
            return Ok(Window {
                messages: vec![],
                has_more_older: false,
                has_more_newer: false,
            });
        }
        let bottom = matches!(limit, message::Limit::Bottom(_));
        let mut messages = self.select_window(
            &source,
            WindowQuery {
                clear,
                boundary: Bound::Unbounded,
                descending: bottom,
                count: count.saturating_add(1),
            },
            monitored,
        )?;
        let more = messages.len() > count;
        messages.truncate(count);
        if bottom {
            messages.reverse();
        }
        Ok(Window {
            messages,
            has_more_older: bottom && more,
            has_more_newer: !bottom && more,
        })
    }

    fn select_window(
        &self,
        source: &WindowSource,
        query: WindowQuery,
        monitored: &[(i64, &Kind)],
    ) -> Result<Vec<message::Message>, Error> {
        const COLUMNS: &str =
            "id, time, time_from_server, msgid, content, history";
        if let Some((history, monitor)) = source.single_history() {
            return self.select(
                &query.sql(COLUMNS, "?1", monitor),
                query.params(history.into()),
                monitored,
            );
        }
        let keys = self.window_keys(source, query)?;
        let ids: Vec<_> = keys.iter().map(|(_, id)| id).collect();
        self.select(
            &format!("SELECT {COLUMNS} FROM message WHERE id IN (SELECT value FROM json_each(?1)) ORDER BY {}", query.order()),
            [serde_json::to_string(&ids)?], monitored,
        )
    }

    fn window_keys(
        &self,
        source: &WindowSource,
        query: WindowQuery,
    ) -> Result<Vec<(i64, i64)>, Error> {
        if query.count == 0 {
            return Ok(vec![]);
        }
        if let Some((history, monitor)) = source.single_history() {
            return Ok(self
                .connection
                .prepare_cached(&query.sql("time, id", "?1", monitor))?
                .query_map(query.params(history.into()), |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?
                .collect::<Result<_, _>>()?);
        }
        let WindowSource::Monitor(histories) = source else {
            unreachable!()
        };
        // Keep one key per history in a priority queue. Only the winning
        // history advances, avoiding both unrelated archives and histories ×
        // page-size candidates. Message payloads are decoded only after merging.
        let seed = WindowQuery { count: 1, ..query };
        let mut statement = self.connection.prepare_cached(&format!(
            "SELECT message.time, message.id, message.history
             FROM (SELECT DISTINCT value FROM json_each(?1)) AS histories
             JOIN message ON message.id = (SELECT id FROM ({}))",
            seed.sql("time, id", "histories.value", true),
        ))?;
        let mut heads = statement
            .query_map(
                seed.params(serde_json::to_string(histories)?.into()),
                |row| {
                    Ok(MonitorHead {
                        key: (row.get(0)?, row.get(1)?),
                        history: row.get(2)?,
                        descending: query.descending,
                    })
                },
            )?
            .collect::<Result<BinaryHeap<_>, _>>()?;
        let next = WindowQuery {
            boundary: Bound::Excluded((0, 0)),
            count: 1,
            ..query
        };
        let mut statement = self
            .connection
            .prepare_cached(&next.sql("time, id", "?1", true))?;
        let mut keys = Vec::new();
        while keys.len() < query.count {
            let Some(head) = heads.pop() else { break };
            keys.push(head.key);
            if keys.len() == query.count {
                break;
            }
            let next = WindowQuery {
                boundary: Bound::Excluded(head.key),
                ..next
            };
            if let Some(key) = statement
                .query_row(next.params(head.history.into()), |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .optional()?
            {
                heads.push(MonitorHead { key, ..head });
            }
        }
        Ok(keys)
    }

    fn select(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
        monitored: &[(i64, &Kind)],
    ) -> Result<Vec<message::Message>, Error> {
        Ok(self
            .connection
            .prepare_cached(sql)?
            .query_map(params, |row| {
                let mut message = row_message(row)?;
                let history: i64 = row.get(5)?;
                if let Some((_, Kind::Channel(server, channel))) =
                    monitored.iter().find(|(id, _)| *id == history)
                {
                    message.target = message::Target::ChannelMonitor {
                        server: server.clone(),
                        channel: channel.clone(),
                        history_id: message.history_id,
                    };
                }
                Ok(message)
            })?
            .collect::<Result<_, _>>()?)
    }

    pub fn message(
        &self,
        kind: &Kind,
        id: Id,
    ) -> Result<Option<message::Message>, Error> {
        let Id::Determined(id) = id else {
            return Ok(None);
        };
        Ok(self.connection.prepare_cached(
            "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content FROM message m
             JOIN history h ON h.id = m.history
             WHERE h.kind = ?1 AND m.id = ?2",
        )?.query_row(params![history_key(kind)?, sql_id(id)?], row_message).optional()?)
    }

    pub fn highlight_source(
        &self,
        kind: &Kind,
        highlight: Id,
    ) -> Result<Option<Id>, Error> {
        let Id::Determined(id) = highlight else {
            return Ok(None);
        };
        let archive = self.connection.prepare_cached(
            "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content FROM message m
             JOIN history h ON h.id = m.history WHERE h.kind = ?1 AND m.id = ?2",
        )?.query_row(params![history_key(&Kind::Highlights)?, sql_id(id)?], row_message).optional()?;
        let Some(archive) = archive else {
            return Ok(None);
        };
        let message::Target::Highlights {
            server,
            channel,
            history_id,
        } = &archive.target
        else {
            return Ok(None);
        };
        if *kind != Kind::Channel(server.clone(), channel.clone()) {
            return Ok(None);
        }
        Ok(self
            .message(kind, *history_id)?
            .map(|message| message.history_id))
    }

    #[cfg(test)]
    pub fn search(
        &self,
        query: &search::Query,
        before: Option<search::Cursor>,
        limit: usize,
    ) -> Result<search::Page, Error> {
        search(&self.connection, query, before, limit)
    }

    pub fn reference(
        &self,
        kind: &Kind,
        query: storage::ReferenceQuery,
        reference_types: &[isupport::MessageReferenceType],
    ) -> Result<Option<isupport::MessageReference>, Error> {
        let msgid = reference_types
            .contains(&isupport::MessageReferenceType::MessageId);
        let timestamp = reference_types
            .contains(&isupport::MessageReferenceType::Timestamp);
        let (sql, before) = match query {
            storage::ReferenceQuery::Oldest => (
                "SELECT m.time, m.time_from_server, m.msgid FROM message m
                 JOIN history h ON h.id = m.history WHERE h.kind = ?1 AND m.referenceable = 1
                 AND ((?2 AND m.msgid IS NOT NULL) OR (?3 AND m.time_from_server = 1))
                 ORDER BY m.time, m.id LIMIT 1", None,
            ),
            storage::ReferenceQuery::Before(time) => (
                "SELECT m.time, m.time_from_server, m.msgid FROM message m
                 JOIN history h ON h.id = m.history WHERE h.kind = ?1 AND m.referenceable = 1
                 AND ((?2 AND m.msgid IS NOT NULL) OR (?3 AND m.time_from_server = 1)) AND m.time < ?4
                 ORDER BY m.time DESC, m.id DESC LIMIT 1", Some(time),
            ),
        };
        let mut statement = self.connection.prepare_cached(sql)?;
        let key = history_key(kind)?;
        let row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<(
            DateTime<Utc>,
            message::MessageReferences,
        )> {
            let time: i64 = row.get(0)?;
            let time = DateTime::from_timestamp_micros(time)
                .ok_or(rusqlite::Error::IntegralValueOutOfRange(0, time))?;
            let server: bool = row.get(1)?;
            Ok((
                time,
                message::MessageReferences {
                    timestamp: server.then_some(time),
                    id: row.get::<_, Option<String>>(2)?.map(Into::into),
                },
            ))
        };
        let mut reference = if let Some(before) = before {
            statement
                .query_row(
                    params![key, msgid, timestamp, before.timestamp_micros()],
                    row,
                )
                .optional()?
        } else {
            statement
                .query_row(params![key, msgid, timestamp], row)
                .optional()?
        };
        if let Some(before) = before
            && let Some((time, cached)) =
                self.metadata(kind)?.latest_chathistory_references
            && time.utc < before
            && cached.message_reference(reference_types).is_some()
            && reference
                .as_ref()
                .is_none_or(|(stored_time, _)| time.utc > *stored_time)
        {
            reference = Some((time.utc, cached));
        }
        Ok(reference.and_then(|(_, reference)| {
            reference.message_reference(reference_types)
        }))
    }

    pub fn metadata(&self, kind: &Kind) -> Result<Metadata, Error> {
        read_metadata(&self.connection, kind)
    }
}

pub(super) struct Write<'a> {
    transaction: Transaction<'a>,
    histories: HashMap<Kind, i64>,
    searchable: HashSet<i64>,
    indexed: i64,
    // Pre-transaction maximum and reconciliation-confirmed append classification.
    appends: HashMap<i64, (Option<(i64, i64)>, bool)>,
    serialization_buffer: Vec<u8>,
}

impl Write<'_> {
    pub fn append_only(&self, kind: &Kind) -> bool {
        self.histories
            .get(kind)
            .and_then(|history| self.appends.get(history))
            .is_some_and(|(_, append)| *append)
    }

    pub fn insert_normalized(
        &mut self,
        kind: &Kind,
        mut incoming: message::MessageWithContext,
        casemapping: isupport::CaseMap,
    ) -> Result<(message::Message, bool), Error> {
        normalize(&mut incoming.inner, casemapping);
        incoming.inner.time.utc = incoming.inner.time.utc.trunc_subsecs(6);
        let history = self.history(kind)?;
        let fuzz = chrono::Duration::seconds(
            if incoming.inner.is_echo()
                && incoming.labeled_response_context.is_none()
            {
                300
            } else {
                1
            },
        );
        let time = incoming.inner.time.utc;
        let mut candidates = if !incoming.inner.is_echo()
            && incoming.labeled_response_context.is_none()
        {
            self.transaction.prepare_cached(
                "SELECT id, time, time_from_server, msgid, content FROM message INDEXED BY message_msgid
                 WHERE history = ?1 AND msgid = ?4 AND time >= ?2 AND time < ?3
                 UNION ALL
                 SELECT id, time, time_from_server, msgid, content FROM message
                 WHERE ?5 AND history = ?1 AND time = ?6 AND (?4 IS NULL OR msgid IS NULL)
                 ORDER BY time, id",
            )?
            .query_map(
                params![
                    history,
                    (time - fuzz).timestamp_micros(),
                    (time + fuzz).timestamp_micros(),
                    incoming.inner.id.as_ref().map(AsRef::<str>::as_ref),
                    incoming.historical,
                    time.timestamp_micros(),
                ],
                row_message,
            )?
            .collect::<Result<Vec<_>, _>>()?
        } else if incoming.labeled_response_context.is_none() {
            // Echoes can replace a matching msgid, a sent message without a
            // shared msgid, or a historical message at the exact same time.
            // Filter before decoding; a busy channel's other messages cannot
            // match. Do not cap candidates or narrow them by the current nick.
            self.transaction
                .prepare_cached(UNLABELED_ECHO_CANDIDATES)?
                .query_map(
                    params![
                        history,
                        (time - fuzz).timestamp_micros(),
                        (time + fuzz).timestamp_micros(),
                        incoming.inner.id.as_deref(),
                        incoming.historical,
                        time.timestamp_micros(),
                    ],
                    row_message,
                )?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let label_time = incoming
                .labeled_response_context
                .as_ref()
                .map_or(time, |context| context.time.utc);
            self.transaction.prepare_cached(
                "SELECT id, time, time_from_server, msgid, content FROM message
                 WHERE history = ?1 AND time >= ?2 AND time < ?3
                 UNION ALL
                 SELECT id, time, time_from_server, msgid, content FROM message
                 WHERE history = ?1 AND time >= ?4 AND time < ?5 AND (time < ?2 OR time >= ?3)
                 ORDER BY time, id",
            )?
            .query_map(
                params![
                    history,
                    (time - fuzz).timestamp_micros(),
                    (time + fuzz).timestamp_micros(),
                    (label_time - fuzz).timestamp_micros(),
                    (label_time + fuzz).timestamp_micros()
                ],
                row_message,
            )?
            .collect::<Result<Vec<_>, _>>()?
        };
        for candidate in &mut candidates {
            normalize(candidate, casemapping);
        }
        if let message::Target::Highlights {
            server, channel, ..
        } = &incoming.inner.target
        {
            candidates.retain(|candidate| matches!(&candidate.target,
                message::Target::Highlights { server: origin_server, channel: origin_channel, .. }
                    if origin_server == server && origin_channel == channel
            ));
        }
        incoming.inner.history_id = Id::Undetermined;
        let (id, time, replaced_sent) =
            storage::reconcile_message(&mut candidates, incoming);
        let position = candidates
            .iter()
            .position(|message| {
                message.history_id == id && message.time == time
            })
            .expect("reconciliation returns the inserted message");
        let mut message = candidates.remove(position);
        if let hashbrown::hash_map::Entry::Vacant(entry) =
            self.appends.entry(history)
        {
            let maximum = self.transaction.prepare_cached(
                "SELECT time, id FROM message WHERE history = ?1 ORDER BY time DESC, id DESC LIMIT 1",
            )?.query_row([history], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
            entry.insert((maximum, true));
        }
        let inserted = message.history_id == Id::Undetermined;
        self.store(history, &mut message)?;
        let (maximum, append_only) = self
            .appends
            .get_mut(&history)
            .expect("classification initialized");
        let Id::Determined(id) = message.history_id else {
            unreachable!()
        };
        *append_only &= inserted
            && maximum.is_none_or(|maximum| {
                (message.time.utc.timestamp_micros(), id as i64) > maximum
            });
        Ok((message, replaced_sent))
    }

    pub fn message(
        &self,
        kind: &Kind,
        id: Id,
    ) -> Result<Option<message::Message>, Error> {
        let Id::Determined(id) = id else {
            return Ok(None);
        };
        Ok(self.transaction.prepare_cached(
            "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content FROM message m
             JOIN history h ON h.id = m.history WHERE h.kind = ?1 AND m.id = ?2",
        )?.query_row(params![history_key(kind)?, sql_id(id)?], row_message).optional()?)
    }

    pub fn message_by_id(
        &self,
        kind: &Kind,
        id: &message::Id,
        time: message::Time,
    ) -> Result<Option<message::Message>, Error> {
        let key = history_key(kind)?;
        let before = self.transaction.prepare_cached(
            "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content FROM message m
             JOIN history h ON h.id = m.history WHERE h.kind = ?1 AND m.msgid = ?2 AND m.time < ?3
             ORDER BY m.time DESC, m.id DESC LIMIT 1",
        )?.query_row(params![key, id.as_ref(), (time.utc + chrono::Duration::seconds(1)).timestamp_micros()], row_message).optional()?;
        if before.is_some() {
            return Ok(before);
        }
        Ok(self.transaction.prepare_cached(
            "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content FROM message m
             JOIN history h ON h.id = m.history WHERE h.kind = ?1 AND m.msgid = ?2
             ORDER BY m.time DESC, m.id DESC LIMIT 1",
        )?.query_row(params![key, id.as_ref()], row_message).optional()?)
    }

    pub fn reaction(
        &mut self,
        kind: &Kind,
        mut reaction: reaction::ReactionWithContext,
        casemapping: isupport::CaseMap,
    ) -> Result<Option<message::Message>, Error> {
        let Some(mut message) = self.message_by_id(
            kind,
            &reaction.in_reply_to,
            reaction.inner.time,
        )?
        else {
            return Ok(None);
        };
        normalize(&mut message, casemapping);
        reaction.inner.sender.renormalize(casemapping);
        storage::insert_reaction(&mut message.reactions, reaction);
        let history = self.history(kind)?;
        self.store(history, &mut message)?;
        Ok(Some(message))
    }

    pub fn redaction(
        &mut self,
        kind: &Kind,
        redaction: redaction::RedactionWithContext,
    ) -> Result<Option<message::Message>, Error> {
        let Some(mut message) =
            self.message_by_id(kind, &redaction.redacts, redaction.time)?
        else {
            return Ok(None);
        };
        message.redaction = Some(redaction.into());
        let history = self.history(kind)?;
        self.store(history, &mut message)?;
        Ok(Some(message))
    }

    pub fn preview(
        &mut self,
        kind: &Kind,
        id: Id,
        url: Url,
        visible: bool,
    ) -> Result<Option<message::Message>, Error> {
        let Some(mut message) = self.message(kind, id)? else {
            return Ok(None);
        };
        if visible {
            message.hidden_urls.remove(&url);
        } else {
            message.hidden_urls.insert(url);
        }
        let history = self.history(kind)?;
        self.store(history, &mut message)?;
        Ok(Some(message))
    }

    pub fn remove(
        &self,
        kind: &Kind,
        id: Id,
    ) -> Result<Option<message::Message>, Error> {
        let message = self.message(kind, id)?;
        if let Some(message) = &message {
            let Id::Determined(id) = message.history_id else {
                unreachable!()
            };
            self.transaction.execute(
                "DELETE FROM message WHERE history = (SELECT id FROM history WHERE kind = ?1) AND id = ?2",
                params![history_key(kind)?, sql_id(id)?],
            )?;
            if is_searchable(kind)
                && sql_id(id)? <= self.indexed
                && let Some(key) =
                    search_key(message.time.utc.timestamp_micros(), sql_id(id)?)
            {
                self.transaction.execute(
                    "INSERT OR REPLACE INTO search_pending VALUES (?1, ?2)",
                    params![key, sql_id(id)?],
                )?;
            }
        }
        Ok(message)
    }

    pub fn get_metadata(&self, kind: &Kind) -> Result<Metadata, Error> {
        read_metadata(&self.transaction, kind)
    }

    pub fn smart_previous(
        &self,
        kind: &Kind,
        message: &message::Message,
        nick: &user::Nick,
        away: bool,
        seconds: i64,
        casemapping: isupport::CaseMap,
    ) -> Result<Option<DateTime<Utc>>, Error> {
        let Id::Determined(id) = message.history_id else {
            return Ok(None);
        };
        let lower = message.time.utc
            - chrono::Duration::seconds(seconds.saturating_add(1));
        let mut statement = self.transaction.prepare_cached(
            "SELECT m.time, m.content FROM message m
             JOIN history h ON h.id = m.history WHERE h.kind = ?1
             AND m.time >= ?2 AND (m.time, m.id) < (?3, ?4)
             ORDER BY m.time DESC, m.id DESC",
        )?;
        let mut rows = statement.query(params![
            history_key(kind)?,
            lower.timestamp_micros(),
            message.time.utc.timestamp_micros(),
            sql_id(id)?
        ])?;
        let nick = casemapping.normalize(nick.as_str());
        while let Some(row) = rows.next()? {
            let StoredSource { source } = serde_json::from_slice(
                row.get_ref(1)?.as_blob().map_err(rusqlite::Error::from)?,
            )?;
            let matches = match &source {
                message::Source::User(user) if !away => {
                    casemapping.normalize(user.nickname().as_str()) == nick
                }
                message::Source::Server(Some(source)) if away => {
                    source.kind == message::Kind::Away
                        && source.nick.as_ref().is_some_and(|previous| {
                            casemapping.normalize(previous.as_str()) == nick
                        })
                }
                _ => false,
            };
            if matches {
                let time: i64 = row.get(0)?;
                return Ok(Some(DateTime::from_timestamp_micros(time).ok_or(
                    rusqlite::Error::IntegralValueOutOfRange(0, time),
                )?));
            }
        }
        Ok(None)
    }

    pub fn import(
        &mut self,
        kind: &Kind,
        messages: &[message::Message],
        metadata: &Metadata,
    ) -> Result<(), Error> {
        if is_imported(&self.transaction, kind)? {
            return Ok(());
        }
        let history = self.history(kind)?;
        for incoming in messages {
            let mut message = incoming.clone();
            message.history_id = Id::Undetermined;
            message.time.utc = message.time.utc.trunc_subsecs(6);
            self.link_highlight(&mut message)?;
            self.store(history, &mut message)?;
        }
        if matches!(kind, Kind::Channel(..)) {
            self.relink_highlights(kind)?;
        }
        self.metadata(kind, metadata)?;
        self.transaction
            .prepare_cached(
                "UPDATE history SET imported_at = ?2 WHERE id = ?1",
            )?
            .execute(params![history, Utc::now().timestamp_micros()])?;
        Ok(())
    }

    fn link_highlight(
        &self,
        message: &mut message::Message,
    ) -> Result<(), Error> {
        if let message::Target::Highlights {
            server,
            channel,
            history_id,
        } = &mut message.target
        {
            *history_id = Id::Undetermined;
            let source = Kind::Channel(server.clone(), channel.clone());
            let original = if let Some(id) = &message.id {
                self.message_by_id(&source, id, message.time)?
            } else {
                let mut statement = self.transaction.prepare_cached(
                        "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content FROM message m
                         JOIN history h ON h.id = m.history WHERE h.kind = ?1 AND m.time = ?2
                         ORDER BY m.id DESC",
                    )?;
                let mut rows = statement.query(params![
                    history_key(&source)?,
                    message.time.utc.timestamp_micros()
                ])?;
                let mut original = None;
                while let Some(row) = rows.next()? {
                    let candidate = row_message(row)?;
                    if candidate.content == message.content
                        && candidate.source == message.source
                    {
                        original = Some(candidate);
                        break;
                    }
                }
                original
            };
            if let Some(original) = original {
                *history_id = original.history_id;
            }
        }
        Ok(())
    }

    fn relink_highlights(&mut self, kind: &Kind) -> Result<(), Error> {
        let Kind::Channel(server, channel) = kind else {
            return Ok(());
        };
        let mut statement = self.transaction.prepare_cached(
            "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content FROM message m
             JOIN history h ON h.id = m.history WHERE h.kind = ?1 ORDER BY m.time, m.id",
        )?;
        let mut rows = statement.query([history_key(&Kind::Highlights)?])?;
        let mut updates = Vec::new();
        while let Some(row) = rows.next()? {
            let mut message = row_message(row)?;
            if let message::Target::Highlights {
                server: origin_server,
                channel: origin_channel,
                history_id,
            } = &message.target
                && origin_server == server
                && origin_channel == channel
                && *history_id == Id::Undetermined
            {
                self.link_highlight(&mut message)?;
                if matches!(
                    &message.target,
                    message::Target::Highlights {
                        history_id: Id::Determined(_),
                        ..
                    }
                ) {
                    updates.push(message);
                }
            }
        }
        drop(rows);
        drop(statement);
        if !updates.is_empty() {
            let history = self.history(&Kind::Highlights)?;
            for mut message in updates {
                self.store(history, &mut message)?;
            }
        }
        Ok(())
    }

    pub fn mark_as_read(&mut self, kind: &Kind) -> Result<Metadata, Error> {
        let mut metadata = self.get_metadata(kind)?;
        metadata.read_marker =
            metadata.read_marker.max(metadata.latest.map(Into::into));
        self.metadata(kind, &metadata)?;
        Ok(metadata)
    }

    pub fn clear(&mut self, kind: &Kind) -> Result<(), Error> {
        let history = self.history(kind)?;
        if is_searchable(kind) {
            self.transaction.execute(
                "INSERT OR REPLACE INTO search_pending SELECT time * 1024 + (id & 1023), id FROM message WHERE history = ?1 AND id <= ?2",
                [history, self.indexed],
            )?;
        }
        self.transaction
            .execute("DELETE FROM message WHERE history = ?1", [history])?;
        self.transaction.execute(
            "UPDATE history SET read_marker = NULL, latest = NULL, latest_triggers_unread = NULL,
             latest_triggers_highlight = NULL, chathistory_references = NULL WHERE id = ?1", [history],
        )?;
        Ok(())
    }

    pub fn redact_highlight(
        &mut self,
        kind: &Kind,
        source_id: Id,
        redaction: redaction::RedactionWithContext,
    ) -> Result<bool, Error> {
        let Kind::Channel(server, channel) = kind else {
            return Ok(false);
        };
        let mut statement = self.transaction.prepare_cached(
            "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content FROM message m
             JOIN history h ON h.id = m.history WHERE h.kind = ?1 AND m.msgid = ?2
             ORDER BY m.time DESC, m.id DESC",
        )?;
        let mut rows = statement.query(params![
            history_key(&Kind::Highlights)?,
            redaction.redacts.as_ref()
        ])?;
        while let Some(row) = rows.next()? {
            let mut message = row_message(row)?;
            if let message::Target::Highlights {
                server: origin_server,
                channel: origin_channel,
                history_id,
            } = &message.target
                && origin_server == server
                && origin_channel == channel
                && *history_id == source_id
            {
                message.redaction = Some(redaction.into());
                drop(rows);
                drop(statement);
                let history = self.history(&Kind::Highlights)?;
                self.store(history, &mut message)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn metadata(
        &mut self,
        kind: &Kind,
        metadata: &Metadata,
    ) -> Result<(), Error> {
        let history = self.history(kind)?;
        self.transaction.prepare_cached(
            "UPDATE history SET
                read_marker = CASE WHEN read_marker IS NULL OR read_marker < ?2 THEN ?2 ELSE read_marker END,
                latest = CASE WHEN latest IS NULL OR latest < ?3 THEN ?3 ELSE latest END,
                latest_triggers_unread = CASE WHEN latest_triggers_unread IS NULL OR latest_triggers_unread < ?4 THEN ?4 ELSE latest_triggers_unread END,
                latest_triggers_highlight = CASE WHEN latest_triggers_highlight IS NULL OR latest_triggers_highlight < ?5 THEN ?5 ELSE latest_triggers_highlight END,
                chathistory_references = COALESCE(?6, chathistory_references)
             WHERE id = ?1")?.execute(
            params![history, metadata.read_marker.map(|marker| marker.as_date_time().timestamp_micros()),
                metadata.latest.map(|time| time.timestamp_micros()), metadata.latest_triggers_unread.map(|time| time.timestamp_micros()),
                metadata.latest_triggers_highlight.map(|time| time.timestamp_micros()),
                metadata.latest_chathistory_references.as_ref().map(serde_json::to_string).transpose()?],
        )?;
        Ok(())
    }

    pub fn commit(self) -> Result<(), Error> {
        Ok(self.transaction.commit()?)
    }

    fn history(&mut self, kind: &Kind) -> Result<i64, Error> {
        if let Some(id) = self.histories.get(kind) {
            return Ok(*id);
        }
        let key = history_key(kind)?;
        let (server, target) = match kind {
            Kind::Server(server) => (Some(server), None),
            Kind::Channel(server, channel) => {
                (Some(server), Some(channel.as_str()))
            }
            Kind::Query(server, query) => (Some(server), Some(query.as_str())),
            Kind::Logs | Kind::Highlights | Kind::ChannelMonitor => {
                (None, None)
            }
        };
        let server = server.map(|server| format!("{server:b}"));
        self.transaction.prepare_cached(
            "INSERT INTO history(kind, server, target) VALUES (?1, ?2, ?3) ON CONFLICT(kind) DO NOTHING",
        )?.execute(params![&key, server, target])?;
        let id = self
            .transaction
            .prepare_cached("SELECT id FROM history WHERE kind = ?1")?
            .query_row([&key], |row| row.get(0))?;
        self.histories.insert(kind.clone(), id);
        if is_searchable(kind) {
            self.searchable.insert(id);
        }
        Ok(id)
    }

    fn store(
        &mut self,
        history: i64,
        message: &mut message::Message,
    ) -> Result<(), Error> {
        let referenceable =
            !message.is_rerouted() && message.can_reference(REFERENCE_TYPES);
        // Monitor membership is structural. Current filters and the monitored
        // channel set are applied on read, so unblocking can reveal old rows.
        let monitor = message.show_in_channel_monitor();
        let body = StoredMessage {
            direction: message.direction.clone(),
            source: message.source.clone(),
            target: message.target.clone(),
            content: message.content.clone(),
            hidden_urls: message.hidden_urls.clone(),
            reactions: message.reactions.clone(),
            relayed_by: message.relayed_by.clone(),
            rerouted_from: message.rerouted_from.clone(),
            redaction: message.redaction.clone(),
            reply_to: message.reply_to.clone(),
        };
        self.serialization_buffer.clear();
        serde_json::to_writer(&mut self.serialization_buffer, &body)?;
        let id = match message.history_id {
            Id::Undetermined => None,
            Id::Determined(id) => Some(sql_id(id)?),
        };
        let user = self
            .searchable
            .contains(&history)
            .then(|| message.source.user())
            .flatten();
        let previous_time: Option<i64> = match (user, id) {
            (Some(_), Some(id)) if id <= self.indexed => self
                .transaction
                .prepare_cached("SELECT time FROM message WHERE id = ?1")?
                .query_row([id], |row| row.get(0))
                .optional()?,
            _ => None,
        };
        // msgid is not unique: Halloy's historical/echo matching decides which
        // row is replaced, not an SQL uniqueness constraint.
        let changed = self.transaction.prepare_cached(
            "INSERT INTO message(id, history, time, time_from_server, msgid, referenceable, in_channel_monitor, content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET time=excluded.time, time_from_server=excluded.time_from_server,
                 msgid=excluded.msgid, referenceable=excluded.referenceable,
                 in_channel_monitor=excluded.in_channel_monitor, content=excluded.content
             WHERE message.history = excluded.history",
        )?.execute(params![id, history, message.time.utc.timestamp_micros(),
            matches!(message.time.source, message::time::Source::Server),
            message.id.as_deref(), referenceable, monitor, self.serialization_buffer.as_slice()])?;
        if changed == 0 {
            return Ok(());
        }
        let rowid = match id {
            Some(id) => id,
            None => {
                let rowid = self.transaction.last_insert_rowid();
                message.history_id = Id::Determined(rowid as u64);
                rowid
            }
        };
        if let Some(key) =
            previous_time.and_then(|time| search_key(time, rowid))
        {
            self.transaction
                .prepare_cached(
                    "INSERT OR REPLACE INTO search_pending VALUES (?1, ?2)",
                )?
                .execute([key, rowid])?;
        }
        Ok(())
    }
}

pub(super) fn normalize(
    message: &mut message::Message,
    casemapping: isupport::CaseMap,
) {
    message.renormalize(casemapping);
    match &mut message.target {
        message::Target::Channel { channel }
        | message::Target::Highlights { channel, .. }
        | message::Target::ChannelMonitor { channel, .. } => {
            channel.renormalize(casemapping);
        }
        message::Target::Query { query } => query.renormalize(casemapping),
        _ => (),
    }
    for reaction in &mut message.reactions {
        reaction.sender.renormalize(casemapping);
    }
    if let Some(redaction) = &mut message.redaction {
        redaction.from.renormalize(casemapping);
    }
}

fn is_imported(connection: &Connection, kind: &Kind) -> Result<bool, Error> {
    Ok(connection
        .prepare_cached(
            "SELECT imported_at IS NOT NULL FROM history WHERE kind = ?1",
        )?
        .query_row([history_key(kind)?], |row| row.get(0))
        .optional()?
        .unwrap_or(false))
}

fn read_metadata(
    connection: &Connection,
    kind: &Kind,
) -> Result<Metadata, Error> {
    let values = connection.prepare_cached(
            "SELECT read_marker, latest, latest_triggers_unread, latest_triggers_highlight, chathistory_references
             FROM history WHERE kind = ?1")?.query_row([history_key(kind)?], |row| {
                Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?, row.get::<_, Option<String>>(4)?))
            },
        ).optional()?;
    let Some((read, latest, unread, highlight, references)) = values else {
        return Ok(Metadata::default());
    };
    let date = |value: Option<i64>| -> Result<Option<DateTime<Utc>>, Error> {
        value
            .map(|time| {
                DateTime::from_timestamp_micros(time).ok_or_else(|| {
                    rusqlite::Error::IntegralValueOutOfRange(0, time).into()
                })
            })
            .transpose()
    };
    Ok(Metadata {
        read_marker: date(read)?.map(Into::into),
        latest: date(latest)?,
        latest_triggers_unread: date(unread)?,
        latest_triggers_highlight: date(highlight)?,
        latest_chathistory_references: references
            .map(|value| serde_json::from_str(&value))
            .transpose()?,
    })
}

fn history_key(kind: &Kind) -> Result<String, serde_json::Error> {
    let (tag, server, target) = match kind {
        Kind::Server(server) => ("server", Some(server), None),
        Kind::Channel(server, channel) => {
            ("channel", Some(server), Some(channel.as_normalized_str()))
        }
        Kind::Query(server, query) => {
            ("query", Some(server), Some(query.as_normalized_str()))
        }
        Kind::Logs => ("logs", None, None),
        Kind::Highlights => ("highlights", None, None),
        Kind::ChannelMonitor => ("channel_monitor", None, None),
    };
    serde_json::to_string(&(
        tag,
        server.map(|server| format!("{server:b}")),
        target,
    ))
}

pub(super) fn search(
    connection: &Connection,
    query: &search::Query,
    before: Option<search::Cursor>,
    limit: usize,
) -> Result<search::Page, Error> {
    let Some(fts) = query.fts() else {
        return Ok(search::Page {
            hits: vec![],
            next: None,
        });
    };
    let limit = i64::try_from(limit).unwrap_or(i64::MAX);
    let mut statement = connection.prepare_cached(
        "SELECT m.id, m.time, m.time_from_server, m.msgid, m.content,
                h.server, h.target, json_extract(h.kind, '$[0]'), s.rowid
         FROM message_search s
         JOIN message m ON m.id = s.message
         JOIN history h ON h.id = m.history
         WHERE message_search MATCH ?1
           AND (?2 IS NULL OR h.target = ?2 COLLATE NOCASE)
           AND (?3 IS NULL OR s.rowid < ?3)
         ORDER BY s.rowid DESC
         LIMIT ?4",
    )?;
    let rows = statement
        .query_map(
            params![fts, query.within, before.map(|cursor| cursor.key), limit],
            |row| {
                let message = row_message(row)?;
                let server: String = row.get(5)?;
                let target: Option<String> = row.get(6)?;
                let tag: String = row.get(7)?;
                let buffer = match (tag.as_str(), target) {
                    ("channel", Some(target)) => {
                        search::Buffer::Channel(target)
                    }
                    ("query", Some(target)) => search::Buffer::Query(target),
                    _ => search::Buffer::Server,
                };
                let key: i64 = row.get(8)?;
                Ok((
                    search::Hit {
                        server,
                        buffer,
                        message,
                    },
                    key,
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let next = (rows.len() as i64 == limit)
        .then(|| rows.last())
        .flatten()
        .map(|(_, key)| search::Cursor { key: *key });
    let hits = rows
        .into_iter()
        .map(|(hit, _)| hit)
        .filter(|hit| hit.message.redaction.is_none())
        .collect();
    Ok(search::Page { hits, next })
}

fn indexable_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(i64, i64, Vec<u8>, bool)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn search_key(time: i64, id: i64) -> Option<i64> {
    time.checked_mul(1024)?.checked_add(id & 1023)
}

fn is_searchable(kind: &Kind) -> bool {
    matches!(kind, Kind::Server(_) | Kind::Channel(..) | Kind::Query(..))
}

fn row_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<message::Message> {
    let timestamp: i64 = row.get(1)?;
    let utc =
        DateTime::<Utc>::from_timestamp_micros(timestamp).ok_or_else(|| {
            rusqlite::Error::IntegralValueOutOfRange(1, timestamp)
        })?;
    let content = row.get_ref(4)?.as_blob()?;
    let body: StoredMessage =
        serde_json::from_slice(content).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Blob,
                Box::new(error),
            )
        })?;
    let server: bool = row.get(2)?;
    Ok(message::Message {
        history_id: Id::Determined(
            u64::try_from(row.get::<_, i64>(0)?).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?,
        ),
        time: message::Time {
            utc,
            source: if server {
                message::time::Source::Server
            } else {
                message::time::Source::Client
            },
        },
        id: row.get::<_, Option<String>>(3)?.map(Into::into),
        direction: body.direction,
        source: body.source,
        target: body.target,
        content: body.content,
        hidden_urls: body.hidden_urls,
        reactions: body.reactions,
        relayed_by: body.relayed_by,
        rerouted_from: body.rerouted_from,
        redaction: body.redaction,
        reply_to: body.reply_to,
    })
}

fn sql_id(id: u64) -> rusqlite::Result<i64> {
    i64::try_from(id).map_err(|error| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
    })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::history::search::{Buffer, Query};
    use crate::user::Nick;
    use crate::{Server, User, isupport, target};

    fn database() -> Database {
        Database::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn server() -> Server {
        Server::from(std::sync::Arc::<str>::from("libera"))
    }

    fn channel(name: &str) -> Kind {
        Kind::Channel(
            server(),
            target::Channel::from_str(
                name,
                &['#'],
                isupport::CaseMap::default(),
            ),
        )
    }

    fn user(nick: &str) -> User {
        User::from(Nick::from_str(nick, isupport::CaseMap::default()))
    }

    fn message(seconds: i64, nick: &str, text: &str) -> message::Message {
        message::Message {
            history_id: Id::Undetermined,
            time: message::Time::client(
                Utc.timestamp_opt(1_700_000_000 + seconds, 0).unwrap(),
            ),
            direction: message::Direction::Received { is_echo: false },
            source: message::Source::User(user(nick)),
            target: message::Target::Server,
            content: message::Content::Plain(text.to_string()),
            id: None,
            hidden_urls: HashSet::new(),
            reactions: vec![],
            relayed_by: None,
            rerouted_from: None,
            redaction: None,
            reply_to: None,
        }
    }

    fn store(
        database: &mut Database,
        kind: &Kind,
        messages: Vec<message::Message>,
    ) -> Vec<message::Message> {
        let mut write = database.transaction().unwrap();
        let history = write.history(kind).unwrap();
        let stored = messages
            .into_iter()
            .map(|mut message| {
                write.store(history, &mut message).unwrap();
                message
            })
            .collect();
        write.commit().unwrap();
        while database.index(10).unwrap() {}
        stored
    }

    fn texts(database: &mut Database, query: &str) -> Vec<String> {
        while database.index(10).unwrap() {}
        database
            .search(&Query::parse(query), None, 100)
            .unwrap()
            .hits
            .into_iter()
            .map(|hit| hit.message.content.text().into_owned())
            .collect()
    }

    #[test]
    fn search_finds_words_and_prefixes_newest_first() {
        let mut database = database();
        store(
            &mut database,
            &channel("#Halloy"),
            vec![
                message(1, "casper", "the release is out"),
                message(2, "andrew", "Released a fix"),
                message(3, "casper", "unrelated chatter"),
            ],
        );

        assert_eq!(
            texts(&mut database, "release"),
            ["Released a fix", "the release is out"]
        );
        assert_eq!(texts(&mut database, "release "), ["the release is out"]);
        assert_eq!(
            texts(&mut database, "\"release is\""),
            ["the release is out"]
        );
        assert!(texts(&mut database, "nothing").is_empty());

        let page = database
            .search(&Query::parse("chatter"), None, 100)
            .unwrap();
        let hit = &page.hits[0];
        assert_eq!(hit.server, "libera");
        assert_eq!(hit.buffer, Buffer::Channel("#Halloy".to_string()));
    }

    #[test]
    fn search_filters_by_nick_and_target() {
        let mut database = database();
        store(
            &mut database,
            &channel("#halloy"),
            vec![
                message(1, "Casper", "hello halloy"),
                message(2, "andrew", "hello halloy"),
                message(5, "casper_away", "hello away"),
            ],
        );
        store(
            &mut database,
            &channel("#other"),
            vec![message(3, "casper", "hello other")],
        );
        let query = Kind::Query(
            server(),
            target::Query::from(Nick::from_str(
                "casper",
                isupport::CaseMap::default(),
            )),
        );
        store(
            &mut database,
            &query,
            vec![message(4, "casper", "hello query")],
        );

        assert_eq!(
            texts(&mut database, "from:CASPER hello"),
            ["hello query", "hello other", "hello halloy"]
        );
        assert_eq!(
            texts(&mut database, "from:casper in:#HALLOY"),
            ["hello halloy"]
        );
        assert_eq!(texts(&mut database, "in:casper hello"), ["hello query"]);

        let page = database.search(&Query::parse("query"), None, 100).unwrap();
        assert_eq!(page.hits[0].buffer, Buffer::Query("casper".to_string()));
    }

    #[test]
    fn only_user_messages_in_upstream_histories_are_searchable() {
        let mut database = database();
        let mut event = message(1, "casper", "casper joined the channel");
        event.source = message::Source::Server(None);
        store(&mut database, &channel("#halloy"), vec![event]);
        store(
            &mut database,
            &Kind::Highlights,
            vec![message(2, "casper", "casper highlighted")],
        );
        store(
            &mut database,
            &Kind::Server(server()),
            vec![message(3, "casper", "casper on the server")],
        );

        assert_eq!(texts(&mut database, "casper"), ["casper on the server"]);
        let page = database.search(&Query::parse("casper"), None, 100).unwrap();
        assert_eq!(page.hits[0].buffer, Buffer::Server);
    }

    #[test]
    fn rewrites_redactions_and_deletes_update_the_index() {
        let mut database = database();
        let kind = channel("#halloy");
        let mut stored = store(
            &mut database,
            &kind,
            vec![
                message(1, "casper", "first draft"),
                message(2, "casper", "second draft"),
                message(3, "casper", "third draft"),
            ],
        );

        stored[0].content = message::Content::Plain("first edit".to_string());
        let rewritten = store(&mut database, &kind, vec![stored[0].clone()]);
        assert_eq!(rewritten[0].history_id, stored[0].history_id);
        assert_eq!(texts(&mut database, "first"), ["first edit"]);
        assert_eq!(
            texts(&mut database, "draft"),
            ["third draft", "second draft"]
        );

        stored[0].time = message(10, "casper", "").time;
        store(&mut database, &kind, vec![stored[0].clone()]);
        assert_eq!(texts(&mut database, "first"), ["first edit"]);
        assert_eq!(
            texts(&mut database, "from:casper"),
            ["first edit", "third draft", "second draft"]
        );

        stored[1].redaction = Some(redaction::Redaction {
            from: Nick::from_str("casper", isupport::CaseMap::default()),
            reason: None,
        });
        store(&mut database, &kind, vec![stored[1].clone()]);
        assert_eq!(texts(&mut database, "draft"), ["third draft"]);
        assert!(texts(&mut database, "from:casper").len() == 2);

        let write = database.transaction().unwrap();
        write.remove(&kind, stored[2].history_id).unwrap();
        write.commit().unwrap();
        assert!(texts(&mut database, "draft").is_empty());

        let mut write = database.transaction().unwrap();
        write.clear(&kind).unwrap();
        write.commit().unwrap();
        assert!(texts(&mut database, "first").is_empty());
        let count: i64 = database
            .connection
            .query_row("SELECT count(*) FROM message_search", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn redacted_messages_are_hidden_before_reindexing() {
        let mut database = database();
        let kind = channel("#halloy");
        let mut stored =
            store(&mut database, &kind, vec![message(1, "casper", "secret")]);

        stored[0].redaction = Some(redaction::Redaction {
            from: Nick::from_str("casper", isupport::CaseMap::default()),
            reason: None,
        });
        let mut write = database.transaction().unwrap();
        let history = write.history(&kind).unwrap();
        write.store(history, &mut stored[0]).unwrap();
        write.commit().unwrap();

        let page = database.search(&Query::parse("secret"), None, 100).unwrap();
        assert!(page.hits.is_empty());
    }

    #[test]
    fn search_pages_without_gaps_or_overlaps() {
        let mut database = database();
        let messages = (0..25)
            .map(|index| {
                message(index / 2, "casper", &format!("paged {index}"))
            })
            .collect();
        store(&mut database, &channel("#halloy"), messages);

        let query = Query::parse("paged");
        let mut seen = vec![];
        let mut before = None;
        loop {
            let page = database.search(&query, before, 10).unwrap();
            seen.extend(page.hits.iter().map(|hit| hit.message.history_id));
            match page.next {
                Some(next) => before = Some(next),
                None => break,
            }
        }

        let mut expected: Vec<_> = (1..=25).rev().map(Id::Determined).collect();
        expected.dedup();
        assert_eq!(seen, expected);
    }
}
