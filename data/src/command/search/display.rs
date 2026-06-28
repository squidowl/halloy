use std::ops::Range;

use chrono::{DateTime, Utc};
use fancy_regex::Regex;

use super::{Command, Expr, Modifiers, Output, Selector};

#[derive(Debug, Clone)]
pub struct DisplayMatch {
    pub prefix: String,
    pub text: String,
    pub text_highlights: Vec<Range<usize>>,
}

/// Formats one search result as a plain line for inline status output.
///
/// Used by the input view when `view=inline`. `message` is the matched Halloy
/// message and `output` carries display flags such as `--textonly` and
/// `--notimestamp`. Returns the complete rendered line. Produces no output or
/// side effects.
pub fn format_match(message: &crate::Message, output: Output) -> String {
    // Build the structured display form first so inline and pane output share
    // one timestamp/text-stripping path.
    let display = format_display_match(message, output, None);

    format!("{}{}", display.prefix, display.text)
}

/// Formats one search result as structured text plus highlight ranges.
///
/// Used by the search result pane renderer. `message` is the matched Halloy
/// message, `output` carries display flags, and `command` optionally supplies
/// the query whose positive text predicates should be highlighted. Returns the
/// prefix, body text, and body-text highlight byte ranges. Produces no output
/// or side effects.
pub fn format_display_match(
    message: &crate::Message,
    output: Output,
    command: Option<&Command>,
) -> DisplayMatch {
    // Status/internal messages do not have a user source, so display them with
    // the same `status` pseudo-origin Halloy already uses elsewhere.
    let origin = message.user().map_or_else(
        || "status".to_string(),
        |user| user.nickname().to_string(),
    );

    // `--textonly` is an output transform, not a search transform: match
    // semantics stay tied to Halloy's stored message content, while display
    // strips IRC and ANSI control bytes from the rendered result line.
    let text = if output.text_only {
        strip_control_codes(&message.text())
    } else {
        message.text().into_owned()
    };

    // Highlighting is deliberately tied to display text, so `--textonly`
    // cannot produce byte ranges that point into stripped control bytes.
    let text_highlights = command
        .map(|command| highlight_ranges(command.query.as_ref(), &text))
        .unwrap_or_default();

    DisplayMatch {
        // `--notimestamp` removes only the timestamp prefix. The origin remains
        // visible so result lines stay attributable.
        prefix: if output.no_timestamp {
            format!("<{}> ", origin)
        } else {
            format!(
                "[{}] <{}> ",
                format_server_time(message.server_time),
                origin
            )
        },
        text,
        text_highlights,
    }
}

