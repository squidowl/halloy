use std::str::FromStr;

use log::warn;
use regex::Regex;

use crate::{config, theme, Server};

#[derive(Debug, Clone)]
pub enum Url {
    ServerConnect {
        url: String,
        server: Server,
        config: config::Server,
    },
    Theme {
        url: String,
        colors: theme::Colors,
    },
    Unknown(String),
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Url::ServerConnect { url, .. } | Url::Theme { url, .. } | Url::Unknown(url) => url,
            }
        )
    }
}

pub fn theme(colors: &theme::Colors) -> String {
    format!("halloy:///theme?e={}", colors.encode_base64())
}

impl Url {
    pub fn find_in(mut args: impl Iterator<Item = String>) -> Option<Self> {
        args.find_map(|arg| arg.parse().ok())
    }
}

impl FromStr for Url {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = s.parse::<url::Url>().map_err(|_| ())?;

        if ["irc", "ircs", "halloy"].contains(&url.scheme()) {
            Ok(parse(url.clone())
                .inspect_err(|err| warn!("Failed to parse url {url}: {err}"))
                .unwrap_or(Url::Unknown(url.to_string())))
        } else {
            Err(())
        }
    }
}

fn parse(url: url::Url) -> Result<Url, Error> {
    match url.scheme().to_lowercase().as_str() {
        "irc" | "ircs" => {
            let config = parse_server_config(&url).ok_or(Error::ParseServer)?;
            let server = generate_server_name(config.server.as_str());
            let url = url.into();

            Ok(Url::ServerConnect {
                url,
                server: server.into(),
                config,
            })
        }
        "halloy" if url.path() == "/theme" => {
            let (_, encoded) = url
                .query_pairs()
                .find(|(key, _)| key == "e")
                .ok_or(Error::MissingQueryPair)?;

            let colors = theme::Colors::decode_base64(&encoded)?;

            Ok(Url::Theme {
                url: url.into(),
                colors,
            })
        }
        _ => Err(Error::Unknown),
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ParseUrl(#[from] url::ParseError),
    #[error("can't convert url to a valid server")]
    ParseServer,
    #[error("unknown route")]
    Unknown,
    #[error("missing query pair")]
    MissingQueryPair,
    #[error("failed to parse encoded theme: {0}")]
    ParseEncodedTheme(#[from] theme::Error),
}
