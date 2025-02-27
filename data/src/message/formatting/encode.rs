//! Internal formatting specification
use std::{convert::identity, fmt::Write};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, char, satisfy},
    combinator::{cond, cut, eof, map, map_opt, not, opt, peek, recognize, value, verify},
    multi::{count, many_m_n, many_till},
    sequence::{pair, preceded, tuple},
    Finish, IResult,
};

use super::{Color, Modifier};

pub fn encode(text: &str, markdown_only: bool) -> String {
    let Some(tokens) = parse(text, markdown_only) else {
        return text.to_string();
    };

    let mut out = String::with_capacity(irc::proto::format::BYTE_LIMIT);

    for token in tokens {
        token.encode(&mut out);
    }

    out
}

fn parse(input: &str, markdown_only: bool) -> Option<Vec<Token>> {
    let tokens = many_till(|i| token(input, i, markdown_only), eof);

    cut(tokens)(input)
        .finish()
        .ok()
        .map(|(_, (tokens, _))| tokens)
}

fn token<'a>(source: &'a str, input: &'a str, markdown_only: bool) -> IResult<&'a str, Token> {
    alt((
        map(escaped(markdown_only), Token::Escaped),
        map(markdown(source, markdown_only), Token::Markdown),
        skip(markdown_only, map(dollar, Token::Dollar)),
        map(anychar, Token::Plain),
    ))(input)
}

fn escaped<'a>(markdown_only: bool) -> impl FnMut(&'a str) -> IResult<&'a str, char> {
    alt((
        value('*', tag("\\*")),
        value('_', tag("\\_")),
        value('`', tag("\\`")),
        value('`', tag("``")),
        value('|', tag("\\|")),
        skip(markdown_only, value('$', tag("\\$"))),
        skip(markdown_only, value('$', tag("$$"))),
    ))
}

