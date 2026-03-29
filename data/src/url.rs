use std::str::FromStr;

use fancy_regex::Regex;
use percent_encoding::percent_decode_str;

use crate::appearance::theme;
use crate::server::ServerName;
use crate::{config, isupport};

#[derive(Debug, Clone)]
pub enum Url {
    ServerConnect {
        url: String,
        server: ServerName,
        config: config::Server,
    },
    Theme {
        url: String,
        styles: theme::Styles,
    },
    Unknown(String),
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Url::ServerConnect { url, .. }
                | Url::Theme { url, .. }
                | Url::Unknown(url) => url,
            }
        )
    }
}

pub fn theme(colors: &theme::Styles) -> String {
    format!("halloy:///theme?e={}", colors.encode_base64())
}

pub fn theme_submit(colors: &theme::Styles) -> String {
    format!(
        "https://themes.halloy.chat/submit?e={}",
        colors.encode_base64()
    )
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
            Ok(parse(url.clone()).unwrap_or(Url::Unknown(url.to_string())))
        } else {
            Err(())
        }
    }
}

fn parse(url: url::Url) -> Result<Url, Error> {
    match url.scheme().to_lowercase().as_str() {
        "irc" | "irc+insecure" | "ircs" => {
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

            let styles = theme::Styles::decode_base64(&encoded)?;

            Ok(Url::Theme {
                url: url.into(),
                styles,
            })
        }
        _ => Err(Error::Unknown),
    }
}

fn generate_server_name(host: &str) -> &str {
    let pattern = Regex::new(r"irc\.([^.]+)").unwrap();

    if let Ok(Some(captures)) = pattern.captures(host)
        && let Some(matched) = captures.get(1)
    {
        return matched.as_str();
    }

    host
}

fn parse_server_config(url: &url::Url) -> Option<config::Server> {
    let nickname = config::random_nickname();

    // Match on the host so IPv6 literals are stored without the square
    // brackets that `url::Host::to_string()` would include.
    let server = match url.host()? {
        url::Host::Ipv6(address) => address.to_string(),
        host => host.to_string(),
    };
    let port = url.port();
    let use_tls = match url.scheme().to_lowercase().as_str() {
        "irc" | "irc+insecure" => Some(false),
        "ircs" => Some(true),
        _ => None,
    }?;
    let channels = {
        let default_chantype =
            isupport::DEFAULT_CHANTYPES.first().copied().unwrap_or('#');

        let normalize_channel = |channel: &str| -> Option<String> {
            let channel = percent_decode_str(channel).decode_utf8_lossy();

            if channel.is_empty() {
                return None;
            }

            // URL parsing runs before we know the server's CHANTYPES, so fall
            // back to the default chantype set and prepend its first prefix
            // for bare targets.
            if channel.starts_with(isupport::DEFAULT_CHANTYPES) {
                Some(channel.into_owned())
            } else {
                Some(format!("{default_chantype}{channel}"))
            }
        };

        let mut channels = url.fragment().map_or(vec![], |fragment| {
            // Fragment starts with #. We consider them as channels.
            // Eg: [...]/#channel1,#channel2
            fragment
                .split(',')
                .filter_map(normalize_channel)
                .collect::<Vec<_>>()
        });

        if !url.path().is_empty() {
            // We also consider path as channels separated by ','.
            // Eg: [...]/channel1,channel2
            channels.extend(
                url.path()[1..]
                    .split(',')
                    .filter_map(normalize_channel)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_server_connect(
        input: &str,
        expected_server_name: &str,
        expected_host: &str,
        expected_port: u16,
        expected_channels: &[&str],
        expected_use_tls: bool,
    ) {
        let url = Url::from_str(input).unwrap();

        let Url::ServerConnect { server, config, .. } = url else {
            panic!("expected server connect URL");
        };

        assert_eq!(&*server, expected_server_name);
        assert_eq!(config.server, expected_host);
        assert_eq!(config.port, expected_port);
        assert_eq!(config.use_tls, expected_use_tls);
        assert_eq!(
            config.channels,
            expected_channels
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn parses_hostname_without_channels() {
        assert_server_connect(
            "irc://irc.libera.chat",
            "libera",
            "irc.libera.chat",
            6667,
            &[],
            false,
        );
    }

    #[test]
    fn parses_hostname_with_fragment_channels_and_explicit_tls_port() {
        assert_server_connect(
            "ircs://irc.libera.chat:7000/#halloy,#rust",
            "libera",
            "irc.libera.chat",
            7000,
            &["#halloy", "#rust"],
            true,
        );
    }

    #[test]
    fn parses_ipv4_host_with_path_channels() {
        assert_server_connect(
            "irc://127.0.0.1:6669/channel,%26local,%2Bops,!safe",
            "127.0.0.1",
            "127.0.0.1",
            6669,
            &["#channel", "&local", "#+ops", "#!safe"],
            false,
        );
    }

    #[test]
    fn parses_ipv6_host_without_channels() {
        assert_server_connect(
            "ircs://[2001:db8::1]",
            "2001:db8::1",
            "2001:db8::1",
            6697,
            &[],
            true,
        );
    }

    #[test]
    fn parse_server_config_strips_ipv6_brackets() {
        let url = url::Url::parse("irc://[2001:db8::1]/channel").unwrap();
        let config = parse_server_config(&url).unwrap();

        assert_eq!(config.server, "2001:db8::1");
        assert_eq!(config.port, 6667);
        assert_eq!(config.channels, vec!["#channel"]);
        assert!(!config.use_tls);
    }

    #[test]
    fn parse_server_config_decodes_percent_encoded_path_channels() {
        let url =
            url::Url::parse("irc://irc.example.org/%23foo%25bar,%26local")
                .unwrap();
        let config = parse_server_config(&url).unwrap();

        assert_eq!(config.channels, vec!["#foo%bar", "&local"]);
    }

    #[test]
    fn parse_server_config_decodes_percent_encoded_fragment_channels() {
        let url =
            url::Url::parse("irc://irc.example.org/#foo%25bar,%2Bops").unwrap();
        let config = parse_server_config(&url).unwrap();

        assert_eq!(config.channels, vec!["#foo%bar", "#+ops"]);
    }

    #[test]
    fn parses_channels_with_percent_encoded_special_characters() {
        assert_server_connect(
            "irc://irc.example.org/%23ops%5Btest%5D%7Bdev%7D%5Efoo,%23foo%25bar",
            "example",
            "irc.example.org",
            6667,
            &["#ops[test]{dev}^foo", "#foo%bar"],
            false,
        );
    }
}
