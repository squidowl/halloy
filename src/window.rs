use futures::{stream::BoxStream, Future, Stream, StreamExt};
use iced::{advanced::graphics::futures::subscription, Point, Size, Subscription};

use data::window;

pub use data::window::{Error, Event};
pub use iced::window::{close, open, Id, Settings};
use log::warn;

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Main,
    ThemeEditor,
}

#[derive(Debug, Clone, Copy)]
pub struct Windows {
    pub main: Window,
    pub theme_editor: Option<Window>,
}

impl Windows {
    pub fn kind(&self, id: Id) -> Option<Kind> {
        if id == self.main.id {
            Some(Kind::Main)
        } else if self.theme_editor.as_ref().map(|w| w.id) == Some(id) {
            Some(Kind::ThemeEditor)
        } else {
            None
        }
    }

    pub fn save(&self) -> impl Future<Output = Result<(), Error>> {
        let main = self.main;

        async move { main.data.save().await }
    }

    pub fn close(&mut self, id: Id) -> Option<Kind> {
        if id == self.main.id {
            Some(Kind::Main)
        } else if self.theme_editor.as_ref().map(|w| w.id) == Some(id) {
            self.theme_editor = None;
            Some(Kind::ThemeEditor)
        } else {
            None
        }
    }

    pub fn update(&mut self, id: Id, event: Event) {
        if id == self.main.id {
            self.main.data.update(event);
        } else if self.theme_editor.as_ref().map(|w| w.id) == Some(id) {
            self.theme_editor.as_mut().unwrap().data.update(event);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub id: Id,
    pub data: data::Window,
}

impl Window {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            data: data::Window::default(),
        }
    }

    pub fn load(id: Id) -> Self {
        Self {
            id,
            data: data::Window::load()
                .inspect_err(|err| warn!("Failed to load window data, {err}"))
                .unwrap_or_default(),
        }
    }
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
        position: window::Position,
    },
    Resizing {
        stream: T,
        id: Id,
        size: window::Size,
    },
    MovingAndResizing {
        stream: T,
        id: Id,
        position: window::Position,
        size: window::Size,
    },
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

        let window_events = events.filter_map(|event| {
            futures::future::ready(match event {
                subscription::Event::Interaction {
                    window: id,
                    event: iced::Event::Window(window_event),
                    status: _,
                } => match window_event {
                    iced::window::Event::Moved(Point { x, y }) => {
                        Some((id, Event::Moved(window::Position::new(x, y))))
                    }
                    iced::window::Event::Resized(Size { width, height }) => {
                        Some((id, Event::Resized(window::Size::new(width, height))))
                    }
                    iced::window::Event::Focused => Some((id, Event::Focused)),
                    iced::window::Event::Unfocused => Some((id, Event::Unfocused)),
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
