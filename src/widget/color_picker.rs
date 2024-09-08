use iced::advanced::renderer::{Quad, Renderer as _};
use iced::widget::{column, container, row};
use iced::{
    advanced, border, event, mouse, touch, Color,
    Length::{self, *},
    Point, Rectangle,
};
use iced::{advanced::Layout, widget::Space};
use iced::{Size, Vector};
use palette::{Hsva, RgbHue};

use super::{decorate, Container, Element, Renderer};
use crate::theme::Theme;

const HANDLE_RADIUS: f32 = 10.0;
const SLIDER_HEIGHT: f32 = 20.0;

pub fn color_picker<'a, Message: 'a>(
    color: Color,
    on_color: impl Fn(Color) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    column![
        row![
            bordered(preview(color)).width(FillPortion(2)),
            bordered(grid(
                Component::Saturation,
                Component::Value,
                color,
                on_color.clone(),
                HANDLE_RADIUS,
            ))
            .width(FillPortion(8))
        ]
        .spacing(4),
        bordered(slider(
            Component::Alpha,
            color,
            on_color.clone(),
            SLIDER_HEIGHT,
            HANDLE_RADIUS
        )),
        bordered(slider(
            Component::Hue,
            color,
            on_color,
            SLIDER_HEIGHT,
            HANDLE_RADIUS
        )),
    ]
    .spacing(4)
    .into()
}

fn bordered<'a, Message: 'a>(element: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(element)
        .padding(1)
        .style(|theme| container::Style {
            text_color: None,
            background: None,
            border: border::rounded(2)
                .width(1)
                .color(theme.colors().general.border),
            shadow: Default::default(),
        })
}

fn preview<'a, Message: 'a>(color: Color) -> Element<'a, Message> {
    decorate(Space::new(Fill, Fill))
        .draw(
            move |_state: &(),
                  _inner: &Element<'a, Message>,
                  _tree: &iced::advanced::widget::Tree,
                  renderer: &mut Renderer,
                  _theme: &Theme,
                  _style: &iced::advanced::renderer::Style,
                  layout: Layout,
                  _cursor: iced::advanced::mouse::Cursor,
                  _viewport: &iced::Rectangle| {
                renderer.fill_quad(
                    Quad {
                        bounds: layout.bounds(),
                        border: Default::default(),
                        shadow: Default::default(),
                    },
                    color,
                )
            },
        )
        .into()
}

#[derive(Debug, Clone, Copy)]
enum Component {
    Hue,
    Saturation,
    Value,
    Alpha,
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy)]
struct Value {
    component: Component,
    direction: Direction,
}

impl Value {
    fn new(component: Component, direction: Direction) -> Self {
        Self {
            component,
            direction,
        }
    }

    fn color(self, mut hsva: Hsva, offset: f32) -> Hsva {
        let mut offset = offset.clamp(0.0, 1.0);

        if matches!(self.direction, Direction::Vertical) {
            offset = 1.0 - offset;
        }

        match self.component {
            Component::Hue => {
                // Prevent handle from overflowing back to left
                hsva.hue = if offset == 1.0 {
                    RgbHue::new(359.9999)
                } else {
                    RgbHue::new(offset * 360.0)
                };
            }
            Component::Saturation => {
                hsva.saturation = offset;
            }
            Component::Value => {
                hsva.value = offset;
            }
            Component::Alpha => {
                hsva.alpha = offset;
            }
        }

        hsva
    }

