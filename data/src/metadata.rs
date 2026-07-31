use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use iced::Color;
use serde::Deserialize;

use crate::target::{Channel, Query, Target, TargetRef};
use crate::{Url, User};

type Metadata = HashMap<String, String>;

#[derive(Copy, Clone, PartialEq, Eq, Deserialize, Debug, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Key {
    Avatar,
    Color,
    DisplayName,
    Homepage,
    Pronouns,
    Status,
}

impl Key {
    pub fn to_str(self) -> &'static str {
        match self {
            Key::Avatar => "avatar",
            Key::Color => "color",
            Key::DisplayName => "display-name",
            Key::Homepage => "homepage",
            Key::Pronouns => "pronouns",
            Key::Status => "status",
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Key::Avatar => "Avatar",
            Key::Color => "Color",
            Key::DisplayName => "Display name",
            Key::Homepage => "Homepage",
            Key::Pronouns => "Pronouns",
            Key::Status => "Status",
        };

        f.write_str(label)
    }
}
use Key::*;

impl FromStr for Key {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "avatar" => Ok(Key::Avatar),
            "color" => Ok(Key::Color),
            "display-name" => Ok(Key::DisplayName),
            "homepage" => Ok(Key::Homepage),
            "pronouns" => Ok(Key::Pronouns),
            "status" => Ok(Key::Status),
            _ => Err(()),
        }
    }
}

pub trait Registry {
    fn get_user(&self, target: &Query, key: Key) -> Option<&str>;
    fn get_channel(&self, target: &Channel, key: Key) -> Option<&str>;

    fn get(&self, target: TargetRef<'_>, key: Key) -> Option<&str> {
        match target {
            TargetRef::Channel(channel) => self.get_channel(channel, key),
            TargetRef::Query(query) => self.get_user(query, key),
        }
    }

    // channel or user metadata
    // https://ircv3.net/registry.html#channel-metadata
    fn avatar(&self, target: TargetRef<'_>) -> Option<&str> {
        self.get(target, Avatar)
    }
    fn display_name(&self, target: TargetRef<'_>) -> Option<&str> {
        self.get(target, DisplayName)
    }

    // user metadata
    // https://ircv3.net/registry.html#user-metadata
    fn color(&self, target: &Query) -> Option<Color> {
        self.get_user(target, Color)
            .and_then(|s| Color::from_str(s).ok())
    }
    fn homepage(&self, target: &Query) -> Option<Url> {
        self.get_user(target, Homepage)
            .and_then(|s| Url::from_str(s).ok())
    }
    fn status(&self, target: &Query) -> Option<&str> {
        self.get_user(target, Status)
    }
    fn pronouns(&self, target: &Query) -> Option<&str> {
        self.get_user(target, Pronouns)
    }
}

#[derive(Debug, Default)]
pub struct ServerRegistry {
    channels: HashMap<Channel, Metadata>,
    users: HashMap<Query, Metadata>,
}

impl ServerRegistry {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            users: HashMap::new(),
        }
    }

    pub fn insert(&mut self, target: Target, key: String, value: String) {
        match target {
            Target::Channel(channel) => {
                self.channels.entry(channel).or_default().insert(key, value);
            }
            Target::Query(query) => {
                self.users.entry(query).or_default().insert(key, value);
            }
        }
    }
}

impl Registry for ServerRegistry {
    fn get_user(&self, target: &Query, key: Key) -> Option<&str> {
        self.users
            .get(target)
            .and_then(|r| r.get(key.to_str()))
            .map(String::as_str)
    }

    fn get_channel(&self, target: &Channel, key: Key) -> Option<&str> {
        self.channels
            .get(target)
            .and_then(|r| r.get(key.to_str()))
            .map(String::as_str)
    }
}

#[derive(Debug, Default)]
pub struct EmptyRegistry();
impl Registry for EmptyRegistry {
    fn get_user(&self, _target: &Query, _key: Key) -> Option<&str> {
        None
    }

    fn get_channel(&self, _target: &Channel, _key: Key) -> Option<&str> {
        None
    }
}

pub static EMPTY: &EmptyRegistry = &EmptyRegistry();

pub fn avatar_url(
    user: &User,
    registry: &dyn Registry,
    size: u16,
) -> Option<url::Url> {
    let query = Query::from(user);
    let avatar = registry.avatar(TargetRef::Query(&query))?;

    // Replace optional `{size}` in the avatar URL with the display size
    // https://ircv3.net/registry#user-metadata
    let sized_avatar = avatar.replace("{size}", &size.to_string());

    url::Url::parse(&sized_avatar).ok()
}
