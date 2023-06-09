#![allow(dead_code)]
pub use self::anchored_overlay::anchored_overlay;
pub use self::collection::Collection;
pub use self::input::input;
pub use self::key_press::key_press;
use crate::Theme;

pub mod anchored_overlay;
pub mod collection;
pub mod input;
pub mod key_press;

pub type Renderer = iced::Renderer<Theme>;
pub type Element<'a, Message> = iced::Element<'a, Message, Renderer>;
pub type Content<'a, Message> = iced::widget::pane_grid::Content<'a, Message, Renderer>;
pub type TitleBar<'a, Message> = iced::widget::pane_grid::TitleBar<'a, Message, Renderer>;
pub type Column<'a, Message> = iced::widget::Column<'a, Message, Renderer>;
pub type Row<'a, Message> = iced::widget::Row<'a, Message, Renderer>;
pub type Text<'a> = iced::widget::Text<'a, Renderer>;
pub type Container<'a, Message> = iced::widget::Container<'a, Message, Renderer>;
pub type Button<'a, Message> = iced::widget::Button<'a, Message>;