    fn offset(self, hsva: Hsva) -> f32 {
        let offset = match self.component {
            Component::Hue => hsva.hue.into_positive_degrees() / 360.0,
            Component::Saturation => hsva.saturation,
            Component::Value => hsva.value,
            Component::Alpha => hsva.alpha,
        };

        if matches!(self.direction, Direction::Vertical) {
            1.0 - offset
        } else {
            offset
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Picker {
    Slider(Value),
    Grid { x: Value, y: Value },
}

impl Picker {
    fn handle_from_color(&self, color: Hsva, bounds: Rectangle, radius: f32) -> Rectangle {
        let width = bounds.width - radius;
        let height = bounds.height - radius;

        match self {
            Picker::Slider(x) => Rectangle {
                x: bounds.x + (x.offset(color) * width) - radius / 2.0,
                y: bounds.center_y() - radius,
                width: radius * 2.0,
                height: radius * 2.0,
            },
            Picker::Grid { x, y } => Rectangle {
                x: bounds.x + (x.offset(color) * width) - radius / 2.0,
                y: bounds.y + (y.offset(color) * height) - radius / 2.0,
                width: radius * 2.0,
                height: radius * 2.0,
            },
        }
    }

    fn handle_from_cursor(&self, cursor: Point, bounds: Rectangle, radius: f32) -> Rectangle {
        match self {
            Picker::Slider(_) => Rectangle {
                x: cursor.x.clamp(bounds.x, bounds.x + bounds.width) - radius,
                y: bounds.center_y() - radius,
                width: radius * 2.0,
                height: radius * 2.0,
            },
            Picker::Grid { .. } => Rectangle {
                x: cursor.x.clamp(bounds.x, bounds.x + bounds.width) - radius,
                y: cursor.y.clamp(bounds.y, bounds.y + bounds.height) - radius,
                width: radius * 2.0,
                height: radius * 2.0,
            },
        }
    }

    fn color_at_handle(&self, color: Hsva, handle: Rectangle, bounds: Rectangle) -> Hsva {
        match self {
            Picker::Slider(x) => x.color(color, (handle.center_x() - bounds.x) / bounds.width),
            Picker::Grid { x, y } => x.color(
                y.color(color, (handle.center_y() - bounds.y) / bounds.height),
                (handle.center_x() - bounds.x) / bounds.width,
            ),
        }
    }

    fn with_cells(&self, color: Hsva, bounds: Rectangle, mut f: impl FnMut(usize, usize, Hsva)) {
        match self {
            Picker::Slider(x_value) => {
                let color = match x_value.component {
                    // Full S/V and cycle every hue
                    Component::Hue => Hsva::new_srgb(0.0, 1.0, 1.0, 1.0),
                    // Otherwise slide should change based on color
                    _ => color,
                };

                for x in 0..bounds.width.round() as usize {
                    let color = x_value.color(color, x as f32 / bounds.width);

                    (f)(x, 0, color);
                }
            }
            Picker::Grid {
                x: x_value,
                y: y_value,
            } => {
                let color = Hsva::new_srgb(color.hue, 1.0, 1.0, 1.0);

                for x in 0..bounds.width.round() as usize {
                    for y in 0..bounds.height.round() as usize {
                        let color = x_value.color(
                            y_value.color(color, y as f32 / bounds.height),
                            x as f32 / bounds.width,
                        );

                        (f)(x, y, color);
                    }
                }
            }
        }
    }
}

fn grid<'a, Message: 'a>(
    x: Component,
    y: Component,
    color: Color,
    on_color: impl Fn(Color) -> Message + Clone + 'a,
    handle_radius: f32,
) -> Element<'a, Message> {
    let x = Value::new(x, Direction::Horizontal);
    let y = Value::new(y, Direction::Vertical);

    picker(
        Picker::Grid { x, y },
        color,
        on_color,
        Fill,
        Fill,
        handle_radius,
    )
}

fn slider<'a, Message: 'a>(
    component: Component,
    color: Color,
    on_color: impl Fn(Color) -> Message + Clone + 'a,
    height: f32,
    handle_radius: f32,
) -> Element<'a, Message> {
    picker(
        Picker::Slider(Value::new(component, Direction::Horizontal)),
        color,
        on_color,
        Fill,
        height,
        handle_radius,
    )
}

fn picker<'a, Message: 'a>(
    picker: Picker,
    color: Color,
    on_color: impl Fn(Color) -> Message + Clone + 'a,
    width: impl Into<Length>,
    height: impl Into<Length>,
    handle_radius: f32,
) -> Element<'a, Message> {
    let color = data::theme::to_hsva(color);

