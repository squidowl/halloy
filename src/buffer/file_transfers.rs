use std::path::PathBuf;

use data::file_transfer;
use iced::widget::{button, column, container, scrollable, Scrollable};
use iced::{Command, Length};

use crate::theme;
use crate::widget::{Element, Text};

#[derive(Debug, Clone)]
pub enum Message {
    Approve(file_transfer::Id),
    SavePathSelected(file_transfer::Id, Option<PathBuf>),
    Reject(file_transfer::Id),
    Clear,
    OpenDirectory,
}

pub fn view<'a>(
    _state: &FileTransfers,
    file_transfers: &'a file_transfer::Manager,
) -> Element<'a, Message> {
    let transfers = file_transfers.list();

    let column = column(
        transfers
            .enumerate()
            .map(|(idx, transfer)| container(transfer_row::view(transfer, idx)).into()),
    )
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

    pub fn update(
        &mut self,
        message: Message,
        file_transfers: &mut file_transfer::Manager,
    ) -> Command<Message> {
        match message {
            Message::Approve(id) => {
                if let Some(transfer) = file_transfers.get(&id).cloned() {
                    return Command::perform(
                        async move {
                            // TODO: Config default save directory
                            rfd::AsyncFileDialog::new()
                                .set_directory("/tmp/")
                                .set_file_name(transfer.filename)
                                .save_file()
                                .await
                                .map(|handle| handle.path().to_path_buf())
                        },
                        move |path| Message::SavePathSelected(id, path),
                    );
                }
            }
            Message::SavePathSelected(id, path) => {
                if let Some(path) = path {
                    file_transfers.approve(&id, path);
                }
            }
            Message::Reject(_) => {}
            Message::Clear => {}
            Message::OpenDirectory => {}
        }

        Command::none()
    }
}

mod transfer_row {
    use super::Message;
    use bytesize::ByteSize;
    use data::file_transfer::{self, FileTransfer};
    use iced::widget::{column, container, progress_bar, row, text};
    use iced::{alignment, Length};

    use crate::buffer::file_transfers::transfer_row_button;
    use crate::theme::TEXT_SIZE;
    use crate::widget::Element;
    use crate::{icon, theme};

    pub fn view<'a>(transfer: &FileTransfer, idx: usize) -> Element<'a, Message> {
        let status = container(match &transfer.status {
            file_transfer::Status::Pending => text("Pending").style(theme::text::transparent),
            file_transfer::Status::Queued => text("Queued").style(theme::text::transparent),
            file_transfer::Status::Active {
                transferred,
                elapsed,
            } => text("TODO").style(theme::text::transparent),
            file_transfer::Status::Completed { elapsed, sha256 } => {
                text("TODO").style(theme::text::transparent)
            }
            // file_transfer::Status::Failed { error } => text(error).style(theme::text::error),
            file_transfer::Status::Failed { error } => {
                text("Queued").style(theme::text::transparent)
            }
        });

        let progress = match &transfer.status {
            file_transfer::Status::Active { .. } => 22.0, // TODO
            file_transfer::Status::Completed { .. } => 100.0,
            file_transfer::Status::Pending
            | file_transfer::Status::Queued
            | file_transfer::Status::Failed { .. } => 0.0,
        };

        let progress_bar = container(progress_bar(0.0..=100.0, progress))
            .padding([4, 0])
            .height(11);

        let filename = {
            let filename = text(transfer.filename.clone());
            let secure = transfer.secure.then(|| {
                container(
                    icon::secure()
                        .size(TEXT_SIZE)
                        .style(theme::text::transparent),
                )
                .padding([1, 0, 0, 0])
            });
            let direction = text(format!(
                "{} {}",
                match &transfer.direction {
                    file_transfer::Direction::Sent => "to",
                    file_transfer::Direction::Received => "from",
                },
                transfer.remote_user
            ))
            .style(theme::text::transparent);
            let file_size =
                text(format!("({})", ByteSize::b(transfer.size))).style(theme::text::transparent);

            container(
                row![]
                    .push(filename)
                    .push(direction)
                    .push(file_size)
                    .push_maybe(secure)
                    .spacing(4)
                    .align_items(iced::Alignment::Center),
            )
        };

        let mut buttons = row![].align_items(iced::Alignment::Center).spacing(2);

        let content = column![filename, status, progress_bar].spacing(0);

        match &transfer.status {
            file_transfer::Status::Pending => {
                buttons = buttons.push(transfer_row_button(
                    icon::download(),
                    Message::Approve(transfer.id),
                ));
            }
            file_transfer::Status::Queued => {}
            file_transfer::Status::Active { .. } | file_transfer::Status::Completed { .. } => {
                buttons = buttons.push(transfer_row_button(icon::folder(), Message::OpenDirectory));
                buttons = buttons.push(transfer_row_button(icon::close(), Message::Clear));
            }
            file_transfer::Status::Failed { .. } => {
                buttons = buttons.push(transfer_row_button(icon::close(), Message::Clear));
            }
        }

        let row = row![content, buttons]
            .spacing(6)
            .align_items(iced::Alignment::Center);

        container(row)
            .padding(6)
            .width(Length::Fill)
            .align_y(alignment::Vertical::Center)
            .style(move |theme, status| theme::container::table_row(theme, status, idx))
            .into()
    }
}

fn transfer_row_button<'a>(icon: Text<'a>, message: Message) -> Element<'a, Message> {
    button(
        container(icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y(),
    )
    .on_press(message)
    .padding(5)
    .width(25)
    .height(25)
    .style(theme::button::side_menu)
    .into()
}
