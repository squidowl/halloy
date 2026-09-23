use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak, mpsc};
use std::time::{Duration, Instant};
use std::{fs, thread};

use hashbrown::HashSet;
use rusqlite::Connection;

use super::{Error, Event, Message, batch, cache};
use crate::history::{Kind, Metadata, ReadMarker, database, legacy};
use crate::{client, environment, message};

const QUEUE_WARNING: usize = 10_000;
const MAX_ACTIVE_IMPORTS: usize = 2;
const INDEX_BATCH: usize = 1_000;
const INDEX_INTERVAL: Duration = Duration::from_secs(2);

type Events = tokio::sync::mpsc::UnboundedSender<Vec<Event>>;

#[derive(Debug)]
pub(super) enum Command {
    Initialize(Kind),
    Write(batch::Batch),
    Read(Kind, cache::Read, Vec<Kind>),
    Reference(client::ChathistoryLookup),
    Resend(client::ResendLookup),
    HighlightSource(crate::history::model::Navigation, crate::history::Id),
    MarkRead(Kind, Option<ReadMarker>, bool),
    Exit(Vec<(Kind, bool)>, Option<Result<usize, Error>>),
    Imported(
        Kind,
        Result<(Vec<message::Message>, Metadata), legacy::Error>,
    ),
}

