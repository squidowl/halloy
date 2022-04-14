use palette::{FromColor, Hsl, Shade, Srgb};

#[derive(Debug, Clone, Copy)]
pub struct Color(iced::Color);

impl Color {
    pub fn raw(&self) -> iced::Color {
        self.0
    }

    pub fn lighten(&self) -> iced::Color {
        let hsl = Hsl::from_color(Srgb::from(self.raw())).lighten(0.05);
        Srgb::from_color(hsl).into()
    }

    pub fn darken(&self) -> iced::Color {
        let hsl = Hsl::from_color(Srgb::from(self.raw())).darken(0.05);
        Srgb::from_color(hsl).into()
    }
}

impl Into<iced::Color> for Color {
    fn into(self) -> iced::Color {
        self.0
    }
}

pub struct Theme {
    pub name: String,
    pub palette: Palette,
}

impl Default for Theme {
    fn default() -> Theme {
        ayu_dark_theme()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color,
    pub text: Color,
    pub primary: Color,
    pub secondary: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub success: Color,
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

fn ayu_dark_theme() -> Theme {
    Theme {
        name: "Ayu Dark".to_string(),
        palette: Palette {
            background: hex_to_color("#0A0E14").unwrap(),
            text: hex_to_color("#B3B1AD").unwrap(),
            primary: hex_to_color("#53BDFA").unwrap(),
            secondary: hex_to_color("#91B362").unwrap(),
            error: hex_to_color("#EA6C73").unwrap(),
            warning: hex_to_color("#F9AF4F").unwrap(),
            info: hex_to_color("#FAE994").unwrap(),
            success: hex_to_color("#90E1C6").unwrap(),
        },
    }
}
