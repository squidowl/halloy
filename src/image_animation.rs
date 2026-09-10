use std::path::PathBuf;

use data::image::animation;
use iced::widget::image::Handle;
use iced::{Task, task};
use uuid::Uuid;

use crate::widget::{Element, animated_image, image};

#[derive(Debug)]
pub struct Animation {
    id: Uuid,
    task: task::Handle,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub error: String,
}

impl Animation {
    pub fn new(path: PathBuf) -> (Self, Task<Message>) {
        let id = Uuid::now_v7();
        let task = play(id, path);
        let (task, handle) = task.abortable();

        (
            Self {
                id,
                task: handle.abort_on_drop(),
            },
            task,
        )
    }

    pub fn update(&mut self, message: Message) {
        if message.id != self.id {
            return;
        }

        log::warn!("Failed to play GIF: {}", message.error);
        self.task.abort();
    }

    pub fn view<'a, M: 'a>(&self, data: &data::Image) -> Element<'a, M> {
        let still = image::from_data(data, false, iced::ContentFit::Contain);
        if self.task.is_aborted() {
            still
        } else {
            animated_image::view(self.id, &data.path, still)
        }
    }
}

pub fn play(id: Uuid, path: PathBuf) -> Task<Message> {
    animated_image::connect(id).then(move |connection| match connection {
        Ok(connection) => play_frames(id, path.clone(), connection),
        Err(error) => Task::done(Message {
            id,
            error: error.to_string(),
        }),
    })
}

fn play_frames(
    id: Uuid,
    path: PathBuf,
    connection: animated_image::Connection,
) -> Task<Message> {
    let mut next_frame_at = tokio::time::Instant::now();
    Task::run(animation::frames(path), std::convert::identity).then(
        move |frame| match frame {
            Ok(animation::Frame { pixels, delay }) => {
                let present_at = next_frame_at.max(tokio::time::Instant::now());
                next_frame_at = present_at + delay;
                let (width, height) = pixels.dimensions();
                let handle =
                    Handle::from_rgba(width, height, pixels.into_raw());

                // Load the next frame before it is due.
                let connection = connection.clone();
                Task::future(async move {
                    let frame = connection.allocate(handle).await?;
                    tokio::time::sleep_until(present_at).await;
                    connection.present(frame).await
                })
                .then(move |result| match result {
                    Ok(()) => Task::none(),
                    Err(error) => Task::done(Message {
                        id,
                        error: error.to_string(),
                    }),
                })
            }
            Err(error) => Task::done(Message {
                id,
                error: error.to_string(),
            }),
        },
    )
}
