use iced_core::Color;
use palette::rgb::Rgb;
use palette::{FromColor, Okhsl, Srgb};
use rand::prelude::*;
use rand_chacha::ChaChaRng;

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

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub general: General,
    pub text: Text,
    pub buffer: Buffer,
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
    pub border: Color,
    pub horizontal_rule: Color,
    pub unread_indicator: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct Buffer {
    pub action: Color,
    pub background: Color,
    pub background_text_input: Color,
    pub background_title_bar: Color,
    pub border: Color,
    pub border_selected: Color,
    pub code: Color,
    pub highlight: Color,
    pub nickname: Color,
    pub selection: Color,
    pub server_messages: ServerMessages,
    pub timestamp: Color,
    pub topic: Color,
    pub url: Color,
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
        toml::from_str(DEFAULT_THEME_CONTENT).expect("parse default theme")
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

pub fn from_hsl(hsl: Okhsl) -> Color {
    Srgb::from_color(hsl).into()
}

pub fn alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
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
                #[serde(default)]
                general: HexGeneral,
                #[serde(default)]
                buffer: HexBuffer,
                #[serde(default)]
                text: HexText,
                #[serde(default)]
                buttons: HexButtons,
            }

            #[derive(Deserialize, Default)]
            struct HexGeneral {
                #[serde(default)]
                background: Option<String>,
                #[serde(default)]
                horizontal_rule: Option<String>,
                #[serde(default)]
                unread_indicator: Option<String>,
                #[serde(default)]
                border: Option<String>,
            }

            #[derive(Deserialize, Default)]
            struct HexBuffer {
                #[serde(default)]
                background: Option<String>,
                #[serde(default)]
                background_text_input: Option<String>,
                #[serde(default)]
                background_title_bar: Option<String>,
                #[serde(default)]
                timestamp: Option<String>,
                #[serde(default)]
                server_messages: HexServerMessages,
                #[serde(default)]
                action: Option<String>,
                #[serde(default)]
                topic: Option<String>,
                #[serde(default)]
                highlight: Option<String>,
                #[serde(default)]
                nickname: Option<String>,
                #[serde(default)]
                url: Option<String>,
                #[serde(default)]
                code: Option<String>,
                #[serde(default)]
                selection: Option<String>,
                #[serde(default)]
                pub border: Option<String>,
                #[serde(default)]
                pub border_selected: Option<String>,
            }

            #[derive(Deserialize, Default)]
            struct HexText {
                #[serde(default)]
                pub primary: Option<String>,
                #[serde(default)]
                pub secondary: Option<String>,
                #[serde(default)]
                pub tertiary: Option<String>,
                #[serde(default)]
                pub success: Option<String>,
                #[serde(default)]
                pub error: Option<String>,
            }

            #[derive(Deserialize, Default)]
            struct HexButtons {
                #[serde(default)]
                primary: HexButton,
                #[serde(default)]
                secondary: HexButton,
            }

            #[derive(Deserialize, Default)]
            struct HexButton {
                #[serde(default)]
                background: Option<String>,
                #[serde(default)]
                background_hover: Option<String>,
                #[serde(default)]
                background_selected: Option<String>,
                #[serde(default)]
                background_selected_hover: Option<String>,
            }

            #[derive(Deserialize, Default)]
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
                #[serde(default)]
                pub default: Option<String>,
            }

            let hex_colors: HexColors = serde::Deserialize::deserialize(deserializer)?;

            let color_or_transparent = |color: Option<&String>| {
                color
                    .and_then(|hex| hex_to_color(hex))
                    .unwrap_or(iced_core::Color::TRANSPARENT)
            };

            let color_or_none = |color: Option<&String>| color.and_then(|hex| hex_to_color(hex));

            Ok(Colors {
                general: General {
                    background: color_or_transparent(hex_colors.general.background.as_ref()),
                    horizontal_rule: color_or_transparent(
                        hex_colors.general.horizontal_rule.as_ref(),
                    ),
                    unread_indicator: color_or_transparent(
                        hex_colors.general.unread_indicator.as_ref(),
                    ),
                    border: color_or_transparent(hex_colors.general.border.as_ref()),
                },
                buffer: Buffer {
                    background: color_or_transparent(hex_colors.buffer.background.as_ref()),
                    background_text_input: color_or_transparent(
                        hex_colors.buffer.background_text_input.as_ref(),
                    ),
                    background_title_bar: color_or_transparent(
                        hex_colors.buffer.background_title_bar.as_ref(),
                    ),
                    timestamp: color_or_transparent(hex_colors.buffer.timestamp.as_ref()),
                    action: color_or_transparent(hex_colors.buffer.action.as_ref()),
                    topic: color_or_transparent(hex_colors.buffer.topic.as_ref()),
                    highlight: color_or_transparent(hex_colors.buffer.highlight.as_ref()),
                    nickname: color_or_transparent(hex_colors.buffer.nickname.as_ref()),
                    url: color_or_transparent(hex_colors.buffer.url.as_ref()),
                    code: color_or_transparent(hex_colors.buffer.code.as_ref()),
                    selection: color_or_transparent(hex_colors.buffer.selection.as_ref()),
                    server_messages: ServerMessages {
                        join: color_or_none(hex_colors.buffer.server_messages.join.as_ref()),
                        part: color_or_none(hex_colors.buffer.server_messages.part.as_ref()),
                        quit: color_or_none(hex_colors.buffer.server_messages.quit.as_ref()),
                        reply_topic: color_or_none(
                            hex_colors.buffer.server_messages.reply_topic.as_ref(),
                        ),
                        change_host: color_or_none(
                            hex_colors.buffer.server_messages.change_host.as_ref(),
                        ),
                        default: color_or_transparent(
                            hex_colors.buffer.server_messages.default.as_ref(),
                        ),
                    },
                    border: color_or_transparent(hex_colors.buffer.border.as_ref()),
                    border_selected: color_or_transparent(
                        hex_colors.buffer.border_selected.as_ref(),
                    ),
                },
                text: Text {
                    primary: color_or_transparent(hex_colors.text.primary.as_ref()),
                    secondary: color_or_transparent(hex_colors.text.secondary.as_ref()),
                    tertiary: color_or_transparent(hex_colors.text.tertiary.as_ref()),
                    success: color_or_transparent(hex_colors.text.success.as_ref()),
                    error: color_or_transparent(hex_colors.text.error.as_ref()),
                },
                buttons: Buttons {
                    primary: Button {
                        background: color_or_transparent(
                            hex_colors.buttons.primary.background.as_ref(),
                        ),
                        background_hover: color_or_transparent(
                            hex_colors.buttons.primary.background_hover.as_ref(),
                        ),
                        background_selected: color_or_transparent(
                            hex_colors.buttons.primary.background_selected.as_ref(),
                        ),
                        background_selected_hover: color_or_transparent(
                            hex_colors
                                .buttons
                                .primary
                                .background_selected_hover
                                .as_ref(),
                        ),
                    },
                    secondary: Button {
                        background: color_or_transparent(
                            hex_colors.buttons.secondary.background.as_ref(),
                        ),
                        background_hover: color_or_transparent(
                            hex_colors.buttons.secondary.background_hover.as_ref(),
                        ),
                        background_selected: color_or_transparent(
                            hex_colors.buttons.secondary.background_selected.as_ref(),
                        ),
                        background_selected_hover: color_or_transparent(
                            hex_colors
                                .buttons
                                .secondary
                                .background_selected_hover
                                .as_ref(),
                        ),
                    },
                },
            })
        }
    }
}
