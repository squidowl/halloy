use palette::{FromColor, Hsl, Shade, Srgb};

#[derive(Debug, Clone, Copy)]
pub struct Color(iced::Color);

impl Color {
    pub fn lighten(&self) -> iced::Color {
        self.lighten_by(0.05)
    }

    pub fn darken(&self) -> iced::Color {
        self.darken_by(0.05)
    }

    pub fn darken_by(&self, float: f32) -> iced::Color {
        let hsl = Hsl::from_color(Srgb::from(self.0)).darken(float);
        Srgb::from_color(hsl).into()
    }

    pub fn lighten_by(&self, float: f32) -> iced::Color {
        let hsl = Hsl::from_color(Srgb::from(self.0)).lighten(float);
        Srgb::from_color(hsl).into()
    }

    pub fn as_hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (255.0 * self.0.r).round() as u8,
            (255.0 * self.0.g).round() as u8,
            (255.0 * self.0.b).round() as u8
        )
    }
}

impl Into<iced::Color> for Color {
    fn into(self) -> iced::Color {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub text: Color,
    pub primary: Color,
    pub secondary: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub success: Color,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            background: hex_to_color("#0A0E14").unwrap(),
            text: hex_to_color("#B3B1AD").unwrap(),
            primary: hex_to_color("#53BDFA").unwrap(),
            secondary: hex_to_color("#91B362").unwrap(),
            error: hex_to_color("#EA6C73").unwrap(),
            warning: hex_to_color("#F9AF4F").unwrap(),
            info: hex_to_color("#FAE994").unwrap(),
            success: hex_to_color("#90E1C6").unwrap(),
        }
    }
}

fn hex_to_color(hex: &str) -> Option<Color> {
    if hex.len() == 7 {
        let hash = &hex[0..1];
        let r = u8::from_str_radix(&hex[1..3], 16);
        let g = u8::from_str_radix(&hex[3..5], 16);
        let b = u8::from_str_radix(&hex[5..7], 16);

        return match (hash, r, g, b) {
            ("#", Ok(r), Ok(g), Ok(b)) => Some(Color(iced::Color {
                r: r as f32 / 255.0,
                g: g as f32 / 255.0,
                b: b as f32 / 255.0,
                a: 1.0,
            })),
            _ => None,
        };
    }

    None
}

pub mod theme_serde {
    use super::{hex_to_color, Theme};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    struct HexTheme {
        background: String,
        text: String,
        primary: String,
        secondary: String,
        error: String,
        warning: String,
        info: String,
        success: String,
    }

    impl Serialize for Theme {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let hex_theme = HexTheme {
                background: self.background.as_hex(),
                text: self.text.as_hex(),
                primary: self.primary.as_hex(),
                secondary: self.secondary.as_hex(),
                error: self.error.as_hex(),
                warning: self.warning.as_hex(),
                info: self.info.as_hex(),
                success: self.success.as_hex(),
            };

            hex_theme.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Theme {
        fn deserialize<D>(deserializer: D) -> Result<Theme, D::Error>
        where
            D: Deserializer<'de>,
        {
            let hex_theme: HexTheme = serde::Deserialize::deserialize(deserializer)?;

            Ok(Theme {
                background: hex_to_color(hex_theme.background.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                text: hex_to_color(hex_theme.text.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                primary: hex_to_color(hex_theme.primary.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                secondary: hex_to_color(hex_theme.secondary.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                error: hex_to_color(hex_theme.error.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                warning: hex_to_color(hex_theme.warning.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                info: hex_to_color(hex_theme.info.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                success: hex_to_color(hex_theme.success.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
            })
        }
    }
}
