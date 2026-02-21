use data::appearance::theme::FontStyle;
use iced::widget::pick_list;

use super::Element;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontStylePick(Option<FontStyle>);

impl From<Option<FontStyle>> for FontStylePick {
    fn from(font_style: Option<FontStyle>) -> Self {
        FontStylePick(font_style)
    }
}

impl From<FontStylePick> for Option<FontStyle> {
    fn from(font_style_pick: FontStylePick) -> Self {
        font_style_pick.0
    }
}

impl std::fmt::Display for FontStylePick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self(None) => "None",
            Self(Some(FontStyle::Normal)) => "Normal",
            Self(Some(FontStyle::Bold)) => "Bold",
            Self(Some(FontStyle::Italic)) => "Italic",
            Self(Some(FontStyle::ItalicBold)) => "Bold & Italic",
        })
    }
}

pub fn font_style_pick_list<'a, Message: 'a + Clone>(
    font_style: Option<FontStyle>,
    on_selected: impl Fn(FontStylePick) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let picks = [
        FontStylePick(None),
        FontStylePick(Some(FontStyle::Normal)),
        FontStylePick(Some(FontStyle::Bold)),
        FontStylePick(Some(FontStyle::Italic)),
        FontStylePick(Some(FontStyle::ItalicBold)),
    ];

    pick_list(Some(FontStylePick::from(font_style)), picks, |pick| {
        pick.to_string()
    })
    .on_select(on_selected)
    .placeholder("Font style")
    .into()
}
