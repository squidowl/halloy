use std::path::PathBuf;

use base64::Engine;
use iced_core::Color;
use palette::rgb::{Rgb, Rgba};
use palette::{FromColor, Hsva, Okhsl, Srgba};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;

const DEFAULT_THEME_NAME: &str = "Ferra";
const DEFAULT_THEME_CONTENT: &str =
    include_str!("../../../assets/themes/ferra.toml");

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
// and `binary` representation
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
        let bytes = binary::encode(self);

        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
    }

    pub fn decode_base64(content: &str) -> Result<Self, Error> {
        let bytes =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(content)?;

        Ok(binary::decode(&bytes))
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
    #[serde(default, with = "color_serde_maybe")]
    pub standard_reply_fail: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub standard_reply_warn: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub standard_reply_note: Option<Color>,
    #[serde(default, with = "color_serde_maybe")]
    pub wallops: Option<Color>,
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
    let _ = write!(&mut hex, "{r:02X}");
    let _ = write!(&mut hex, "{g:02X}");
    let _ = write!(&mut hex, "{b:02X}");

    if a < u8::MAX {
        let _ = write!(&mut hex, "{a:02X}");
    }

    hex
}

/// Adjusts the transparency of the foreground color based on the background color's lightness.
pub fn alpha_color_calculate(
    min_alpha: f32,
    max_alpha: f32,
    background: Color,
    foreground: Color,
) -> Color {
    alpha_color(
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
    let randomized_hue: f32 = rng.random_range(0.0..=360.0);
    let randomized_hsl = Okhsl::new(
        randomized_hue,
        original_hsl.saturation,
        original_hsl.lightness,
    );

    // Convert the randomized HSL color back to Color
    from_hsl(randomized_hsl)
}

pub fn to_hsl(color: Color) -> Okhsl {
    let mut hsl = Okhsl::from_color(to_rgb(color));
    if hsl.saturation.is_nan() {
        hsl.saturation = Okhsl::max_saturation();
    }

    hsl
}

pub fn to_hsva(color: Color) -> Hsva {
    Hsva::from_color(to_rgba(color))
}

pub fn from_hsva(color: Hsva) -> Color {
    to_color(Srgba::from_color(color))
}

pub fn from_hsl(hsl: Okhsl) -> Color {
    to_color(Srgba::from_color(hsl))
}

pub fn alpha_color(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn default_transparent() -> Color {
    Color::TRANSPARENT
}

fn to_rgb(color: Color) -> Rgb {
    Rgb {
        red: color.r,
        green: color.g,
        blue: color.b,
        ..Rgb::default()
    }
}

fn to_rgba(color: Color) -> Rgba {
    Rgba {
        alpha: color.a,
        color: to_rgb(color),
    }
}

fn to_color(rgba: Rgba) -> Color {
    Color {
        r: rgba.color.red,
        g: rgba.color.green,
        b: rgba.color.blue,
        a: rgba.alpha,
    }
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

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<Color>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<String>::deserialize(deserializer)?
            .and_then(|hex| super::hex_to_color(&hex)))
    }

    pub fn serialize<S>(
        color: &Option<Color>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        color.map(super::color_to_hex).serialize(serializer)
    }
}

mod binary {
    use iced_core::Color;
    use strum::{IntoEnumIterator, VariantArray};

    use super::{Buffer, Buttons, Colors, General, Text};

