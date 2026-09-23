use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use futures::channel::oneshot;
use rusqlite::{Connection, InterruptHandle, OpenFlags};

use crate::environment;
use crate::history::{database, search};

type Reply = oneshot::Sender<Result<search::Page, search::Error>>;

struct Request {
    query: search::Query,
    before: Option<search::Cursor>,
    reply: Reply,
}

pub(super) struct Searcher {
    sender: mpsc::Sender<Request>,
    interrupt: Arc<Mutex<Option<InterruptHandle>>>,
}

impl std::fmt::Debug for Searcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Searcher").finish_non_exhaustive()
    }
}

impl Searcher {
    pub fn new() -> Self {
        Self::start(Some(environment::data_dir()))
    }

    #[cfg(test)]
    pub fn test() -> Self {
        Self::start(None)
    }

    fn start(path: Option<PathBuf>) -> Self {
        let (sender, receiver) = mpsc::channel::<Request>();
        let interrupt = Arc::new(Mutex::new(None));
        let handle = interrupt.clone();

        thread::spawn(move || {
            let mut connection = None;

            while let Ok(mut request) = receiver.recv() {
                while let Ok(newer) = receiver.try_recv() {
                    request = newer;
                }
                if request.reply.is_canceled() {
                    continue;
                }

                let result = open(&mut connection, path.as_deref(), &handle)
                    .and_then(|connection| {
                        database::search(
                            connection,
                            &request.query,
                            request.before,
                            search::PAGE_SIZE,
                        )
                        .map_err(|error| {
                            search::Error::Database(error.to_string())
                        })
                    });

                let _ = request.reply.send(result);
            }
        });

        Self { sender, interrupt }
    }

    pub fn search(
        &self,
        query: search::Query,
        before: Option<search::Cursor>,
    ) -> impl Future<Output = Result<search::Page, search::Error>> + use<> {
        if let Ok(handle) = self.interrupt.lock()
            && let Some(handle) = handle.as_ref()
        {
            handle.interrupt();
        }

        let (reply, receiver) = oneshot::channel();
        let sent = self
            .sender
            .send(Request {
                query,
                before,
                reply,
            })
            .is_ok();

        async move {
            if !sent {
                return Err(search::Error::Stopped);
            }
            receiver.await.unwrap_or(Err(search::Error::Stopped))
        }
    }
}

fn open<'a>(
    connection: &'a mut Option<Connection>,
    path: Option<&Path>,
    interrupt: &Mutex<Option<InterruptHandle>>,
) -> Result<&'a Connection, search::Error> {
    if connection.is_none() {
        let Some(path) = path else {
            return Err(search::Error::Database(
                "history is not stored on disk".to_string(),
            ));
        };
        let opened = Connection::open_with_flags(
            path.join(database::FILE_NAME),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .and_then(|opened| {
            opened.busy_timeout(database::BUSY_TIMEOUT)?;
            Ok(opened)
        })
        .map_err(|error| search::Error::Database(error.to_string()))?;

        if let Ok(mut handle) = interrupt.lock() {
            *handle = Some(opened.get_interrupt_handle());
        }
        *connection = Some(opened);
    }

    Ok(connection.as_ref().expect("opened search connection"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::history::{Id, Kind, Metadata};
    use crate::user::Nick;
    use crate::{Server, User, isupport, message, target};

    #[test]
    fn searches_a_database_open_for_writing() {
        let dir = tempfile::tempdir().unwrap();
        let mut database = database::Database::new(
            Connection::open(dir.path().join(database::FILE_NAME)).unwrap(),
        )
        .unwrap();

        let casemap = isupport::CaseMap::default();
        let kind = Kind::Channel(
            Server::from(Arc::<str>::from("libera")),
            target::Channel::from_str("#halloy", &['#'], casemap),
        );
        let message = message::Message {
            history_id: Id::Undetermined,
            time: message::Time::client(chrono::Utc::now()),
            direction: message::Direction::Received { is_echo: false },
            source: message::Source::User(User::from(Nick::from_str(
                "casper", casemap,
            ))),
            target: message::Target::Server,
            content: message::Content::Plain("written elsewhere".to_string()),
            id: None,
            hidden_urls: hashbrown::HashSet::default(),
            reactions: vec![],
            relayed_by: None,
            rerouted_from: None,
            redaction: None,
            reply_to: None,
        };
        let mut write = database.transaction().unwrap();
        write
            .import(&kind, &[message], &Metadata::default())
            .unwrap();
        write.commit().unwrap();
        database.index(100).unwrap();

        let searcher = Searcher::start(Some(dir.path().to_path_buf()));
        let page = futures::executor::block_on(
            searcher.search(search::Query::parse("elsewhere"), None),
        )
        .unwrap();
        assert_eq!(page.hits.len(), 1);
        assert!(page.next.is_none());

        drop(database);
    }
}
