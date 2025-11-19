use futures::StreamExt;
use futures::channel::mpsc;
use futures::stream::BoxStream;
use iced::Subscription;
use iced::advanced::graphics::futures::subscription;
use iced::advanced::subscription::Hasher;
use signal_hook::consts::SIGUSR1;
use signal_hook::iterator::Signals;

struct UnixSignal {
    signals: Vec<i32>,
}

impl subscription::Recipe for UnixSignal {
    type Output = i32;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
        self.signals.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream,
    ) -> BoxStream<'static, Self::Output> {
        let (tx, rx) = mpsc::unbounded();
        let signals = self.signals.clone();

        std::thread::spawn(move || {
            let mut signals_iter = match Signals::new(&signals) {
                Ok(iter) => iter,
                Err(e) => {
                    log::error!("Failed to create signal listener: {e}");
                    return;
                }
            };

            let handle = signals_iter.handle();

            for signal in signals_iter.forever() {
                if tx.unbounded_send(signal).is_err() {
                    break;
                }
            }

            handle.close();
        });

        rx.boxed()
    }
}

pub fn subscription() -> Subscription<i32> {
    subscription::from_recipe(UnixSignal {
        signals: vec![SIGUSR1],
    })
}
