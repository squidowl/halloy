use super::*;
use std::collections::HashSet;

use chrono::{Duration, Utc};

use crate::history;
use crate::message;
use crate::reaction::Reaction;
use crate::time::Posix;
use crate::user::Nick;

fn search(raw: &str) -> Command {
    parse(Kind::Search, raw).expect("valid search")
}

fn query(raw: &str) -> Expr {
    search(raw).query.expect("query")
}

#[test]
fn parses_simple_selector() {
    assert_eq!(
        query("text=rust"),
        Expr::Predicate(Predicate {
            selector: Selector::Text,
            value: Value::String("rust".into()),
            modifiers: Modifiers::default(),
        })
    );
}

#[test]
fn quoted_selector_like_text_is_literal_text() {
    assert_eq!(
        query("\"origin=alice\""),
        Expr::Predicate(Predicate {
            selector: Selector::Text,
            value: Value::String("origin=alice".into()),
            modifiers: Modifiers::default(),
        })
    );

    assert_eq!(
        query("origin=\"alice\""),
        Expr::Predicate(Predicate {
            selector: Selector::Origin,
            value: Value::String("alice".into()),
            modifiers: Modifiers::default(),
        })
    );
}

#[test]
fn quoted_values_support_quote_and_backslash_escapes() {
    assert_eq!(
        query("text=\"say \\\"hello\\\"\""),
        Expr::Predicate(Predicate {
            selector: Selector::Text,
            value: Value::String("say \"hello\"".into()),
            modifiers: Modifiers::default(),
        })
    );

    assert_eq!(
        query("\"path \\\\tmp\""),
        Expr::Predicate(Predicate {
            selector: Selector::Text,
            value: Value::String("path \\tmp".into()),
            modifiers: Modifiers::default(),
        })
    );
}

#[test]
fn parses_parenthesized_boolean_expression() {
    assert!(matches!(
        query("origin=alice AND (text=rust OR text=python)"),
        Expr::And(parts)
            if parts.len() == 2 && matches!(parts[1], Expr::Or(_))
    ));
}

#[test]
fn parses_not_keyword() {
    assert!(matches!(
        query("NOT reaction=love"),
        Expr::Not(expr)
            if matches!(*expr, Expr::Predicate(Predicate {
                selector: Selector::Reaction,
                ..
            }))
    ));
}

#[test]
fn normalizes_react_alias() {
    assert!(matches!(
        query("react=love"),
        Expr::Predicate(Predicate {
            selector: Selector::Reaction,
            ..
        })
    ));
}

#[test]
fn parses_string_modifiers() {
    assert_eq!(
        query("text=ix\"timeout.*socket\""),
        Expr::Predicate(Predicate {
            selector: Selector::Text,
            value: Value::String("timeout.*socket".into()),
            modifiers: Modifiers {
                case_insensitive: true,
                regex: true,
                ..Modifiers::default()
            },
        })
    );
}

#[test]
fn parses_textonly_output_option() {
    let command = search("--textonly text=hello");

    assert!(command.output.text_only);
    assert!(command.query.is_some());
}

#[test]
fn parses_notimestamp_output_option() {
    let command = search("--notimestamp text=hello");

    assert!(command.output.no_timestamp);
    assert!(command.query.is_some());
}

#[test]
fn parses_other_output_option() {
    let command = search("--other text=hello");

    assert!(command.output.other);
    assert!(command.query.is_some());
}

#[test]
fn parses_context_output_option() {
    let command = search("context=2 text=hello");

    assert_eq!(command.output.context, 2);
    assert!(command.query.is_some());
}

#[test]
fn rejects_invalid_context_option() {
    assert!(matches!(
        parse(Kind::Search, "context=abc text=hello"),
        Err(Error::InvalidContext)
    ));
}

#[test]
fn defaults_to_inline_view() {
    let command = search("text=hello");

    assert_eq!(command.output.view, View::Inline);
}

#[test]
fn parses_result_view_option() {
    let command = search("view=pane text=hello");

    assert_eq!(command.output.view, View::Pane);
    assert!(command.query.is_some());
}

#[test]
fn rejects_invalid_result_view() {
    assert!(matches!(
        parse(Kind::Search, "view=sidebar text=hello"),
        Err(Error::InvalidView)
    ));
}

#[test]
fn last_uses_current_buffer_default_span() {
    let command = parse(Kind::Last, "text=hello").expect("valid last");

    assert_eq!(command.kind, Kind::Last);
    assert_eq!(command.default_span, DefaultSpan::CurrentBuffer);
}

#[test]
fn adjacent_predicates_default_to_and() {
    assert!(matches!(
        query("origin=alice text=python"),
        Expr::And(parts) if parts.len() == 2
    ));
}

#[test]
fn rejects_invalid_regex() {
    assert!(matches!(
        parse(Kind::Search, "regex=\"(\""),
        Err(Error::InvalidRegex(_))
    ));
}

