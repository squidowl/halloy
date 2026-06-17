use super::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    Word(String),
    Equal,
    LeftParen,
    RightParen,
}

/// Splits raw `/search` input into parser tokens.
///
/// Used by the search parser entry point. `raw` is the argument tail after the
/// command name. Returns words, equals signs, and parentheses while preserving
/// quoted text as a single word. Produces no output or side effects.
pub(super) fn tokenize(raw: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut chars = raw.char_indices().peekable();

    while let Some((_, ch)) = chars.peek().copied() {
        match ch {
            ch if ch.is_whitespace() => {
                chars.next();
            }
            '=' => {
                chars.next();
                tokens.push(Token::Equal);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LeftParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RightParen);
            }
            _ => tokens.push(Token::Word(read_word(raw, &mut chars)?)),
        }
    }

    Ok(tokens)
}

/// Reads one word token from the current lexer position.
///
/// Used by `tokenize`. `raw` is the full input and `chars` points at the first
/// character of the word. Returns the raw word slice, including quotes when
/// present. Advances `chars` to the next delimiter. Produces no output or
/// external side effects.
fn read_word(
    raw: &str,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<String, Error> {
    // Capture the starting byte index so quoted values preserve their original
    // spelling until the parser decides whether to unquote or modify them.
    let start = chars.peek().map(|(index, _)| *index).unwrap_or_default();
    let mut in_quote = false;

    while let Some((index, ch)) = chars.peek().copied() {
        if in_quote && ch == '\\' {
            chars.next();

            if chars.peek().is_some() {
                chars.next();
            }
        } else if ch == '"' {
            in_quote = !in_quote;
            chars.next();
        } else if !in_quote
            && (ch.is_whitespace() || matches!(ch, '=' | '(' | ')'))
        {
            return Ok(raw[start..index].to_string());
        } else {
            chars.next();
        }
    }

    if in_quote {
        return Err(Error::Syntax("unterminated quoted value"));
    }

    Ok(raw[start..].to_string())
}

/// Removes surrounding quotes from a parser word when present.
///
/// Used by selector and literal parsing. `value` is a raw word token. Returns
/// the unquoted and unescaped value, or the original value when it is not
/// quoted. Produces no output or side effects.
pub(super) fn unquote(value: String) -> Result<String, Error> {
    if value.starts_with('"') {
        value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .map(unescape_quoted)
            .ok_or(Error::Syntax("unterminated quoted value"))
    } else {
        Ok(value)
    }
}

/// Applies the quoted-string escape rules for `/search`.
///
/// Used by `unquote` and modifier parsing. `value` is the inside of a quoted
/// value. Returns text where `\"` becomes `"` and `\\` becomes `\`; unknown
/// escapes are preserved. Produces no output or side effects.
pub(super) fn unescape_quoted(value: &str) -> String {
    let mut unescaped = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\'
            && let Some(next) = chars.next()
        {
            match next {
                '"' | '\\' => unescaped.push(next),
                _ => {
                    unescaped.push(ch);
                    unescaped.push(next);
                }
            }
        } else {
            unescaped.push(ch);
        }
    }

    unescaped
}
