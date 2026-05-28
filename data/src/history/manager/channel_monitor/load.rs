use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt, stream};

use super::is_channel_message;
use crate::history::manager::{Data, History};
use crate::{client, history};

const MAX_CONCURRENT: usize = 4;

enum Source {
    Memory {
        server: crate::Server,
        messages: Vec<crate::Message>,
    },
    Storage {
        server: crate::Server,
        channel: crate::target::Channel,
        seed: Option<history::Seed>,
        unflushed_messages: Vec<crate::Message>,
    },
}

pub(super) fn all(
    data: &Data,
    clients: &client::Map,
) -> BoxFuture<'static, history::Loaded> {
    let sources = clients
        .connected_servers()
        .flat_map(|server| {
            clients.get_channels(server).map(|channel| {
                history::Kind::Channel(server.clone(), channel.clone())
            })
        })
        .filter_map(|kind| source(data, &kind, clients))
        .collect();

    combine(sources).boxed()
}

pub(super) fn channel(
    data: &Data,
    kind: &history::Kind,
    clients: &client::Map,
) -> Option<BoxFuture<'static, history::Loaded>> {
    Some(combine(vec![source(data, kind, clients)?]).boxed())
}

fn source(
    data: &Data,
    kind: &history::Kind,
    clients: &client::Map,
) -> Option<Source> {
    let history::Kind::Channel(server, channel) = kind else {
        return None;
    };

    Some(match data.map.get(kind) {
        Some(History::Full {
            messages,
            last_updated_at,
            cleared,
            ..
        }) if *cleared || last_updated_at.is_some() => Source::Memory {
            server: server.clone(),
            messages: if *cleared {
                vec![]
            } else {
                messages
                    .iter()
                    .filter(|message| is_channel_message(message))
                    .cloned()
                    .collect()
            },
        },
        partial => {
            let unflushed_messages = match partial {
                Some(History::Partial {
                    pending_messages,
                    flushing_messages,
                    ..
                }) => pending_messages
                    .iter()
                    .chain(flushing_messages)
                    .filter(|(message, _)| is_channel_message(message))
                    .map(|(message, _)| message.clone())
                    .collect(),
                _ => vec![],
            };

            Source::Storage {
                server: server.clone(),
                channel: channel.clone(),
                seed: clients.get_seed(kind),
                unflushed_messages,
            }
        }
    })
}

async fn combine(sources: Vec<Source>) -> history::Loaded {
    let histories = stream::iter(sources)
        .map(|source| async move {
            match source {
                Source::Memory { server, messages } => {
                    Ok::<_, history::Error>((server, messages))
                }
                Source::Storage {
                    server,
                    channel,
                    seed,
                    unflushed_messages,
                } => {
                    let mut loaded = history::load(
                        history::Kind::Channel(server.clone(), channel),
                        seed,
                    )
                    .await?;

                    for message in unflushed_messages {
                        history::insert_message(
                            &mut loaded.messages,
                            message,
                            None,
                        );
                    }

                    Ok((server, loaded.messages))
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT);
    futures::pin_mut!(histories);

    let mut combined = vec![];

    while let Some(result) = histories.next().await {
        let (server, messages) = match result {
            Ok(history) => history,
            Err(error) => {
                log::warn!(
                    "failed to load channel monitor channel history: {error}"
                );
                continue;
            }
        };

        combined.extend(messages.into_iter().filter_map(move |mut message| {
            if !is_channel_message(&message) {
                return None;
            }

            let crate::message::Target::Channel { channel, source } =
                message.target
            else {
                return None;
            };

            message.target = crate::message::Target::ChannelMonitor {
                server: server.clone(),
                channel,
                source,
            };

            Some(message)
        }));

        if combined.len() > history::MAX_MESSAGES {
            let keep_from = combined.len() - history::MAX_MESSAGES;
            combined.select_nth_unstable_by_key(keep_from, |message| {
                message.server_time
            });
            combined.drain(..keep_from);
        }
    }

    combined.sort_unstable_by_key(|message| message.server_time);

    history::Loaded {
        messages: combined,
        metadata: history::Metadata::default(),
    }
}
