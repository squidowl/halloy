use std::io::{self, Write};

use interprocess::local_socket::LocalSocketStream;

use super::server;

#[cfg(not(windows))]
fn connect() -> Result<LocalSocketStream, io::Error> {
    futures::executor::block_on(server::with_socket_path(|path| async {
        LocalSocketStream::connect(path)
    }))
}

#[cfg(windows)]
fn connect() -> Result<LocalSocketStream, io::Error> {
    let register_path = server::server_path_register_path();
    let client_path = std::fs::read_to_string(register_path)?;

    LocalSocketStream::connect(client_path)
}

pub fn connect_and_send(url: impl AsRef<[u8]>) -> bool {
    match connect() {
        Ok(mut conn) => conn.write_all(url.as_ref()).is_ok(),
        Err(_) => false,
    }
}
