use fancy_regex::Regex;

use super::eval::parse_span;
use super::token::{Token, tokenize, unescape_quoted, unquote};
use super::{
    Error, Expr, Join, Modifiers, Output, Predicate, Selector, Value, View,
};

/// Parses search output options and the optional query expression.
///
/// Used by the public search command parser. `raw` is the command argument
/// tail after `/search` or `/last`. Returns display options plus an optional
/// expression tree. Produces no output or side effects.
pub(super) fn parse_tail(raw: &str) -> Result<(Output, Option<Expr>), Error> {
    let tokens = tokenize(raw)?;
    let (output, tokens) = parse_options(tokens)?;
    let query = if tokens.is_empty() {
        None
    } else {
        Some(Parser::new(tokens).parse()?)
    };

    Ok((output, query))
}

/// Separates output flags/options from query expression tokens.
///
/// Used by `parse_tail`. `tokens` is the tokenized command tail. Returns the
/// parsed output settings and the remaining query tokens. Produces no output or
/// side effects.
fn parse_options(tokens: Vec<Token>) -> Result<(Output, Vec<Token>), Error> {
    let mut output = Output::default();
    let mut query = Vec::new();
    let mut tokens = tokens.into_iter().peekable();

    while let Some(token) = tokens.next() {
        match token {
            Token::Word(word) if word == "--textonly" => {
                output.text_only = true;
            }
            Token::Word(word) if word == "--notimestamp" => {
                output.no_timestamp = true;
            }
            Token::Word(word) if word == "--other" => {
                output.other = true;
            }
            Token::Word(word)
                if word.eq_ignore_ascii_case("view")
                    && tokens.peek() == Some(&Token::Equal) =>
            {
                tokens.next();

                let Some(Token::Word(value)) = tokens.next() else {
                    return Err(Error::InvalidView);
                };

                output.view = parse_view(&value)?;
            }
            Token::Word(word)
                if word.eq_ignore_ascii_case("context")
                    && tokens.peek() == Some(&Token::Equal) =>
            {
                tokens.next();

                let Some(Token::Word(value)) = tokens.next() else {
                    return Err(Error::InvalidContext);
                };

                output.context = parse_context(&value)?;
            }
            Token::Word(word) if word.starts_with("--") => {
                return Err(Error::Syntax("unknown output option"));
            }
            token => query.push(token),
        }
    }

    Ok((output, query))
}

/// Parses the `view=` output option.
///
/// Used by `parse_options`. `value` is the raw option value. Returns a typed
/// view enum or `Error::InvalidView`. Produces no output or side effects.
fn parse_view(value: &str) -> Result<View, Error> {
    match value.to_ascii_lowercase().as_str() {
        "inline" => Ok(View::Inline),
        "pane" => Ok(View::Pane),
        "tab" => Ok(View::Tab),
        _ => Err(Error::InvalidView),
    }
}

