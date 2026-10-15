use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;

use iced::Subscription;

#[cfg(target_os = "macos")]
mod macos;
mod network;

#[derive(Debug, Clone)]
pub enum Event {
    Suspending,
    Resumed,
    NetworkChanged {
        available: bool,
        addresses: Arc<HashSet<IpAddr>>,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Awake,
    Suspended,
}

impl State {
    pub fn suppresses_connection_events(self) -> bool {
        !matches!(self, Self::Awake)
    }
}

pub fn events() -> Subscription<Event> {
    Subscription::batch([power_events(), network::events()])
}

#[cfg(target_os = "macos")]
fn power_events() -> Subscription<Event> {
    macos::events()
}

#[cfg(not(target_os = "macos"))]
fn power_events() -> Subscription<Event> {
    Subscription::none()
}
