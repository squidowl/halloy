use crate::config;

#[derive(Debug, Clone)]
pub enum Url {
    ServerConnect {
        url: String,
        server: String,
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
    pub fn parse(url: &str) -> Result<Self, Error> {
        let url = url::Url::parse(url)?;

        match url.scheme().to_lowercase().as_str() {
            "irc" | "ircs" => {
                let config = parse_server_config(&url).ok_or(Error::ParseServer)?;
                let server = generate_server_name(config.server.as_str());
                let url = url.into();

                Ok(Self::ServerConnect {
                    url,
                    server,
                    config,
                })
            }
            _ => Err(Error::Unsupported),
        }
    }

    pub fn find_in(mut args: impl Iterator<Item = String>) -> Option<Self> {
        args.find_map(|arg| Self::parse(&arg).ok())
    }
}

fn generate_server_name(host: &str) -> String {
    let pattern = regex::Regex::new(r"irc\.([^.]+)").unwrap();

    if let Some(captures) = pattern.captures(host) {
        if let Some(matched) = captures.get(1) {
            return matched.as_str().to_string();
        }
    }

    host.to_string()
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

// #[cfg(test)]
// mod tests {
//     use rand::prelude::*;
//     use rand_chacha::ChaCha8Rng;

//     use super::*;

//     fn nickname() -> String {
//         let mut rng = ChaCha8Rng::seed_from_u64(1337);
//         data::config::random_nickname_with_seed(&mut rng)
//     }

//     #[test]
//     fn with_port() {
//         let url = "irc://irc.libera.chat:6667";
//         let route = Route::parse(url).unwrap();
//         let nickname = nickname();

//         let ref_server = Server::new(
//             String::from("irc.libera.chat"),
//             Some(6667),
//             nickname.clone(),
//             vec![],
//             false,
//         );

//         let mut server = route.server;
//         server.nickname = nickname;

//         assert_eq!(ref_server, server);
//         assert_eq!(url, route.raw);
//     }

//     #[test]
//     fn with_channels_using_fragment() {
//         let url = "irc://irc.libera.chat/#hello,#world";
//         let route = Route::parse(url).unwrap();
//         let nickname = nickname();

//         let ref_server = Server::new(
//             String::from("irc.libera.chat"),
//             Some(6667),
//             nickname.clone(),
//             vec![String::from("#hello"), String::from("#world")],
//             false,
//         );

//         let mut server = route.server;
//         server.nickname = nickname;

//         assert_eq!(ref_server, server);
//         assert_eq!(url, route.raw);
//     }

//     #[test]
//     fn with_channels_using_path() {
//         let url = "irc://irc.libera.chat/hello";
//         let route = Route::parse(url).unwrap();
//         let nickname = nickname();

//         let ref_server = Server::new(
//             String::from("irc.libera.chat"),
//             Some(6667),
//             nickname.clone(),
//             vec![String::from("#hello")],
//             false,
//         );

//         let mut server = route.server;
//         server.nickname = nickname;

//         assert_eq!(ref_server, server);
//         assert_eq!(url, route.raw);
//     }

//     #[test]
//     fn with_channels_using_mixed_fragment_path() {
//         let url = "irc://irc.libera.chat/halloy,#world,boo";
//         let route = Route::parse(url).unwrap();
//         let nickname = nickname();

//         let ref_server = Server::new(
//             String::from("irc.libera.chat"),
//             Some(6667),
//             nickname.clone(),
//             vec![
//                 String::from("#world"),
//                 String::from("#boo"),
//                 String::from("#halloy"),
//             ],
//             false,
//         );

//         let mut server = route.server;
//         server.nickname = nickname;

//         assert_eq!(ref_server, server);
//         assert_eq!(url, route.raw);
//     }

//     #[test]
//     fn with_tls() {
//         let url = "ircs://irc.libera.chat:6667";
//         let route = Route::parse(url).unwrap();
//         let nickname = nickname();

//         let ref_server = Server::new(
//             String::from("irc.libera.chat"),
//             Some(6667),
//             nickname.clone(),
//             vec![],
//             true,
//         );

//         let mut server = route.server;
//         server.nickname = nickname;

//         assert_eq!(ref_server, server);
//         assert_eq!(url, route.raw);
//     }
// }
