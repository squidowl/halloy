use futures::stream::BoxStream;
use iced::advanced::subscription::{self, Hasher};
use iced::{self, Subscription};
use ipc::server::Message;

#[cfg(target_os = "macos")]
pub fn listen() -> Subscription<Message> {
    use futures::stream::StreamExt;
    use iced::event::{self, Event};
    use ipc::url::Route;

    struct OnUrl;

    impl subscription::Recipe for OnUrl {
        type Output = Message;

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
                        )) => Route::parse(&url).map(Message::RouteReceived),
                        _ => None,
                    };

                    futures::future::ready(result)
                })
                .boxed()
        }
    }

    Subscription::from_recipe(OnUrl)
}

#[cfg(not(target_os = "macos"))]
pub fn listen() -> Subscription<Message> {
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

    Subscription::from_recipe(Listener)
}
