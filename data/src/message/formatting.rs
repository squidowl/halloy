use std::{collections::HashSet, mem};

use iced_core::color;
use itertools::PeekingNext;
use serde::{Deserialize, Serialize};

use crate::theme;

pub fn parse(text: &str) -> Option<Vec<Fragment>> {
    let mut fragments = vec![];

    let mut current_text = String::new();
    let mut modifiers = HashSet::new();
    let mut fg = None;
    let mut bg = None;

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
                        Formatting::new(&modifiers, fg, bg),
                    ));
                }
            }

            match modifier {
                Modifier::Reset => {
                    modifiers.clear();
                    fg = None;
                    bg = None;
                }
                Modifier::Color => {
                    // Trailing digit for new color, otherwise resets
                    if let Some(c) = iter.peeking_next(char::is_ascii_digit) {
                        // 1-2 digiits
                        let mut digits = c.to_string();
                        if let Some(c) = iter.peeking_next(char::is_ascii_digit) {
                            digits.push(c);
                        }

                        let code = digits.parse().ok()?;

                        fg = Color::code(code);

                        if let Some(comma) = iter.peeking_next(|c| *c == ',') {
                            // Has background
                            if let Some(c) = iter.peeking_next(char::is_ascii_digit) {
                                // 1-2 digits
                                let mut digits = c.to_string();
                                if let Some(c) = iter.peeking_next(char::is_ascii_digit) {
                                    digits.push(c);
                                }

                                let code = digits.parse().ok()?;

                                bg = Color::code(code);
                            }
                            // Nope, just a normal char
                            else {
                                current_text.push(comma);
                            }
                        }
                    } else {
                        fg = None;
                        bg = None;
                    }
                }
                Modifier::HexColor => {
                    // Trailing digit for new color, otherwise resets
                    if let Some(c) = iter.peeking_next(char::is_ascii_hexdigit) {
                        // 6 digits (hex)
                        let mut hex = c.to_string();
                        for _ in 0..5 {
                            hex.push(iter.next()?);
                        }

                        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

                        fg = Some(Color::Rgb(r, g, b));

                        if let Some(comma) = iter.peeking_next(|c| *c == ',') {
                            // Has background
                            if let Some(c) = iter.peeking_next(char::is_ascii_hexdigit) {
                                // 6 digits (hex)
                                let mut hex = c.to_string();
                                for _ in 0..5 {
                                    hex.push(iter.next()?);
                                }

                                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

                                bg = Some(Color::Rgb(r, g, b));
                            }
                            // Nope, just a normal char
                            else {
                                current_text.push(comma);
                            }
                        }
                    } else {
                        fg = None;
                        bg = None;
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
                Formatting::new(&modifiers, fg, bg),
            ));
        }
    }

    if fragments.is_empty()
        || (fragments.len() == 1 && matches!(fragments.first(), Some(Fragment::Unformatted(_))))
    {
        None
    } else {
        Some(fragments)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    fn new(modifiers: &HashSet<Modifier>, fg: Option<Color>, bg: Option<Color>) -> Self {
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
enum Modifier {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    White,
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
    Rgb(u8, u8, u8),
    Default,
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
            16 => Some(Self::hex(0x470000)),
            17 => Some(Self::hex(0x472100)),
            18 => Some(Self::hex(0x474700)),
            19 => Some(Self::hex(0x324700)),
            20 => Some(Self::hex(0x004700)),
            21 => Some(Self::hex(0x00472c)),
            22 => Some(Self::hex(0x004747)),
            23 => Some(Self::hex(0x002747)),
            24 => Some(Self::hex(0x000047)),
            25 => Some(Self::hex(0x2e0047)),
            26 => Some(Self::hex(0x470047)),
            27 => Some(Self::hex(0x47002a)),
            28 => Some(Self::hex(0x740000)),
            29 => Some(Self::hex(0x743a00)),
            30 => Some(Self::hex(0x747400)),
            31 => Some(Self::hex(0x517400)),
            32 => Some(Self::hex(0x007400)),
            33 => Some(Self::hex(0x007449)),
            34 => Some(Self::hex(0x007474)),
            35 => Some(Self::hex(0x004074)),
            36 => Some(Self::hex(0x000074)),
            37 => Some(Self::hex(0x4b0074)),
            38 => Some(Self::hex(0x740074)),
            39 => Some(Self::hex(0x740045)),
            40 => Some(Self::hex(0xb50000)),
            41 => Some(Self::hex(0xb56300)),
            42 => Some(Self::hex(0xb5b500)),
            43 => Some(Self::hex(0x7db500)),
            44 => Some(Self::hex(0x00b500)),
            45 => Some(Self::hex(0x00b571)),
            46 => Some(Self::hex(0x00b5b5)),
            47 => Some(Self::hex(0x0063b5)),
            48 => Some(Self::hex(0x0000b5)),
            49 => Some(Self::hex(0x7500b5)),
            50 => Some(Self::hex(0xb500b5)),
            51 => Some(Self::hex(0xb5006b)),
            52 => Some(Self::hex(0xff0000)),
            53 => Some(Self::hex(0xff8c00)),
            54 => Some(Self::hex(0xffff00)),
            55 => Some(Self::hex(0xb2ff00)),
            56 => Some(Self::hex(0x00ff00)),
            57 => Some(Self::hex(0x00ffa0)),
            58 => Some(Self::hex(0x00ffff)),
            59 => Some(Self::hex(0x008cff)),
            60 => Some(Self::hex(0x0000ff)),
            61 => Some(Self::hex(0xa500ff)),
            62 => Some(Self::hex(0xff00ff)),
            63 => Some(Self::hex(0xff0098)),
            64 => Some(Self::hex(0xff5959)),
            65 => Some(Self::hex(0xffb459)),
            66 => Some(Self::hex(0xffff71)),
            67 => Some(Self::hex(0xcfff60)),
            68 => Some(Self::hex(0x6fff6f)),
            69 => Some(Self::hex(0x65ffc9)),
            70 => Some(Self::hex(0x6dffff)),
            71 => Some(Self::hex(0x59b4ff)),
            72 => Some(Self::hex(0x5959ff)),
            73 => Some(Self::hex(0xc459ff)),
            74 => Some(Self::hex(0xff66ff)),
            75 => Some(Self::hex(0xff59bc)),
            76 => Some(Self::hex(0xff9c9c)),
            77 => Some(Self::hex(0xffd39c)),
            78 => Some(Self::hex(0xffff9c)),
            79 => Some(Self::hex(0xe2ff9c)),
            80 => Some(Self::hex(0x9cff9c)),
            81 => Some(Self::hex(0x9cffdb)),
            82 => Some(Self::hex(0x9cffff)),
            83 => Some(Self::hex(0x9cd3ff)),
            84 => Some(Self::hex(0x9c9cff)),
            85 => Some(Self::hex(0xdc9cff)),
            86 => Some(Self::hex(0xff9cff)),
            87 => Some(Self::hex(0xff94d3)),
            88 => Some(Self::hex(0x000000)),
            89 => Some(Self::hex(0x131313)),
            90 => Some(Self::hex(0x282828)),
            91 => Some(Self::hex(0x363636)),
            92 => Some(Self::hex(0x4d4d4d)),
            93 => Some(Self::hex(0x656565)),
            94 => Some(Self::hex(0x818181)),
            95 => Some(Self::hex(0x9f9f9f)),
            96 => Some(Self::hex(0xbcbcbc)),
            97 => Some(Self::hex(0xe2e2e2)),
            98 => Some(Self::hex(0xffffff)),
            99 => Some(Self::Default),
            _ => None,
        }
    }

    fn hex(hex: u32) -> Self {
        let r = (hex & 0xff0000) >> 16;
        let g = (hex & 0xff00) >> 8;
        let b = hex & 0xff;

        Self::Rgb(r as u8, g as u8, b as u8)
    }

    pub fn into_iced(self, _colors: &theme::Colors) -> Option<iced_core::Color> {
        // TODO: Theme aware 0 - 15 colors
        match self {
            Color::White => Some(color!(0xffffff)),
            Color::Black => Some(color!(0x000000)),
            Color::Blue => Some(color!(0x00007f)),
            Color::Green => Some(color!(0x009300)),
            Color::Red => Some(color!(0xff0000)),
            Color::Brown => Some(color!(0x7f0000)),
            Color::Magenta => Some(color!(0x9c009c)),
            Color::Orange => Some(color!(0xfc7f00)),
            Color::Yellow => Some(color!(0xffff00)),
            Color::LightGreen => Some(color!(0x00fc00)),
            Color::Cyan => Some(color!(0x009393)),
            Color::LightCyan => Some(color!(0x00ffff)),
            Color::LightBlue => Some(color!(0x0000fc)),
            Color::Pink => Some(color!(0xff00ff)),
            Color::Grey => Some(color!(0x7f7f7f)),
            Color::LightGrey => Some(color!(0xd2d2d2)),
            Color::Rgb(r, g, b) => Some(iced_core::Color::from_rgb8(r, g, b)),
            Color::Default => None,
        }
    }
}
