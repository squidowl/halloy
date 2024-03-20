use iced::widget::{column, container, scrollable, Scrollable};
use iced::{Command, Length};

use crate::theme;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    ApproveIncomingTransfer,
    RejectIncomingTransfer,
    ClearFinishedTransfer,
}

#[derive(Debug, Clone)]
pub enum Event {}

#[derive(Debug, Clone)]
struct FileTransferEvent {
    direction: Direction,
    filename: String,
    size: f32,
}

#[derive(Debug, Clone)]
enum Direction {
    Upload(Status),
    Download(Status),
}

impl Direction {
    pub fn status(&self) -> &Status {
        match self {
            Direction::Upload(status) => status,
            Direction::Download(status) => status,
        }
    }
}

#[derive(Debug, Clone)]
enum Status {
    Waiting,
    Failed,
    Process(f32),
    Success,
}

pub fn view(_state: &FileTransfers) -> Element<'_, Message> {
    let transfers = vec![
        FileTransferEvent {
            direction: Direction::Download(Status::Waiting),
            filename: "Ubunutu.zip".to_string(),
            size: 123221.0,
        },
        FileTransferEvent {
            direction: Direction::Upload(Status::Failed),
            filename: "Arch.zip".to_string(),
            size: 423221.0,
        },
        FileTransferEvent {
            direction: Direction::Upload(Status::Process(12.5)),
            filename: "Solus.zip".to_string(),
            size: 2123.0,
        },
        FileTransferEvent {
            direction: Direction::Upload(Status::Process(99.0)),
            filename: "Solus2.zip".to_string(),
            size: 21232.0,
        },
        FileTransferEvent {
            direction: Direction::Upload(Status::Success),
            filename: "Solus2.zip".to_string(),
            size: 21232.0,
        },
        FileTransferEvent {
            direction: Direction::Download(Status::Success),
            filename: "Solus52.zip".to_string(),
            size: 1232.0,
        },
    ];

    let column = column(transfers.iter().enumerate().map(|(idx, transfer)| {
        container(transfer_row::view(transfer, idx))
            .width(Length::Fill)
            .height(35)
            .into()
    }))
    .spacing(1)
    .padding([0, 2]);

    container(Scrollable::with_direction_and_style(
        column,
        scrollable::Direction::Vertical(scrollable::Properties::new().width(1).scroller_width(1)),
        theme::scrollable::hidden,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[derive(Debug, Default, Clone)]
pub struct FileTransfers;

impl FileTransfers {
    pub fn new() -> Self {
        FileTransfers
    }

    pub fn update(&mut self, _message: Message) -> (Command<Message>, Option<Event>) {
        (Command::none(), None)
    }
}

mod transfer_row {
    use super::{FileTransferEvent, Message};
    use iced::widget::{button, container, horizontal_space, row, text};
    use iced::{alignment, Length};

    use crate::widget::Element;
    use crate::{icon, theme};

    pub fn view<'a>(transfer: &FileTransferEvent, idx: usize) -> Element<'a, Message> {
        let status = container(match transfer.direction.status() {
            super::Status::Waiting => match transfer.direction {
                super::Direction::Upload(_) => {
                    container(text("Waiting for them to accept".to_string())
                        .style(theme::text::transparent))
                }
                super::Direction::Download(_) => {
                    container(text("Waiting to begin".to_string()).style(theme::text::transparent))
                }
            },
            super::Status::Failed => {
                container(text("Failed".to_string()).style(theme::text::transparent))
            }
            super::Status::Process(progress) => container(row![
                container(text(format!("{progress}%")).style(theme::text::transparent))
                    .center_x()
                    .width(30),
                horizontal_space(),
                text("-"),
                horizontal_space(),
                text("24 MiB/s").style(theme::text::transparent)
            ]
            .width(105)
            .align_items(iced::Alignment::Center)),
            super::Status::Success => {
                container(text("Completed".to_string()).style(theme::text::transparent))
            },
        })
        .width(Length::Shrink);

        let mut buttons = row![]
            .height(Length::Fill)
            .align_items(iced::Alignment::Center)
            .spacing(2);

        match transfer.direction.status() {
            super::Status::Waiting => {
                let approve_button = button(
                    container(icon::checkmark())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y(),
                )
                .on_press(Message::ApproveIncomingTransfer)
                .padding(5)
                .width(25)
                .height(25)
                .style(theme::button::pane);

                let reject_button = button(
                    container(icon::close())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y(),
                )
                .on_press(Message::RejectIncomingTransfer)
                .padding(5)
                .width(25)
                .height(25)
                .style(theme::button::pane);

                buttons = buttons.push(approve_button);
                buttons = buttons.push(reject_button);
            }
            super::Status::Failed => {
                let clear_button = button(
                    container(icon::trashcan())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y(),
                )
                .on_press(Message::ClearFinishedTransfer)
                .padding(5)
                .width(25)
                .height(25)
                .style(theme::button::pane);

                buttons = buttons.push(clear_button)
            }
            super::Status::Process(_) => {
                let reject_button = button(
                    container(icon::close())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y(),
                )
                .on_press(Message::RejectIncomingTransfer)
                .padding(5)
                .width(25)
                .height(25)
                .style(theme::button::pane);

                buttons = buttons.push(reject_button);
            },
            super::Status::Success => {

                match transfer.direction {
                    super::Direction::Download(_) => {
                        let folder_button = button(
                            container(icon::folder())
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .center_x()
                                .center_y(),
                        )
                        .on_press(Message::ClearFinishedTransfer)
                        .padding(5)
                        .width(25)
                        .height(25)
                        .style(theme::button::pane);

                        buttons = buttons.push(folder_button);
                    },
                    _ => {}
                }


                let clear_button = button(
                    container(icon::trashcan())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y(),
                )
                .on_press(Message::ClearFinishedTransfer)
                .padding(5)
                .width(25)
                .height(25)
                .style(theme::button::pane);

                buttons = buttons.push(clear_button);
            },
        }

        let icon = container(match transfer.direction {
            super::Direction::Upload(_) => icon::arrow_up(),
            super::Direction::Download(_) => icon::arrow_down(),
        })
        .width(22)
        .height(22)
        .center_x()
        .center_y();

        let left_side = container(
            row![
                icon,
                text(transfer.filename.clone()),
                text(format!("({:?} mb)", transfer.size)).style(theme::text::transparent)
            ]
            .align_items(iced::Alignment::Center)
            .width(Length::Fill)
            .spacing(4),
        );

        let right_side = container(
            row![
                status,
                buttons
            ]
            .align_items(iced::Alignment::Center)
            .spacing(4)
        );

        let row = row![
            left_side,
            right_side
        ]
        .spacing(0)
        .align_items(iced::Alignment::Center);

        let content = container(row)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([0, 4])
            .align_y(alignment::Vertical::Center)
            .style(move |theme, status| theme::container::table_row(theme, status, idx));

        return content.into();
    }
}
