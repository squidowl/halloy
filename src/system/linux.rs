use futures::{SinkExt, StreamExt};
use iced::Subscription;

use super::Event;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

pub fn events() -> Subscription<Event> {
    Subscription::run(|| {
        iced::stream::channel(10, async |mut output| {
            if let Err(error) = watch(&mut output).await {
                log::warn!("power monitoring unavailable: {error}");
            }
        })
    })
}

async fn watch(
    output: &mut futures::channel::mpsc::Sender<Event>,
) -> zbus::Result<()> {
    let connection = zbus::Connection::system().await?;
    let manager = ManagerProxy::new(&connection).await?;
    let mut signals = manager.receive_prepare_for_sleep().await?;

    while let Some(signal) = signals.next().await {
        let suspending = signal.args()?.start;
        let event = if suspending {
            Event::Suspending
        } else {
            Event::Resumed
        };

        if output.send(event).await.is_err() {
            break;
        }
    }

    Ok(())
}