#[test]
fn finds_visible_messages_with_boolean_expression() {
    let messages = vec![
        message("Rust socket timeout", vec![]),
        message("Python socket timeout", vec![]),
        message("Rust parser", vec![]),
    ];
    let view = view(&messages);
    let command = search("itext=rust AND text=timeout");

    let matches = find(&command, &view, ExecutionContext::default())
        .expect("search matches");

    assert_eq!(matches.total, 1);
    assert_eq!(matches.messages[0].text(), "Rust socket timeout");
}

#[test]
fn finds_messages_with_case_insensitive_regex() {
    let messages = vec![message("Socket timed out", vec![])];
    let view = view(&messages);
    let command = search("text=ix\"socket.*OUT\"");

    let matches = find(&command, &view, ExecutionContext::default())
        .expect("search matches");

    assert_eq!(matches.total, 1);
}

#[test]
fn finds_messages_by_reaction_alias() {
    let messages = vec![message("ship it", vec![reaction("love")])];
    let view = view(&messages);
    let command = search("react=love");

    let matches = find(&command, &view, ExecutionContext::default())
        .expect("search matches");

    assert_eq!(matches.total, 1);
}

#[test]
fn other_excludes_own_utterances() {
    let messages = vec![
        message_with_flags("from me", message::Direction::Sent, false, vec![]),
        message_with_flags(
            "echo from me",
            message::Direction::Received,
            true,
            vec![],
        ),
        message("from someone else", vec![]),
    ];
    let view = view(&messages);
    let command = search("--other itext=from");

    let matches = find(&command, &view, ExecutionContext::default())
        .expect("search matches");

    assert_eq!(matches.total, 1);
    assert_eq!(matches.messages[0].text(), "from someone else");
}

#[test]
fn other_excludes_received_messages_from_own_nick() {
    let own_nick = Nick::from_str("Roey", crate::isupport::CaseMap::default());
    let messages = vec![
        user_message("from me in history", own_nick.clone()),
        user_message(
            "from someone else",
            Nick::from_str("Alice", crate::isupport::CaseMap::default()),
        ),
    ];
    let view = view(&messages);
    let command = search("--other itext=from");

    let matches = find(
        &command,
        &view,
        ExecutionContext {
            own_nick: Some(own_nick.as_nickref()),
        },
    )
    .expect("search matches");

    assert_eq!(matches.total, 1);
    assert_eq!(matches.messages[0].text(), "from someone else");
}

#[test]
fn context_includes_neighboring_loaded_lines() {
    let messages = vec![
        message("before", vec![]),
        message("needle", vec![]),
        message("after", vec![]),
    ];
    let view = view(&messages);
    let command = search("context=1 text=needle");

    let matches = find(&command, &view, ExecutionContext::default())
        .expect("search matches");

    assert_eq!(matches.total, 1);
    assert_eq!(matches.messages.len(), 3);
    assert_eq!(matches.messages[0].text(), "before");
    assert_eq!(matches.messages[1].text(), "needle");
    assert_eq!(matches.messages[2].text(), "after");
}

#[test]
fn parses_supported_span_units() {
    assert!(parse(Kind::Search, "span=3d").is_ok());
    assert!(parse(Kind::Search, "span=2h").is_ok());
    assert!(parse(Kind::Search, "span=5m").is_ok());
}

#[test]
fn rejects_invalid_span_values() {
    assert!(matches!(
        parse(Kind::Search, "span=3w"),
        Err(Error::InvalidSpan)
    ));
    assert!(matches!(
        parse(Kind::Search, "span=0h"),
        Err(Error::InvalidSpan)
    ));
    assert!(matches!(
        parse(Kind::Search, "span=abc"),
        Err(Error::InvalidSpan)
    ));
}

#[test]
fn filters_messages_by_span() {
    let messages = vec![
        message_at("recent", Utc::now() - Duration::minutes(30), vec![]),
        message_at("old", Utc::now() - Duration::hours(3), vec![]),
    ];
    let view = view(&messages);
    let command = search("span=2h");

    let matches = find(&command, &view, ExecutionContext::default())
        .expect("search matches");

    assert_eq!(matches.total, 1);
    assert_eq!(matches.messages[0].text(), "recent");
}

#[test]
fn formats_textonly_without_control_codes() {
    let server_time =
        chrono::DateTime::parse_from_rfc3339("2026-06-16T16:01:17.068Z")
            .expect("timestamp")
            .with_timezone(&Utc);
    let message = message_at(
        "\u{2}bold\u{2} \u{3}04red \u{1b}[31mansi",
        server_time,
        vec![],
    );

    assert_eq!(
        format_match(
            &message,
            Output {
                text_only: true,
                ..Output::default()
            }
        ),
        "[2026-06-16 16:01:17 UTC] <status> bold red ansi"
    );
}

