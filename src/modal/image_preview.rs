use std::time::{Duration, Instant};

use data::Image;
use iced::widget::{button, center, column, container, row, space};
use iced::{ContentFit, Length};

use super::Message;
use crate::widget::button::transparent_button;
use crate::widget::{Element, image, tooltip};
use crate::{Theme, icon, theme};

pub fn view<'a>(
    data: &'a Image,
    timer: &'a Option<Instant>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    container(transparent_button(
        column![
            container(row![
                space::horizontal(),
                container(
                    row![
                        tooltip(
                            button(center(
                                // Show a checkmark when image was saved to indicate success.
                                match timer {
                                    Some(timer)
                                        if timer.elapsed()
                                            < Duration::from_secs(2) =>
                                        Element::from(
                                            icon::checkmark()
                                                .style(theme::text::success)
                                        ),
                                    _ => Element::from(icon::file_transfer()),
                                }
                            ))
                            .padding(5)
                            .width(22)
                            .height(22)
                            .on_press(Message::ImagePreview(
                                super::ImagePreview::SaveImage(
                                    data.path.clone()
                                )
                            ))
                            .style(
                                move |theme, status| {
                                    theme::button::secondary(
                                        theme, status, false,
                                    )
                                }
                            ),
                            Some("Save image"),
                            tooltip::Position::Bottom,
                            theme,
                        ),
                        tooltip(
                            button(center(icon::share()))
                                .padding(5)
                                .width(22)
                                .height(22)
                                .on_press(Message::OpenURL(
                                    data.url.to_string()
                                ))
                                .style(move |theme, status| {
                                    theme::button::secondary(
                                        theme, status, false,
                                    )
                                }),
                            Some("Open in browser"),
                            tooltip::Position::Bottom,
                            theme,
                        ),
                        tooltip(
                            button(center(icon::cancel()))
                                .padding(5)
                                .width(22)
                                .height(22)
                                .on_press(Message::Cancel)
                                .style(move |theme, status| {
                                    theme::button::secondary(
                                        theme, status, false,
                                    )
                                }),
                            Some("Close"),
                            tooltip::Position::Bottom,
                            theme,
                        )
                    ]
                    .spacing(2)
                )
            ])
            .padding(6),
            container(image::from_data(data, false, ContentFit::Contain))
                .padding(50)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(theme::container::none)
        ],
        Message::Cancel,
    ))
    .into()
}
