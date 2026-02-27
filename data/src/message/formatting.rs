use std::collections::HashSet;
use std::mem;

use iced_core::color;
use itertools::PeekingNext;
use serde::{Deserialize, Serialize};

pub use self::encode::encode;
use crate::appearance::theme;

pub mod encode;

pub fn parse_code_fragments(text: &str) -> Vec<Fragment> {
    let mut modifiers = HashSet::new();

    let mut fragments = vec![];

    let mut current_text = String::new();

    let iter = text.chars().peekable();

    for c in iter {
        if let Ok(modifier) = Modifier::try_from(c) {
            if !current_text.is_empty() {
                let text = mem::take(&mut current_text);

                if modifiers.is_empty() {
                    fragments.push(Fragment::Unformatted(text));
                } else {
                    fragments.push(Fragment::Formatted(
                        text,
                        Formatting::new(&modifiers, None, None),
                    ));
                }
            }

            match modifier {
                Modifier::Reset => {
                    modifiers.clear();
                }
                Modifier::Monospace => {
                    if modifiers.contains(&Modifier::Monospace) {
                        modifiers.remove(&Modifier::Monospace);
                    } else {
                        modifiers.insert(Modifier::Monospace);
                    }
                }
                _ => {
                    if !modifiers.contains(&Modifier::Monospace) {
                        current_text.push(c);
                    }
                }
            }
        } else {
            current_text.push(c);
        }
    }

    if !current_text.is_empty() {
        let text = mem::take(&mut current_text);

        if modifiers.is_empty() {
            fragments.push(Fragment::Unformatted(text));
        } else {
            fragments.push(Fragment::Formatted(
                text,
                Formatting::new(&modifiers, None, None),
            ));
        }
    }

    fragments
}

