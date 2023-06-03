use iced::Color;
use palette::{rgb::Rgb, DarkenAssign, FromColor, LightenAssign, Okhsl, Srgb};

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color,
    pub text: Color,
    pub action: Color,
    pub accent: Color,
    pub alert: Color,
    pub error: Color,
    pub info: Color,
    pub success: Color,
}

impl Default for Palette {
    fn default() -> Palette {
        Palette {
            background: hex_to_color("#2b292d").unwrap(),
            text: hex_to_color("#fecdb2").unwrap(),
            action: hex_to_color("#b1b695").unwrap(),
            accent: hex_to_color("#d1d1e0").unwrap(),
            alert: hex_to_color("#ffa07a").unwrap(),
            error: hex_to_color("#e06b75").unwrap(),
            info: hex_to_color("#f5d76e").unwrap(),
            success: hex_to_color("#b1b695").unwrap(),
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

pub fn lightness(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    hsl.lightness = amount;

    from_hsl(hsl)
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

pub fn mute(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    if is_dark(color) {
        hsl.lighten_fixed_assign(amount)
    } else {
        hsl.darken_fixed_assign(amount)
    };

    from_hsl(hsl)
}

pub fn intensify(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    if is_dark(color) {
        hsl.darken_fixed_assign(amount)
    } else {
        hsl.lighten_fixed_assign(amount)
    };

    from_hsl(hsl)
}

pub mod palette_serde {
    use super::{hex_to_color, Palette};
    use iced::Color;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    struct HexPalette {
        background: String,
        text: String,
        action: String,
        accent: String,
        alert: String,
        error: String,
        info: String,
        success: String,
    }

    impl Serialize for Palette {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            fn as_hex(color: Color) -> String {
                format!(
                    "#{:02x}{:02x}{:02x}",
                    (255.0 * color.r).round() as u8,
                    (255.0 * color.g).round() as u8,
                    (255.0 * color.b).round() as u8
                )
            }

            let hex_theme = HexPalette {
                background: as_hex(self.background),
                text: as_hex(self.text),
                action: as_hex(self.action),
                accent: as_hex(self.accent),
                alert: as_hex(self.alert),
                error: as_hex(self.error),
                info: as_hex(self.info),
                success: as_hex(self.success),
            };

            hex_theme.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Palette {
        fn deserialize<D>(deserializer: D) -> Result<Palette, D::Error>
        where
            D: Deserializer<'de>,
        {
            let hex_palette: HexPalette = serde::Deserialize::deserialize(deserializer)?;

            Ok(Palette {
                background: hex_to_color(hex_palette.background.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                text: hex_to_color(hex_palette.text.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                action: hex_to_color(hex_palette.action.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                accent: hex_to_color(hex_palette.accent.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                alert: hex_to_color(hex_palette.alert.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                error: hex_to_color(hex_palette.error.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                info: hex_to_color(hex_palette.info.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
                success: hex_to_color(hex_palette.success.as_str())
                    .ok_or_else(|| serde::de::Error::custom("not a valid hex"))?,
            })
        }
    }
}
