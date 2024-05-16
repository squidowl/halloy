use std::io;
use std::path::PathBuf;
use std::time;

use interprocess::local_socket::tokio::LocalSocketListener;

#[cfg(windows)]
fn server_path() -> String {
    use std::time;

    let nonce = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    format!("halloy-{}", nonce)
}

#[cfg(windows)]
pub fn server_path_register_path() -> PathBuf {
    data::environment::data_dir().join("ipc.txt")
}

#[cfg(not(windows))]
pub fn socket_directory() -> PathBuf {
    data::environment::data_dir()
}

#[cfg(not(windows))]
pub async fn with_socket_path<T, Fut>(f: impl FnOnce(PathBuf) -> Fut) -> T
where
    Fut: futures::Future<Output = T>,
{
    let file = socket_directory().join("urlserver.sock");
    f(file).await
}

#[cfg(not(windows))]
pub async fn spawn_server() -> Result<LocalSocketListener, io::Error> {
    with_socket_path(|path| async {
        let _ = tokio::fs::remove_file(path.clone()).await;
        LocalSocketListener::bind(path)
    })
    .await
}

#[cfg(windows)]
async fn spawn_server() -> Result<LocalSocketListener, io::Error> {
    let path = server_path();
    let named_pipe_addr_file = server_path_register_path();

    tokio::fs::write(named_pipe_addr_file, &path).await?;
    LocalSocketListener::bind(path)
}

pub fn listen() -> futures::stream::BoxStream<'static, String> {
    use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use futures::stream::StreamExt;

    enum State {
        Uninitialized,
        Waiting(LocalSocketListener),
    }

    futures::stream::unfold(State::Uninitialized {}, move |state| async move {
        match state {
            State::Uninitialized => match spawn_server().await {
                Ok(server) => Some((None, State::Waiting(server))),
                Err(err) => {
                    println!("error: {:?}", err);
                    None
                }
            },
            State::Waiting(server) => {
                let conn = server.accept().await;

                let Ok(conn) = conn else {
                    return Some((None, State::Waiting(server)));
                };

                let mut conn = BufReader::new(conn);
                let mut buffer = String::new();

                let msg = tokio::time::timeout(
                    time::Duration::from_millis(1_000),
                    conn.read_line(&mut buffer),
                )
                .await;

                let _ = conn.close().await;

                match msg {
                    Ok(Ok(_)) => Some((Some(buffer), State::Waiting(server))),
                    Err(_) | Ok(Err(_)) => Some((None, State::Waiting(server))),
                }
            }
        }
    })
    .filter_map(|value| async move { value })
    .boxed()
}
