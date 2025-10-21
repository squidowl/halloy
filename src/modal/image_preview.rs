use std::path::PathBuf;
use std::time::{Duration, Instant};

use iced::widget::{button, center, column, container, image, row, space};
use iced::{ContentFit, Length};

use super::Message;
use crate::widget::button::transparent_button;
use crate::widget::{Element, tooltip};
use crate::{Theme, icon, theme};

pub fn view<'a>(
    source: &'a PathBuf,
    url: &'a url::Url,
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
                            tooltip::Position::Bottom,
                            theme,
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
