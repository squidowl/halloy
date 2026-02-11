#![allow(dead_code)]
use data::Config;
use iced::advanced::text;
use iced::widget::text::LineHeight;

pub use self::anchored_overlay::anchored_overlay;
pub use self::color_picker::color_picker;
pub use self::combo_box::combo_box;
pub use self::context_menu::context_menu;
pub use self::decorate::decorate;
pub use self::double_pass::double_pass;
pub use self::font_style_pick_list::font_style_pick_list;
pub use self::key_press::key_press;
pub use self::message_content::message_content;
pub use self::modal::modal;
pub use self::notify_visibility::notify_visibility;
pub use self::on_resize::on_resize;
pub use self::selectable_rich_text::selectable_rich_text;
pub use self::selectable_text::selectable_text;
pub use self::shortcut::shortcut;
pub use self::tooltip::tooltip;
use crate::appearance::theme::TEXT_SIZE;
use crate::{Theme, font, theme};

pub mod anchored_overlay;
pub mod color_picker;
pub mod combo_box;
pub mod context_menu;
pub mod decorate;
pub mod double_click;
pub mod double_pass;
pub mod font_style_pick_list;
pub mod key_press;
pub mod message_content;
pub mod modal;
pub mod notify_visibility;
pub mod on_resize;
pub mod pick_list;
pub mod selectable_rich_text;
pub mod selectable_text;
pub mod shortcut;
pub mod tooltip;

pub type Renderer = iced::Renderer;
pub type Element<'a, Message> = iced::Element<'a, Message, Theme, Renderer>;
pub type Content<'a, Message> =
    iced::widget::pane_grid::Content<'a, Message, Theme, Renderer>;
pub type TitleBar<'a, Message> =
    iced::widget::pane_grid::TitleBar<'a, Message, Theme, Renderer>;
pub type Column<'a, Message> =
    iced::widget::Column<'a, Message, Theme, Renderer>;
pub type Row<'a, Message> = iced::widget::Row<'a, Message, Theme, Renderer>;
pub type Text<'a> = iced::widget::Text<'a, Theme, Renderer>;
pub type Container<'a, Message> =
    iced::widget::Container<'a, Message, Theme, Renderer>;
pub type Button<'a, Message> = iced::widget::Button<'a, Message, Theme>;

pub enum Marker {
    Dot,
    Expand,
    Contract,
    None,
}

pub fn text<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
    config: &Config,
) -> Text<'a> {
    iced::widget::text(content).line_height(theme::line_height(&config.font))
}

pub fn message_marker<'a, M>(
    marker: Marker,
    width: Option<f32>,
    config: &'a Config,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
    on_press: Option<M>,
) -> Element<'a, M>
where
    M: Clone + 'a,
{
    let font_size = config.font.size.map_or(TEXT_SIZE, f32::from);

    let def_line_height = 1.3;
    let cfg_line_height = config.font.line_height.unwrap_or(def_line_height);
    let line_height_ratio = cfg_line_height / def_line_height;

    let (text, font_size, line_height) = match marker {
        Marker::Dot => (
            "\u{E81A}",
            font_size * font::MESSAGE_MARKER_FONT_SCALE,
            LineHeight::Relative(
                cfg_line_height / font::MESSAGE_MARKER_FONT_SCALE,
            ),
        ),
        Marker::Expand => (
            "\u{E81B}",
            font_size * 0.75,
            LineHeight::Relative(1.75 * line_height_ratio),
        ),
        Marker::Contract => (
            "\u{E81C}",
            font_size * 0.75,
            LineHeight::Relative(1.75 * line_height_ratio),
        ),
        Marker::None => ("", font_size, LineHeight::Relative(1.0)),
    };

    let mut marker: Element<'a, M> = selectable_text(text)
        .line_height(line_height)
        .font(font::ICON)
        .style(style)
        .size(font_size)
        .into();

    if let Some(on_press) = on_press {
        marker = button::transparent_button(marker, on_press);
    }

    if let Some(width) = width {
        iced::widget::container(marker)
            .width(width)
            .align_x(text::Alignment::Right)
            .into()
    } else {
        marker
    }
}

pub mod button {
    use super::Element;
    use crate::appearance::theme;

    /// Transparent button which simply makes the given content
    /// into a clickable button without additional styling.
    pub fn transparent_button<'a, Message>(
        content: impl Into<Element<'a, Message>>,
        message: Message,
    ) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        iced::widget::button(content)
            .padding(0)
            .style(theme::button::bare)
            .on_press(message)
            .into()
    }
}
