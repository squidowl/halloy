use std::fmt::Write;

use crate::message;

const MIN_TEXT_LENGTH: usize = 2;

pub const PAGE_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub struct Page {
    pub hits: Vec<Hit>,
    pub next: Option<Cursor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub(super) key: i64,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub server: String,
    pub buffer: Buffer,
    pub message: message::Message,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Buffer {
    Server,
    Channel(String),
    Query(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Database(String),
    #[error("search stopped")]
    Stopped,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    terms: Vec<Term>,
    prefix: bool,
    pub from: Option<String>,
    pub within: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Term {
    Word(String),
    Phrase(String),
}

impl Term {
    fn as_str(&self) -> &str {
        match self {
            Term::Word(text) | Term::Phrase(text) => text,
        }
    }
}

impl Query {
    pub fn parse(input: &str) -> Self {
        let mut query = Query::default();
        let mut chars = input.chars().peekable();
        let mut last_was_word = false;

        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
                last_was_word = false;
                continue;
            }

            if c == '"' {
                chars.next();
                let phrase: String =
                    chars.by_ref().take_while(|c| *c != '"').collect();
                if has_text(&phrase) {
                    query.terms.push(Term::Phrase(phrase));
                }
                last_was_word = false;
                continue;
            }

            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '"' {
                    break;
                }
                word.push(c);
                chars.next();
            }

            last_was_word = false;
            if let Some(nick) = strip_prefix_ignore_case(&word, "from:") {
                if !nick.is_empty() {
                    query.from = Some(nick.to_lowercase());
                }
            } else if let Some(target) = strip_prefix_ignore_case(&word, "in:")
            {
                if !target.is_empty() {
                    query.within = Some(target.to_owned());
                }
            } else if has_text(&word) {
                query.terms.push(Term::Word(word));
                last_was_word = true;
            }
        }

        query.prefix = last_was_word;
        query
    }

    pub fn fts(&self) -> Option<String> {
        let mut fts = String::new();

        if let Some(nick) = &self.from {
            let _ = write!(fts, "nick : \"{}\"", nick_token(nick));
        }

        let last = self.terms.len().saturating_sub(1);

        for (index, term) in self.terms.iter().enumerate() {
            if !fts.is_empty() {
                fts.push(' ');
            }
            let _ = write!(
                fts,
                "text : \"{}\"",
                term.as_str().replace('"', "\"\"")
            );
            if index == last && self.prefix {
                fts.push('*');
            }
        }

        (!fts.is_empty()).then_some(fts)
    }

    pub fn is_runnable(&self) -> bool {
        let text_length: usize = self
            .terms
            .iter()
            .map(|term| {
                term.as_str()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .count()
            })
            .sum();

        self.from.is_some() || text_length >= MIN_TEXT_LENGTH
    }

    pub fn terms(&self) -> impl Iterator<Item = &str> {
        self.terms.iter().map(Term::as_str)
    }
}

pub(super) fn nick_token(nick: &str) -> String {
    nick.to_lowercase()
        .bytes()
        .fold(String::new(), |mut token, byte| {
            let _ = write!(token, "{byte:02x}");
            token
        })
}

fn has_text(term: &str) -> bool {
    term.chars().any(char::is_alphanumeric)
}

fn strip_prefix_ignore_case<'a>(
    word: &'a str,
    prefix: &str,
) -> Option<&'a str> {
    word.get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &word[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_quoted_and_last_is_prefix() {
        let query = Query::parse("hello wor");
        assert_eq!(
            query.fts().as_deref(),
            Some("text : \"hello\" text : \"wor\"*")
        );
    }

    #[test]
    fn trailing_space_disables_prefix() {
        let query = Query::parse("hello world ");
        assert_eq!(
            query.fts().as_deref(),
            Some("text : \"hello\" text : \"world\"")
        );
    }

    #[test]
    fn phrases() {
        let query = Query::parse("\"exact phrase\" more");
        assert_eq!(
            query.fts().as_deref(),
            Some("text : \"exact phrase\" text : \"more\"*")
        );

        let query = Query::parse("\"exact phr");
        assert_eq!(query.fts().as_deref(), Some("text : \"exact phr\""));
    }

    #[test]
    fn syntax_characters_are_escaped() {
        let query = Query::parse("-foo bar:baz a*b ^x (y) NOT");
        assert_eq!(
            query.fts().as_deref(),
            Some(
                "text : \"-foo\" text : \"bar:baz\" text : \"a*b\" text : \"^x\" text : \"(y)\" text : \"NOT\"*"
            )
        );

        let query = Query::parse("it\"s");
        assert_eq!(query.fts().as_deref(), Some("text : \"it\" text : \"s\""));
    }

    #[test]
    fn terms_without_text_are_dropped() {
        let query = Query::parse("- * \"\" foo");
        assert_eq!(query.fts().as_deref(), Some("text : \"foo\"*"));
        assert_eq!(Query::parse("- *").fts(), None);
    }

    #[test]
    fn filters() {
        let query = Query::parse("From:Casper in:#Halloy release");
        assert_eq!(query.from.as_deref(), Some("casper"));
        assert_eq!(query.within.as_deref(), Some("#Halloy"));
        assert_eq!(
            query.fts().as_deref(),
            Some("nick : \"636173706572\" text : \"release\"*")
        );

        let query = Query::parse("from:casper");
        assert_eq!(query.fts().as_deref(), Some("nick : \"636173706572\""));

        let query = Query::parse("from: in:");
        assert_eq!(query, Query::default());
    }

    #[test]
    fn runnable() {
        assert!(!Query::parse("").is_runnable());
        assert!(!Query::parse("a").is_runnable());
        assert!(!Query::parse("- -").is_runnable());
        assert!(Query::parse("ab").is_runnable());
        assert!(!Query::parse("in:#halloy").is_runnable());
        assert!(Query::parse("from:casper").is_runnable());
    }
}
