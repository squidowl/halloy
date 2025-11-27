use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TextInput {
    pub visibility: Visibility,
    pub auto_format: AutoFormat,
    pub autocomplete: Autocomplete,
    pub nickname: Nickname,
    pub key_bindings: KeyBindings,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyBindings {
    #[default]
    Default,
    Emacs,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Focused,
    #[default]
    Always,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Nickname {
    pub enabled: bool,
    pub show_access_level: bool,
}

impl Default for Nickname {
    fn default() -> Self {
        Self {
            enabled: true,
            show_access_level: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutoFormat {
    #[default]
    Disabled,
    Markdown,
    All,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Autocomplete {
    pub order_by: OrderBy,
    pub sort_direction: SortDirection,
    pub completion_suffixes: [String; 2],
}

impl Default for Autocomplete {
    fn default() -> Self {
        Self {
            order_by: OrderBy::default(),
            sort_direction: SortDirection::default(),
            completion_suffixes: [": ".to_string(), " ".to_string()],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderBy {
    Alpha,
    #[default]
    Recent,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}
