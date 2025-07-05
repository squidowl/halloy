#![allow(dead_code)]
use iced::advanced::text;

pub use self::anchored_overlay::anchored_overlay;
pub use self::color_picker::color_picker;
pub use self::combo_box::combo_box;
pub use self::context_menu::context_menu;
pub use self::decorate::decorate;
pub use self::double_pass::double_pass;
pub use self::key_press::key_press;
pub use self::message_content::message_content;
pub use self::modal::modal;
pub use self::notify_visibility::notify_visibility;
pub use self::selectable_rich_text::selectable_rich_text;
pub use self::selectable_text::selectable_text;
pub use self::shortcut::shortcut;
pub use self::tooltip::tooltip;
use crate::Theme;

pub mod anchored_overlay;
pub mod collection;
pub mod color_picker;
pub mod combo_box;
pub mod context_menu;
pub mod decorate;
pub mod double_click;
pub mod double_pass;
pub mod key_press;
pub mod message_content;
pub mod modal;
pub mod notify_visibility;
pub mod selectable_rich_text;
pub mod selectable_text;
pub mod shortcut;
pub mod tooltip;
pub mod reaction_row;

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

pub fn message_marker<'a, M: 'a>(
    width: Option<f32>,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
) -> Element<'a, M> {
    let marker = selectable_text(MESSAGE_MARKER_TEXT);

    if let Some(width) = width {
        marker.width(width).align_x(text::Alignment::Right)
    } else {
        marker
    }
    .style(style)
    .into()
}

pub const MESSAGE_MARKER_TEXT: &str = " ∙";

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
