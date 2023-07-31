#![allow(dead_code)]
pub use self::anchored_overlay::anchored_overlay;
pub use self::collection::Collection;
pub use self::context_menu::context_menu;
pub use self::double_click::double_click;
pub use self::double_pass::double_pass;
pub use self::hover::hover;
pub use self::input::input;
pub use self::key_press::key_press;
pub use self::selectable_text::selectable_text;
pub use self::shortcut::shortcut;
use crate::Theme;

pub mod anchored_overlay;
pub mod collection;
pub mod context_menu;
pub mod double_click;
mod double_pass;
pub mod hover;
pub mod input;
pub mod key_press;
pub mod selectable_text;
pub mod shortcut;

pub type Renderer = iced::Renderer<Theme>;
pub type Element<'a, Message> = iced::Element<'a, Message, Renderer>;
pub type Content<'a, Message> = iced::widget::pane_grid::Content<'a, Message, Renderer>;
pub type TitleBar<'a, Message> = iced::widget::pane_grid::TitleBar<'a, Message, Renderer>;
pub type Column<'a, Message> = iced::widget::Column<'a, Message, Renderer>;
pub type Row<'a, Message> = iced::widget::Row<'a, Message, Renderer>;
pub type Text<'a> = iced::widget::Text<'a, Renderer>;
pub type Container<'a, Message> = iced::widget::Container<'a, Message, Renderer>;
pub type Button<'a, Message> = iced::widget::Button<'a, Message>;