pub fn parse_fragments(
    text: &str,
    modifiers: &mut HashSet<Modifier>,
    fg: &mut Option<Color>,
    bg: &mut Option<Color>,
) -> Option<Vec<Fragment>> {
    let mut fragments = vec![];

    let mut current_text = String::new();

    let mut iter = text.chars().peekable();

    while let Some(c) = iter.next() {
        if let Ok(modifier) = Modifier::try_from(c) {
            if !current_text.is_empty() {
                let text = mem::take(&mut current_text);

                if modifiers.is_empty() && fg.is_none() && bg.is_none() {
                    fragments.push(Fragment::Unformatted(text));
                } else {
                    fragments.push(Fragment::Formatted(
                        text,
                        Formatting::new(modifiers, *fg, *bg),
                    ));
                }
            }

            match modifier {
                Modifier::Reset => {
                    modifiers.clear();
                    *fg = None;
                    *bg = None;
                }
                Modifier::Color => {
                    // Trailing digit for new color, otherwise resets
                    if let Some(c) = iter.peeking_next(char::is_ascii_digit) {
                        // 1-2 digiits
                        let mut digits = c.to_string();
                        if let Some(c) = iter.peeking_next(char::is_ascii_digit)
                        {
                            digits.push(c);
                        }

                        let code = digits.parse().ok()?;

                        *fg = Color::code(code);

                        if let Some(comma) = iter.peeking_next(|c| *c == ',') {
                            // Has background
                            if let Some(c) =
                                iter.peeking_next(char::is_ascii_digit)
                            {
                                // 1-2 digits
                                let mut digits = c.to_string();
                                if let Some(c) =
                                    iter.peeking_next(char::is_ascii_digit)
                                {
                                    digits.push(c);
                                }

                                let code = digits.parse().ok()?;

                                *bg = Color::code(code);
                            }
                            // Nope, just a normal char
                            else {
                                current_text.push(comma);
                            }
                        }
                    } else {
                        *fg = None;
                        *bg = None;
                    }
                }
                Modifier::HexColor => {
                    // Trailing digit for new color, otherwise resets
                    if let Some(c) = iter.peeking_next(char::is_ascii_hexdigit)
                    {
                        // 6 digits (hex)
                        let mut hex = Vec::from([c]);
                        for _ in 0..5 {
                            hex.push(iter.next()?);
                        }

                        let r = u8::from_str_radix(
                            &hex.iter().take(2).collect::<String>(),
                            16,
                        )
                        .ok()?;
                        let g = u8::from_str_radix(
                            &hex.iter().skip(2).take(2).collect::<String>(),
                            16,
                        )
                        .ok()?;
                        let b = u8::from_str_radix(
                            &hex.iter().skip(4).take(2).collect::<String>(),
                            16,
                        )
                        .ok()?;

                        *fg = Some(Color::Rgb(r, g, b));

                        if let Some(comma) = iter.peeking_next(|c| *c == ',') {
                            // Has background
                            if let Some(c) =
                                iter.peeking_next(char::is_ascii_hexdigit)
                            {
                                // 6 digits (hex)
                                let mut hex = Vec::from([c]);
                                for _ in 0..5 {
                                    hex.push(iter.next()?);
                                }

                                let r = u8::from_str_radix(
                                    &hex.iter().take(2).collect::<String>(),
                                    16,
                                )
                                .ok()?;
                                let g = u8::from_str_radix(
                                    &hex.iter()
                                        .skip(2)
                                        .take(2)
                                        .collect::<String>(),
                                    16,
                                )
                                .ok()?;
                                let b = u8::from_str_radix(
                                    &hex.iter()
                                        .skip(4)
                                        .take(2)
                                        .collect::<String>(),
                                    16,
                                )
                                .ok()?;

                                *bg = Some(Color::Rgb(r, g, b));
                            }
                            // Nope, just a normal char
                            else {
                                current_text.push(comma);
                            }
                        }
                    } else {
                        *fg = None;
                        *bg = None;
                    }
                }
                m => {
                    if modifiers.contains(&m) {
                        modifiers.remove(&m);
                    } else {
                        modifiers.insert(m);
                    }
                }
            }
        } else {
            current_text.push(c);
        }
    }

    if !current_text.is_empty() {
        let text = mem::take(&mut current_text);

        if modifiers.is_empty() && fg.is_none() && bg.is_none() {
            fragments.push(Fragment::Unformatted(text));
        } else {
            fragments.push(Fragment::Formatted(
                text,
                Formatting::new(modifiers, *fg, *bg),
            ));
        }
    }

    // Only return None if parsing failed; return an empty fragments if there
    // are no non-formatting characters after parsing
    Some(fragments)
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Formatting {
    pub bold: bool,
    pub italics: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub monospace: bool,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

impl Formatting {
    fn new(
        modifiers: &HashSet<Modifier>,
        fg: Option<Color>,
        bg: Option<Color>,
    ) -> Self {
        let (fg, bg) = if modifiers.contains(&Modifier::ReverseColor) {
            (bg, fg)
        } else {
            (fg, bg)
        };

        Self {
            bold: modifiers.contains(&Modifier::Bold),
            italics: modifiers.contains(&Modifier::Italics),
            underline: modifiers.contains(&Modifier::Underline),
            strikethrough: modifiers.contains(&Modifier::Strikethrough),
            monospace: modifiers.contains(&Modifier::Monospace),
            fg,
            bg,
        }
    }
}

#[derive(Debug)]
pub enum Fragment {
    Unformatted(String),
    Formatted(String, Formatting),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Modifier {
    Bold = 0x02,
    Italics = 0x1D,
    Underline = 0x1F,
    Strikethrough = 0x1E,
    Monospace = 0x11,
    Color = 0x03,
    HexColor = 0x04,
    ReverseColor = 0x16,
    Reset = 0x0F,
}

impl Modifier {
    fn char(&self) -> char {
        *self as u8 as char
    }
}

impl TryFrom<char> for Modifier {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let Ok(byte) = u8::try_from(value) else {
            return Err(());
        };

        Ok(match byte {
            0x02 => Self::Bold,
            0x1D => Self::Italics,
            0x1F => Self::Underline,
            0x1E => Self::Strikethrough,
            0x11 => Self::Monospace,
            0x03 => Self::Color,
            0x04 => Self::HexColor,
            0x16 => Self::ReverseColor,
            0x0F => Self::Reset,
            _ => return Err(()),
        })
    }
}

/// https://modern.ircdocs.horse/formatting.html#colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black,
    Blue,
    Green,
    Red,
    Brown,
    Magenta,
    Orange,
    Yellow,
    LightGreen,
    Cyan,
    LightCyan,
    LightBlue,
    Pink,
    Grey,
    LightGrey,
    Code16,
    Code17,
    Code18,
    Code19,
    Code20,
    Code21,
    Code22,
    Code23,
    Code24,
    Code25,
    Code26,
    Code27,
    Code28,
    Code29,
    Code30,
    Code31,
    Code32,
    Code33,
    Code34,
    Code35,
    Code36,
    Code37,
    Code38,
    Code39,
    Code40,
    Code41,
    Code42,
    Code43,
    Code44,
    Code45,
    Code46,
    Code47,
    Code48,
    Code49,
    Code50,
    Code51,
    Code52,
    Code53,
    Code54,
    Code55,
    Code56,
    Code57,
    Code58,
    Code59,
    Code60,
    Code61,
    Code62,
    Code63,
    Code64,
    Code65,
    Code66,
    Code67,
    Code68,
    Code69,
    Code70,
    Code71,
    Code72,
    Code73,
    Code74,
    Code75,
    Code76,
    Code77,
    Code78,
    Code79,
    Code80,
    Code81,
    Code82,
    Code83,
    Code84,
    Code85,
    Code86,
    Code87,
    Code88,
    Code89,
    Code90,
    Code91,
    Code92,
    Code93,
    Code94,
    Code95,
    Code96,
    Code97,
    Code98,
    Default,
    Rgb(u8, u8, u8),
}

impl Color {
    fn code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::White),
            1 => Some(Self::Black),
            2 => Some(Self::Blue),
            3 => Some(Self::Green),
            4 => Some(Self::Red),
            5 => Some(Self::Brown),
            6 => Some(Self::Magenta),
            7 => Some(Self::Orange),
            8 => Some(Self::Yellow),
            9 => Some(Self::LightGreen),
            10 => Some(Self::Cyan),
            11 => Some(Self::LightCyan),
            12 => Some(Self::LightBlue),
            13 => Some(Self::Pink),
            14 => Some(Self::Grey),
            15 => Some(Self::LightGrey),
            16 => Some(Self::Code16),
            17 => Some(Self::Code17),
            18 => Some(Self::Code18),
            19 => Some(Self::Code19),
            20 => Some(Self::Code20),
            21 => Some(Self::Code21),
            22 => Some(Self::Code22),
            23 => Some(Self::Code23),
            24 => Some(Self::Code24),
            25 => Some(Self::Code25),
            26 => Some(Self::Code26),
            27 => Some(Self::Code27),
            28 => Some(Self::Code28),
            29 => Some(Self::Code29),
            30 => Some(Self::Code30),
            31 => Some(Self::Code31),
            32 => Some(Self::Code32),
            33 => Some(Self::Code33),
            34 => Some(Self::Code34),
            35 => Some(Self::Code35),
            36 => Some(Self::Code36),
            37 => Some(Self::Code37),
            38 => Some(Self::Code38),
            39 => Some(Self::Code39),
            40 => Some(Self::Code40),
            41 => Some(Self::Code41),
            42 => Some(Self::Code42),
            43 => Some(Self::Code43),
            44 => Some(Self::Code44),
            45 => Some(Self::Code45),
            46 => Some(Self::Code46),
            47 => Some(Self::Code47),
            48 => Some(Self::Code48),
            49 => Some(Self::Code49),
            50 => Some(Self::Code50),
            51 => Some(Self::Code51),
            52 => Some(Self::Code52),
            53 => Some(Self::Code53),
            54 => Some(Self::Code54),
            55 => Some(Self::Code55),
            56 => Some(Self::Code56),
            57 => Some(Self::Code57),
            58 => Some(Self::Code58),
            59 => Some(Self::Code59),
            60 => Some(Self::Code60),
            61 => Some(Self::Code61),
            62 => Some(Self::Code62),
            63 => Some(Self::Code63),
            64 => Some(Self::Code64),
            65 => Some(Self::Code65),
            66 => Some(Self::Code66),
            67 => Some(Self::Code67),
            68 => Some(Self::Code68),
            69 => Some(Self::Code69),
            70 => Some(Self::Code70),
            71 => Some(Self::Code71),
            72 => Some(Self::Code72),
            73 => Some(Self::Code73),
            74 => Some(Self::Code74),
            75 => Some(Self::Code75),
            76 => Some(Self::Code76),
            77 => Some(Self::Code77),
            78 => Some(Self::Code78),
            79 => Some(Self::Code79),
            80 => Some(Self::Code80),
            81 => Some(Self::Code81),
            82 => Some(Self::Code82),
            83 => Some(Self::Code83),
            84 => Some(Self::Code84),
            85 => Some(Self::Code85),
            86 => Some(Self::Code86),
            87 => Some(Self::Code87),
            88 => Some(Self::Code88),
            89 => Some(Self::Code89),
            90 => Some(Self::Code90),
            91 => Some(Self::Code91),
            92 => Some(Self::Code92),
            93 => Some(Self::Code93),
            94 => Some(Self::Code94),
            95 => Some(Self::Code95),
            96 => Some(Self::Code96),
            97 => Some(Self::Code97),
            98 => Some(Self::Code98),
            99 => Some(Self::Default),
            _ => None,
        }
    }

    fn digit(self) -> u8 {
        match self {
            Color::White => 0,
            Color::Black => 1,
            Color::Blue => 2,
            Color::Green => 3,
            Color::Red => 4,
            Color::Brown => 5,
            Color::Magenta => 6,
            Color::Orange => 7,
            Color::Yellow => 8,
            Color::LightGreen => 9,
            Color::Cyan => 10,
            Color::LightCyan => 11,
            Color::LightBlue => 12,
            Color::Pink => 13,
            Color::Grey => 14,
            Color::LightGrey => 15,
            Color::Code16 => 16,
            Color::Code17 => 17,
            Color::Code18 => 18,
            Color::Code19 => 19,
            Color::Code20 => 20,
            Color::Code21 => 21,
            Color::Code22 => 22,
            Color::Code23 => 23,
            Color::Code24 => 24,
            Color::Code25 => 25,
            Color::Code26 => 26,
            Color::Code27 => 27,
            Color::Code28 => 28,
            Color::Code29 => 29,
            Color::Code30 => 30,
            Color::Code31 => 31,
            Color::Code32 => 32,
            Color::Code33 => 33,
            Color::Code34 => 34,
            Color::Code35 => 35,
            Color::Code36 => 36,
            Color::Code37 => 37,
            Color::Code38 => 38,
            Color::Code39 => 39,
            Color::Code40 => 40,
            Color::Code41 => 41,
            Color::Code42 => 42,
            Color::Code43 => 43,
            Color::Code44 => 44,
            Color::Code45 => 45,
            Color::Code46 => 46,
            Color::Code47 => 47,
            Color::Code48 => 48,
            Color::Code49 => 49,
            Color::Code50 => 50,
            Color::Code51 => 51,
            Color::Code52 => 52,
            Color::Code53 => 53,
            Color::Code54 => 54,
            Color::Code55 => 55,
            Color::Code56 => 56,
            Color::Code57 => 57,
            Color::Code58 => 58,
            Color::Code59 => 59,
            Color::Code60 => 60,
            Color::Code61 => 61,
            Color::Code62 => 62,
            Color::Code63 => 63,
            Color::Code64 => 64,
            Color::Code65 => 65,
            Color::Code66 => 66,
            Color::Code67 => 67,
            Color::Code68 => 68,
            Color::Code69 => 69,
            Color::Code70 => 70,
            Color::Code71 => 71,
            Color::Code72 => 72,
            Color::Code73 => 73,
            Color::Code74 => 74,
            Color::Code75 => 75,
            Color::Code76 => 76,
            Color::Code77 => 77,
            Color::Code78 => 78,
            Color::Code79 => 79,
            Color::Code80 => 80,
            Color::Code81 => 81,
            Color::Code82 => 82,
            Color::Code83 => 83,
            Color::Code84 => 84,
            Color::Code85 => 85,
            Color::Code86 => 86,
            Color::Code87 => 87,
            Color::Code88 => 88,
            Color::Code89 => 89,
            Color::Code90 => 90,
            Color::Code91 => 91,
            Color::Code92 => 92,
            Color::Code93 => 93,
            Color::Code94 => 94,
            Color::Code95 => 95,
            Color::Code96 => 96,
            Color::Code97 => 97,
            Color::Code98 => 98,
            Color::Default => 99,
            // Can only be used w/ HexColor encoding
            Color::Rgb(_, _, _) => u8::MAX,
        }
    }

    pub fn into_iced(self, styles: &theme::Styles) -> Option<iced_core::Color> {
        match self {
            Color::White => styles.formatting.white.or(Some(color!(0xffffff))),
            Color::Black => styles.formatting.black.or(Some(color!(0x000000))),
            Color::Blue => styles.formatting.blue.or(Some(color!(0x00007f))),
            Color::Green => styles.formatting.green.or(Some(color!(0x009300))),
            Color::Red => styles.formatting.red.or(Some(color!(0xff0000))),
            Color::Brown => styles.formatting.brown.or(Some(color!(0x7f0000))),
            Color::Magenta => {
                styles.formatting.magenta.or(Some(color!(0x9c009c)))
            }
            Color::Orange => {
                styles.formatting.orange.or(Some(color!(0xfc7f00)))
            }
            Color::Yellow => {
                styles.formatting.yellow.or(Some(color!(0xffff00)))
            }
            Color::LightGreen => {
                styles.formatting.lightgreen.or(Some(color!(0x00fc00)))
            }
            Color::Cyan => styles.formatting.cyan.or(Some(color!(0x009393))),
            Color::LightCyan => {
                styles.formatting.lightcyan.or(Some(color!(0x00ffff)))
            }
            Color::LightBlue => {
                styles.formatting.lightblue.or(Some(color!(0x0000fc)))
            }
            Color::Pink => styles.formatting.pink.or(Some(color!(0xff00ff))),
            Color::Grey => styles.formatting.grey.or(Some(color!(0x7f7f7f))),
            Color::LightGrey => {
                styles.formatting.lightgrey.or(Some(color!(0xd2d2d2)))
            }
            Color::Code16 => Some(color!(0x470000)),
            Color::Code17 => Some(color!(0x472100)),
            Color::Code18 => Some(color!(0x474700)),
            Color::Code19 => Some(color!(0x324700)),
            Color::Code20 => Some(color!(0x004700)),
            Color::Code21 => Some(color!(0x00472c)),
            Color::Code22 => Some(color!(0x004747)),
            Color::Code23 => Some(color!(0x002747)),
            Color::Code24 => Some(color!(0x000047)),
            Color::Code25 => Some(color!(0x2e0047)),
            Color::Code26 => Some(color!(0x470047)),
            Color::Code27 => Some(color!(0x47002a)),
            Color::Code28 => Some(color!(0x740000)),
            Color::Code29 => Some(color!(0x743a00)),
            Color::Code30 => Some(color!(0x747400)),
            Color::Code31 => Some(color!(0x517400)),
            Color::Code32 => Some(color!(0x007400)),
            Color::Code33 => Some(color!(0x007449)),
            Color::Code34 => Some(color!(0x007474)),
            Color::Code35 => Some(color!(0x004074)),
            Color::Code36 => Some(color!(0x000074)),
            Color::Code37 => Some(color!(0x4b0074)),
            Color::Code38 => Some(color!(0x740074)),
            Color::Code39 => Some(color!(0x740045)),
            Color::Code40 => Some(color!(0xb50000)),
            Color::Code41 => Some(color!(0xb56300)),
            Color::Code42 => Some(color!(0xb5b500)),
            Color::Code43 => Some(color!(0x7db500)),
            Color::Code44 => Some(color!(0x00b500)),
            Color::Code45 => Some(color!(0x00b571)),
            Color::Code46 => Some(color!(0x00b5b5)),
            Color::Code47 => Some(color!(0x0063b5)),
            Color::Code48 => Some(color!(0x0000b5)),
            Color::Code49 => Some(color!(0x7500b5)),
            Color::Code50 => Some(color!(0xb500b5)),
            Color::Code51 => Some(color!(0xb5006b)),
            Color::Code52 => Some(color!(0xff0000)),
            Color::Code53 => Some(color!(0xff8c00)),
            Color::Code54 => Some(color!(0xffff00)),
            Color::Code55 => Some(color!(0xb2ff00)),
            Color::Code56 => Some(color!(0x00ff00)),
            Color::Code57 => Some(color!(0x00ffa0)),
            Color::Code58 => Some(color!(0x00ffff)),
            Color::Code59 => Some(color!(0x008cff)),
            Color::Code60 => Some(color!(0x0000ff)),
            Color::Code61 => Some(color!(0xa500ff)),
            Color::Code62 => Some(color!(0xff00ff)),
            Color::Code63 => Some(color!(0xff0098)),
            Color::Code64 => Some(color!(0xff5959)),
            Color::Code65 => Some(color!(0xffb459)),
            Color::Code66 => Some(color!(0xffff71)),
            Color::Code67 => Some(color!(0xcfff60)),
            Color::Code68 => Some(color!(0x6fff6f)),
            Color::Code69 => Some(color!(0x65ffc9)),
            Color::Code70 => Some(color!(0x6dffff)),
            Color::Code71 => Some(color!(0x59b4ff)),
            Color::Code72 => Some(color!(0x5959ff)),
            Color::Code73 => Some(color!(0xc459ff)),
            Color::Code74 => Some(color!(0xff66ff)),
            Color::Code75 => Some(color!(0xff59bc)),
            Color::Code76 => Some(color!(0xff9c9c)),
            Color::Code77 => Some(color!(0xffd39c)),
            Color::Code78 => Some(color!(0xffff9c)),
            Color::Code79 => Some(color!(0xe2ff9c)),
            Color::Code80 => Some(color!(0x9cff9c)),
            Color::Code81 => Some(color!(0x9cffdb)),
            Color::Code82 => Some(color!(0x9cffff)),
            Color::Code83 => Some(color!(0x9cd3ff)),
            Color::Code84 => Some(color!(0x9c9cff)),
            Color::Code85 => Some(color!(0xdc9cff)),
            Color::Code86 => Some(color!(0xff9cff)),
            Color::Code87 => Some(color!(0xff94d3)),
            Color::Code88 => Some(color!(0x000000)),
            Color::Code89 => Some(color!(0x131313)),
            Color::Code90 => Some(color!(0x282828)),
            Color::Code91 => Some(color!(0x363636)),
            Color::Code92 => Some(color!(0x4d4d4d)),
            Color::Code93 => Some(color!(0x656565)),
            Color::Code94 => Some(color!(0x818181)),
            Color::Code95 => Some(color!(0x9f9f9f)),
            Color::Code96 => Some(color!(0xbcbcbc)),
            Color::Code97 => Some(color!(0xe2e2e2)),
            Color::Code98 => Some(color!(0xffffff)),
            Color::Default => None,
            Color::Rgb(r, g, b) => Some(iced_core::Color::from_rgb8(r, g, b)),
        }
    }
}
