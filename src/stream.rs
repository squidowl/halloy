use data::server;
pub use data::stream::{self, *};
use iced::{subscription, Subscription};

pub fn run(entry: server::Entry) -> Subscription<stream::Update> {
    // Channel messages are batched every 50ms so channel size 10 ~= 500ms which
    // app thread should more than easily keep up with
    subscription::channel(entry.server.clone(), 10, move |sender| {
        stream::run(entry, sender)
    })
}
