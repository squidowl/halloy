use std::fmt;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Server(String);

impl From<String> for Server {
    fn from(server: String) -> Self {
        Self(server)
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
