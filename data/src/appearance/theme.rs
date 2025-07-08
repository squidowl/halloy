use std::path::PathBuf;

use base64::Engine;
use iced_core::Color;
use palette::rgb::{Rgb, Rgba};
use palette::{FromColor, Hsva, Okhsl, Srgba};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use tokio::fs;

const DEFAULT_THEME_NAME: &str = "Ferra";
const DEFAULT_THEME_CONTENT: &str =
    include_str!("../../../assets/themes/ferra.toml");

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub styles: Styles,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: DEFAULT_THEME_NAME.to_string(),
            styles: Styles::default(),
        }
    }
}

impl Theme {
    pub fn new(name: String, styles: Styles) -> Self {
        Theme { name, styles }
    }
}

// IMPORTANT: Make sure any new components are added to the theme editor
// and `binary` representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Styles {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub text: Text,
    #[serde(default)]
    pub buffer: Buffer,
    #[serde(default)]
    pub buttons: Buttons,
}

impl Styles {
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
    #[serde(default)]
    pub action: TextStyle,
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
    #[serde(default)]
    pub code: TextStyle,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub highlight: Color,
    #[serde(default)]
    pub nickname: TextStyle,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub selection: Color,
    #[serde(default)]
    pub server_messages: ServerMessages,
    #[serde(default)]
    pub timestamp: TextStyle,
    #[serde(default)]
    pub topic: TextStyle,
    #[serde(default)]
    pub url: TextStyle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ServerMessages {
    #[serde(default)]
    pub join: OptionalTextStyle,
    #[serde(default)]
    pub part: OptionalTextStyle,
    #[serde(default)]
    pub quit: OptionalTextStyle,
    #[serde(default)]
    pub reply_topic: OptionalTextStyle,
    #[serde(default)]
    pub change_host: OptionalTextStyle,
    #[serde(default)]
    pub change_mode: OptionalTextStyle,
    #[serde(default)]
    pub change_nick: OptionalTextStyle,
    #[serde(default)]
    pub monitored_online: OptionalTextStyle,
    #[serde(default)]
    pub monitored_offline: OptionalTextStyle,
    #[serde(default)]
    pub standard_reply_fail: OptionalTextStyle,
    #[serde(default)]
    pub standard_reply_warn: OptionalTextStyle,
    #[serde(default)]
    pub standard_reply_note: OptionalTextStyle,
    #[serde(default)]
    pub wallops: OptionalTextStyle,
    #[serde(default)]
    pub default: TextStyle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Text {
    #[serde(default)]
    pub primary: TextStyle,
    #[serde(default)]
    pub secondary: TextStyle,
    #[serde(default)]
    pub tertiary: TextStyle,
    #[serde(default)]
    pub success: TextStyle,
    #[serde(default)]
    pub error: TextStyle,
    #[serde(default)]
    pub warning: OptionalTextStyle,
    #[serde(default)]
    pub info: OptionalTextStyle,
    #[serde(default)]
    pub debug: OptionalTextStyle,
    #[serde(default)]
    pub trace: OptionalTextStyle,
}

impl Default for Styles {
    fn default() -> Self {
        toml::from_str(DEFAULT_THEME_CONTENT).expect("parse default theme")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    pub color: Color,
    pub font_style: Option<FontStyle>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::TRANSPARENT,
            font_style: None,
        }
    }
}

impl<'de> Deserialize<'de> for TextStyle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Data {
            Basic(String),
            Extended {
                color: String,
                font_style: Option<FontStyle>,
            },
        }

        let data = Data::deserialize(deserializer)?;

        let (hex, font_style) = match data {
            Data::Basic(color) => (color, None),
            Data::Extended { color, font_style } => (color, font_style),
        };

        Ok(TextStyle {
            color: hex_to_color(&hex).unwrap_or(Color::TRANSPARENT),
            font_style,
        })
    }
}