fn skip<'a, O>(
    skip: bool,
    inner: impl FnMut(&'a str) -> IResult<&'a str, O>,
) -> impl FnMut(&'a str) -> IResult<&'a str, O> {
    map_opt(cond(!skip, inner), identity)
}

fn markdown<'a>(
    source: &'a str,
    markdown_only: bool,
) -> impl FnMut(&'a str) -> IResult<&'a str, Markdown> {
    // EOF is considered WS in common mark spec
    let ws = |i| alt((satisfy(char::is_whitespace), map(eof, |_| ' ')))(i);
    let punc = |d| move |i| satisfy(|c| c != d && c.is_ascii_punctuation())(i);

    // Previous consumed character
    let prev = |i: &str| source[..source.len() - i.len()].chars().last();
    // Prev char is whitespace or punctuation, not matching delimiter char
    let prev_is_ws_or_punc = move |d, i: &str| {
        // None == Start of input which is considered whitespace
        prev(i).is_none_or(|c| {
            c != d && (c.is_whitespace() || c.is_ascii_punctuation())
        })
    };
    // Prev char is punctuation, not matching delimiter char
    let prev_is_punc =
        move |d, i: &str| prev(i).is_some_and(|c| c != d && c.is_ascii_punctuation());

    // A delimiter run is a sequence of one or more characters
    let delimiter_run = |d, n| count(char(d), n);

    // A left-flanking delimiter run is a
    let left_flanking = move |d, n| {
        move |i: &'a str| {
            alt((
                // delimiter run that is not followed by Unicode whitespace or punctuation
                map(
                    pair(delimiter_run(d, n), peek(not(alt((ws, punc(d)))))),
                    move |_| prev(i),
                ),
                // OR delimiter run followed by a Unicode punctuation character and preceded
                // by Unicode whitespace or a Unicode punctuation character
                map(
                    tuple((
                        verify(delimiter_run(d, n), move |_: &Vec<_>| {
                            prev_is_ws_or_punc(d, i)
                        }),
                        peek(punc(d)),
                    )),
                    move |_| prev(i),
                ),
            ))(i)
        }
    };
    // A right-flanking delimiter run is a
    let right_flanking = move |d, n| {
        move |i: &'a str| {
            alt((
                // delimiter run that is not preceded by Unicode whitespace or punctutation
                map(
                    verify(delimiter_run(d, n), move |_: &Vec<_>| {
                        !prev_is_ws_or_punc(d, i)
                    }),
                    move |_| prev(i),
                ),
                // OR delimiter run that is preceded by a Unicode punctuation character and
                // followed by Unicode whitespace or a Unicode punctuation character
                map(
                    pair(
                        verify(delimiter_run(d, n), move |_: &Vec<_>| prev_is_punc(d, i)),
                        peek(alt((ws, punc(d)))),
                    ),
                    move |_| prev(i),
                ),
            ))(i)
        }
    };

    // can open emphasis if it is part of a left-flanking delimiter run
    let open_emphasis_relaxed = left_flanking;
    // can open emphasis if it is part of a right-flanking delimiter run
    let close_emphasis_relaxed = right_flanking;

    // can open emphasis if it is
    let open_emphasis_strict = |d, n| {
        preceded(
            peek(alt((
                // part of a left-flanking delimiter run and not part of a right-flanking delimiter run
                map(
                    pair(peek(left_flanking(d, n)), not(right_flanking(d, n))),
                    |_| (),
                ),
                // OR part of a left-flanking delimiter run and part of a right-flanking delimiter run
                // preceded by a Unicode punctuation character
                map(
                    pair(
                        peek(left_flanking(d, n)),
                        verify(right_flanking(d, n), move |c| {
                            c.map_or(false, |c| c != d && c.is_ascii_punctuation())
                        }),
                    ),
                    |_| (),
                ),
            ))),
            left_flanking(d, n),
        )
    };
    // can close emphasis if it is
    let close_emphasis_strict = |d, n| {
        preceded(
            peek(alt((
                // part of a right-flanking delimiter run and not part of a left-flanking delimiter run
                map(
                    pair(peek(right_flanking(d, n)), not(left_flanking(d, n))),
                    |_| (),
                ),
                // OR part of a right-flanking delimiter run and part of a left-flanking delimiter run
                // followed by a Unicode punctuation character
                map(
                    tuple((
                        peek(right_flanking(d, n)),
                        left_flanking(d, n),
                        satisfy(move |c| c != d && c.is_ascii_punctuation()),
                    )),
                    |_| (),
                ),
            ))),
            right_flanking(d, n),
        )
    };

    // open <tokens> close
    let relaxed_run = |d, n| {
        map(
            pair(
                open_emphasis_relaxed(d, n),
                many_till(
                    move |input| token(source, input, markdown_only),
                    close_emphasis_relaxed(d, n),
                ),
            ),
            |(_, (tokens, _))| tokens,
        )
    };
    // open <tokens> close
    let strict_run = |d, n| {
        map(
            pair(
                open_emphasis_strict(d, n),
                many_till(
                    move |input| token(source, input, markdown_only),
                    close_emphasis_strict(d, n),
                ),
            ),
            |(_, (tokens, _))| tokens,
        )
    };

    let italic = alt((relaxed_run('*', 1), strict_run('_', 1)));
    let bold = alt((relaxed_run('*', 2), strict_run('_', 2)));
    let italic_bold = alt((relaxed_run('*', 3), strict_run('_', 3)));
    let strikethrough = relaxed_run('~', 2);
    let spoiler = relaxed_run('|', 2);
    let code = map(
        alt((
            pair(
                tag("` "),
                many_till(move |input| token(source, input, markdown_only), tag(" `")),
            ),
            pair(
                tag("`"),
                many_till(move |input| token(source, input, markdown_only), tag("`")),
            ),
        )),
        |(_, (tokens, _))| tokens,
    );

    alt((
        map(italic_bold, Markdown::ItalicBold),
        map(bold, Markdown::Bold),
        map(italic, Markdown::Italic),
        map(strikethrough, Markdown::Strikethrough),
        map(spoiler, Markdown::Spoiler),
        map(code, Markdown::Code),
    ))
}

fn dollar(input: &str) -> IResult<&str, Dollar> {
    let color_name = |input| {
        alt((
            map(tag("white"), |_| Color::White),
            map(tag("black"), |_| Color::Black),
            map(tag("blue"), |_| Color::Blue),
            map(tag("green"), |_| Color::Green),
            map(tag("red"), |_| Color::Red),
            map(tag("brown"), |_| Color::Brown),
            map(tag("magenta"), |_| Color::Magenta),
            map(tag("orange"), |_| Color::Orange),
            map(tag("yellow"), |_| Color::Yellow),
            map(tag("lightgreen"), |_| Color::LightGreen),
            map(tag("cyan"), |_| Color::Cyan),
            map(tag("lightcyan"), |_| Color::LightCyan),
            map(tag("lightblue"), |_| Color::LightBlue),
            map(tag("pink"), |_| Color::Pink),
            map(tag("grey"), |_| Color::Grey),
            map(tag("lightgrey"), |_| Color::LightGrey),
        ))(input)
    };
    // 1-2 digits -> Color
    let color_digit = |input| {
        map_opt(
            recognize(many_m_n(1, 2, satisfy(|c| c.is_ascii_digit()))),
            |s: &str| s.parse().ok().and_then(Color::code),
        )(input)
    };
    let color = move |input| alt((color_name, color_digit))(input);

    // Optional , then Color
    let background = map(opt(tuple((char(','), color))), |maybe| {
        maybe.map(|(_, color)| color)
    });

    // $cFG[,BG]
    let start_color = map(
        tuple((tag("$c"), tuple((color, background)))),
        |(_, (fg, bg))| (fg, bg),
    );

    alt((
        map(tag("$b"), |_| Dollar::Bold),
        map(tag("$i"), |_| Dollar::Italics),
        map(tag("$m"), |_| Dollar::Monospace),
        map(tag("$s"), |_| Dollar::Strikethrough),
        map(tag("$u"), |_| Dollar::Underline),
        map(tag("$r"), |_| Dollar::Reset),
        map(start_color, |(fg, bg)| Dollar::StartColor(fg, bg)),
        // No valid colors after code == end
        map(tag("$c"), |_| Dollar::EndColor),
    ))(input)
}

