use data::stream;
use iced::futures::stream::BoxStream;
use iced::widget::runtime::core::Hasher;
use iced::widget::runtime::futures::subscription::{EventStream, Recipe};
use iced::Subscription;

pub fn run() -> Subscription<stream::Result> {
    Subscription::from_recipe(Client {})
}

pub struct Client {}

impl Recipe for Client {
    type Output = stream::Result;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;

        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        stream::run()
    }
}