impl Serialize for TextStyle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Data {
            color: String,
            font_style: Option<FontStyle>,
        }

        let hex = color_to_hex(self.color);

        if self.font_style.is_some() {
            Data {
                color: hex,
                font_style: self.font_style,
            }
            .serialize(serializer)
        } else {
            hex.serialize(serializer)
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OptionalTextStyle {
    pub color: Option<Color>,
    pub font_style: Option<FontStyle>,
}

impl<'de> Deserialize<'de> for OptionalTextStyle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Data {
            Basic(Option<String>),
            Extended {
                color: Option<String>,
                font_style: Option<FontStyle>,
            },
        }

        let data = Data::deserialize(deserializer)?;

        let (hex, font_style) = match data {
            Data::Basic(color) => (color, None),
            Data::Extended { color, font_style } => (color, font_style),
        };

        Ok(OptionalTextStyle {
            color: hex.and_then(|hex| hex_to_color(&hex)),
            font_style,
        })
    }
}

impl Serialize for OptionalTextStyle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Data {
            color: Option<String>,
            font_style: Option<FontStyle>,
        }

        let hex = self.color.map(color_to_hex);

        if self.font_style.is_some() {
            Data {
                color: hex,
                font_style: self.font_style,
            }
            .serialize(serializer)
        } else {
            hex.serialize(serializer)
        }
    }
}

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    #[default]
    Normal,
    Bold,
    Italic,
    #[serde(alias = "bold-italic")]
    ItalicBold,
}

impl FontStyle {
    pub fn new(bold: bool, italic: bool) -> Self {
        match (bold, italic) {
            (false, false) => FontStyle::Normal,
            (false, true) => FontStyle::Italic,
            (true, false) => FontStyle::Bold,
            (true, true) => FontStyle::ItalicBold,
        }
    }
}

impl std::ops::Add<FontStyle> for FontStyle {
    type Output = FontStyle;

    fn add(self, rhs: FontStyle) -> FontStyle {
        match self {
            FontStyle::Normal => rhs,
            FontStyle::Italic => match rhs {
                FontStyle::Normal | FontStyle::Italic => FontStyle::Italic,
                FontStyle::Bold | FontStyle::ItalicBold => {
                    FontStyle::ItalicBold
                }
            },
            FontStyle::Bold => match rhs {
                FontStyle::Normal | FontStyle::Bold => FontStyle::Bold,
                FontStyle::Italic | FontStyle::ItalicBold => {
                    FontStyle::ItalicBold
                }
            },
            FontStyle::ItalicBold => self,
        }
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

mod binary {
    use iced_core::Color;
    use strum::{IntoEnumIterator, VariantArray};

    use super::{Buffer, Buttons, General, Styles, Text};

    pub fn encode(styles: &Styles) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Tag::VARIANTS.len() * (1 + 4));

        for tag in Tag::iter() {
            if let Some(color) = tag.encode(styles) {
                bytes.push(tag as u8);
                bytes.extend(color);
            }
        }

        bytes
    }

