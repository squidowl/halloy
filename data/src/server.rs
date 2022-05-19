#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Server(String);

impl From<String> for Server {
    fn from(server: String) -> Self {
        Self(server)
    }
}

impl Into<String> for Server {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    _server: Server,
    _raw: irc::client::data::Config,
}
