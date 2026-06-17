use chrono::{DateTime, Duration, Utc};
use fancy_regex::Regex;

use super::{
    Command, Error, Expr, Join, Modifiers, Predicate, Selector, Value,
};
use crate::user::NickRef;

const MAX_RESULTS: usize = 50;

#[derive(Debug)]
pub struct Matches<'a> {
    pub messages: Vec<&'a crate::Message>,
    pub total: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionContext<'a> {
    pub own_nick: Option<NickRef<'a>>,
}

/// Evaluates a parsed search command against Halloy's loaded in-memory history.
///
/// Used by the input view command dispatcher for `/search` and `/last`.
/// `command` supplies the parsed query and output controls, `view` supplies the
/// currently loaded visible history window, and `context` supplies local client
/// details such as the current nick for `--other`. Returns matching messages,
/// the count of actual matches before context-line expansion, and whether the
/// display cap truncated output. Produces no network, filesystem, or
/// persistence side effects.
pub fn find<'a>(
    command: &Command,
    view: &'a crate::history::View<'a>,
    context: ExecutionContext<'_>,
) -> Result<Matches<'a>, Error> {
    // Search only the already-visible in-memory view. This first execution
    // slice avoids filesystem scans, network access, and persistence.
    let now = Utc::now();
    let visible = view
        .old_messages
        .iter()
        .chain(view.new_messages.iter())
        .copied()
        .collect::<Vec<_>>();
    let matching = visible
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
            if command.output.other && is_own_utterance(message, context) {
                return None;
            }

            command
                .query
                .as_ref()
                .is_none_or(|expr| matches_expr(expr, message, now))
                .then_some(index)
        })
        .collect::<Vec<_>>();

    let total = matching.len();
    let mut messages = Vec::new();
    let mut last_included = None;
    let mut truncated = false;

    for index in matching {
        let start = index.saturating_sub(command.output.context);
        let end = (index + command.output.context + 1).min(visible.len());

        for (include, message) in
            visible.iter().enumerate().take(end).skip(start)
        {
            if last_included.is_some_and(|last| include <= last) {
                continue;
            }

            if messages.len() < MAX_RESULTS {
                messages.push(*message);
            } else {
                truncated = true;
            }

            last_included = Some(include);
        }
    }

    Ok(Matches {
        truncated,
        messages,
        total,
    })
}

/// Determines whether a message should be excluded by `--other`.
///
/// Used only by `find`. `message` is the candidate history message and
/// `context` contains the optional local nick for received echo detection.
/// Returns true for sent messages, echo messages, or received messages whose
/// source nick matches the local nick. Produces no output or side effects.
fn is_own_utterance(
    message: &crate::Message,
    context: ExecutionContext<'_>,
) -> bool {
    matches!(message.direction, crate::message::Direction::Sent)
        || message.is_echo
        || context.own_nick.is_some_and(|own_nick| {
            message
                .user()
                .is_some_and(|user| user.nickname() == own_nick)
        })
}

/// Recursively evaluates a boolean search expression against one message.
///
/// Used by `find` for each candidate message. `expr` is the parsed query tree,
/// `message` is the candidate, and `now` anchors relative `span=` predicates.
/// Returns true when the full expression matches. Produces no output or side
/// effects.
fn matches_expr(
    expr: &Expr,
    message: &crate::Message,
    now: DateTime<Utc>,
) -> bool {
    match expr {
        Expr::Predicate(predicate) => {
            matches_predicate(predicate, message, now)
        }
        Expr::Not(expr) => !matches_expr(expr, message, now),
        Expr::And(exprs) => {
            exprs.iter().all(|expr| matches_expr(expr, message, now))
        }
        Expr::Or(exprs) => {
            exprs.iter().any(|expr| matches_expr(expr, message, now))
        }
    }
}

