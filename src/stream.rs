use std::hash::Hash;

pub use data::stream::{self, *};
use data::{config, server};
use futures::Stream;
use iced::Subscription;

pub fn run(
    entry: server::Entry,
    proxy: Option<config::Proxy>,
) -> Subscription<stream::Update> {
    struct State {
        entry: server::Entry,
        proxy: Option<config::Proxy>,
    }

    impl State {
        fn run(&self) -> impl Stream<Item = stream::Update> + use<> {
            stream::run(self.entry.clone(), self.proxy.clone())
        }
    }

    impl PartialEq for State {
        fn eq(&self, other: &Self) -> bool {
            self.entry.server.eq(&other.entry.server)
        }
    }

    impl Hash for State {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.entry.server.hash(state);
        }
    }

    Subscription::run_with(State { entry, proxy }, State::run)
}