    pub fn encode(colors: &Colors) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Tag::VARIANTS.len() * (1 + 4));

        for tag in Tag::iter() {
            if let Some(color) = tag.encode(colors) {
                bytes.push(tag as u8);
                bytes.extend(color);
            }
        }

        bytes
    }

    pub fn decode(bytes: &[u8]) -> Colors {
        let mut colors = Colors {
            general: General::default(),
            text: Text::default(),
            buffer: Buffer::default(),
            buttons: Buttons::default(),
        };

        for chunk in bytes.chunks(5) {
            if chunk.len() == 5 {
                if let Ok(tag) = Tag::try_from(chunk[0]) {
                    let color = Color::from_rgba8(
                        chunk[1],
                        chunk[2],
                        chunk[3],
                        chunk[4] as f32 / 255.0,
                    );

                    tag.update_colors(&mut colors, color);
                }
            }
        }

        colors
    }

    // IMPORTANT: Tags cannot be rearranged or deleted to preserve
    // backwards compatibility. Only append new items in the future
    #[derive(
        Debug,
        Clone,
        Copy,
        strum::EnumIter,
        strum::VariantArray,
        derive_more::TryFrom,
    )]
    #[try_from(repr)]
    #[repr(u8)]
    pub enum Tag {
        GeneralBackground = 0,
        GeneralBorder = 1,
        GeneralHorizontalRule = 2,
        GeneralUnreadIndicator = 3,
        TextPrimary = 4,
        TextSecondary = 5,
        TextTertiary = 6,
        TextSuccess = 7,
        TextError = 8,
        BufferAction = 9,
        BufferBackground = 10,
        BufferBackgroundTextInput = 11,
        BufferBackgroundTitleBar = 12,
        BufferBorder = 13,
        BufferBorderSelected = 14,
        BufferCode = 15,
        BufferHighlight = 16,
        BufferNickname = 17,
        BufferSelection = 18,
        BufferTimestamp = 19,
        BufferTopic = 20,
        BufferUrl = 21,
        BufferServerMessagesJoin = 22,
        BufferServerMessagesPart = 23,
        BufferServerMessagesQuit = 24,
        BufferServerMessagesReplyTopic = 25,
        BufferServerMessagesChangeHost = 26,
        BufferServerMessagesMonitoredOnline = 27,
        BufferServerMessagesMonitoredOffline = 28,
        BufferServerMessagesDefault = 29,
        ButtonsPrimaryBackground = 30,
        ButtonsPrimaryBackgroundHover = 31,
        ButtonsPrimaryBackgroundSelected = 32,
        ButtonsPrimaryBackgroundSelectedHover = 33,
        ButtonsSecondaryBackground = 34,
        ButtonsSecondaryBackgroundHover = 35,
        ButtonsSecondaryBackgroundSelected = 36,
        ButtonsSecondaryBackgroundSelectedHover = 37,
        BufferServerMessagesStandardReplyFail = 38,
        BufferServerMessagesStandardReplyWarn = 39,
        BufferServerMessagesStandardReplyNote = 40,
        BufferServerMessagesWallops = 41,
    }

    impl Tag {
        pub fn encode(&self, colors: &Colors) -> Option<[u8; 4]> {
            let color = match self {
                Tag::GeneralBackground => colors.general.background,
                Tag::GeneralBorder => colors.general.border,
                Tag::GeneralHorizontalRule => colors.general.horizontal_rule,
                Tag::GeneralUnreadIndicator => colors.general.unread_indicator,
                Tag::TextPrimary => colors.text.primary,
                Tag::TextSecondary => colors.text.secondary,
                Tag::TextTertiary => colors.text.tertiary,
                Tag::TextSuccess => colors.text.success,
                Tag::TextError => colors.text.error,
                Tag::BufferAction => colors.buffer.action,
                Tag::BufferBackground => colors.buffer.background,
                Tag::BufferBackgroundTextInput => {
                    colors.buffer.background_text_input
                }
                Tag::BufferBackgroundTitleBar => {
                    colors.buffer.background_title_bar
                }
                Tag::BufferBorder => colors.buffer.border,
                Tag::BufferBorderSelected => colors.buffer.border_selected,
                Tag::BufferCode => colors.buffer.code,
                Tag::BufferHighlight => colors.buffer.highlight,
                Tag::BufferNickname => colors.buffer.nickname,
                Tag::BufferSelection => colors.buffer.selection,
                Tag::BufferTimestamp => colors.buffer.timestamp,
                Tag::BufferTopic => colors.buffer.topic,
                Tag::BufferUrl => colors.buffer.url,
                Tag::BufferServerMessagesJoin => {
                    colors.buffer.server_messages.join?
                }
                Tag::BufferServerMessagesPart => {
                    colors.buffer.server_messages.part?
                }
                Tag::BufferServerMessagesQuit => {
                    colors.buffer.server_messages.quit?
                }
                Tag::BufferServerMessagesReplyTopic => {
                    colors.buffer.server_messages.reply_topic?
                }
                Tag::BufferServerMessagesChangeHost => {
                    colors.buffer.server_messages.change_host?
                }
                Tag::BufferServerMessagesMonitoredOnline => {
                    colors.buffer.server_messages.monitored_online?
                }
                Tag::BufferServerMessagesMonitoredOffline => {
                    colors.buffer.server_messages.monitored_offline?
                }
                Tag::BufferServerMessagesStandardReplyFail => {
                    colors.buffer.server_messages.standard_reply_fail?
                }
                Tag::BufferServerMessagesStandardReplyWarn => {
                    colors.buffer.server_messages.standard_reply_warn?
                }
                Tag::BufferServerMessagesStandardReplyNote => {
                    colors.buffer.server_messages.standard_reply_note?
                }
                Tag::BufferServerMessagesWallops => {
                    colors.buffer.server_messages.wallops?
                }
                Tag::BufferServerMessagesDefault => {
                    colors.buffer.server_messages.default
                }
                Tag::ButtonsPrimaryBackground => {
                    colors.buttons.primary.background
                }
                Tag::ButtonsPrimaryBackgroundHover => {
                    colors.buttons.primary.background_hover
                }
                Tag::ButtonsPrimaryBackgroundSelected => {
                    colors.buttons.primary.background_selected
                }
                Tag::ButtonsPrimaryBackgroundSelectedHover => {
                    colors.buttons.primary.background_selected_hover
                }
                Tag::ButtonsSecondaryBackground => {
                    colors.buttons.secondary.background
                }
                Tag::ButtonsSecondaryBackgroundHover => {
                    colors.buttons.secondary.background_hover
                }
                Tag::ButtonsSecondaryBackgroundSelected => {
                    colors.buttons.secondary.background_selected
                }
                Tag::ButtonsSecondaryBackgroundSelectedHover => {
                    colors.buttons.secondary.background_selected_hover
                }
            };

            Some(color.into_rgba8())
        }

        pub fn update_colors(&self, colors: &mut Colors, color: Color) {
            match self {
                Tag::GeneralBackground => colors.general.background = color,
                Tag::GeneralBorder => colors.general.border = color,
                Tag::GeneralHorizontalRule => {
                    colors.general.horizontal_rule = color;
                }
                Tag::GeneralUnreadIndicator => {
                    colors.general.unread_indicator = color;
                }
                Tag::TextPrimary => colors.text.primary = color,
                Tag::TextSecondary => colors.text.secondary = color,
                Tag::TextTertiary => colors.text.tertiary = color,
                Tag::TextSuccess => colors.text.success = color,
                Tag::TextError => colors.text.error = color,
                Tag::BufferAction => colors.buffer.action = color,
                Tag::BufferBackground => colors.buffer.background = color,
                Tag::BufferBackgroundTextInput => {
                    colors.buffer.background_text_input = color;
                }
                Tag::BufferBackgroundTitleBar => {
                    colors.buffer.background_title_bar = color;
                }
                Tag::BufferBorder => colors.buffer.border = color,
                Tag::BufferBorderSelected => {
                    colors.buffer.border_selected = color;
                }
                Tag::BufferCode => colors.buffer.code = color,
                Tag::BufferHighlight => colors.buffer.highlight = color,
                Tag::BufferNickname => colors.buffer.nickname = color,
                Tag::BufferSelection => colors.buffer.selection = color,
                Tag::BufferTimestamp => colors.buffer.timestamp = color,
                Tag::BufferTopic => colors.buffer.topic = color,
                Tag::BufferUrl => colors.buffer.url = color,
                Tag::BufferServerMessagesJoin => {
                    colors.buffer.server_messages.join = Some(color);
                }
                Tag::BufferServerMessagesPart => {
                    colors.buffer.server_messages.part = Some(color);
                }
                Tag::BufferServerMessagesQuit => {
                    colors.buffer.server_messages.quit = Some(color);
                }
                Tag::BufferServerMessagesReplyTopic => {
                    colors.buffer.server_messages.reply_topic = Some(color);
                }
                Tag::BufferServerMessagesChangeHost => {
                    colors.buffer.server_messages.change_host = Some(color);
                }
                Tag::BufferServerMessagesMonitoredOnline => {
                    colors.buffer.server_messages.monitored_online =
                        Some(color);
                }
                Tag::BufferServerMessagesMonitoredOffline => {
                    colors.buffer.server_messages.monitored_offline =
                        Some(color);
                }
                Tag::BufferServerMessagesStandardReplyFail => {
                    colors.buffer.server_messages.standard_reply_fail =
                        Some(color);
                }
                Tag::BufferServerMessagesStandardReplyWarn => {
                    colors.buffer.server_messages.standard_reply_warn =
                        Some(color);
                }
                Tag::BufferServerMessagesStandardReplyNote => {
                    colors.buffer.server_messages.standard_reply_note =
                        Some(color);
                }
                Tag::BufferServerMessagesWallops => {
                    colors.buffer.server_messages.wallops = Some(color);
                }
                Tag::BufferServerMessagesDefault => {
                    colors.buffer.server_messages.default = color;
                }
                Tag::ButtonsPrimaryBackground => {
                    colors.buttons.primary.background = color;
                }
                Tag::ButtonsPrimaryBackgroundHover => {
                    colors.buttons.primary.background_hover = color;
                }
                Tag::ButtonsPrimaryBackgroundSelected => {
                    colors.buttons.primary.background_selected = color;
                }
                Tag::ButtonsPrimaryBackgroundSelectedHover => {
                    colors.buttons.primary.background_selected_hover = color;
                }
                Tag::ButtonsSecondaryBackground => {
                    colors.buttons.secondary.background = color;
                }
                Tag::ButtonsSecondaryBackgroundHover => {
                    colors.buttons.secondary.background_hover = color;
                }
                Tag::ButtonsSecondaryBackgroundSelected => {
                    colors.buttons.secondary.background_selected = color;
                }
                Tag::ButtonsSecondaryBackgroundSelectedHover => {
                    colors.buttons.secondary.background_selected_hover = color;
                }
            }
        }
    }
}
