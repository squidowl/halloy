use std::path::PathBuf;

use iced::{
    ContentFit, Length,
    widget::{button, center, column, container, horizontal_space, image, row},
};

use super::Message;
use crate::{
    icon, theme,
    widget::{Element, tooltip},
};

/*
            row![
                horizontal_space(),
                button(center(icon::mark_as_read()))
                    .padding(5)
                    .width(22)
                    .height(22)
                    .on_press(Message::Cancel)
                    .style(move |theme, status| {
                        theme::button::secondary(theme, status, false)
                    })
            ],

*/

pub fn view(path: &PathBuf) -> Element<Message> {
    container(column![
        container(row![
            horizontal_space(),
            container(
                row![
                    tooltip(
                        button(center(icon::mark_as_read()))
                            .padding(5)
                            .width(22)
                            .height(22)
                            .on_press(Message::Cancel)
                            .style(move |theme, status| {
                                theme::button::secondary(theme, status, false)
                            }),
                        Some("Save Image"),
                        tooltip::Position::Bottom
                    ),
                    tooltip(
                        button(center(icon::share()))
                            .padding(5)
                            .width(22)
                            .height(22)
                            .on_press(Message::Cancel)
                            .style(move |theme, status| {
                                theme::button::secondary(theme, status, false)
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
                                theme::button::secondary(theme, status, false)
                            }),
                        Some("Close"),
                        tooltip::Position::Bottom
                    )
                ]
                .spacing(2)
            )
        ])
        .padding(6),
        container(image(path).content_fit(ContentFit::Contain))
            .padding(50)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(theme::container::none),
    ])
    .into()
}
