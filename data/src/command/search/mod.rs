#[cfg(test)]
mod tests;

mod display;
mod eval;
mod parser;
mod token;

pub use self::display::{DisplayMatch, format_display_match, format_match};
pub use self::eval::{ExecutionContext, Matches, find};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub kind: Kind,
    pub output: Output,
    pub query: Option<Expr>,
    pub default_span: DefaultSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Search,
    Last,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Output {
    pub text_only: bool,
    pub no_timestamp: bool,
    pub other: bool,
    pub context: usize,
    pub view: View,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Inline,
    Pane,
    Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultSpan {
    None,
    CurrentBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Predicate(Predicate),
    Not(Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Predicate {
    pub selector: Selector,
    pub value: Value,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selector {
    Text,
    Origin,
    Target,
    Type,
    Span,
    Reaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub case_insensitive: bool,
    pub negated: bool,
    pub regex: bool,
    pub join: Option<Join>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid search syntax: {0}")]
    Syntax(&'static str),
    #[error("unknown search selector: {0}")]
    UnknownSelector(String),
    #[error("invalid search string modifier: {0}")]
    InvalidModifier(char),
    #[error("invalid search regex: {0}")]
    InvalidRegex(String),
    #[error("invalid search span: expected Nd, Nh, or Nm")]
    InvalidSpan,
    #[error("invalid search view: expected inline, pane, or tab")]
    InvalidView,
    #[error("invalid search context: expected a non-negative line count")]
    InvalidContext,
}

#[derive(Debug, Clone, Copy)]
pub struct SelectorMetadata {
    pub canonical: &'static str,
    pub aliases: &'static [&'static str],
    pub value: &'static str,
}

pub const SELECTORS: &[SelectorMetadata] = &[
    SelectorMetadata {
        canonical: "text",
        aliases: &["itext", "regex", "iregex", "regexp", "re", "rx", "exp"],
        value: "string",
    },
    SelectorMetadata {
        canonical: "origin",
        aliases: &["from", "sender", "nick", "name"],
        value: "nickname",
    },
    SelectorMetadata {
        canonical: "target",
        aliases: &["to"],
        value: "nickname or channel",
    },
    SelectorMetadata {
        canonical: "type",
        aliases: &["kind"],
        value: "message type list",
    },
    SelectorMetadata {
        canonical: "span",
        aliases: &["since"],
        value: "duration",
    },
    SelectorMetadata {
        canonical: "reaction",
        aliases: &["react"],
        value: "reaction name list",
    },
];

/// Parses `/search` or `/last` arguments into a typed search command.
///
/// Used by the top-level command parser. `kind` identifies whether the command
/// was `/search` or `/last`, and `raw` is the argument tail. Returns a command
/// containing output options, optional query expression, and the default span
/// behavior. Produces no output or side effects.
pub fn parse(kind: Kind, raw: &str) -> Result<Command, Error> {
    let (output, query) = parser::parse_tail(raw)?;

    Ok(Command {
        kind,
        output,
        query,
        default_span: match kind {
            Kind::Search => DefaultSpan::None,
            Kind::Last => DefaultSpan::CurrentBuffer,
        },
    })
}

impl Value {
    /// Iterates over the string values inside a scalar or list value.
    ///
    /// Used by validation, evaluation, and highlighting. Returns an iterator
    /// over borrowed strings. Produces no output or side effects.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        match self {
            Self::String(value) => std::slice::from_ref(value).iter(),
            Self::List(values) => values.iter(),
        }
        .map(String::as_str)
    }
}
