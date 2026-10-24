use futures::{SinkExt, StreamExt};
use iced::Subscription;
use zbus::zvariant::OwnedFd;

use super::Event;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Manager {
    fn inhibit(
        &self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> zbus::Result<OwnedFd>;

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

    // logind delays sleep until this lock is released, which gives the
    // suspending event time to land before the process freezes
    let mut _lock = inhibit(&manager).await;

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

        _lock = if suspending {
            None
        } else {
            inhibit(&manager).await
        };
    }

    Ok(())
}

async fn inhibit(manager: &ManagerProxy<'_>) -> Option<OwnedFd> {
    manager
        .inhibit("sleep", "Halloy", "Pause IRC connections", "delay")
        .await
        .inspect_err(|error| {
            log::debug!("sleep inhibitor unavailable: {error}");
        })
        .ok()
}
