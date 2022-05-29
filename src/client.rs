use data::stream;
use iced::futures::stream::BoxStream;
use iced::Subscription;
use iced_native::subscription::Recipe;
use iced_native::Hasher;

pub fn run() -> Subscription<stream::Result> {
    Subscription::from_recipe(Client {})
}

pub struct Client {}

impl<E> Recipe<iced_native::Hasher, E> for Client {
    type Output = stream::Result;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;

        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<E>) -> BoxStream<Self::Output> {
        stream::run()
    }
}