impl Command {
    fn kinds(&self) -> Vec<Kind> {
        match self {
            Self::Initialize(kind)
            | Self::MarkRead(kind, ..)
            | Self::Imported(kind, ..) => vec![kind.clone()],
            Self::Write(batch) => batch.kinds().collect(),
            Self::Read(kind, _, monitored) => {
                let mut kinds = vec![kind.clone()];
                if *kind == Kind::ChannelMonitor {
                    kinds.extend(monitored.iter().cloned());
                }
                kinds
            }
            Self::Reference(lookup) => vec![Kind::from_target(
                lookup.server.clone(),
                lookup.target.clone(),
            )],
            Self::HighlightSource(navigation, _) => {
                vec![Kind::Highlights, Kind::from(navigation.buffer.clone())]
            }
            Self::Resend(lookup) => vec![Kind::from(lookup.buffer.clone())],
            Self::Exit(histories, _) => {
                histories.iter().map(|(kind, _)| kind.clone()).collect()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct Worker {
    sender: Arc<mpsc::Sender<Command>>,
    events: Events,
    queued: Arc<AtomicUsize>,
}

impl Worker {
    pub fn new(events: Events) -> Self {
        Self::start(events, Some(environment::data_dir()))
    }

    #[cfg(test)]
    pub fn test(events: Events) -> Self {
        Self::start(events, None)
    }

    fn start(events: Events, path: Option<PathBuf>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let sender = Arc::new(sender);
        let weak_sender = Arc::downgrade(&sender);
        let queued = Arc::new(AtomicUsize::new(0));
        let output = events.clone();
        let count = queued.clone();
        thread::spawn(move || {
            let mut state = State {
                database: None,
                path,
                events: output,
                sender: weak_sender,
                queued: count,
                ready: HashSet::new(),
                importing: HashSet::new(),
                waiting_imports: VecDeque::new(),
                active_imports: 0,
                pending: VecDeque::new(),
                blocked: HashSet::new(),
                exit_pending: false,
                failure: None,
                index_backlog: true,
                index_failed: false,
            };
            match state.open() {
                Ok(database) => state.database = Some(database),
                Err(error) => state.fail(error),
            }
            let mut changed_at: Option<Instant> = None;
            loop {
                let received = if state.index_backlog {
                    receiver.recv_timeout(Duration::ZERO)
                } else if let Some(changed_at) = changed_at {
                    receiver.recv_timeout(
                        INDEX_INTERVAL.saturating_sub(changed_at.elapsed()),
                    )
                } else {
                    receiver
                        .recv()
                        .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
                };
                match received {
                    Ok(command) => {
                        changed_at.get_or_insert_with(Instant::now);
                        if state.accept(command) {
                            break;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => (),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
                if state.index_backlog
                    || changed_at.is_some_and(|changed_at| {
                        changed_at.elapsed() >= INDEX_INTERVAL
                    })
                {
                    state.index();
                    changed_at = None;
                }
            }
        });
        Self {
            sender,
            events,
            queued,
        }
    }

    pub fn send(&self, command: Command) {
        enqueue(&self.sender, &self.queued, &self.events, command);
    }
}

fn enqueue(
    sender: &mpsc::Sender<Command>,
    queued: &AtomicUsize,
    events: &Events,
    command: Command,
) {
    if queued.fetch_add(1, Ordering::Relaxed) == QUEUE_WARNING {
        log::warn!("history queue has exceeded {QUEUE_WARNING} commands");
    }
    if let Err(mpsc::SendError(command)) = sender.send(command) {
        queued.fetch_sub(1, Ordering::Relaxed);
        let error = "history worker stopped".to_string();
        let _ =
            events.send(vec![Event::History(Message::Failed(error.clone()))]);
        reject(events, command, &error);
    }
}

struct State {
    database: Option<database::Database>,
    path: Option<PathBuf>,
    events: Events,
    sender: Weak<mpsc::Sender<Command>>,
    queued: Arc<AtomicUsize>,
    ready: HashSet<Kind>,
    importing: HashSet<Kind>,
    waiting_imports: VecDeque<Kind>,
    // A slot stays occupied until the decoded payload is committed or dropped.
    active_imports: usize,
    pending: VecDeque<(Command, Vec<Kind>)>,
    blocked: HashSet<Kind>,
    exit_pending: bool,
    failure: Option<String>,
    index_backlog: bool,
    index_failed: bool,
}

impl State {
    fn index(&mut self) {
        if self.failure.is_some() || self.index_failed {
            return;
        }
        let Some(database) = self.database.as_mut() else {
            return;
        };
        match database.index(INDEX_BATCH) {
            Ok(more) => self.index_backlog = more,
            Err(error) => {
                log::warn!("search indexing stopped: {error}");
                self.index_failed = true;
                self.index_backlog = false;
            }
        }
    }

    fn open(&self) -> Result<database::Database, String> {
        let connection = if let Some(path) = &self.path {
            fs::create_dir_all(path).map_err(string)?;
            Connection::open(path.join(database::FILE_NAME)).map_err(string)?
        } else {
            Connection::open_in_memory().map_err(string)?
        };
        let mut database =
            database::Database::new(connection).map_err(string)?;
        let mut transaction = database.transaction().map_err(string)?;
        transaction.clear(&Kind::Logs).map_err(string)?;
        transaction
            .import(&Kind::Logs, &[], &Metadata::default())
            .map_err(string)?;
        transaction.commit().map_err(string)?;
        Ok(database)
    }

    fn initialize(&mut self, kind: Kind) -> Result<(), String> {
        if self.ready.contains(&kind) || self.importing.contains(&kind) {
            return Ok(());
        }
        let database = self.database.as_ref().expect("opened history database");
        if database.is_imported(&kind).map_err(string)? {
            let metadata = database.metadata(&kind).map_err(string)?;
            self.ready.insert(kind.clone());
            self.emit(Message::Initialized(kind, metadata));
        } else if let Some(path) = &self.path
            && legacy::exists(path, &kind).map_err(string)?
        {
            self.importing.insert(kind.clone());
            self.emit(Message::Importing(kind.clone()));
            self.waiting_imports.push_back(kind);
            self.start_imports();
        } else {
            self.import(kind, Ok((vec![], Metadata::default())));
        }
        Ok(())
    }

    fn start_imports(&mut self) {
        if self.failure.is_some() {
            return;
        }
        while self.active_imports < MAX_ACTIVE_IMPORTS {
            let Some(kind) = self.waiting_imports.pop_front() else {
                break;
            };
            let Some(sender) = self.sender.upgrade() else {
                break;
            };
            let path = self.path.clone().expect("legacy history directory");
            let queued = self.queued.clone();
            let events = self.events.clone();
            self.active_imports += 1;
            thread::spawn(move || {
                let result = legacy::load(&path, &kind);
                enqueue(
                    &sender,
                    &queued,
                    &events,
                    Command::Imported(kind, result),
                );
            });
        }
    }

    fn import(
        &mut self,
        kind: Kind,
        result: Result<(Vec<message::Message>, Metadata), legacy::Error>,
    ) {
        self.importing.remove(&kind);
        if self.failure.is_some() {
            return;
        }
        let result = result.map_err(string).and_then(|(messages, metadata)| {
            let database =
                self.database.as_mut().expect("opened history database");
            retry(|| {
                let mut transaction = database.transaction()?;
                transaction.import(&kind, &messages, &metadata)?;
                transaction.commit().map_err(Error::from)
            })?;
            database.metadata(&kind).map_err(string)
        });
        match result {
            Ok(metadata) => {
                self.ready.insert(kind.clone());
                self.emit(Message::Initialized(kind, metadata));
            }
            Err(error) => self.fail(format!("cannot migrate {kind}: {error}")),
        }
    }

    fn accept(&mut self, command: Command) -> bool {
        // Import results bypass commands waiting for those histories, including Exit.
        if let Command::Imported(kind, result) = command {
            self.queued.fetch_sub(1, Ordering::Relaxed);
            self.import(kind, result);
            self.active_imports -= 1;
            self.start_imports();
            return self.drain();
        }
        let kinds = command.kinds();
        if self.failure.is_none() {
            for kind in &kinds {
                if let Err(error) = self.initialize(kind.clone()) {
                    self.fail(error);
                    break;
                }
            }
        }
        let exiting = matches!(command, Command::Exit(..));
        if self.failure.is_some()
            || (!self.exit_pending
                && (!exiting || self.pending.is_empty())
                && kinds.iter().all(|kind| {
                    self.ready.contains(kind) && !self.blocked.contains(kind)
                }))
        {
            self.queued.fetch_sub(1, Ordering::Relaxed);
            if let Some(error) = &self.failure {
                reject(&self.events, command, error);
            } else {
                self.execute(command);
            }
            exiting || (self.failure.is_some() && self.drain())
        } else {
            self.blocked.extend(kinds.iter().cloned());
            self.exit_pending |= exiting;
            self.pending.push_back((command, kinds));
            false
        }
    }

    fn drain(&mut self) -> bool {
        let mut blocked = HashSet::new();
        let mut barrier = false;
        let count = self.pending.len();
        for _ in 0..count {
            let (command, kinds) =
                self.pending.pop_front().expect("pending command");
            let exiting = matches!(command, Command::Exit(..));
            if self.failure.is_none()
                && (barrier
                    || kinds.iter().any(|kind| {
                        !self.ready.contains(kind) || blocked.contains(kind)
                    })
                    || (exiting && !blocked.is_empty()))
            {
                blocked.extend(kinds.iter().cloned());
                barrier |= exiting;
                self.pending.push_back((command, kinds));
                continue;
            }
            self.queued.fetch_sub(1, Ordering::Relaxed);
            if let Some(error) = &self.failure {
                reject(&self.events, command, error);
            } else {
                self.execute(command);
            }
            if exiting {
                return true;
            }
        }
        self.blocked = blocked;
        self.exit_pending = barrier;
        false
    }

    fn execute(&mut self, command: Command) {
        let database = self.database.as_mut().expect("opened history database");
        match command {
            Command::Initialize(_) => {}
            Command::Write(batch) => {
                let result = retry(|| {
                    let mut transaction = database.transaction()?;
                    let committed = batch.apply(&mut transaction)?;
                    transaction.commit()?;
                    Ok(committed)
                });
                match result {
                    Ok(committed) => self.emit(Message::Committed(committed)),
                    Err(error) => {
                        self.fail(error);
                        self.emit(Message::Unrecorded(batch));
                    }
                }
            }
            Command::Read(kind, read, monitored) => {
                let result = if read.extension.is_some() {
                    database.read_window(
                        &kind,
                        &monitored,
                        read.limit,
                        read.clear,
                        read.read_marker,
                        read.extension,
                    )
                } else if kind == Kind::ChannelMonitor {
                    database.monitor_window(
                        &monitored,
                        read.limit,
                        read.clear,
                        read.read_marker,
                    )
                } else {
                    database.window(
                        &kind,
                        read.limit,
                        read.clear,
                        read.read_marker,
                    )
                };
                match result {
                    Ok(window) => self.emit(Message::Read(kind, read, window)),
                    Err(error) => self.fail(string(error)),
                }
            }
            Command::Reference(lookup) => {
                let kind = Kind::from_target(
                    lookup.server.clone(),
                    lookup.target.clone(),
                );
                let reference = match database.reference(
                    &kind,
                    lookup.query,
                    &lookup.reference_types,
                ) {
                    Ok(reference) => reference,
                    Err(error) => {
                        self.fail(string(error));
                        None
                    }
                };
                let _ = self.events.send(vec![Event::Client(
                    client::Message::ChathistoryReference(lookup, reference),
                )]);
            }
            Command::HighlightSource(mut navigation, highlight) => {
                if navigation.token.strong_count() == 0 {
                    return;
                }
                let kind = Kind::from(navigation.buffer.clone());
                match database.highlight_source(&kind, highlight) {
                    Ok(message) => navigation.message = message,
                    Err(error) => self.fail(string(error)),
                }
                let _ = self.events.send(vec![Event::Model(
                    crate::history::model::Message::GoToMessage(navigation),
                )]);
            }
            Command::Resend(lookup) => {
                let kind = Kind::from(lookup.buffer.clone());
                let message = match database.message(&kind, lookup.history_id) {
                    Ok(message) => message.map(Box::new),
                    Err(error) => {
                        self.fail(string(error));
                        None
                    }
                };
                let _ = self.events.send(vec![Event::Client(
                    client::Message::ResendMessage(lookup, message),
                )]);
            }
            Command::MarkRead(kind, marker, send) => {
                let result = retry(|| {
                    let mut transaction = database.transaction()?;
                    let previous = transaction.get_metadata(&kind)?.read_marker;
                    let metadata = if let Some(marker) = marker {
                        let mut metadata = transaction.get_metadata(&kind)?;
                        metadata.read_marker =
                            metadata.read_marker.max(Some(marker));
                        transaction.metadata(&kind, &metadata)?;
                        metadata
                    } else {
                        transaction.mark_as_read(&kind)?
                    };
                    transaction.commit()?;
                    let advanced = metadata.read_marker > previous;
                    Ok((metadata, advanced))
                });
                match result {
                    Ok((metadata, advanced)) => self.emit(Message::Committed(
                        vec![marked(kind, metadata, send && advanced)],
                    )),
                    Err(error) => self.fail(error),
                }
            }
            Command::Exit(histories, input_saved) => {
                let result = retry(|| {
                    let mut transaction = database.transaction()?;
                    let mut committed = vec![];
                    for (kind, mark_latest) in &histories {
                        if *mark_latest {
                            let previous =
                                transaction.get_metadata(kind)?.read_marker;
                            let metadata = transaction.mark_as_read(kind)?;
                            let advanced = metadata.read_marker > previous;
                            committed.push(marked(
                                kind.clone(),
                                metadata,
                                advanced,
                            ));
                        }
                    }
                    transaction.commit()?;
                    Ok(committed)
                });
                let (result, mut events) = match result {
                    Ok(committed) => (
                        database.shutdown().map_err(string),
                        committed_events(committed),
                    ),
                    Err(error) => (Err(error), vec![]),
                };
                if let Err(error) = &result {
                    self.fail(error.clone());
                }
                events
                    .push(Event::History(Message::Exited(result, input_saved)));
                let _ = self.events.send(events);
            }
            Command::Imported(..) => {
                unreachable!("import completions bypass pending operations")
            }
        }
    }

    fn emit(&self, message: Message) {
        let events = match message {
            Message::Committed(committed) => committed_events(committed),
            message => vec![Event::History(message)],
        };
        let _ = self.events.send(events);
    }

    fn fail(&mut self, error: String) {
        if self.failure.is_none() {
            self.emit(Message::Failed(error.clone()));
            self.failure = Some(error);
            self.waiting_imports.clear();
        }
    }
}

fn committed_events(mut committed: Vec<batch::Committed>) -> Vec<Event> {
    let effects: Vec<_> = committed
        .iter_mut()
        .flat_map(|committed| std::mem::take(&mut committed.events))
        .collect();
    let mut events = vec![Event::History(Message::Committed(committed))];
    events.extend(effects);
    events
}

fn marked(kind: Kind, metadata: Metadata, send: bool) -> batch::Committed {
    let mut events = vec![];
    if send
        && let Some(marker) = metadata.read_marker
        && let Some(server) = kind.as_server()
        && let Some(target) = kind.target()
    {
        events.push(Event::Client(client::Message::SendMarkread(
            server.clone(),
            target,
            marker,
        )));
    }
    batch::Committed {
        change: batch::Change::Metadata,
        kind,
        display_read_marker: metadata.read_marker,
        metadata,
        show_in_sidebar: None,
        admitted_latest: None,
        events,
    }
}

fn reject(events: &Events, command: Command, error: &str) {
    let event = match command {
        Command::Write(batch) => Event::History(Message::Unrecorded(batch)),
        Command::Reference(lookup) => {
            Event::Client(client::Message::ChathistoryReference(lookup, None))
        }
        Command::HighlightSource(navigation, _) => Event::Model(
            crate::history::model::Message::GoToMessage(navigation),
        ),
        Command::Resend(lookup) => {
            Event::Client(client::Message::ResendMessage(lookup, None))
        }
        Command::Exit(_, input_saved) => {
            Event::History(Message::Exited(Err(error.to_string()), input_saved))
        }
        _ => return,
    };
    let _ = events.send(vec![event]);
}

fn retry<T>(
    mut operation: impl FnMut() -> Result<T, Error>,
) -> Result<T, String> {
    let started = Instant::now();
    for attempt in 0..3 {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) => {
                let busy = matches!(
                    &error,
                    Error::Database(database::Error::Sqlite(
                        rusqlite::Error::SqliteFailure(error, _)
                    )) if error.code == rusqlite::ErrorCode::DatabaseBusy
                );
                // BUSY can return immediately. Retry those cases, but do not
                // repeat a full busy_timeout wait and stall every other history.
                if attempt == 2
                    || (busy && started.elapsed() >= database::BUSY_TIMEOUT)
                {
                    return Err(string(error));
                }
                thread::sleep(Duration::from_millis(50 * (attempt + 1)));
            }
        }
    }
    unreachable!("bounded attempts return a value or the final error")
}

fn string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
