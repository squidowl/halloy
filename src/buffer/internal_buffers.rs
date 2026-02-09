use data::buffer::Internal;
use iced::Length;
use iced::widget::{button, column, container, row, scrollable, text};

use crate::widget::Element;
use crate::{Theme, font, icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Open(Internal),
}

#[derive(Debug, Default, Clone)]
pub struct InternalBuffers;

impl InternalBuffers {
    pub fn new() -> Self {
        InternalBuffers
    }
}

pub fn view<'a>(
    _state: &InternalBuffers,
    open_internals: &[Internal],
    theme: &'a Theme,
) -> Element<'a, Message> {
    let rows = Internal::ALL
        .iter()
        .filter(|i| !matches!(i, Internal::InternalBuffers))
        .map(|internal| {
            let is_open = open_internals.contains(internal);

            let icon = match internal {
                Internal::FileTransfers => icon::file_transfer(),
                Internal::Logs => icon::logs(),
                Internal::Highlights => icon::highlights(),
                Internal::ChannelDiscovery(_) => icon::channel_discovery(),
                Internal::InternalBuffers => icon::file_transfer(),
            };

            let content = container(
                row![
                    icon.size(theme::TEXT_SIZE).style(theme::text::primary),
                    text(internal.to_string()).font_maybe(
                        theme::font_style::primary(theme).map(font::get)
                    ),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .width(Length::Fill);

            button(content)
                .padding([6, 8])
                .width(Length::Fill)
                .style(move |theme, status| {
                    theme::button::sidebar_buffer(theme, status, false, is_open)
                })
                .on_press(Message::Open(internal.clone()))
                .into()
        });

    container(
        scrollable(column(rows).spacing(1).padding([0, 2]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(1).scroller_width(1),
            ))
            .style(theme::scrollable::hidden),
    )
    .padding(4) // TODO: Make this configurable
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
