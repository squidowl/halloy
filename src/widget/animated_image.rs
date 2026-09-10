use std::any::Any;
use std::path::Path;

use futures::channel::oneshot;
use iced::advanced::image::{Id, Renderer as _};
use iced::advanced::widget::{self, Operation};
use iced::advanced::{Renderer as _, Shell, layout, renderer, shell};
use iced::widget::image::{self, Allocation, Handle};
use iced::{ContentFit, Event, Length, Rectangle, Size, Task, mouse};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::{Element, decorate};

pub mod hover;

#[derive(Default)]
struct State {
    id: Option<Uuid>,
    // Keep the last frame on screen when playback stops.
    frame: Option<(Id, Allocation)>,
    dimensions: Option<(Id, Size<u32>)>,
    hover: hover::State,
    pending: Option<oneshot::Sender<Connection>>,
    receiver: Option<mpsc::Receiver<Command>>,
}

impl State {
    fn update_id(&mut self, id: Option<Uuid>) -> bool {
        if self.id == id {
            return false;
        }
        self.id = id;
        self.pending = None;
        self.receiver = None;
        true
    }
}

enum Command {
    Allocate(Handle, oneshot::Sender<Result<Allocation, image::Error>>),
    Present(Allocation),
}

enum Action {
    Connect(oneshot::Sender<Connection>),
    Failed,
}

#[derive(Clone)]
pub struct Connection {
    sender: mpsc::Sender<Command>,
    waker: shell::Waker,
}

impl Connection {
    pub async fn allocate(
        &self,
        handle: Handle,
    ) -> Result<Allocation, image::Error> {
        let (sender, receiver) = oneshot::channel();
        self.send(Command::Allocate(handle, sender)).await?;
        receiver.await.unwrap_or(Err(image::Error::Unsupported))
    }

    pub async fn present(&self, frame: Allocation) -> Result<(), image::Error> {
        self.send(Command::Present(frame)).await
    }

    async fn send(&self, command: Command) -> Result<(), image::Error> {
        self.sender
            .send(command)
            .await
            .map_err(|_| image::Error::Unsupported)?;
        self.waker.wake();
        Ok(())
    }
}

enum Playback<'a, Message> {
    Preview(Uuid),
    Hover(&'a Path, fn(hover::Request) -> Message),
}

impl<Message> Copy for Playback<'_, Message> {}

impl<Message> Clone for Playback<'_, Message> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Message> Playback<'_, Message> {
    fn id(&self, state: &State) -> Option<Uuid> {
        match self {
            Self::Preview(id) => Some(*id),
            Self::Hover(path, _) => state.hover.id(path),
        }
    }
}

struct Target {
    id: Uuid,
    action: Option<Action>,
}

impl<T> Operation<T> for Target {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn custom(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Any,
    ) {
        if let Some(target) = state.downcast_mut::<Target>()
            && target.id == self.id
        {
            target.action = self.action.take();
        }
    }
}

pub fn connect(id: Uuid) -> Task<Result<Connection, image::Error>> {
    let (sender, receiver) = oneshot::channel();
    widget::operate(Target {
        id,
        action: Some(Action::Connect(sender)),
    })
    .chain(Task::future(async move {
        receiver.await.map_err(|_| image::Error::Unsupported)
    }))
}

fn failed<Message: Send + 'static>(id: Uuid) -> Task<Message> {
    widget::operate(Target {
        id,
        action: Some(Action::Failed),
    })
}

pub fn view<'a, Message: 'a>(
    id: Uuid,
    path: &Path,
    still: Element<'a, Message>,
) -> Element<'a, Message> {
    viewer(
        still,
        Handle::from_path(path),
        Playback::Preview(id),
        false,
        ContentFit::Contain,
    )
}

pub fn on_hover<'a, Message: 'a>(
    path: &'a Path,
    still: Element<'a, Message>,
    on_start: fn(hover::Request) -> Message,
    round_corners: bool,
    content_fit: ContentFit,
) -> Element<'a, Message> {
    viewer(
        still,
        Handle::from_path(path),
        Playback::Hover(path, on_start),
        round_corners,
        content_fit,
    )
}

