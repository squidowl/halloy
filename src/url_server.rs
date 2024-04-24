use futures::stream::BoxStream;
use iced::advanced::subscription::{self, Hasher};
use iced::{self, Subscription};
use ipc::server::Message;


pub fn listen() -> Subscription<Message> {
    Subscription::from_recipe(Listener)
}

struct Listener;

impl subscription::Recipe for Listener {
    type Output = Message;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;

        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream,
    ) -> BoxStream<'static, Self::Output> {
        ipc::server::run()
    }
}