#[test]
fn formats_match_timestamps_to_seconds() {
    let server_time =
        chrono::DateTime::parse_from_rfc3339("2026-06-16T16:01:17.068Z")
            .expect("timestamp")
            .with_timezone(&Utc);
    let message = message_at("hello", server_time, vec![]);

    assert_eq!(
        format_match(&message, Output::default()),
        "[2026-06-16 16:01:17 UTC] <status> hello"
    );
}

#[test]
fn formats_match_without_timestamp() {
    let server_time =
        chrono::DateTime::parse_from_rfc3339("2026-06-16T16:01:17.068Z")
            .expect("timestamp")
            .with_timezone(&Utc);
    let message = message_at("hello", server_time, vec![]);

    assert_eq!(
        format_match(
            &message,
            Output {
                no_timestamp: true,
                ..Output::default()
            }
        ),
        "<status> hello"
    );
}

#[test]
fn display_match_highlights_text_predicates() {
    let message = message("hello world hello", vec![]);
    let command = search("text=hello");
    let display =
        format_display_match(&message, Output::default(), Some(&command));

    assert_eq!(display.text_highlights, vec![0..5, 12..17]);
}

#[test]
fn display_match_highlights_case_insensitive_text_predicates() {
    let message = message("Hello world", vec![]);
    let command = search("itext=hello");
    let display =
        format_display_match(&message, Output::default(), Some(&command));

    assert_eq!(display.text_highlights, vec![0..5]);
}

#[test]
fn display_match_highlights_regex_text_predicates() {
    let message = message("socket timeout", vec![]);
    let command = search("regex=\"t[a-z]+out\"");
    let display =
        format_display_match(&message, Output::default(), Some(&command));

    assert_eq!(display.text_highlights, vec![7..14]);
}

#[test]
fn display_match_does_not_highlight_negated_text_predicates() {
    let message = message("hello world", vec![]);
    let command = search("NOT text=missing");
    let display =
        format_display_match(&message, Output::default(), Some(&command));

    assert!(display.text_highlights.is_empty());
}

fn view(messages: &[message::Message]) -> history::View<'_> {
    history::View {
        total: messages.len(),
        has_more_older_messages: false,
        has_more_newer_messages: false,
        old_messages: vec![],
        new_messages: messages.iter().collect(),
        cleared: false,
    }
}

fn message(text: &str, reactions: Vec<Reaction>) -> message::Message {
    message_at(text, Utc::now(), reactions)
}

fn message_at(
    text: &str,
    server_time: chrono::DateTime<Utc>,
    reactions: Vec<Reaction>,
) -> message::Message {
    message_at_with_flags(
        text,
        server_time,
        message::Direction::Received,
        false,
        reactions,
    )
}

fn message_with_flags(
    text: &str,
    direction: message::Direction,
    is_echo: bool,
    reactions: Vec<Reaction>,
) -> message::Message {
    message_at_with_flags(text, Utc::now(), direction, is_echo, reactions)
}

fn message_at_with_flags(
    text: &str,
    server_time: chrono::DateTime<Utc>,
    direction: message::Direction,
    is_echo: bool,
    reactions: Vec<Reaction>,
) -> message::Message {
    let received_at = Posix::now();
    let content = message::plain(text.to_string());
    let hash = message::Hash::new(&server_time, &content, &received_at);

    message::Message {
        received_at,
        server_time,
        direction,
        target: message::Target::Server {
            source: message::Source::Internal(
                message::source::Internal::Status(
                    message::source::Status::Success,
                ),
            ),
        },
        content,
        id: None,
        reply_to: None,
        reply_preview: None,
        hash,
        hidden_urls: HashSet::default(),
        is_echo,
        received_with_server_time: false,
        blocked: false,
        condensed: None,
        expanded: false,
        command: None,
        reactions,
        rerouted_from: None,
        deduplicate: false,
        redaction: None,
    }
}

fn user_message(text: &str, nick: Nick) -> message::Message {
    let received_at = Posix::now();
    let server_time = Utc::now();
    let content = message::plain(text.to_string());
    let hash = message::Hash::new(&server_time, &content, &received_at);

    message::Message {
        received_at,
        server_time,
        direction: message::Direction::Received,
        target: message::Target::Server {
            source: message::Source::User(crate::User::from(nick)),
        },
        content,
        id: None,
        reply_to: None,
        reply_preview: None,
        hash,
        hidden_urls: HashSet::default(),
        is_echo: false,
        received_with_server_time: false,
        blocked: false,
        condensed: None,
        expanded: false,
        command: None,
        reactions: vec![],
        rerouted_from: None,
        deduplicate: false,
        redaction: None,
    }
}

fn reaction(text: &str) -> Reaction {
    Reaction {
        sender: Nick::from_str("tester", crate::isupport::CaseMap::default()),
        text: text.to_string(),
        unreact: false,
        id: None,
        server_time: Utc::now(),
    }
}