fn viewer<'a, Message: 'a>(
    still: Element<'a, Message>,
    handle: Handle,
    playback: Playback<'a, Message>,
    round_corners: bool,
    content_fit: ContentFit,
) -> Element<'a, Message> {
    let source = handle.id();
    let update_playback = playback;
    let operate_playback = playback;
    decorate(still)
        .layout(
            move |state: &mut State,
                  inner: &mut Element<'a, Message>,
                  tree: &mut widget::Tree,
                  renderer: &iced::Renderer,
                  limits: &layout::Limits| {
                if state.frame.as_ref().is_some_and(|(id, _)| *id != source) {
                    state.frame = None;
                }
                // Keep the image's size if its still image leaves the cache.
                let dimensions = state
                    .dimensions
                    .filter(|(id, _)| *id == source)
                    .map(|(_, dimensions)| dimensions)
                    .or_else(|| {
                        state.frame.as_ref().map(|(_, frame)| frame.size())
                    })
                    .or_else(|| renderer.measure_image(&handle));

                if let Some(dimensions) = dimensions {
                    state.dimensions = Some((source, dimensions));
                    let size = inner.as_widget().size();
                    let dimensions = Size::new(
                        dimensions.width as f32,
                        dimensions.height as f32,
                    );
                    let bounds =
                        limits.resolve(size.width, size.height, dimensions);
                    let fitted = content_fit.fit(dimensions, bounds);
                    layout::Node::new(Size {
                        width: if size.width == Length::Shrink {
                            bounds.width.min(fitted.width)
                        } else {
                            bounds.width
                        },
                        height: if size.height == Length::Shrink {
                            bounds.height.min(fitted.height)
                        } else {
                            bounds.height
                        },
                    })
                } else {
                    inner.as_widget_mut().layout(tree, renderer, limits)
                }
            },
        )
        .update(
            move |state: &mut State,
                  inner: &mut Element<'a, Message>,
                  tree: &mut widget::Tree,
                  event: &Event,
                  layout: layout::Layout<'_>,
                  cursor: mouse::Cursor,
                  renderer: &iced::Renderer,
                  shell: &mut Shell<'_, Message>,
                  viewport: &Rectangle| {
                if let Playback::Hover(path, on_start) = update_playback
                    && let Some(request) = state.hover.update(
                        path,
                        event,
                        layout.bounds(),
                        cursor,
                        viewport,
                    )
                {
                    shell.publish(on_start(request));
                }
                if state.update_id(update_playback.id(state)) {
                    shell.request_redraw();
                }

                if let Some(sender) = state.pending.take() {
                    let (input, receiver) = mpsc::channel(1);
                    if sender
                        .send(Connection {
                            sender: input,
                            waker: shell.waker().clone(),
                        })
                        .is_ok()
                    {
                        state.receiver = Some(receiver);
                    }
                }

                if let Some(receiver) = &mut state.receiver {
                    while let Ok(command) = receiver.try_recv() {
                        match command {
                            Command::Allocate(handle, sender) => {
                                renderer.allocate_image(
                                    &handle,
                                    move |result| {
                                        let _ = sender.send(result);
                                    },
                                );
                            }
                            Command::Present(frame) => {
                                state.frame = Some((source, frame));
                                shell.request_redraw();
                            }
                        }
                    }
                }
                if !layout.bounds().intersects(viewport) {
                    state.frame = None;
                }
                inner.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, shell, viewport,
                );
            },
        )
        .operate(
            move |state: &mut State,
                  inner: &mut Element<'a, Message>,
                  tree: &mut widget::Tree,
                  layout: layout::Layout<'_>,
                  renderer: &iced::Renderer,
                  operation: &mut dyn Operation| {
                state.update_id(operate_playback.id(state));
                let mut target = operate_playback
                    .id(state)
                    .map(|id| Target { id, action: None });
                if let Some(target) = &mut target {
                    operation.custom(None, layout.bounds(), target);
                }
                match target.and_then(|target| target.action) {
                    Some(Action::Connect(sender)) => {
                        state.pending = Some(sender);
                    }
                    Some(Action::Failed) => {
                        state.pending = None;
                        state.receiver = None;
                        state.hover.failed();
                    }
                    None => {}
                }
                inner
                    .as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            },
        )
        .draw(
            move |state: &State,
                  inner: &Element<'a, Message>,
                  tree: &widget::Tree,
                  renderer: &mut iced::Renderer,
                  theme: &crate::Theme,
                  style: &renderer::Style,
                  layout: layout::Layout<'_>,
                  cursor: iced::mouse::Cursor,
                  viewport: &Rectangle| {
                if let Some((id, frame)) = &state.frame
                    && *id == source
                {
                    image::draw(
                        renderer,
                        layout,
                        frame.handle(),
                        None,
                        (if round_corners { 4.0 } else { 0.0 }).into(),
                        content_fit,
                        image::FilterMethod::default(),
                        iced::Rotation::default(),
                        1.0,
                        1.0,
                    );
                } else {
                    inner.as_widget().draw(
                        tree, renderer, theme, style, layout, cursor, viewport,
                    );
                }
            },
        )
        .into()
}