/// Parses the `context=` output option.
///
/// Used by `parse_options`. `value` is the raw line-count string. Returns the
/// number of context lines or `Error::InvalidContext`. Produces no output or
/// side effects.
fn parse_context(value: &str) -> Result<usize, Error> {
    value.parse().map_err(|_| Error::InvalidContext)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Creates a parser over query-expression tokens.
    ///
    /// Used by `parse_tail`. `tokens` are the non-option tokens from the
    /// command tail. Returns a parser positioned at the first token. Produces
    /// no output or side effects.
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parses a complete search expression.
    ///
    /// Used by `parse_tail`. Takes ownership of the parser and returns an
    /// expression tree, rejecting trailing tokens. Produces no output or side
    /// effects.
    fn parse(mut self) -> Result<Expr, Error> {
        let expr = self.parse_or()?;

        if self.peek().is_some() {
            return Err(Error::Syntax("unexpected token after expression"));
        }

        Ok(expr)
    }

    /// Parses the lowest-precedence OR expression layer.
    ///
    /// Used by `parse` and parenthesized subexpressions. Reads from the current
    /// parser position and returns an expression tree. Produces no output or
    /// side effects.
    fn parse_or(&mut self) -> Result<Expr, Error> {
        let mut exprs = vec![self.parse_and()?];

        while self.consume_keyword("OR") {
            exprs.push(self.parse_and()?);
        }

        Ok(flatten_bool(exprs, Expr::Or))
    }

    /// Parses the AND expression layer, including implicit adjacent AND.
    ///
    /// Used by `parse_or`. Reads from the current parser position and returns
    /// an expression tree. Produces no output or side effects.
    fn parse_and(&mut self) -> Result<Expr, Error> {
        let mut exprs = vec![self.parse_unary()?];

        loop {
            if self.consume_keyword("AND") || self.starts_primary() {
                exprs.push(self.parse_unary()?);
            } else {
                break;
            }
        }

        Ok(flatten_bool(exprs, Expr::And))
    }

    /// Parses unary operators such as `NOT`.
    ///
    /// Used by `parse_and`. Reads from the current parser position and returns
    /// the unary expression or primary expression. Produces no output or side
    /// effects.
    fn parse_unary(&mut self) -> Result<Expr, Error> {
        if self.consume_keyword("NOT") {
            Ok(Expr::Not(Box::new(self.parse_unary()?)))
        } else {
            self.parse_primary()
        }
    }

    /// Parses a parenthesized expression or predicate.
    ///
    /// Used by `parse_unary`. Reads from the current parser position and
    /// returns a primary expression. Produces no output or side effects.
    fn parse_primary(&mut self) -> Result<Expr, Error> {
        if self.consume(Token::LeftParen) {
            let expr = self.parse_or()?;
            if !self.consume(Token::RightParen) {
                return Err(Error::Syntax("missing closing parenthesis"));
            }

            return Ok(expr);
        }

        self.parse_predicate().map(Expr::Predicate)
    }

    /// Parses one selector predicate or bare text predicate.
    ///
    /// Used by `parse_primary`. Reads from the current parser position and
    /// returns a typed predicate. Produces no output or side effects.
    fn parse_predicate(&mut self) -> Result<Predicate, Error> {
        let Some(token) = self.next() else {
            return Err(Error::Syntax("expected predicate"));
        };

        let Token::Word(word) = token else {
            return Err(Error::Syntax("expected predicate"));
        };

        if self.consume(Token::Equal) {
            let (selector, mut modifiers) = selector(&word)?;
            let value = self.parse_value(&mut modifiers)?;

            validate_predicate(selector, &value, modifiers)?;

            Ok(Predicate {
                selector,
                value,
                modifiers,
            })
        } else {
            Ok(Predicate {
                selector: Selector::Text,
                value: Value::String(unquote(word)?),
                modifiers: Modifiers::default(),
            })
        }
    }

    /// Parses the value side of a selector predicate.
    ///
    /// Used by `parse_predicate`. `modifiers` carries selector-level modifiers
    /// and is updated with quoted string modifiers when present. Returns a
    /// string or comma-list value. Produces no output or side effects.
    fn parse_value(
        &mut self,
        modifiers: &mut Modifiers,
    ) -> Result<Value, Error> {
        let Some(token) = self.next() else {
            return Err(Error::Syntax("expected selector value"));
        };

        let Token::Word(mut value) = token else {
            return Err(Error::Syntax("expected selector value"));
        };

        if let Some((parsed_modifiers, parsed_value)) = parse_modified(&value)?
        {
            *modifiers = modifiers.merge(parsed_modifiers);
            value = parsed_value;
        }

        Ok(list_or_string(unquote(value)?))
    }

    /// Reports whether the next token can start a primary expression.
    ///
    /// Used by `parse_and` to implement implicit AND between adjacent
    /// predicates. Returns true when parsing can continue. Produces no output
    /// or side effects.
    fn starts_primary(&self) -> bool {
        match self.peek() {
            Some(Token::LeftParen) => true,
            Some(Token::Word(word)) => !is_keyword(word, "OR"),
            _ => false,
        }
    }

    /// Consumes a case-insensitive keyword when it is next.
    ///
    /// Used by expression parsing. `keyword` is the expected keyword. Returns
    /// true and advances when matched, otherwise false. Produces no output or
    /// side effects.
    fn consume_keyword(&mut self, keyword: &str) -> bool {
        match self.peek() {
            Some(Token::Word(word)) if is_keyword(word, keyword) => {
                self.position += 1;
                true
            }
            _ => false,
        }
    }

    /// Consumes an exact token when it is next.
    ///
    /// Used by expression parsing. `token` is the expected token. Returns true
    /// and advances when matched, otherwise false. Produces no output or side
    /// effects.
    fn consume(&mut self, token: Token) -> bool {
        if self.peek() == Some(&token) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    /// Removes and returns the next token.
    ///
    /// Used by predicate/value parsing. Returns the next token when available
    /// and advances the parser position. Produces no output or side effects.
    fn next(&mut self) -> Option<Token> {
        let token = self.peek()?.clone();
        self.position += 1;
        Some(token)
    }

    /// Borrows the next token without advancing.
    ///
    /// Used throughout expression parsing. Returns the next token reference or
    /// `None` at end of input. Produces no output or side effects.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }
}

impl Modifiers {
    /// Combines selector-level and value-level string modifiers.
    ///
    /// Used by value parsing. `self` is the existing modifier set and `other`
    /// is the modifier set parsed from a quoted value prefix. Returns the
    /// merged modifiers. Produces no output or side effects.
    fn merge(self, other: Self) -> Self {
        Self {
            case_insensitive: self.case_insensitive || other.case_insensitive,
            negated: self.negated || other.negated,
            regex: self.regex || other.regex,
            join: other.join.or(self.join),
        }
    }
}

/// Collapses one-child boolean expressions into the child expression.
///
/// Used by `parse_or` and `parse_and`. `exprs` are parsed child expressions and
/// `make` constructs the boolean node when more than one child exists. Returns
/// the simplified expression. Produces no output or side effects.
fn flatten_bool(
    mut exprs: Vec<Expr>,
    make: impl FnOnce(Vec<Expr>) -> Expr,
) -> Expr {
    if exprs.len() == 1 {
        exprs.remove(0)
    } else {
        make(exprs)
    }
}

/// Parses a selector name and any selector-implied modifiers.
///
/// Used by predicate parsing. `name` is the raw selector token. Returns the
/// canonical selector plus modifiers implied by aliases such as `itext` or
/// `regex`. Produces no output or side effects.
fn selector(name: &str) -> Result<(Selector, Modifiers), Error> {
    let mut modifiers = Modifiers::default();
    let selector = match name.to_ascii_lowercase().as_str() {
        "text" => Selector::Text,
        "itext" => {
            modifiers.case_insensitive = true;
            Selector::Text
        }
        "regex" | "regexp" | "re" | "rx" | "exp" => {
            modifiers.regex = true;
            Selector::Text
        }
        "iregex" => {
            modifiers.case_insensitive = true;
            modifiers.regex = true;
            Selector::Text
        }
        "origin" | "from" | "sender" | "nick" | "name" => Selector::Origin,
        "target" | "to" => Selector::Target,
        "type" | "kind" => Selector::Type,
        "span" | "since" => Selector::Span,
        "reaction" | "react" => Selector::Reaction,
        _ => return Err(Error::UnknownSelector(name.to_string())),
    };

    Ok((selector, modifiers))
}

/// Parses compact string modifiers before a quoted value.
///
/// Used by selector value parsing. `value` is the raw value token. Returns
/// modifiers plus the unescaped quoted value when the token has a modifier
/// prefix, `None` when it is an ordinary value, or a syntax error. Produces no
/// output or side effects.
fn parse_modified(value: &str) -> Result<Option<(Modifiers, String)>, Error> {
    let Some(quote_index) = value.find('"') else {
        return Ok(None);
    };

    if !value.ends_with('"') {
        return Err(Error::Syntax("unterminated quoted value"));
    }

    let (raw_modifiers, quoted) = value.split_at(quote_index);
    if raw_modifiers.is_empty() {
        return Ok(None);
    }

    let mut modifiers = Modifiers::default();
    for modifier in raw_modifiers.chars() {
        match modifier {
            'i' => modifiers.case_insensitive = true,
            'n' => modifiers.negated = true,
            'a' => modifiers.join = Some(Join::And),
            'o' => modifiers.join = Some(Join::Or),
            'x' => modifiers.regex = true,
            _ => return Err(Error::InvalidModifier(modifier)),
        }
    }

    Ok(Some((
        modifiers,
        unescape_quoted(
            quoted
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .unwrap_or_default(),
        ),
    )))
}

/// Converts a parsed string into a scalar or comma-list value.
///
/// Used by value parsing. `value` is the unquoted selector value. Returns a
/// list when commas are present, otherwise a scalar string. Produces no output
/// or side effects.
fn list_or_string(value: String) -> Value {
    if value.contains(',') {
        Value::List(value.split(',').map(ToString::to_string).collect())
    } else {
        Value::String(value)
    }
}

/// Validates selector-specific predicate constraints after parsing.
///
/// Used by predicate parsing. `selector`, `value`, and `modifiers` describe the
/// candidate predicate. Returns `Ok` when regex/span constraints are valid.
/// Produces no output or side effects.
fn validate_predicate(
    selector: Selector,
    value: &Value,
    modifiers: Modifiers,
) -> Result<(), Error> {
    if selector == Selector::Text && modifiers.regex {
        for value in value.iter() {
            Regex::new(value).map_err(|err| {
                Error::InvalidRegex(format!("{value}: {err}"))
            })?;
        }
    }

    if selector == Selector::Span {
        if !matches!(value, Value::String(_)) {
            return Err(Error::InvalidSpan);
        }

        for value in value.iter() {
            parse_span(value)?;
        }
    }

    Ok(())
}

/// Compares a token to a parser keyword case-insensitively.
///
/// Used by expression parsing. `word` is a token string and `keyword` is the
/// expected keyword. Returns true when they are equal ignoring ASCII case.
/// Produces no output or side effects.
fn is_keyword(word: &str, keyword: &str) -> bool {
    word.eq_ignore_ascii_case(keyword)
}
