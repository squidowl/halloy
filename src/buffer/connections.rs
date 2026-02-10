use data::{buffer, history};
use iced::Length;
use iced::widget::{button, column, container, row, scrollable, space, text};

use crate::widget::Element;
use crate::{Theme, font, icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Open(buffer::Upstream),
}

#[derive(Debug, Default, Clone)]
pub struct Connections;

impl Connections {
    pub fn new() -> Self {
        Connections
    }
}

pub fn view<'a>(
    _state: &Connections,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let mut rows: Vec<Element<'a, Message>> = vec![];

    for (server, state) in clients.iter() {
        let connected = matches!(state, data::client::State::Ready(_));

        let server_icon = if server.is_bouncer_network() {
            icon::link()
        } else {
            icon::connected()
        }
        .style(if connected {
            theme::text::primary
        } else {
            theme::text::error
        });

        let server_label = text(server.to_string())
            .style(if connected {
                theme::text::primary
            } else {
                theme::text::error
            })
            .font_maybe(theme::font_style::primary(theme).map(font::get))
            .shaping(text::Shaping::Advanced);

        let server_button = button(
            row![
                server_icon
                    .size(theme::TEXT_SIZE)
                    .width(Length::Fixed(12.0)),
                server_label,
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding([6, 8])
        .width(Length::Fill)
        .style(|theme, status| {
            theme::button::sidebar_buffer(theme, status, false, false)
        })
        .on_press(Message::Open(buffer::Upstream::Server(server.clone())));

        rows.push(server_button.into());

        if let data::client::State::Ready(connection) = state {
            for channel in connection.channels() {
                let has_unread = history.has_unread(&history::Kind::Channel(
                    server.clone(),
                    channel.clone(),
                ));

                let label = text(channel.to_string())
                    .style(if has_unread {
                        theme::text::unread_indicator
                    } else {
                        theme::text::primary
                    })
                    .font_maybe(
                        theme::font_style::primary(theme).map(font::get),
                    )
                    .shaping(text::Shaping::Advanced);

                let channel_button =
                    button(container(label).padding(iced::padding::left(20)))
                        .padding([4, 8])
                        .width(Length::Fill)
                        .style(|theme, status| {
                            theme::button::sidebar_buffer(
                                theme, status, false, false,
                            )
                        })
                        .on_press(Message::Open(buffer::Upstream::Channel(
                            server.clone(),
                            channel.clone(),
                        )));

                rows.push(channel_button.into());
            }

            let queries = history.get_unique_queries(server);
            for query in queries {
                let query =
                    clients.resolve_query(server, query).unwrap_or(query);

                let label = text(query.to_string())
                    .style(theme::text::primary)
                    .font_maybe(
                        theme::font_style::primary(theme).map(font::get),
                    )
                    .shaping(text::Shaping::Advanced);

                let query_button =
                    button(container(label).padding(iced::padding::left(20)))
                        .padding([4, 8])
                        .width(Length::Fill)
                        .style(|theme, status| {
                            theme::button::sidebar_buffer(
                                theme, status, false, false,
                            )
                        })
                        .on_press(Message::Open(buffer::Upstream::Query(
                            server.clone(),
                            query.clone(),
                        )));

                rows.push(query_button.into());
            }

            rows.push(space::vertical().height(4).into());
        }
    }

    container(
        scrollable(column(rows).spacing(1).padding([0, 2]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(1).scroller_width(1),
            ))
            .style(theme::scrollable::hidden),
    )
    .padding(4)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