/// Formats a server timestamp for search output.
///
/// Used by `format_display_match`. `server_time` is a UTC message timestamp.
/// Returns a seconds-only UTC timestamp string. Produces no output or side
/// effects.
fn format_server_time(server_time: DateTime<Utc>) -> String {
    // Search result timestamps intentionally stop at seconds. Milliseconds
    // make manual scanning harder and were not wanted for this output surface.
    server_time.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Computes highlight ranges for positive body-text predicates.
///
/// Used by `format_display_match`. `expr` is the optional search expression and
/// `text` is the rendered message body after any `--textonly` transform.
/// Returns merged byte ranges into `text`. Produces no output or side effects.
fn highlight_ranges(expr: Option<&Expr>, text: &str) -> Vec<Range<usize>> {
    // Collect first, then merge, because separate positive predicates may
    // overlap or repeat the same literal text.
    let mut ranges = Vec::new();
    collect_text_highlights(expr, text, &mut ranges);
    merge_ranges(ranges)
}

/// Recursively collects raw text-highlight ranges from an expression tree.
///
/// Used by `highlight_ranges`. `expr` is the current expression node, `text` is
/// the rendered message body, and `ranges` receives unmerged byte ranges.
/// Returns through mutation of `ranges`. Produces no output or side effects.
fn collect_text_highlights(
    expr: Option<&Expr>,
    text: &str,
    ranges: &mut Vec<Range<usize>>,
) {
    match expr {
        Some(Expr::Predicate(predicate))
            if predicate.selector == Selector::Text
                && !predicate.modifiers.negated =>
        {
            // Only positive body-text predicates produce highlights. Field
            // predicates such as origin/reaction do not map cleanly into the
            // message body span rendered by the current pane.
            for value in predicate.value.iter() {
                ranges.extend(text_ranges(text, value, predicate.modifiers));
            }
        }
        Some(Expr::Predicate(_)) | None => {}
        Some(Expr::Not(_)) => {}
        Some(Expr::And(exprs)) | Some(Expr::Or(exprs)) => {
            // Boolean containers do not affect where literal text occurs, so
            // recurse into their children and let merge_ranges normalize spans.
            for expr in exprs {
                collect_text_highlights(Some(expr), text, ranges);
            }
        }
    }
}

/// Finds all body-text ranges matched by one text predicate value.
///
/// Used by `collect_text_highlights`. `haystack` is the rendered body text,
/// `needle` is a literal or regex pattern, and `modifiers` controls regex and
/// case sensitivity. Returns non-empty byte ranges. Produces no output or side
/// effects.
fn text_ranges(
    haystack: &str,
    needle: &str,
    modifiers: Modifiers,
) -> Vec<Range<usize>> {
    // Empty highlights are visually meaningless and can create zero-width
    // spans in iced rich text.
    if needle.is_empty() {
        return vec![];
    }

    // Literal text is escaped into a regex so the same range collector can
    // handle both regex and non-regex predicates.
    let pattern = if modifiers.regex {
        needle.to_string()
    } else {
        fancy_regex::escape(needle).to_string()
    };

    // Case-insensitive matching is expressed as a scoped regex flag so the
    // original haystack can be used without allocating a lowercase copy.
    let pattern = if modifiers.case_insensitive {
        format!("(?i:{pattern})")
    } else {
        pattern
    };

    Regex::new(&pattern)
        .ok()
        .map(|regex| {
            regex
                .find_iter(haystack)
                .filter_map(Result::ok)
                .map(|match_| match_.start()..match_.end())
                .filter(|range| !range.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Sorts and merges overlapping highlight ranges.
///
/// Used by `highlight_ranges`. `ranges` is an unsorted list of byte ranges.
/// Returns normalized ranges suitable for sequential rendering. Produces no
/// output or side effects.
fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    // Sort by start/end so adjacent overlapping ranges can be collapsed in one
    // linear pass.
    ranges.sort_by_key(|range| (range.start, range.end));

    let mut merged: Vec<Range<usize>> = Vec::new();

    for range in ranges {
        let Some(last) = merged.last_mut() else {
            merged.push(range);
            continue;
        };

        if range.start <= last.end {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }

    merged
}

/// Removes IRC formatting controls and ANSI color sequences from display text.
///
/// Used by `format_display_match` for `--textonly`. `text` is the stored
/// message body. Returns display text with presentation controls removed.
/// Produces no output or side effects.
fn strip_control_codes(text: &str) -> String {
    // Remove presentation controls only. This is intentionally conservative:
    // malformed color/ANSI sequences are consumed only as far as their control
    // grammar allows, then normal text resumes.
    let mut stripped = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\x02' | '\x0f' | '\x16' | '\x1d' | '\x1f' => {}
            '\x03' => {
                skip_mirc_color(&mut chars);
            }
            '\x1b' if chars.peek() == Some(&'[') => {
                chars.next();
                skip_ansi_sequence(&mut chars);
            }
            _ => stripped.push(ch),
        }
    }

    stripped
}

/// Consumes the optional numeric payload of one mIRC color control.
///
/// Used by `strip_control_codes` after it sees `\x03`. `chars` is the
/// remaining character stream and is advanced past foreground/background color
/// digits when present. Returns through iterator mutation. Produces no output
/// or external side effects.
fn skip_mirc_color<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    // mIRC foreground colors are at most two digits.
    for _ in 0..2 {
        if chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
        }
    }

    // A comma introduces an optional background color, also at most two digits.
    if chars.peek() == Some(&',') {
        chars.next();

        for _ in 0..2 {
            if chars.peek().is_some_and(char::is_ascii_digit) {
                chars.next();
            }
        }
    }
}

/// Consumes one ANSI CSI sequence after the introducer.
///
/// Used by `strip_control_codes` after it sees ESC `[`. `chars` is advanced
/// through the command terminator byte. Returns through iterator mutation.
/// Produces no output or external side effects.
fn skip_ansi_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    // ANSI CSI sequences end at an alphabetic command byte. Unknown or partial
    // sequences are consumed conservatively until that terminator appears.
    for ch in chars.by_ref() {
        if ch.is_ascii_alphabetic() {
            break;
        }
    }
}
