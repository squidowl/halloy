use std::path::{Path, PathBuf};

use futures::channel::oneshot;
use futures::future::{FutureExt, Shared};
use iced::{Event, Rectangle, Task, mouse, window};
use uuid::Uuid;

use crate::image_animation;

#[derive(Debug, Clone)]
pub struct Request {
    id: Uuid,
    path: PathBuf,
    cancelled: Shared<oneshot::Receiver<()>>,
}

impl Request {
    pub fn play<Message: Send + 'static>(self) -> Task<Message> {
        let (playback, handle) =
            image_animation::play(self.id, self.path).abortable();
        let abort_on_error = handle.clone();
        let playback = playback.then(move |message| {
            abort_on_error.abort();
            log::warn!("Failed to play GIF: {}", message.error);
            super::failed(message.id)
        });

        Task::batch([
            playback,
            Task::future(async move {
                let _ = self.cancelled.await;
                handle.abort();
            })
            .discard(),
        ])
    }
}

#[derive(Default)]
enum Playback {
    #[default]
    Idle,
    Playing {
        id: Uuid,
        _cancel: oneshot::Sender<()>,
    },
    Failed,
}

pub(super) struct State {
    source: Option<PathBuf>,
    playback: Playback,
    focused: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            source: None,
            playback: Playback::Idle,
            focused: true,
        }
    }
}

impl State {
    pub fn id(&self, path: &Path) -> Option<Uuid> {
        if self.source.as_deref() == Some(path)
            && let Playback::Playing { id, .. } = self.playback
        {
            Some(id)
        } else {
            None
        }
    }

    pub fn failed(&mut self) {
        self.playback = Playback::Failed;
    }

    pub fn update(
        &mut self,
        path: &Path,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) -> Option<Request> {
        match event {
            Event::Window(window::Event::Unfocused) => self.focused = false,
            Event::Window(window::Event::Focused) => self.focused = true,
            _ => {}
        }

        if self.source.as_deref() != Some(path) {
            self.playback = Playback::Idle;
            self.source = Some(path.to_owned());
        }

        let hovered = self.focused
            && !matches!(event, Event::Mouse(mouse::Event::CursorLeft))
            && viewport
                .intersection(&bounds)
                .is_some_and(|visible| cursor.is_over(visible));

        if !hovered {
            self.playback = Playback::Idle;
        } else if matches!(self.playback, Playback::Idle) {
            let id = Uuid::now_v7();
            let (cancel, cancelled) = oneshot::channel();
            self.playback = Playback::Playing {
                id,
                _cancel: cancel,
            };

            return Some(Request {
                id,
                path: path.to_owned(),
                cancelled: cancelled.shared(),
            });
        }

        None
    }
}
