use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, BoxStream};
use futures::{SinkExt, StreamExt};
use tokio::time;

use crate::user::NickRef;
use crate::{Command, Target, command, config, isupport, message, server};

#[derive(Debug)]
pub enum Event {
    OpenBuffers(Vec<Target>),
    LeaveBuffers(Vec<Target>, Option<String>),
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
    server_config: Arc<config::Server>,
    our_nickname: NickRef,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
    config: &config::Config,
) -> Stream {
    let commands = server_config
        .on_connect
        .iter()
        .filter_map(|command| {
            command::parse(
                command,
                None,
                Some(our_nickname),
                true,
                isupport,
                config,
            )
            .ok()
        })
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
                                && let Err(e) =
                                    handle.send(message.into()).await
                            {
                                log::warn!("Error sending message: {e}");
                            }
                            None
                        }
                        Command::Internal(cmd) => match cmd {
                            command::Internal::OpenBuffers(targets) => {
                                Some(Event::OpenBuffers(targets))
                            }
                            command::Internal::LeaveBuffers(
                                targets,
                                reason,
                            ) => Some(Event::LeaveBuffers(targets, reason)),
                            command::Internal::Detach(channels) => {
                                Some(Event::LeaveBuffers(
                                    channels
                                        .into_iter()
                                        .map(Target::Channel)
                                        .collect(),
                                    Some("detach".to_string()),
                                ))
                            }
                            command::Internal::Delay(seconds) => {
                                time::sleep(Duration::from_secs(seconds)).await;
                                None
                            }
                            // We don't handle hop, clear-buffer, sysinfo,
                            // channel_discovery, connect, or reconnect when
                            // called on connection.
                            command::Internal::ClearBuffer
                            | command::Internal::ChannelDiscovery
                            | command::Internal::Connect(_)
                            | command::Internal::Hop(_, _)
                            | command::Internal::SysInfo
                            | command::Internal::Reconnect => None,
                        },
                    }
                }
            })
            .boxed(),
    )
}
