use std::path::PathBuf;

use base64::Engine;
use iced_core::Color;
use palette::rgb::{Rgb, Rgba};
use palette::{FromColor, Hsva, Okhsl, Srgb, Srgba};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;

use crate::compression;

const DEFAULT_THEME_NAME: &str = "Ferra";
const DEFAULT_THEME_CONTENT: &str = include_str!("../../assets/themes/ferra.toml");

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: Colors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: DEFAULT_THEME_NAME.to_string(),
            colors: Colors::default(),
        }
    }
}

impl Theme {
    pub fn new(name: String, colors: Colors) -> Self {
        Theme { name, colors }
    }
}

// IMPORTANT: Make sure any new components are added to the theme editor
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Colors {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub text: Text,
    #[serde(default)]
    pub buffer: Buffer,
    #[serde(default)]
    pub buttons: Buttons,
}

impl Colors {
    pub async fn save(self, path: PathBuf) -> Result<(), Error> {
        let content = toml::to_string(&self)?;

        fs::write(path, &content).await?;

        Ok(())
    }

    pub fn encode_base64(&self) -> String {
        let Ok(compressed) = compression::compress(&compact::Colors::from(*self)) else {
            // "Impossible" state
            return String::new();
        };

        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&compressed)
    }

    pub fn decode_base64(content: &str) -> Result<Self, Error> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(content)?;

        compression::decompress::<compact::Colors>(&bytes)
            .map_err(Error::Decompress)
            .map(Self::from)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to serialize theme to toml: {0}")]
    Encode(#[from] toml::ser::Error),
    #[error("Failed to write theme file: {0}")]
    Write(#[from] std::io::Error),
    #[error("Failed to decode base64 theme string: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("Failed to decompress theme: {0}")]
    Decompress(#[source] compression::Error),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Buttons {
    #[serde(default)]
    pub primary: Button,
    #[serde(default)]
    pub secondary: Button,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Button {
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background_hover: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background_selected: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background_selected_hover: Color,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct General {
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub border: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub horizontal_rule: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub unread_indicator: Color,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Buffer {
    #[serde(default = "default_transparent", with = "color_serde")]
    pub action: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background_text_input: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background_title_bar: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub border: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub border_selected: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub code: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub highlight: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub nickname: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub selection: Color,
    #[serde(default)]
    pub server_messages: ServerMessages,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub timestamp: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub topic: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub url: Color,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ServerMessages {
    #[serde(default, with = "color_serde_maybe")]
    pub join: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub part: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub quit: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub reply_topic: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub change_host: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub monitored_online: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub monitored_offline: Option<Color>,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub default: Color,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Text {
    #[serde(default = "default_transparent", with = "color_serde")]
    pub primary: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub secondary: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub tertiary: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub success: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub error: Color,
}

impl Default for Colors {
    fn default() -> Self {
        toml::from_str(DEFAULT_THEME_CONTENT).expect("parse default theme")
    }
}

pub fn hex_to_color(hex: &str) -> Option<Color> {
    if hex.len() == 7 || hex.len() == 9 {
        let hash = &hex[0..1];
        let r = u8::from_str_radix(&hex[1..3], 16);
        let g = u8::from_str_radix(&hex[3..5], 16);
        let b = u8::from_str_radix(&hex[5..7], 16);
        let a = (hex.len() == 9)
            .then(|| u8::from_str_radix(&hex[7..9], 16).ok())
            .flatten();

        return match (hash, r, g, b, a) {
            ("#", Ok(r), Ok(g), Ok(b), None) => Some(Color {
                r: r as f32 / 255.0,
                g: g as f32 / 255.0,
                b: b as f32 / 255.0,
                a: 1.0,
            }),
            ("#", Ok(r), Ok(g), Ok(b), Some(a)) => Some(Color {
                r: r as f32 / 255.0,
                g: g as f32 / 255.0,
                b: b as f32 / 255.0,
                a: a as f32 / 255.0,
            }),
            _ => None,
        };
    }

    None
}

pub fn color_to_hex(color: Color) -> String {
    use std::fmt::Write;

    let mut hex = String::with_capacity(9);

    let [r, g, b, a] = color.into_rgba8();

    let _ = write!(&mut hex, "#");
    let _ = write!(&mut hex, "{:02X}", r);
    let _ = write!(&mut hex, "{:02X}", g);
    let _ = write!(&mut hex, "{:02X}", b);

    if a < u8::MAX {
        let _ = write!(&mut hex, "{:02X}", a);
    }

    hex
}

/// Adjusts the transparency of the foreground color based on the background color's lightness.
pub fn alpha_color(min_alpha: f32, max_alpha: f32, background: Color, foreground: Color) -> Color {
    alpha(
        foreground,
        min_alpha + to_hsl(background).lightness * (max_alpha - min_alpha),
    )
}

/// Randomizes the hue value of an `iced::Color` based on a seed.
pub fn randomize_color(original_color: Color, seed: &str) -> Color {
    // Generate a 64-bit hash from the seed string
    let seed_hash = seahash::hash(seed.as_bytes());

    // Create a random number generator from the seed
    let mut rng = ChaChaRng::seed_from_u64(seed_hash);

    // Convert the original color to HSL
    let original_hsl = to_hsl(original_color);

    // Randomize the hue value using the random number generator
    let randomized_hue: f32 = rng.gen_range(0.0..=360.0);
    let randomized_hsl = Okhsl::new(
        randomized_hue,
        original_hsl.saturation,
        original_hsl.lightness,
    );

    // Convert the randomized HSL color back to Color
    from_hsl(randomized_hsl)
}

pub fn to_hsl(color: Color) -> Okhsl {
    let mut hsl = Okhsl::from_color(Rgb::from(color));
    if hsl.saturation.is_nan() {
        hsl.saturation = Okhsl::max_saturation();
    }

    hsl
}

pub fn to_hsva(color: Color) -> Hsva {
    Hsva::from_color(Rgba::from(color))
}

pub fn from_hsva(color: Hsva) -> Color {
    Srgba::from_color(color).into()
}

pub fn from_hsl(hsl: Okhsl) -> Color {
    Srgb::from_color(hsl).into()
}

pub fn alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn default_transparent() -> Color {
    Color::TRANSPARENT
}

mod color_serde {
    use iced_core::Color;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(String::deserialize(deserializer)
            .map(|hex| super::hex_to_color(&hex))?
            .unwrap_or(Color::TRANSPARENT))
    }

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::color_to_hex(*color).serialize(serializer)
    }
}

mod color_serde_maybe {
    use iced_core::Color;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<String>::deserialize(deserializer)?.and_then(|hex| super::hex_to_color(&hex)))
    }

    pub fn serialize<S>(color: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        color.map(super::color_to_hex).serialize(serializer)
    }
}

mod compact {
    use iced_core::Color;
    use serde::{Deserialize, Serialize};

    use super::default_transparent;

    impl From<Colors> for super::Colors {
        fn from(colors: Colors) -> Self {
            super::Colors {
                general: super::General {
                    background: colors.g.ba,
                    border: colors.g.bo,
                    horizontal_rule: colors.g.hr,
                    unread_indicator: colors.g.ui,
                },
                text: super::Text {
                    primary: colors.t.p,
                    secondary: colors.t.se,
                    tertiary: colors.t.t,
                    success: colors.t.su,
                    error: colors.t.e,
                },
                buffer: super::Buffer {
                    action: colors.bf.a,
                    background: colors.bf.ba,
                    background_text_input: colors.bf.bati,
                    background_title_bar: colors.bf.batb,
                    border: colors.bf.bo,
                    border_selected: colors.bf.bos,
                    code: colors.bf.c,
                    highlight: colors.bf.h,
                    nickname: colors.bf.n,
                    selection: colors.bf.sl,
                    server_messages: super::ServerMessages {
                        join: colors.bf.sm.j,
                        part: colors.bf.sm.p,
                        quit: colors.bf.sm.q,
                        reply_topic: colors.bf.sm.rt,
                        change_host: colors.bf.sm.ch,
                        monitored_online: colors.bf.sm.mon,
                        monitored_offline: colors.bf.sm.mof,
                        default: colors.bf.sm.d,
                    },
                    timestamp: colors.bf.ti,
                    topic: colors.bf.to,
                    url: colors.bf.u,
                },
                buttons: super::Buttons {
                    primary: super::Button {
                        background: colors.bt.p.b,
                        background_hover: colors.bt.p.bh,
                        background_selected: colors.bt.p.bs,
                        background_selected_hover: colors.bt.p.bsh,
                    },
                    secondary: super::Button {
                        background: colors.bt.s.b,
                        background_hover: colors.bt.s.bh,
                        background_selected: colors.bt.s.bs,
                        background_selected_hover: colors.bt.s.bsh,
                    },
                },
            }
        }
    }

    impl From<super::Colors> for Colors {
        fn from(colors: super::Colors) -> Self {
            Colors {
                g: General {
                    ba: colors.general.background,
                    bo: colors.general.border,
                    hr: colors.general.horizontal_rule,
                    ui: colors.general.unread_indicator,
                },
                t: Text {
                    p: colors.text.primary,
                    se: colors.text.secondary,
                    t: colors.text.tertiary,
                    su: colors.text.success,
                    e: colors.text.error,
                },
                bf: Buffer {
                    a: colors.buffer.action,
                    ba: colors.buffer.background,
                    bati: colors.buffer.background_text_input,
                    batb: colors.buffer.background_title_bar,
                    bo: colors.buffer.border,
                    bos: colors.buffer.border_selected,
                    c: colors.buffer.code,
                    h: colors.buffer.highlight,
                    n: colors.buffer.nickname,
                    sl: colors.buffer.selection,
                    sm: ServerMessages {
                        j: colors.buffer.server_messages.join,
                        p: colors.buffer.server_messages.part,
                        q: colors.buffer.server_messages.quit,
                        rt: colors.buffer.server_messages.reply_topic,
                        ch: colors.buffer.server_messages.change_host,
                        mon: colors.buffer.server_messages.monitored_online,
                        mof: colors.buffer.server_messages.monitored_offline,
                        d: colors.buffer.server_messages.default,
                    },
                    ti: colors.buffer.timestamp,
                    to: colors.buffer.topic,
                    u: colors.buffer.url,
                },
                bt: Buttons {
                    p: Button {
                        b: colors.buttons.primary.background,
                        bh: colors.buttons.primary.background_hover,
                        bs: colors.buttons.primary.background_selected,
                        bsh: colors.buttons.primary.background_selected_hover,
                    },
                    s: Button {
                        b: colors.buttons.secondary.background,
                        bh: colors.buttons.secondary.background_hover,
                        bs: colors.buttons.secondary.background_selected,
                        bsh: colors.buttons.secondary.background_selected_hover,
                    },
                },
            }
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct Colors {
        #[serde(default)]
        pub g: General,
        #[serde(default)]
        pub t: Text,
        #[serde(default)]
        pub bf: Buffer,
        #[serde(default)]
        pub bt: Buttons,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct Buttons {
        #[serde(default)]
        pub p: Button,
        #[serde(default)]
        pub s: Button,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct Button {
        #[serde(default = "default_transparent", with = "color_serde")]
        pub b: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub bh: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub bs: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub bsh: Color,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct General {
        #[serde(default = "default_transparent", with = "color_serde")]
        pub ba: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub bo: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub hr: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub ui: Color,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct Buffer {
        #[serde(default = "default_transparent", with = "color_serde")]
        pub a: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub ba: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub bati: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub batb: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub bo: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub bos: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub c: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub h: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub n: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub sl: Color,
        #[serde(default)]
        pub sm: ServerMessages,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub ti: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub to: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub u: Color,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct ServerMessages {
        #[serde(default, with = "color_serde_maybe")]
        pub j: Option<Color>,
        #[serde(default, with = "color_serde_maybe")]
        pub p: Option<Color>,
        #[serde(default, with = "color_serde_maybe")]
        pub q: Option<Color>,
        #[serde(default, with = "color_serde_maybe")]
        pub rt: Option<Color>,
        #[serde(default, with = "color_serde_maybe")]
        pub ch: Option<Color>,
        #[serde(default, with = "color_serde_maybe")]
        pub mon: Option<Color>,
        #[serde(default, with = "color_serde_maybe")]
        pub mof: Option<Color>,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub d: Color,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct Text {
        #[serde(default = "default_transparent", with = "color_serde")]
        pub p: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub se: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub t: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub su: Color,
        #[serde(default = "default_transparent", with = "color_serde")]
        pub e: Color,
    }

    mod color_serde {
        use iced_core::Color;
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
        where
            D: Deserializer<'de>,
        {
            u32::deserialize(deserializer).map(|int| {
                let [r, g, b, a] = int.to_be_bytes();
                Color::from_rgba8(r, g, b, a as f32 / 255.0)
            })
        }

        pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            u32::from_be_bytes(color.into_rgba8()).serialize(serializer)
        }
    }

    mod color_serde_maybe {
        use iced_core::Color;
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
        where
            D: Deserializer<'de>,
        {
            Ok(Option::<u32>::deserialize(deserializer)?.map(|int| {
                let [r, g, b, a] = int.to_be_bytes();
                Color::from_rgba8(r, g, b, a as f32 / 255.0)
            }))
        }

        pub fn serialize<S>(color: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            color
                .map(|color| u32::from_be_bytes(color.into_rgba8()))
                .serialize(serializer)
        }
    }
}