/// Evaluates a single selector predicate against one message.
///
/// Used by `matches_expr`. `predicate` contains the selector, value, and
/// modifiers; `message` is the candidate; `now` anchors relative span checks.
/// Returns true when the predicate matches after applying local negation.
/// Produces no output or side effects.
fn matches_predicate(
    predicate: &Predicate,
    message: &crate::Message,
    now: DateTime<Utc>,
) -> bool {
    // Selector matching stays intentionally local and side-effect free. That
    // makes the security boundary easy to audit: no selector can trigger I/O,
    // command execution, or external expansion.
    let matched = match predicate.selector {
        Selector::Text => {
            let haystack = message.text();
            value_matches(&haystack, &predicate.value, predicate.modifiers)
        }
        Selector::Origin => message.user().is_some_and(|user| {
            value_matches(
                user.nickname().as_str(),
                &predicate.value,
                predicate.modifiers,
            )
        }),
        Selector::Target => message.target.raw().is_some_and(|target| {
            value_matches(target, &predicate.value, predicate.modifiers)
        }),
        Selector::Type => {
            let kind = match message.target.source() {
                crate::message::Source::User(_) => "message",
                crate::message::Source::Action(_) => "action",
                crate::message::Source::Server(_) => "server",
                crate::message::Source::Internal(_) => "internal",
            };

            value_matches(kind, &predicate.value, predicate.modifiers)
        }
        Selector::Reaction => message.reactions.iter().any(|reaction| {
            !reaction.unreact
                && value_matches(
                    &reaction.text,
                    &predicate.value,
                    predicate.modifiers,
                )
        }),
        Selector::Span => predicate.value.iter().any(|value| {
            parse_span(value)
                .is_ok_and(|duration| message.server_time >= now - duration)
        }),
    };

    if predicate.modifiers.negated {
        !matched
    } else {
        matched
    }
}

/// Parses the `span=` duration grammar.
///
/// Used by parser validation and runtime span evaluation. `value` is expected
/// to be `Nd`, `Nh`, or `Nm` with a positive integer prefix. Returns a chrono
/// duration or `Error::InvalidSpan`. Produces no output or side effects.
pub(super) fn parse_span(value: &str) -> Result<Duration, Error> {
    let (amount, unit) = value.split_at(value.len().saturating_sub(1));
    let amount = amount.parse::<i64>().map_err(|_| Error::InvalidSpan)?;

    if amount <= 0 {
        return Err(Error::InvalidSpan);
    }

    match unit {
        "d" => Ok(Duration::days(amount)),
        "h" => Ok(Duration::hours(amount)),
        "m" => Ok(Duration::minutes(amount)),
        _ => Err(Error::InvalidSpan),
    }
}

/// Applies a possibly-list-valued predicate value to a text haystack.
///
/// Used by selector evaluation. `haystack` is the field being searched,
/// `value` is either a single string or comma-list, and `modifiers` controls
/// AND/OR, regex, and case sensitivity. Returns true when the configured list
/// semantics match. Produces no output or side effects.
fn value_matches(haystack: &str, value: &Value, modifiers: Modifiers) -> bool {
    let matches = |needle| string_matches(haystack, needle, modifiers);

    match modifiers.join {
        Some(Join::And) => value.iter().all(matches),
        Some(Join::Or) | None => value.iter().any(matches),
    }
}

/// Matches one string or regex needle against one haystack.
///
/// Used by `value_matches`. `haystack` is the message field, `needle` is the
/// literal or regex pattern, and `modifiers` selects regex and
/// case-insensitive behavior. Returns true on match; invalid runtime regex
/// behavior is treated as a non-match. Produces no output or side effects.
fn string_matches(haystack: &str, needle: &str, modifiers: Modifiers) -> bool {
    if modifiers.regex {
        // Regexes are validated during parsing, then recompiled here for
        // execution. fancy-regex can report runtime errors for advanced
        // features, so runtime errors are treated as non-matches.
        let pattern = if modifiers.case_insensitive {
            format!("(?i:{needle})")
        } else {
            needle.to_string()
        };

        return Regex::new(&pattern)
            .ok()
            .and_then(|regex| regex.is_match(haystack).ok())
            .unwrap_or(false);
    }

    if modifiers.case_insensitive {
        haystack.to_lowercase().contains(&needle.to_lowercase())
    } else {
        haystack.contains(needle)
    }
}
