use std::collections::HashMap;

use futures::{SinkExt, StreamExt};
use iced::Subscription;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use super::Event;

#[zbus::proxy(
    interface = "org.freedesktop.portal.Inhibit",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait Inhibit {
    #[zbus(property)]
    fn version(&self) -> zbus::Result<u32>;

    fn create_monitor(
        &self,
        window: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    fn query_end_response(
        &self,
        session_handle: ObjectPath<'_>,
    ) -> zbus::Result<()>;

    fn suspending_response(
        &self,
        session_handle: ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn state_changed(
        &self,
        session_handle: OwnedObjectPath,
        state: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.portal.Request",
    default_service = "org.freedesktop.portal.Desktop"
)]
trait Request {
    #[zbus(signal)]
    fn response(
        &self,
        response: u32,
        results: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Running = 1,
    QueryEnd = 2,
    Ending = 3,
    Suspending = 4,
}

impl TryFrom<u32> for SessionState {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Running),
            2 => Ok(Self::QueryEnd),
            3 => Ok(Self::Ending),
            4 => Ok(Self::Suspending),
            _ => Err(()),
        }
    }
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
    let connection = zbus::Connection::session().await?;

    let inhibit = InhibitProxy::new(&connection).await?;
    if inhibit.version().await? < 4 {
        return Err(zbus::Error::Failure(
            "Inhibit portal version 4 or higher is required".into(),
        ));
    }

    let token = format!("halloy_{}", rand::random::<u64>());
    let sender = connection
        .unique_name()
        .expect("session bus connection has a unique name")
        .as_str()
        .trim_start_matches(':')
        .replace('.', "_");
    let request_path =
        format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    let request = RequestProxy::builder(&connection)
        .path(request_path)?
        .build()
        .await?;

    let mut responses = request.receive_response().await?;
    let mut signals = inhibit.receive_state_changed().await?;

    inhibit
        .create_monitor(
            "", // No idea what goes here.
            HashMap::from([
                ("handle_token", Value::from(token.as_str())),
                ("session_handle_token", Value::from(token.as_str())),
                ("suspend_aware", Value::from(true)),
            ]),
        )
        .await?;
    let response = responses.next().await.ok_or_else(|| {
        zbus::Error::Failure(
            "monitor request did not receive a response".into(),
        )
    })?;
    let args = response.args()?;
    if args.response != 0 {
        return Err(zbus::Error::Failure(format!(
            "monitor request failed: {}",
            args.response
        )));
    }

    // Interface got the wrong type, must be converted...
    let session_handle =
        args.results.get("session_handle").ok_or_else(|| {
            zbus::Error::Failure(
                "monitor response has no session handle".into(),
            )
        })?;
    let session_handle =
        ObjectPath::try_from(<&str>::try_from(session_handle)?)?;

    let mut suspended = false;
    while let Some(signal) = signals.next().await {
        let args = signal.args()?;

        // Check if the signal comes from our session.
        if args.session_handle.as_str() != session_handle.as_str() {
            continue;
        }

        let Some(state) = args.state.get("session-state") else {
            continue;
        };

        let Ok(state) = SessionState::try_from(u32::try_from(state)?) else {
            continue;
        };

        let event = match state {
            SessionState::Suspending => {
                inhibit.suspending_response(session_handle.clone()).await?;

                if suspended {
                    continue;
                }

                suspended = true;
                Event::Suspending
            }
            SessionState::Running if suspended => {
                suspended = false;
                Event::Resumed
            }
            SessionState::QueryEnd => {
                inhibit.query_end_response(session_handle.clone()).await?;
                continue;
            }
            _ => continue,
        };

        if output.send(event).await.is_err() {
            break;
        }
    }

    Ok(())
}
