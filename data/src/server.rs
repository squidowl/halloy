#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Server(String);

impl From<String> for Server {
    fn from(server: String) -> Self {
        Self(server)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    server: Server,
    raw: irc::client::data::Config,
}
