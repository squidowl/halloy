use iced::Subscription;

#[cfg(target_os = "macos")]
mod macos;

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Suspending,
    Resumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Power {
    Awake,
    Suspended,
}

pub struct State {
    power: Power,
    #[cfg(target_os = "macos")]
    activity: macos::Activity,
}

impl State {
    pub fn new(maintain_connections: bool) -> Self {
        #[cfg(not(target_os = "macos"))]
        let _ = maintain_connections;

        Self {
            power: Power::Awake,
            #[cfg(target_os = "macos")]
            activity: macos::Activity::new(maintain_connections),
        }
    }

    pub fn suspending(&mut self) {
        self.power = Power::Suspended;
    }

    pub fn resumed(&mut self) {
        self.power = Power::Awake;
    }

    pub fn set_maintain_connections(&mut self, maintain_connections: bool) {
        #[cfg(target_os = "macos")]
        self.activity.set_active(maintain_connections);

        #[cfg(not(target_os = "macos"))]
        let _ = maintain_connections;
    }

    pub fn suppresses_connection_events(&self) -> bool {
        !matches!(self.power, Power::Awake)
    }
}

#[cfg(target_os = "macos")]
pub fn events() -> Subscription<Event> {
    macos::events()
}

#[cfg(not(target_os = "macos"))]
pub fn events() -> Subscription<Event> {
    Subscription::none()
}
