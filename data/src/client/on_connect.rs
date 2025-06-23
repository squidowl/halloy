use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, BoxStream};
use futures::{SinkExt, StreamExt};
use tokio::time;

use crate::{Command, Target, command, config, isupport, message, server};

#[derive(Debug)]
pub enum Event {
    OpenBuffers(Vec<Target>),
}

pub struct Stream(BoxStream<'static, Event>);

impl futures::Stream for Stream {
    type Item = Event;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}

impl fmt::Debug for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stream").finish()
    }
}

pub fn on_connect(
    handle: server::Handle,
    config: Arc<config::Server>,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
) -> Stream {
    let commands = config
        .on_connect
        .iter()
        .filter_map(|command| command::parse(command, None, isupport).ok())
        .collect::<Vec<_>>();

    Stream(
        stream::iter(commands)
            .filter_map(move |command| {
                let mut handle = handle.clone();

                async move {
                    match command {
                        Command::Irc(command) => {
                            if let Ok(message) =
                                message::Encoded::try_from(command)
                            {
                                if let Err(e) =
                                    handle.send(message.into()).await
                                {
                                    log::warn!("Error sending message: {e}");
                                }
                            }
                            None
                        }
                        Command::Internal(cmd) => match cmd {
                            command::Internal::OpenBuffers(targets) => {
                                Some(Event::OpenBuffers(targets))
                            }
                            // We don't handle hop when called from connected.
                            command::Internal::Hop(_, _) => None,
                            command::Internal::Delay(seconds) => {
                                time::sleep(Duration::from_secs(seconds)).await;
                                None
                            }
                        },
                    }
                }
            })
            .boxed(),
    )
}
