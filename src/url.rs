use futures::stream::{BoxStream, StreamExt};
use iced::advanced::subscription::{self, Hasher};
use iced::event::{self, Event};
use iced::{self, Subscription};
use ipc::url::Route;

pub fn on_url() -> Subscription<Route> {
    Subscription::from_recipe(OnUrl)
}

struct OnUrl;

impl subscription::Recipe for OnUrl {
    type Output = Route;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;

        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        input: subscription::EventStream,
    ) -> BoxStream<'static, Self::Output> {
        input
            .filter_map(move |(event, status)| {
                if let event::Status::Captured = status {
                    return futures::future::ready(None);
                }

                let result = match event {
                    Event::PlatformSpecific(event::PlatformSpecific::MacOS(
                        event::MacOS::ReceivedUrl(url),
                    )) => Route::parse(&url),
                    _ => None,
                };

                futures::future::ready(result)
            })
            .boxed()
    }
}