#[derive(Debug)]
enum Token {
    Escaped(char),
    Markdown(Markdown),
    Dollar(Dollar),
    Plain(char),
}

impl Token {
    fn encode(self, out: &mut String) {
        match self {
            Token::Escaped(c) => out.push(c),
            Token::Markdown(markdown) => match markdown {
                Markdown::Bold(tokens) => {
                    let b = Modifier::Bold.char();
                    out.push(b);
                    for token in tokens {
                        token.encode(out);
                    }
                    out.push(b);
                }
                Markdown::Italic(tokens) => {
                    let i = Modifier::Italics.char();
                    out.push(i);
                    for token in tokens {
                        token.encode(out);
                    }
                    out.push(i);
                }
                Markdown::ItalicBold(tokens) => {
                    let b = Modifier::Bold.char();
                    let i = Modifier::Italics.char();
                    out.push(b);
                    out.push(i);
                    for token in tokens {
                        token.encode(out);
                    }
                    out.push(i);
                    out.push(b);
                }
                Markdown::Code(tokens) => {
                    let m = Modifier::Monospace.char();
                    out.push(m);
                    for token in tokens {
                        token.encode(out);
                    }
                    out.push(m);
                }
                Markdown::Spoiler(tokens) => {
                    let c = Modifier::Color.char();
                    let black = Color::Black.digit();
                    let _ = write!(out, "{c}{black},{black}");
                    for token in tokens {
                        token.encode(out);
                    }
                    out.push(c);
                }
                Markdown::Strikethrough(tokens) => {
                    let m = Modifier::Strikethrough.char();
                    out.push(m);
                    for token in tokens {
                        token.encode(out);
                    }
                    out.push(m);
                }
            },
            Token::Dollar(dollar) => match dollar {
                Dollar::Bold => {
                    out.push(Modifier::Bold.char());
                }
                Dollar::Italics => {
                    out.push(Modifier::Italics.char());
                }
                Dollar::Monospace => {
                    out.push(Modifier::Monospace.char());
                }
                Dollar::Strikethrough => {
                    out.push(Modifier::Strikethrough.char());
                }
                Dollar::Underline => {
                    out.push(Modifier::Underline.char());
                }
                Dollar::Reset => {
                    out.push(Modifier::Reset.char());
                }
                Dollar::StartColor(fg, bg) => {
                    let c = Modifier::Color.char();
                    let fg = fg.digit();
                    let _ = write!(out, "{c}{fg}");

                    if let Some(bg) = bg.map(Color::digit) {
                        let _ = write!(out, ",{bg}");
                    }
                }
                Dollar::EndColor => {
                    out.push(Modifier::Color.char());
                }
            },
            Token::Plain(c) => out.push(c),
        }
    }
}

#[derive(Debug)]
enum Markdown {
    Bold(Vec<Token>),
    Italic(Vec<Token>),
    ItalicBold(Vec<Token>),
    Strikethrough(Vec<Token>),
    Code(Vec<Token>),
    Spoiler(Vec<Token>),
}

#[derive(Debug)]
enum Dollar {
    Bold,
    Italics,
    Monospace,
    Strikethrough,
    Underline,
    Reset,
    StartColor(Color, Option<Color>),
    EndColor,
}

#[test]
fn internal_format() {
    let _ = dbg!(encode("_hello_", false));
    let _ = dbg!(encode("hello there friend!!", false));
    let _ = dbg!(encode("hello there _friend_!!", false));
    let _ = dbg!(encode("hello there __friend__!!", false));
    let _ = dbg!(encode("hello there ___friend___!!", false));
    let _ = dbg!(encode("hello there **_fri_end_**!!", false));
    let _ = dbg!(encode("testing__testing__onetwothree", false));
    let _ = dbg!(encode("some code `let x = 0;`", false));
    let _ = dbg!(encode("spoiler --> ||super secret||", false));
    let _ = dbg!(encode(
        "$c1,0black on white $c2now blue on white$r$b BOLD $i BOLD AND ITALIC$r $ccode yo",
        false,
    ));
}
