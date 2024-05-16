use std::str::FromStr;

use regex::Regex;

use crate::{config, Server};

#[derive(Debug, Clone)]
pub enum Url {
    ServerConnect {
        url: String,
        server: Server,
        config: config::Server,
    },
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Url::ServerConnect { url, .. } => url,
            }
        )
    }
}

impl Url {
    pub fn find_in(mut args: impl Iterator<Item = String>) -> Option<Self> {
        args.find_map(|arg| arg.parse().ok())
    }
}

impl FromStr for Url {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = s.parse::<url::Url>()?;

        match url.scheme().to_lowercase().as_str() {
            "irc" | "ircs" => {
                let config = parse_server_config(&url).ok_or(Error::ParseServer)?;
                let server = generate_server_name(config.server.as_str());
                let url = url.into();

                Ok(Self::ServerConnect {
                    url,
                    server: server.into(),
                    config,
                })
            }
            _ => Err(Error::Unsupported),
        }
    }
}

fn generate_server_name(host: &str) -> &str {
    let pattern = Regex::new(r"irc\.([^.]+)").unwrap();

    if let Some(captures) = pattern.captures(host) {
        if let Some(matched) = captures.get(1) {
            return matched.as_str();
        }
    }

    host
}

fn parse_server_config(url: &url::Url) -> Option<config::Server> {
    let nickname = config::random_nickname();
    let server = url.host()?.to_string();
    let port = url.port();
    let use_tls = match url.scheme().to_lowercase().as_str() {
        "irc" => Some(false),
        "ircs" => Some(true),
        _ => None,
    }?;
    let channels = {
        let add_hashtag_if_needed = |channel: &str| -> Option<String> {
            if channel.is_empty() {
                return None;
            }

            if channel.starts_with('#') {
                Some(channel.to_string())
            } else {
                Some(format!("#{}", channel))
            }
        };

        let mut channels = url.fragment().map_or(vec![], |fragment| {
            // Fragment starts with #. We consider them as channels.
            // Eg: [...]/#channel1,#channel2
            fragment
                .split(',')
                .filter_map(add_hashtag_if_needed)
                .collect::<Vec<_>>()
        });

        if !url.path().is_empty() {
            // We also consider path as channels seperated by ','.
            // Eg: [...]/channel1,channel2
            channels.extend(
                url.path()[1..]
                    .split(',')
                    .filter_map(add_hashtag_if_needed)
                    .collect::<Vec<_>>(),
            );
        }

        channels
    };

    Some(config::Server::new(
        server, port, nickname, channels, use_tls,
    ))
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error(transparent)]
    ParseUrl(#[from] url::ParseError),
    #[error("can't convert url to a valid server")]
    ParseServer,
    #[error("unsupported route")]
    Unsupported,
}
