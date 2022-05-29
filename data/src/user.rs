#[derive(Debug, Clone)]
pub struct User(irc::client::data::User);

impl User {
    pub fn nickname(&self) -> &str {
        self.0.get_nickname()
    }
}

impl From<irc::client::data::User> for User {
    fn from(user: irc::client::data::User) -> Self {
        Self(user)
    }
}
