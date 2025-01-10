use futures::{stream::BoxStream, Stream, StreamExt};
use iced::{advanced::graphics::futures::subscription, Point, Size, Subscription};

pub use data::window::{Error, MIN_SIZE};
pub use iced::window::{close, gain_focus, open, Id, Position, Settings};

#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub id: Id,
    pub position: Option<Point>,
    pub size: Size,
    pub focused: bool,
}

impl Window {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            position: None,
            size: Size::default(),
            focused: false,
        }
    }

    pub fn opened(&mut self, position: Option<Point>, size: Size) {
        self.position = position;
        self.size = size;
        self.focused = true;
    }
}

impl From<Window> for data::Window {
    fn from(window: Window) -> Self {
        data::Window {
            position: window.position,
            size: window.size,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Moved(Point),
    Resized(Size),
    Focused,
    Unfocused,
    Opened { position: Option<Point>, size: Size },
    CloseRequested,
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn settings() -> Settings {
    Settings::default()
}

#[cfg(target_os = "linux")]
pub fn settings() -> Settings {
    use data::environment;
    use iced::window;

    Settings {
        platform_specific: window::settings::PlatformSpecific {
            application_id: environment::APPLICATION_ID.to_string(),
            override_redirect: false,
        },
        ..Default::default()
    }
}

#[cfg(target_os = "macos")]
pub fn settings() -> Settings {
    use iced::window;

    Settings {
        platform_specific: window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..Default::default()
    }
}

#[cfg(target_os = "windows")]
pub fn settings() -> Settings {
    use iced::window;
    use image::EncodableLayout;

    let img = image::load_from_memory_with_format(
        include_bytes!("../assets/logo.png"),
        image::ImageFormat::Png,
    );
    match img {
        Ok(img) => match img.as_rgba8() {
            Some(icon) => Settings {
                icon: window::icon::from_rgba(
                    icon.as_bytes().to_vec(),
                    icon.width(),
                    icon.height(),
                )
                .ok(),
                ..Default::default()
            },
            None => Default::default(),
        },
        Err(_) => Settings {
            ..Default::default()
        },
    }
}

pub fn events() -> Subscription<(Id, Event)> {
    subscription::from_recipe(Events)
}

enum State<T: Stream<Item = (Id, Event)>> {
    Idle {
        stream: T,
    },
    Moving {
        stream: T,
        id: Id,
        position: Point,
    },
    Resizing {
        stream: T,
        id: Id,
        size: Size,
    },
    MovingAndResizing {
        stream: T,
        id: Id,
        position: Point,
        size: Size,
    },
}

#[derive(Debug, Clone, Copy)]
struct EventSkipper {
    skipped: i32,
    threshold: i32,
}

impl EventSkipper {
    fn new(threshold: i32) -> EventSkipper {
        EventSkipper {
            skipped: 0,
            threshold,
        }
    }

    fn skip(&mut self) {
        self.skipped += 1;
    }

    fn should_process(&self) -> bool {
        self.skipped >= self.threshold
    }
}

struct Events;

impl subscription::Recipe for Events {
    type Output = (Id, Event);

    fn hash(&self, state: &mut subscription::Hasher) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        events: subscription::EventStream,
    ) -> BoxStream<'static, Self::Output> {
        use futures::stream;
        const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

        let mut move_events = {
            let threshold = if cfg!(target_os = "windows") { 1 } else { 0 };
            EventSkipper::new(threshold)
        };

        let mut resize_events = {
            let threshold = if cfg!(target_os = "windows") { 2 } else { 0 };
            EventSkipper::new(threshold)
        };

        let window_events = events.filter_map(move |event| {
            futures::future::ready(match event {
                subscription::Event::Interaction {
                    window: id,
                    event: iced::Event::Window(window_event),
                    status: _,
                } => match window_event {
                    iced::window::Event::Moved(point) => {
                        if move_events.should_process() {
                            let is_sign_positive =
                                point.x.is_sign_positive() && point.y.is_sign_positive();
                            is_sign_positive.then_some((id, Event::Moved(point)))
                        } else {
                            move_events.skip();
                            None
                        }
                    }
                    iced::window::Event::Resized(size) => {
                        if resize_events.should_process() {
                            let is_above_zero = size.width > 0.0 && size.height > 0.0;
                            is_above_zero.then_some((id, Event::Resized(size)))
                        } else {
                            resize_events.skip();
                            None
                        }
                    }
                    iced::window::Event::Focused => Some((id, Event::Focused)),
                    iced::window::Event::Unfocused => Some((id, Event::Unfocused)),
                    iced::window::Event::Opened { position, size } => {
                        Some((id, Event::Opened { position, size }))
                    }
                    iced::window::Event::CloseRequested => Some((id, Event::CloseRequested)),
                    _ => None,
                },
                _ => None,
            })
        });

        stream::unfold(
            State::Idle {
                stream: window_events,
            },
            move |state| async move {
                match state {
                    State::Idle { mut stream } => {
                        stream.next().await.map(|(id, event)| match event {
                            Event::Moved(position) => (
                                vec![],
                                State::Moving {
                                    stream,
                                    id,
                                    position,
                                },
                            ),
                            Event::Resized(size) => (vec![], State::Resizing { stream, id, size }),
                            Event::Focused => (vec![(id, Event::Focused)], State::Idle { stream }),
                            Event::Unfocused => {
                                (vec![(id, Event::Unfocused)], State::Idle { stream })
                            }
                            Event::Opened { position, size } => (
                                vec![(id, Event::Opened { position, size })],
                                State::Idle { stream },
                            ),
                            Event::CloseRequested => {
                                (vec![(id, Event::CloseRequested)], State::Idle { stream })
                            }
                        })
                    }
                    State::Moving {
                        mut stream,
                        id,
                        position,
                    } => {
                        let next_event = tokio::time::timeout(TIMEOUT, stream.next()).await;

                        match next_event {
                            Ok(Some((next_id, Event::Moved(position)))) if next_id == id => Some((
                                vec![],
                                State::Moving {
                                    stream,
                                    id,
                                    position,
                                },
                            )),
                            Ok(Some((next_id, Event::Resized(size)))) if next_id == id => Some((
                                vec![],
                                State::MovingAndResizing {
                                    stream,
                                    id,
                                    position,
                                    size,
                                },
                            )),
                            _ => Some((vec![(id, Event::Moved(position))], State::Idle { stream })),
                        }
                    }
                    State::Resizing {
                        mut stream,
                        id,
                        size,
                    } => {
                        let next_event = tokio::time::timeout(TIMEOUT, stream.next()).await;

                        match next_event {
                            Ok(Some((next_id, Event::Resized(size)))) if next_id == id => {
                                Some((vec![], State::Resizing { stream, id, size }))
                            }
                            Ok(Some((next_id, Event::Moved(position)))) if next_id == id => Some((
                                vec![],
                                State::MovingAndResizing {
                                    stream,
                                    id,
                                    position,
                                    size,
                                },
                            )),
                            _ => Some((vec![(id, Event::Resized(size))], State::Idle { stream })),
                        }
                    }
                    State::MovingAndResizing {
                        mut stream,
                        id,
                        position,
                        size,
                    } => {
                        let next_event = tokio::time::timeout(TIMEOUT, stream.next()).await;

                        match next_event {
                            Ok(Some((next_id, Event::Moved(position)))) if next_id == id => Some((
                                vec![],
                                State::MovingAndResizing {
                                    stream,
                                    id,
                                    position,
                                    size,
                                },
                            )),
                            Ok(Some((next_id, Event::Resized(size)))) if next_id == id => Some((
                                vec![],
                                State::MovingAndResizing {
                                    stream,
                                    id,
                                    position,
                                    size,
                                },
                            )),
                            _ => Some((
                                vec![(id, Event::Moved(position)), (id, Event::Resized(size))],
                                State::Idle { stream },
                            )),
                        }
                    }
                }
            },
        )
        .map(stream::iter)
        .flatten()
        .boxed()
    }
}
