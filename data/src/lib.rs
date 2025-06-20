#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]

pub use self::appearance::Theme;
pub use self::buffer::Buffer;
pub use self::command::Command;
pub use self::config::Config;
pub use self::dashboard::Dashboard;
pub use self::input::Input;
pub use self::message::Message;
pub use self::mode::Mode;
pub use self::notification::Notification;
pub use self::pane::Pane;
pub use self::preview::Preview;
pub use self::server::Server;
pub use self::shortcut::Shortcut;
pub use self::target::Target;
pub use self::url::Url;
pub use self::user::User;
pub use self::version::Version;
pub use self::window::Window;

pub mod appearance;
pub mod audio;
pub mod buffer;
pub mod channel;
pub mod client;
pub mod command;
mod compression;
pub mod config;
pub mod ctcp;
pub mod dashboard;
pub mod dcc;
pub mod environment;
pub mod file_transfer;
pub mod history;
pub mod input;
pub mod isupport;
pub mod log;
pub mod message;
pub mod mode;
pub mod notification;
pub mod pane;
pub mod preview;

#[cfg(feature = "hexchat-compat")]
pub mod python;

pub mod serde;
pub mod server;
pub mod shortcut;
pub mod stream;
pub mod target;
pub mod time;
pub mod url;
pub mod user;
pub mod version;
pub mod window;
