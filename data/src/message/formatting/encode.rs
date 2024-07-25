//! Internal formatting specification
use std::fmt::Write;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, char, none_of, satisfy},
    combinator::{cut, eof, map, map_opt, opt, recognize, value},
    multi::{many0, many1, many_m_n},
    sequence::{delimited, tuple},
    Finish, IResult,
};

use super::{Color, Modifier};

pub fn encode(text: &str) -> String {
    let Some(tokens) = parse(text) else {
        return text.to_string();
    };

    let mut out = String::with_capacity(irc::proto::format::BYTE_LIMIT);

    for token in tokens {
        match token {
            Token::Plain(plain) => out.push_str(plain),
            Token::Markdown(markdown) => match markdown {
                Markdown::Bold(plain) => {
                    let b = Modifier::Bold.char();
                    let _ = write!(&mut out, "{b}{plain}{b}");
                }
                Markdown::Italic(plain) => {
                    let i = Modifier::Italics.char();
                    let _ = write!(&mut out, "{i}{plain}{i}");
                }
                Markdown::ItalicBold(plain) => {
                    let b = Modifier::Bold.char();
                    let i = Modifier::Italics.char();
                    let _ = write!(&mut out, "{b}{i}{plain}{b}{i}");
                }
                Markdown::Code(plain) => {
                    let m = Modifier::Monospace.char();
                    let _ = write!(&mut out, "{m}{plain}{m}");
                }
                Markdown::Spoiler(plain) => {
                    let c = Modifier::Color.char();
                    let black = Color::Black.digit();
                    let _ = write!(&mut out, "{c}{black},{black}{plain}{c}");
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
                Dollar::Reset => {
                    out.push(Modifier::Reset.char());
                }
                Dollar::StartColor(fg, bg) => {
                    let c = Modifier::Color.char();
                    let fg = fg.digit();
                    let _ = write!(&mut out, "{c}{fg}");

                    if let Some(bg) = bg.map(Color::digit) {
                        let _ = write!(&mut out, ",{bg}");
                    }
                }
                Dollar::EndColor => {
                    out.push(Modifier::Color.char());
                }
            },
            Token::Unknown(char) => out.push(char),
        }
    }

    out
}

fn parse(input: &str) -> Option<Vec<Token>> {
    let tokens = tuple((many0(token), eof));

    cut(tokens)(input)
        .finish()
        .ok()
        .map(|(_, (tokens, _))| tokens)
}

fn token(input: &str) -> IResult<&str, Token> {
    alt((
        map(plain, Token::Plain),
        map(markdown, Token::Markdown),
        map(dollar, Token::Dollar),
        map(anychar, Token::Unknown),
    ))(input)
}

fn plain(input: &str) -> IResult<&str, &str> {
    recognize(many1(escaped))(input)
}

fn escaped(input: &str) -> IResult<&str, char> {
    alt((
        value('*', tag("\\*")),
        value('_', tag("\\_")),
        value('`', tag("\\`")),
        value('|', tag("\\|")),
        value('$', tag("\\$")),
        none_of("*_`|$"),
    ))(input)
}

fn markdown(input: &str) -> IResult<&str, Markdown> {
    let between = |start, end| delimited(tag(start), plain, tag(end));

    let italic = alt((between("_", "_"), between("*", "*")));
    let bold = alt((between("__", "__"), between("**", "**")));
    let italic_bold = alt((
        between("___", "___"),
        between("***", "***"),
        between("**_", "_**"),
        between("__*", "*__"),
    ));
    let code = between("`", "`");
    let spoiler = between("||", "||");

    alt((
        map(italic_bold, Markdown::ItalicBold),
        map(bold, Markdown::Bold),
        map(italic, Markdown::Italic),
        map(code, Markdown::Code),
        map(spoiler, Markdown::Spoiler),
    ))(input)
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
        map(tag("$r"), |_| Dollar::Reset),
        map(start_color, |(fg, bg)| Dollar::StartColor(fg, bg)),
        // No valid colors after code == end
        map(tag("$c"), |_| Dollar::EndColor),
    ))(input)
}

#[derive(Debug)]
enum Token<'a> {
    Plain(&'a str),
    Markdown(Markdown<'a>),
    Dollar(Dollar),
    Unknown(char),
}

#[derive(Debug)]
enum Markdown<'a> {
    Bold(&'a str),
    Italic(&'a str),
    ItalicBold(&'a str),
    Code(&'a str),
    Spoiler(&'a str),
}

#[derive(Debug)]
enum Dollar {
    Bold,
    Italics,
    Monospace,
    Reset,
    StartColor(Color, Option<Color>),
    EndColor,
}

#[test]
fn internal_format() {
    let _ = dbg!(encode("hello there friend!!"));
    let _ = dbg!(encode("hello there _friend_!!"));
    let _ = dbg!(encode("hello there __friend__!!"));
    let _ = dbg!(encode("hello there ___friend___!!"));
    let _ = dbg!(encode("hello there **_\\_fri\\_end\\__**!!"));
    let _ = dbg!(encode("some code `let x = 0;`"));
    let _ = dbg!(encode("spoiler --> ||super secret||"));
    let _ = dbg!(encode(
        "$c1,0black on white $c2now blue on white$r$b BOLD $i BOLD AND ITALIC$r $ccode yo"
    ));
}
