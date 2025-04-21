use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use iced::{
    ContentFit, Length,
    widget::{button, center, column, container, horizontal_space, image, row},
};

use super::Message;
use crate::{
    icon, theme,
    widget::{Element, button::transparent_button, tooltip},
};

pub fn view<'a>(
    source: &'a PathBuf,
    url: &'a url::Url,
    timer: &'a Option<Instant>,
) -> Element<'a, Message> {
    container(transparent_button(
        column![
            container(row![
                horizontal_space(),
                container(
                    row![
                        tooltip(
                            button(center(
                                // Show a checkmark when image was saved to indicate success.
                                match timer {
                                    Some(timer)
                                        if timer.elapsed()
                                            < Duration::from_secs(2) =>
                                        icon::checkmark()
                                            .style(theme::text::success),
                                    _ => icon::file_transfer(),
                                }
                            ))
                            .padding(5)
                            .width(22)
                            .height(22)
                            .on_press(Message::ImagePreview(
                                super::ImagePreview::SaveImage(
                                    source.to_path_buf()
                                )
                            ))
                            .style(
                                move |theme, status| {
                                    theme::button::secondary(
                                        theme, status, false,
                                    )
                                }
                            ),
                            Some("Save Image"),
                            tooltip::Position::Bottom
                        ),
                        tooltip(
                            button(center(icon::share()))
                                .padding(5)
                                .width(22)
                                .height(22)
                                .on_press(Message::OpenURL(url.to_string()))
                                .style(move |theme, status| {
                                    theme::button::secondary(
                                        theme, status, false,
                                    )
                                }),
                            Some("Open in Browser"),
                            tooltip::Position::Bottom
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
                            tooltip::Position::Bottom
                        )
                    ]
                    .spacing(2)
                )
            ])
            .padding(6),
            container(image(source).content_fit(ContentFit::Contain))
                .padding(50)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(theme::container::none)
        ],
        Message::Cancel,
    ))
    .into()
}
