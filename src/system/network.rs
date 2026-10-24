use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use futures::SinkExt;
use hashbrown::HashSet;
use iced::Subscription;
use netwatcher::async_adapter::Tokio;

use super::Event;

pub fn events() -> Subscription<Event> {
    Subscription::run(|| {
        iced::stream::channel(10, async |mut output| {
            let mut watch = match netwatcher::watch_interfaces_async::<Tokio>()
            {
                Ok(watch) => watch,
                Err(error) => {
                    log::warn!("network monitoring unavailable: {error}");
                    return;
                }
            };

            // initial update describes the current interfaces, not a change
            let initial = addresses(&watch.changed().await);
            let mut routable = routable_addresses(&initial);

            if routable.is_empty() {
                log::info!("network unavailable");
            }

            // streams assume the network is up until told otherwise
            if routable.is_empty()
                && output
                    .send(Event::NetworkChanged {
                        available: false,
                        addresses: Arc::new(initial),
                    })
                    .await
                    .is_err()
            {
                return;
            }

            loop {
                let mut update = watch.changed().await;

                // network updates may burst, so we coalesce them over a period
                // to avoid needless network probes down the line.
                //
                // from limited testing on mac, 1-5 requests can occur over
                // ~450ms.
                while let Ok(next) = tokio::time::timeout(
                    Duration::from_secs(1),
                    watch.changed(),
                )
                .await
                {
                    update = next;
                }

                let addresses = addresses(&update);
                let next_routable = routable_addresses(&addresses);

                // ignore changes that leave routable addresses alone
                if next_routable == routable {
                    continue;
                }

                routable = next_routable;
                let available = !routable.is_empty();

                if available {
                    log::info!("network changed: {routable:?}");
                } else {
                    log::info!("network unavailable");
                }

                let event = Event::NetworkChanged {
                    available,
                    addresses: Arc::new(addresses),
                };

                if output.send(event).await.is_err() {
                    break;
                }
            }
        })
    })
}

fn addresses(update: &netwatcher::Update) -> HashSet<IpAddr> {
    update
        .interfaces
        .values()
        .flat_map(|interface| &interface.ips)
        .map(|record| record.ip)
        .collect()
}

fn routable_addresses(addresses: &HashSet<IpAddr>) -> HashSet<IpAddr> {
    addresses.iter().copied().filter(is_routable).collect()
}

fn is_routable(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            !ip.is_loopback()
                && !ip.is_unicast_link_local()
                && !ip.is_unspecified()
        }
    }
}
