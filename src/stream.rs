pub use data::stream::{self, *};
use data::{config, server};
use iced::Subscription;

pub fn run(entry: server::Entry, proxy: Option<config::Proxy>) -> Subscription<stream::Update> {
    Subscription::run_with_id(entry.server.clone(), stream::run(entry, proxy))
}