    pub fn decode(bytes: &[u8]) -> Styles {
        let mut styles = Styles {
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

                    tag.update_color(&mut styles, color);
                }
            }
        }

        styles
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
        BufferServerMessagesChangeMode = 42,
        BufferServerMessagesChangeNick = 43,
        TextWarning = 44,
        TextInfo = 45,
        TextDebug = 46,
        TextTrace = 47,
    }

    impl Tag {
        pub fn encode(&self, styles: &Styles) -> Option<[u8; 4]> {
            let color = match self {
                Tag::GeneralBackground => styles.general.background,
                Tag::GeneralBorder => styles.general.border,
                Tag::GeneralHorizontalRule => styles.general.horizontal_rule,
                Tag::GeneralUnreadIndicator => styles.general.unread_indicator,
                Tag::TextPrimary => styles.text.primary.color,
                Tag::TextSecondary => styles.text.secondary.color,
                Tag::TextTertiary => styles.text.tertiary.color,
                Tag::TextSuccess => styles.text.success.color,
                Tag::TextError => styles.text.error.color,
                Tag::TextWarning => styles.text.warning.color?,
                Tag::TextInfo => styles.text.info.color?,
                Tag::TextDebug => styles.text.debug.color?,
                Tag::TextTrace => styles.text.trace.color?,
                Tag::BufferAction => styles.buffer.action.color,
                Tag::BufferBackground => styles.buffer.background,
                Tag::BufferBackgroundTextInput => {
                    styles.buffer.background_text_input
                }
                Tag::BufferBackgroundTitleBar => {
                    styles.buffer.background_title_bar
                }
                Tag::BufferBorder => styles.buffer.border,
                Tag::BufferBorderSelected => styles.buffer.border_selected,
                Tag::BufferCode => styles.buffer.code.color,
                Tag::BufferHighlight => styles.buffer.highlight,
                Tag::BufferNickname => styles.buffer.nickname.color,
                Tag::BufferSelection => styles.buffer.selection,
                Tag::BufferTimestamp => styles.buffer.timestamp.color,
                Tag::BufferTopic => styles.buffer.topic.color,
                Tag::BufferUrl => styles.buffer.url.color,
                Tag::BufferServerMessagesJoin => {
                    styles.buffer.server_messages.join.color?
                }
                Tag::BufferServerMessagesPart => {
                    styles.buffer.server_messages.part.color?
                }
                Tag::BufferServerMessagesQuit => {
                    styles.buffer.server_messages.quit.color?
                }
                Tag::BufferServerMessagesReplyTopic => {
                    styles.buffer.server_messages.reply_topic.color?
                }
                Tag::BufferServerMessagesChangeHost => {
                    styles.buffer.server_messages.change_host.color?
                }
                Tag::BufferServerMessagesMonitoredOnline => {
                    styles.buffer.server_messages.monitored_online.color?
                }
                Tag::BufferServerMessagesMonitoredOffline => {
                    styles.buffer.server_messages.monitored_offline.color?
                }
                Tag::BufferServerMessagesStandardReplyFail => {
                    styles.buffer.server_messages.standard_reply_fail.color?
                }
                Tag::BufferServerMessagesStandardReplyWarn => {
                    styles.buffer.server_messages.standard_reply_warn.color?
                }
                Tag::BufferServerMessagesStandardReplyNote => {
                    styles.buffer.server_messages.standard_reply_note.color?
                }
                Tag::BufferServerMessagesWallops => {
                    styles.buffer.server_messages.wallops.color?
                }
                Tag::BufferServerMessagesChangeMode => {
                    styles.buffer.server_messages.change_mode.color?
                }
                Tag::BufferServerMessagesChangeNick => {
                    styles.buffer.server_messages.change_nick.color?
                }
                Tag::BufferServerMessagesDefault => {
                    styles.buffer.server_messages.default.color
                }
                Tag::ButtonsPrimaryBackground => {
                    styles.buttons.primary.background
                }
                Tag::ButtonsPrimaryBackgroundHover => {
                    styles.buttons.primary.background_hover
                }
                Tag::ButtonsPrimaryBackgroundSelected => {
                    styles.buttons.primary.background_selected
                }
                Tag::ButtonsPrimaryBackgroundSelectedHover => {
                    styles.buttons.primary.background_selected_hover
                }
                Tag::ButtonsSecondaryBackground => {
                    styles.buttons.secondary.background
                }
                Tag::ButtonsSecondaryBackgroundHover => {
                    styles.buttons.secondary.background_hover
                }
                Tag::ButtonsSecondaryBackgroundSelected => {
                    styles.buttons.secondary.background_selected
                }
                Tag::ButtonsSecondaryBackgroundSelectedHover => {
                    styles.buttons.secondary.background_selected_hover
                }
            };

            Some(color.into_rgba8())
        }

        pub fn update_color(&self, styles: &mut Styles, color: Color) {
            match self {
                Tag::GeneralBackground => {
                    styles.general.background = color;
                }
                Tag::GeneralBorder => styles.general.border = color,
                Tag::GeneralHorizontalRule => {
                    styles.general.horizontal_rule = color;
                }
                Tag::GeneralUnreadIndicator => {
                    styles.general.unread_indicator = color;
                }
                Tag::TextPrimary => styles.text.primary.color = color,
                Tag::TextSecondary => styles.text.secondary.color = color,
                Tag::TextTertiary => styles.text.tertiary.color = color,
                Tag::TextSuccess => styles.text.success.color = color,
                Tag::TextError => styles.text.error.color = color,
                Tag::TextWarning => styles.text.warning.color = Some(color),
                Tag::TextInfo => styles.text.info.color = Some(color),
                Tag::TextDebug => styles.text.debug.color = Some(color),
                Tag::TextTrace => styles.text.trace.color = Some(color),
                Tag::BufferAction => styles.buffer.action.color = color,
                Tag::BufferBackground => styles.buffer.background = color,
                Tag::BufferBackgroundTextInput => {
                    styles.buffer.background_text_input = color;
                }
                Tag::BufferBackgroundTitleBar => {
                    styles.buffer.background_title_bar = color;
                }
                Tag::BufferBorder => styles.buffer.border = color,
                Tag::BufferBorderSelected => {
                    styles.buffer.border_selected = color;
                }
                Tag::BufferCode => styles.buffer.code.color = color,
                Tag::BufferHighlight => styles.buffer.highlight = color,
                Tag::BufferNickname => styles.buffer.nickname.color = color,
                Tag::BufferSelection => styles.buffer.selection = color,
                Tag::BufferTimestamp => styles.buffer.timestamp.color = color,
                Tag::BufferTopic => styles.buffer.topic.color = color,
                Tag::BufferUrl => styles.buffer.url.color = color,
                Tag::BufferServerMessagesJoin => {
                    styles.buffer.server_messages.join.color = Some(color);
                }
                Tag::BufferServerMessagesPart => {
                    styles.buffer.server_messages.part.color = Some(color);
                }
                Tag::BufferServerMessagesQuit => {
                    styles.buffer.server_messages.quit.color = Some(color);
                }
                Tag::BufferServerMessagesReplyTopic => {
                    styles.buffer.server_messages.reply_topic.color =
                        Some(color);
                }
                Tag::BufferServerMessagesChangeHost => {
                    styles.buffer.server_messages.change_host.color =
                        Some(color);
                }
                Tag::BufferServerMessagesMonitoredOnline => {
                    styles.buffer.server_messages.monitored_online.color =
                        Some(color);
                }
                Tag::BufferServerMessagesMonitoredOffline => {
                    styles.buffer.server_messages.monitored_offline.color =
                        Some(color);
                }
                Tag::BufferServerMessagesStandardReplyFail => {
                    styles.buffer.server_messages.standard_reply_fail.color =
                        Some(color);
                }
                Tag::BufferServerMessagesStandardReplyWarn => {
                    styles.buffer.server_messages.standard_reply_warn.color =
                        Some(color);
                }
                Tag::BufferServerMessagesStandardReplyNote => {
                    styles.buffer.server_messages.standard_reply_note.color =
                        Some(color);
                }
                Tag::BufferServerMessagesWallops => {
                    styles.buffer.server_messages.wallops.color = Some(color);
                }
                Tag::BufferServerMessagesChangeMode => {
                    styles.buffer.server_messages.change_mode.color =
                        Some(color);
                }
                Tag::BufferServerMessagesChangeNick => {
                    styles.buffer.server_messages.change_nick.color =
                        Some(color);
                }
                Tag::BufferServerMessagesDefault => {
                    styles.buffer.server_messages.default.color = color;
                }
                Tag::ButtonsPrimaryBackground => {
                    styles.buttons.primary.background = color;
                }
                Tag::ButtonsPrimaryBackgroundHover => {
                    styles.buttons.primary.background_hover = color;
                }
                Tag::ButtonsPrimaryBackgroundSelected => {
                    styles.buttons.primary.background_selected = color;
                }
                Tag::ButtonsPrimaryBackgroundSelectedHover => {
                    styles.buttons.primary.background_selected_hover = color;
                }
                Tag::ButtonsSecondaryBackground => {
                    styles.buttons.secondary.background = color;
                }
                Tag::ButtonsSecondaryBackgroundHover => {
                    styles.buttons.secondary.background_hover = color;
                }
                Tag::ButtonsSecondaryBackgroundSelected => {
                    styles.buttons.secondary.background_selected = color;
                }
                Tag::ButtonsSecondaryBackgroundSelectedHover => {
                    styles.buttons.secondary.background_selected_hover = color;
                }
            }
        }
    }
}
