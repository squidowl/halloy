use data::config::Server;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Route {
    pub raw: String,
    pub server: Server,
}

impl Route {
    pub fn parse(url: &str) -> Option<Self> {
        let url = url::Url::parse(url).ok()?;

        let nickname = data::config::random_nickname();
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

        Some(Route {
            server: Server::new(server, port, nickname, channels, use_tls),
            raw: url.to_string(),
        })
    }

    pub fn find_in(mut args: impl Iterator<Item = String>) -> Option<Self> {
        args.find_map(|arg| Self::parse(&arg))
    }
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;
    use rand_chacha::ChaCha8Rng;

    use super::*;

    fn nickname() -> String {
        let mut rng = ChaCha8Rng::seed_from_u64(1337);
        data::config::random_nickname_with_seed(&mut rng)
    }

    #[test]
    fn with_port() {
        let url = "irc://irc.libera.chat:6667";
        let route = Route::parse(url).unwrap();
        let nickname = nickname();

        let ref_server = Server::new(
            String::from("irc.libera.chat"),
            Some(6667),
            nickname.clone(),
            vec![],
            false,
        );

        let mut server = route.server;
        server.nickname = nickname;

        assert_eq!(ref_server, server);
        assert_eq!(url, route.raw);
    }

    #[test]
    fn with_channels_using_fragment() {
        let url = "irc://irc.libera.chat/#hello,#world";
        let route = Route::parse(url).unwrap();
        let nickname = nickname();

        let ref_server = Server::new(
            String::from("irc.libera.chat"),
            Some(6667),
            nickname.clone(),
            vec![String::from("#hello"), String::from("#world")],
            false,
        );

        let mut server = route.server;
        server.nickname = nickname;

        assert_eq!(ref_server, server);
        assert_eq!(url, route.raw);
    }

    #[test]
    fn with_channels_using_path() {
        let url = "irc://irc.libera.chat/hello";
        let route = Route::parse(url).unwrap();
        let nickname = nickname();

        let ref_server = Server::new(
            String::from("irc.libera.chat"),
            Some(6667),
            nickname.clone(),
            vec![String::from("#hello")],
            false,
        );

        let mut server = route.server;
        server.nickname = nickname;

        assert_eq!(ref_server, server);
        assert_eq!(url, route.raw);
    }

    #[test]
    fn with_channels_using_mixed_fragment_path() {
        let url = "irc://irc.libera.chat/halloy,#world,boo";
        let route = Route::parse(url).unwrap();
        let nickname = nickname();

        let ref_server = Server::new(
            String::from("irc.libera.chat"),
            Some(6667),
            nickname.clone(),
            vec![
                String::from("#world"),
                String::from("#boo"),
                String::from("#halloy"),
            ],
            false,
        );

        let mut server = route.server;
        server.nickname = nickname;

        assert_eq!(ref_server, server);
        assert_eq!(url, route.raw);
    }

    #[test]
    fn with_tls() {
        let url = "ircs://irc.libera.chat:6667";
        let route = Route::parse(url).unwrap();
        let nickname = nickname();

        let ref_server = Server::new(
            String::from("irc.libera.chat"),
            Some(6667),
            nickname.clone(),
            vec![],
            true,
        );

        let mut server = route.server;
        server.nickname = nickname;

        assert_eq!(ref_server, server);
        assert_eq!(url, route.raw);
    }
}