    decorate(Space::new(width, height))
        .state::<Option<Rectangle>>()
        .on_event(
            move |state: &mut Option<Rectangle>,
                  _inner: &mut Element<'a, Message>,
                  _tree: &mut advanced::widget::Tree,
                  event: iced::Event,
                  layout: advanced::Layout<'_>,
                  cursor: advanced::mouse::Cursor,
                  _renderer: &Renderer,
                  _clipboard: &mut dyn advanced::Clipboard,
                  shell: &mut advanced::Shell<'_, Message>,
                  _viewport: &iced::Rectangle| {
                let bounds = layout.bounds();
                let handle = picker.handle_from_color(color, bounds, handle_radius);

                match event {
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | iced::Event::Touch(touch::Event::FingerPressed { .. })
                        if state.is_none() =>
                    {
                        if cursor.is_over(handle) {
                            *state = Some(handle);
                        } else if let Some(position) = cursor.position_over(bounds) {
                            let new_handle =
                                picker.handle_from_cursor(position, bounds, handle_radius);
                            let color = picker.color_at_handle(color, new_handle, bounds);

                            shell.publish((on_color)(data::theme::from_hsva(color)));

                            *state = Some(new_handle);
                        }
                    }
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                    | iced::Event::Touch(touch::Event::FingerLost { .. })
                        if state.is_some() =>
                    {
                        if let Some(last_handle) = state.take() {
                            let color = picker.color_at_handle(color, last_handle, bounds);

                            shell.publish((on_color)(data::theme::from_hsva(color)));
                        }
                    }
                    iced::Event::Mouse(mouse::Event::CursorMoved { position })
                    | iced::Event::Touch(touch::Event::FingerMoved { position, .. })
                        if state.is_some() =>
                    {
                        if let Some(last_handle) = state.as_mut() {
                            match picker {
                                Picker::Slider(_) => {
                                    last_handle.x =
                                        position.x.clamp(bounds.x, bounds.x + bounds.width)
                                            - handle_radius;
                                }
                                Picker::Grid { .. } => {
                                    last_handle.x =
                                        position.x.clamp(bounds.x, bounds.x + bounds.width)
                                            - handle_radius;
                                    last_handle.y =
                                        position.y.clamp(bounds.y, bounds.y + bounds.height)
                                            - handle_radius;
                                }
                            }

                            let color = picker.color_at_handle(color, *last_handle, bounds);

                            shell.publish((on_color)(data::theme::from_hsva(color)));
                        }
                    }
                    _ => {}
                }

                event::Status::Ignored
            },
        )
        .draw(
            move |_state: &Option<Rectangle>,
                  _inner: &Element<'a, Message>,
                  _tree: &iced::advanced::widget::Tree,
                  renderer: &mut Renderer,
                  _theme: &Theme,
                  _style: &iced::advanced::renderer::Style,
                  layout: Layout,
                  _cursor: iced::advanced::mouse::Cursor,
                  viewport: &iced::Rectangle| {
                let bounds = layout.bounds();
                let handle = picker.handle_from_color(color, bounds, handle_radius);

                // Draw checkerboard for alpha slider
                if matches!(
                    picker,
                    Picker::Slider(Value {
                        component: Component::Alpha,
                        ..
                    })
                ) {
                    const WIDTH: f32 = 5.0;
                    const HEIGHT: f32 = 5.0;
                    const COLOR_EVEN: Color = Color::from_rgb(0.9803, 0.9882, 0.9922);
                    const COLOR_ODD: Color = Color::from_rgb(0.7529, 0.7725, 0.8);

                    let num_rows = (bounds.height / HEIGHT) as usize + 1;
                    let num_columns = (bounds.width / WIDTH) as usize + 1;

                    for row in 0..num_rows {
                        for col in 0..num_columns {
                            let x = col as f32 * WIDTH;
                            let y = row as f32 * HEIGHT;
                            let width = (bounds.width - x).min(WIDTH);
                            let height = (bounds.height - y).min(HEIGHT);
                            let top_left = bounds.position() + Vector::new(x, y);
                            let color = if (row + col) % 2 == 0 {
                                COLOR_EVEN
                            } else {
                                COLOR_ODD
                            };

                            renderer.fill_quad(
                                Quad {
                                    bounds: Rectangle::new(top_left, Size::new(width, height)),
                                    border: Default::default(),
                                    shadow: Default::default(),
                                },
                                color,
                            );
                        }
                    }
                }

                let cell_height = match picker {
                    Picker::Slider(_) => bounds.height,
                    Picker::Grid { .. } => 1.0,
                };

                picker.with_cells(color, bounds, |x, y, color| {
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle {
                                x: bounds.x + x as f32,
                                y: bounds.y + y as f32,
                                width: 1.0,
                                height: cell_height,
                            },
                            border: Default::default(),
                            shadow: Default::default(),
                        },
                        data::theme::from_hsva(color),
                    );
                });

                renderer.with_layer(*viewport, |renderer| {
                    renderer.fill_quad(
                        Quad {
                            bounds: handle,
                            border: border::rounded(handle.width / 2.0)
                                .color(Color::BLACK)
                                .width(1.0),
                            shadow: Default::default(),
                        },
                        Color::WHITE,
                    );
                });
            },
        )
        .into()
}
