use iced_core::Color;
use palette::rgb::Rgb;
use palette::{DarkenAssign, FromColor, LightenAssign, Mix, Okhsl, Srgb};
use rand::prelude::*;
use rand_chacha::ChaChaRng;

const DEFAULT_THEME_NAME: &str = "Ferra";

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

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub general: General,
    pub buffer: Buffer,
    pub text: Text,
    pub buttons: Buttons,
}

#[derive(Debug, Clone, Copy)]
pub struct Buttons {
    pub primary: Button,
    pub secondary: Button,
}

#[derive(Debug, Clone, Copy)]
pub struct Button {
    pub background: Color,
    pub background_hover: Color,
    pub background_selected: Color,
    pub background_selected_hover: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct General {
    pub background: Color,
    pub horizontal_rule: Color,
    pub unread_indicator: Color,
    pub border: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct Buffer {
    pub background: Color,
    pub timestamp: Color,
    pub server_messages: ServerMessages,
    pub action: Color,
    pub topic: Color,
    pub text_input: Color,
    pub title_bar: Color,
    pub highlight: Color,
    pub nickname: Color,
    pub url: Color,
    pub code: Color,
    pub selection: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct ServerMessages {
    pub join: Option<Color>,
    pub part: Option<Color>,
    pub quit: Option<Color>,
    pub reply_topic: Option<Color>,
    pub change_host: Option<Color>,
    pub default: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct Text {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub success: Color,
    pub error: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            buffer: Buffer {
                background: hex_to_color("#242226").unwrap(),
                action: hex_to_color("#b1b695").unwrap(),
                server_messages: ServerMessages {
                    join: None,
                    part: None,
                    quit: None,
                    reply_topic: None,
                    change_host: None,
                    default: hex_to_color("#f5d76e").unwrap(),
                },
                timestamp: hex_to_color("#685650").unwrap(),
                topic: hex_to_color("#AB8A79").unwrap(),
                text_input: hex_to_color("#1D1B1E").unwrap(),
                title_bar: hex_to_color("#222024").unwrap(),
                highlight: hex_to_color("#473f30").unwrap(),
                nickname: hex_to_color("#f6b6c9").unwrap(),
                url: hex_to_color("#d7bde2").unwrap(),
                code: hex_to_color("#f6b6c9").unwrap(),
                selection: hex_to_color("#6f5d63").unwrap(),
            },
            general: General {
                background: hex_to_color("#2b292d").unwrap(),
                horizontal_rule: hex_to_color("#323034").unwrap(),
                unread_indicator: hex_to_color("#ffa07a").unwrap(),
                border: hex_to_color("#7D6E76").unwrap(),
            },
            text: Text {
                primary: hex_to_color("#fecdb2").unwrap(),
                secondary: hex_to_color("#AB8A79").unwrap(),
                tertiary: hex_to_color("#d1d1e0").unwrap(),
                success: hex_to_color("#b1b695").unwrap(),
                error: hex_to_color("#e06b75").unwrap(),
            },
            buttons: Buttons {
                primary: Button {
                    background: hex_to_color("#2b292d").unwrap(),
                    background_hover: hex_to_color("#242226").unwrap(),
                    background_selected: hex_to_color("#1d1b1e").unwrap(),
                    background_selected_hover: hex_to_color("#0D0C0D").unwrap(),
                },
                secondary: Button {
                    background: hex_to_color("#323034").unwrap(),
                    background_hover: hex_to_color("#323034").unwrap(),
                    background_selected: hex_to_color("#606155").unwrap(),
                    background_selected_hover: hex_to_color("#6F7160").unwrap(),
                },
            },
        }
    }
}

pub fn hex_to_color(hex: &str) -> Option<Color> {
    if hex.len() == 7 {
        let hash = &hex[0..1];
        let r = u8::from_str_radix(&hex[1..3], 16);
        let g = u8::from_str_radix(&hex[3..5], 16);
        let b = u8::from_str_radix(&hex[5..7], 16);

        return match (hash, r, g, b) {
            ("#", Ok(r), Ok(g), Ok(b)) => Some(Color {
                r: r as f32 / 255.0,
                g: g as f32 / 255.0,
                b: b as f32 / 255.0,
                a: 1.0,
            }),
            _ => None,
        };
    }

    None
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

pub fn is_dark(color: Color) -> bool {
    to_hsl(color).lightness < 0.5
}

pub fn to_hsl(color: Color) -> Okhsl {
    let mut hsl = Okhsl::from_color(Rgb::from(color));
    if hsl.saturation.is_nan() {
        hsl.saturation = Okhsl::max_saturation();
    }

    hsl
}

pub fn from_hsl(hsl: Okhsl) -> Color {
    Srgb::from_color(hsl).into()
}

pub fn alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

pub fn mix(a: Color, b: Color, factor: f32) -> Color {
    let a_hsl = to_hsl(a);
    let b_hsl = to_hsl(b);

    let mixed = a_hsl.mix(b_hsl, factor);
    from_hsl(mixed)
}

pub fn lighten(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    hsl.lighten_fixed_assign(amount);

    from_hsl(hsl)
}

pub fn darken(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    hsl.darken_fixed_assign(amount);

    from_hsl(hsl)
}

pub mod colors_serde {
    use serde::{Deserialize, Deserializer};

    use crate::theme::{Buffer, Button, Buttons, General, ServerMessages, Text};

    use super::{hex_to_color, Colors};

    impl<'de> Deserialize<'de> for Colors {
        fn deserialize<D>(deserializer: D) -> Result<Colors, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            struct HexColors {
                general: HexGeneral,
                buffer: HexBuffer,
                text: HexText,
                buttons: HexButtons,
            }

            #[derive(Deserialize)]
            struct HexGeneral {
                background: String,
                horizontal_rule: String,
                unread_indicator: String,
                border: String,
            }

            #[derive(Deserialize)]
            struct HexBuffer {
                background: String,
                timestamp: String,
                server_messages: HexServerMessages,
                action: String,
                topic: String,
                text_input: String,
                title_bar: String,
                highlight: String,
                nickname: String,
                url: String,
                code: String,
                selection: String,
            }

            #[derive(Deserialize)]
            struct HexText {
                pub primary: String,
                pub secondary: String,
                pub tertiary: String,
                pub success: String,
                pub error: String,
            }

            #[derive(Deserialize)]
            struct HexButtons {
                primary: HexButton,
                secondary: HexButton,
            }

            #[derive(Deserialize)]
            struct HexButton {
                background: String,
                background_hover: String,
                background_selected: String,
                background_selected_hover: String,
            }

            #[derive(Deserialize)]
            pub struct HexServerMessages {
                #[serde(default)]
                pub join: Option<String>,
                #[serde(default)]
                pub part: Option<String>,
                #[serde(default)]
                pub quit: Option<String>,
                #[serde(default)]
                pub reply_topic: Option<String>,
                #[serde(default)]
                pub change_host: Option<String>,
                pub default: String,
            }

            let hex_colors: HexColors = serde::Deserialize::deserialize(deserializer)?;

            let hex_to_color_checked = |hex: &str| {
                hex_to_color(hex).ok_or_else(|| {
                    serde::de::Error::custom(format!("'{}' is not a valid hex color", hex))
                })
            };

            Ok(Colors {
                general: General {
                    background: hex_to_color_checked(&hex_colors.general.background)?,
                    horizontal_rule: hex_to_color_checked(&hex_colors.general.horizontal_rule)?,
                    unread_indicator: hex_to_color_checked(&hex_colors.general.unread_indicator)?,
                    border: hex_to_color_checked(&hex_colors.general.border)?,
                },
                buffer: Buffer {
                    background: hex_to_color_checked(&hex_colors.buffer.background)?,
                    timestamp: hex_to_color_checked(&hex_colors.buffer.timestamp)?,
                    action: hex_to_color_checked(&hex_colors.buffer.action)?,
                    topic: hex_to_color_checked(&hex_colors.buffer.topic)?,
                    text_input: hex_to_color_checked(&hex_colors.buffer.text_input)?,
                    title_bar: hex_to_color_checked(&hex_colors.buffer.title_bar)?,
                    highlight: hex_to_color_checked(&hex_colors.buffer.highlight)?,
                    nickname: hex_to_color_checked(&hex_colors.buffer.nickname)?,
                    url: hex_to_color_checked(&hex_colors.buffer.url)?,
                    code: hex_to_color_checked(&hex_colors.buffer.code)?,
                    selection: hex_to_color_checked(&hex_colors.buffer.selection)?,
                    server_messages: ServerMessages {
                        join: hex_colors
                            .buffer
                            .server_messages
                            .join
                            .as_ref()
                            .and_then(|hex| hex_to_color_checked(hex).ok()),
                        part: hex_colors
                            .buffer
                            .server_messages
                            .part
                            .as_ref()
                            .and_then(|hex| hex_to_color_checked(hex).ok()),
                        quit: hex_colors
                            .buffer
                            .server_messages
                            .quit
                            .as_ref()
                            .and_then(|hex| hex_to_color_checked(hex).ok()),
                        reply_topic: hex_colors
                            .buffer
                            .server_messages
                            .reply_topic
                            .as_ref()
                            .and_then(|hex| hex_to_color_checked(hex).ok()),
                        change_host: hex_colors
                            .buffer
                            .server_messages
                            .change_host
                            .as_ref()
                            .and_then(|hex| hex_to_color_checked(hex).ok()),
                        default: hex_to_color_checked(&hex_colors.buffer.server_messages.default)?,
                    },
                },
                text: Text {
                    primary: hex_to_color_checked(&hex_colors.text.primary)?,
                    secondary: hex_to_color_checked(&hex_colors.text.secondary)?,
                    tertiary: hex_to_color_checked(&hex_colors.text.tertiary)?,
                    success: hex_to_color_checked(&hex_colors.text.success)?,
                    error: hex_to_color_checked(&hex_colors.text.error)?,
                },
                buttons: Buttons {
                    primary: Button {
                        background: hex_to_color_checked(&hex_colors.buttons.primary.background)?,
                        background_hover: hex_to_color_checked(
                            &hex_colors.buttons.primary.background_hover,
                        )?,
                        background_selected: hex_to_color_checked(
                            &hex_colors.buttons.primary.background_selected,
                        )?,
                        background_selected_hover: hex_to_color_checked(
                            &hex_colors.buttons.primary.background_selected_hover,
                        )?,
                    },
                    secondary: Button {
                        background: hex_to_color_checked(&hex_colors.buttons.secondary.background)?,
                        background_hover: hex_to_color_checked(
                            &hex_colors.buttons.secondary.background_hover,
                        )?,
                        background_selected: hex_to_color_checked(
                            &hex_colors.buttons.secondary.background_selected,
                        )?,
                        background_selected_hover: hex_to_color_checked(
                            &hex_colors.buttons.secondary.background_selected_hover,
                        )?,
                    },
                },
            })
        }
    }
}
