pub use theme::Theme;

pub mod theme;

#[derive(Debug, Clone)]
pub struct Appearance {
    pub selected: Selected,
    pub all: Vec<Theme>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            selected: Selected::default(),
            all: vec![Theme::default()],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Selected {
    Static(Theme),
    Dynamic { light: Theme, dark: Theme },
}

impl Default for Selected {
    fn default() -> Self {
        Self::Static(Theme::default())
    }
}

impl Selected {
    pub fn new(first: Theme, second: Option<Theme>) -> Selected {
        match second {
            Some(second) => Selected::Dynamic {
                light: first,
                dark: second,
            },
            None => Selected::Static(first),
        }
    }
}
