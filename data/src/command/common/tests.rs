use super::*;

#[test]
fn parse_defaults_to_global_scope() {
    assert_eq!(
        parse("").expect("common command"),
        Command {
            scope: Scope::Global,
        }
    );
}

#[test]
fn parse_network_scope() {
    assert_eq!(
        parse("scope=network").expect("common command"),
        Command {
            scope: Scope::Network,
        }
    );
}

#[test]
fn parse_global_scope() {
    assert_eq!(
        parse("scope=global").expect("common command"),
        Command {
            scope: Scope::Global,
        }
    );
}

#[test]
fn parse_rejects_invalid_scope() {
    assert_eq!(parse("scope=all"), Err(Error::InvalidScope("all".into())));
}

#[test]
fn summarize_sorts_users_and_channels() {
    let entries = summarize([
        Entry {
            nick: "zara".into(),
            channels: vec!["#rust".into(), "#halloy".into()],
        },
        Entry {
            nick: "alice".into(),
            channels: vec!["#linux".into()],
        },
    ]);

    assert_eq!(
        entries,
        vec![
            Entry {
                nick: "alice".into(),
                channels: vec!["#linux".into()],
            },
            Entry {
                nick: "zara".into(),
                channels: vec!["#halloy".into(), "#rust".into()],
            },
        ]
    );
}

#[test]
fn summarize_omits_users_without_common_channels() {
    let entries = summarize([
        Entry {
            nick: "alice".into(),
            channels: vec![],
        },
        Entry {
            nick: "bob".into(),
            channels: vec!["#halloy".into()],
        },
    ]);

    assert_eq!(
        entries,
        vec![Entry {
            nick: "bob".into(),
            channels: vec!["#halloy".into()],
        }]
    );
}

#[test]
fn summarize_deduplicates_channels() {
    let entries = summarize([Entry {
        nick: "alice".into(),
        channels: vec!["#halloy".into(), "#halloy".into()],
    }]);

    assert_eq!(
        entries,
        vec![Entry {
            nick: "alice".into(),
            channels: vec!["#halloy".into()],
        }]
    );
}
