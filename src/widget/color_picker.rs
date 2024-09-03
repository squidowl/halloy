use iced::advanced::renderer::{Quad, Renderer as _};
use iced::widget::{column, container, row};
use iced::{advanced, border, event, mouse, touch, Color, Length::*, Rectangle};
use iced::{advanced::Layout, widget::Space};
use palette::{Hsva, RgbHue};

use super::{decorate, Element, Renderer};
use crate::theme::Theme;

const HANDLE_RADIUS: f32 = 7.0;
const SLIDER_HEIGHT: f32 = 10.0;

pub fn color_picker<'a, Message: 'a>(
    color: Color,
    on_color: impl Fn(Color) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    column![
        row![
            container(preview(color)).width(FillPortion(2)),
            container(axis(
                Component::Saturation,
                Component::Value,
                color,
                on_color.clone(),
                HANDLE_RADIUS,
            ),)
            .width(FillPortion(8))
        ]
        .spacing(4),
        slider(
            Component::Alpha,
            color,
            on_color.clone(),
            SLIDER_HEIGHT,
            HANDLE_RADIUS
        ),
        slider(
            Component::Hue,
            color,
            on_color,
            SLIDER_HEIGHT,
            HANDLE_RADIUS
        ),
    ]
    .spacing(4)
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

fn axis<'a, Message: 'a>(
    x_component: Component,
    y_component: Component,
    color: Color,
    on_color: impl Fn(Color) -> Message + Clone + 'a,
    handle_radius: f32,
) -> Element<'a, Message> {
    let color = data::theme::to_hsva(color);

    let x_value = Value::new(x_component, Direction::Horizontal);
    let y_value = Value::new(y_component, Direction::Vertical);

    fn axis_handle(
        x_value: Value,
        y_value: Value,
        color: Hsva,
        bounds: Rectangle,
        radius: f32,
    ) -> Rectangle {
        let width = bounds.width - radius;
        let height = bounds.height - radius;

        Rectangle {
            x: bounds.x + (x_value.offset(color) * width) - radius / 2.0,
            y: bounds.y + (y_value.offset(color) * height) - radius / 2.0,
            width: radius * 2.0,
            height: radius * 2.0,
        }
    }

    decorate(Space::new(Fill, Fill))
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
                let handle = axis_handle(x_value, y_value, color, bounds, handle_radius);

                match event {
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | iced::Event::Touch(touch::Event::FingerPressed { .. })
                        if state.is_none() =>
                    {
                        if cursor.is_over(handle) {
                            *state = Some(handle);
                        } else if let Some(position) = cursor.position_over(bounds) {
                            let new_handle = Rectangle {
                                x: position.x.clamp(bounds.x, bounds.x + bounds.width)
                                    - handle_radius,
                                y: position.y.clamp(bounds.y, bounds.y + bounds.height)
                                    - handle_radius,
                                ..handle
                            };

                            let color = y_value.color(
                                x_value.color(
                                    color,
                                    (new_handle.center_x() - bounds.x) / bounds.width,
                                ),
                                (new_handle.center_y() - bounds.y) / bounds.height,
                            );

                            shell.publish((on_color)(data::theme::from_hsva(color)));

                            *state = Some(new_handle);
                        }
                    }
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                    | iced::Event::Touch(touch::Event::FingerLost { .. })
                        if state.is_some() =>
                    {
                        if let Some(last_handle) = state.take() {
                            let color = y_value.color(
                                x_value.color(
                                    color,
                                    (last_handle.center_x() - bounds.x) / bounds.width,
                                ),
                                (last_handle.center_y() - bounds.y) / bounds.height,
                            );

                            shell.publish((on_color)(data::theme::from_hsva(color)));
                        }
                    }
                    iced::Event::Mouse(mouse::Event::CursorMoved { position })
                    | iced::Event::Touch(touch::Event::FingerMoved { position, .. })
                        if state.is_some() =>
                    {
                        if let Some(last_handle) = state.as_mut() {
                            last_handle.x =
                                position.x.clamp(bounds.x, bounds.x + bounds.width) - handle_radius;
                            last_handle.y = position.y.clamp(bounds.y, bounds.y + bounds.height)
                                - handle_radius;

                            let color = y_value.color(
                                x_value.color(
                                    color,
                                    (last_handle.center_x() - bounds.x) / bounds.width,
                                ),
                                (last_handle.center_y() - bounds.y) / bounds.height,
                            );

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
                let handle = axis_handle(x_value, y_value, color, bounds, handle_radius);

                for x in 0..bounds.width as usize {
                    for y in 0..bounds.height as usize {
                        let color = y_value.color(
                            x_value.color(color, x as f32 / bounds.width),
                            y as f32 / bounds.height,
                        );

                        renderer.fill_quad(
                            Quad {
                                bounds: Rectangle {
                                    x: bounds.x + x as f32,
                                    y: bounds.y + y as f32,
                                    width: 1.0,
                                    height: 1.0,
                                },
                                border: Default::default(),
                                shadow: Default::default(),
                            },
                            data::theme::from_hsva(color),
                        )
                    }
                }

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

fn slider<'a, Message: 'a>(
    component: Component,
    color: Color,
    on_color: impl Fn(Color) -> Message + Clone + 'a,
    height: f32,
    handle_radius: f32,
) -> Element<'a, Message> {
    let color = data::theme::to_hsva(color);
    let value = Value::new(component, Direction::Horizontal);

    fn slider_handle(value: Value, color: Hsva, bounds: Rectangle, radius: f32) -> Rectangle {
        let width = bounds.width - radius;

        Rectangle {
            x: bounds.x + (value.offset(color) * width) - radius / 2.0,
            y: bounds.center_y() - radius,
            width: radius * 2.0,
            height: radius * 2.0,
        }
    }

    decorate(Space::new(Fill, height))
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
                let handle = slider_handle(value, color, bounds, handle_radius);

                match event {
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | iced::Event::Touch(touch::Event::FingerPressed { .. })
                        if state.is_none() =>
                    {
                        if cursor.is_over(handle) {
                            *state = Some(handle);
                        } else if let Some(position) = cursor.position_over(bounds) {
                            let new_handle = Rectangle {
                                x: position.x.clamp(bounds.x, bounds.x + bounds.width)
                                    - handle_radius,
                                ..handle
                            };

                            let color = value
                                .color(color, (new_handle.center_x() - bounds.x) / bounds.width);

                            shell.publish((on_color)(data::theme::from_hsva(color)));

                            *state = Some(new_handle);
                        }
                    }
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                    | iced::Event::Touch(touch::Event::FingerLost { .. })
                        if state.is_some() =>
                    {
                        if let Some(last_handle) = state.take() {
                            let color = value
                                .color(color, (last_handle.center_x() - bounds.x) / bounds.width);

                            shell.publish((on_color)(data::theme::from_hsva(color)));
                        }
                    }
                    iced::Event::Mouse(mouse::Event::CursorMoved { position })
                    | iced::Event::Touch(touch::Event::FingerMoved { position, .. })
                        if state.is_some() =>
                    {
                        if let Some(last_handle) = state.as_mut() {
                            last_handle.x =
                                position.x.clamp(bounds.x, bounds.x + bounds.width) - handle_radius;

                            let color = value
                                .color(color, (last_handle.center_x() - bounds.x) / bounds.width);

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
                let handle = slider_handle(value, color, bounds, handle_radius);

                for x in 0..bounds.width as usize {
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle {
                                x: bounds.x + x as f32,
                                y: bounds.y,
                                width: 1.0,
                                height: bounds.height,
                            },
                            border: Default::default(),
                            shadow: Default::default(),
                        },
                        data::theme::from_hsva(value.color(color, x as f32 / bounds.width)),
                    )
                }

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
