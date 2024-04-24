use std::io::Write;

use interprocess::local_socket::LocalSocketStream;

use super::server;

#[cfg(not(windows))]
fn connect() -> Result<LocalSocketStream, server::Error> {
    futures::executor::block_on(server::with_socket_path(|path| async {
        Ok(LocalSocketStream::connect(path)?)
    }))
}

#[cfg(windows)]
fn connect() -> Result<LocalSocketStream, server::Error> {
    let register_path = server::server_path_register_path();
    let client_path = std::fs::read_to_string(register_path)?;

    Ok(LocalSocketStream::connect(client_path)?)
}

pub fn connect_and_send(url: super::Route) -> bool {
    match connect() {
        Ok(mut conn) => {
            let uri = url.to_string();
            conn.write_all(uri.as_bytes()).is_ok()
        },
        Err(_) => false,
    }
}
