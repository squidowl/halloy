use iced_core::Color;
use palette::rgb::{Rgb, Rgba};
use palette::{FromColor, Hsva, Okhsl, Srgb, Srgba};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Deserializer};

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

#[derive(Debug, Clone, Copy, Deserialize)]
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

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct Buttons {
    #[serde(default)]
    pub primary: Button,
    #[serde(default)]
    pub secondary: Button,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct Button {
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub background: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub background_hover: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub background_selected: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub background_selected_hover: Color,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct General {
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub background: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub border: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub horizontal_rule: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub unread_indicator: Color,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct Buffer {
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub action: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub background: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub background_text_input: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub background_title_bar: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub border: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub border_selected: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub code: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub highlight: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub nickname: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub selection: Color,
    pub server_messages: ServerMessages,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub timestamp: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub topic: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub url: Color,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct ServerMessages {
    #[serde(default, deserialize_with = "color_deser_maybe")]
    pub join: Option<Color>,
    #[serde(default, deserialize_with = "color_deser_maybe")]
    pub part: Option<Color>,
    #[serde(default, deserialize_with = "color_deser_maybe")]
    pub quit: Option<Color>,
    #[serde(default, deserialize_with = "color_deser_maybe")]
    pub reply_topic: Option<Color>,
    #[serde(default, deserialize_with = "color_deser_maybe")]
    pub change_host: Option<Color>,
    #[serde(default, deserialize_with = "color_deser_maybe")]
    pub monitored_online: Option<Color>,
    #[serde(default, deserialize_with = "color_deser_maybe")]
    pub monitored_offline: Option<Color>,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub default: Color,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct Text {
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub primary: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub secondary: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub tertiary: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
    pub success: Color,
    #[serde(default = "default_transparent", deserialize_with = "color_deser")]
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

fn color_deser<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(String::deserialize(deserializer)
        .map(|hex| hex_to_color(&hex))?
        .unwrap_or(Color::TRANSPARENT))
}

fn color_deser_maybe<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.and_then(|hex| hex_to_color(&hex)))
}
