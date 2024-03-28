use std::path::PathBuf;

use data::{file_transfer, Config};
use iced::widget::{button, column, container, scrollable, text, Scrollable};
use iced::{Command, Length};

use crate::widget::{Element, Text};
use crate::{icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Approve(file_transfer::Id),
    SavePathSelected(file_transfer::Id, Option<PathBuf>),
    Clear(file_transfer::Id),
    OpenDirectory,
}

pub fn view<'a>(
    _state: &FileTransfers,
    file_transfers: &'a file_transfer::Manager,
) -> Element<'a, Message> {
    if file_transfers.is_empty() {
        return container(container(
            column![
                icon::file_transfer()
                    .size(theme::TEXT_SIZE + 3.0)
                    .style(theme::text::transparent),
                text("No transfers found").style(theme::text::transparent)
            ]
            .spacing(8)
            .align_items(iced::Alignment::Center),
        ))
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let column = column(
        file_transfers
            .list()
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
        config: &Config,
    ) -> Command<Message> {
        match message {
            Message::Approve(id) => {
                if let Some(transfer) = file_transfers.get(&id).cloned() {
                    let save_directory = config.file_transfer.save_directory.clone();
                    return Command::perform(
                        async move {
                            rfd::AsyncFileDialog::new()
                                .set_directory(save_directory)
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
            Message::Clear(id) => {
                file_transfers.remove(&id);
            }
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

    use crate::buffer::file_transfers::row_button;
    use crate::widget::Element;
    use crate::{icon, theme};

    pub fn view<'a>(transfer: &FileTransfer, idx: usize) -> Element<'a, Message> {
        let status = match &transfer.status {
            file_transfer::Status::PendingApproval
            | file_transfer::Status::PendingReverseConfirmation => match &transfer.direction {
                file_transfer::Direction::Sent => container(
                    text(format!(
                        "Transfer to {}. Waiting for them to accept.",
                        transfer.remote_user
                    ))
                    .style(theme::text::transparent),
                ),
                file_transfer::Direction::Received => container(
                    text(format!(
                        "Transfer from {}. Accept to begin.",
                        transfer.remote_user
                    ))
                    .style(theme::text::transparent),
                ),
            },
            file_transfer::Status::Queued => {
                let direction = match transfer.direction {
                    file_transfer::Direction::Sent => "to",
                    file_transfer::Direction::Received => "from",
                };

                container(
                    text(format!(
                        "Transfer {} {}. Waiting for open port.",
                        direction, transfer.remote_user,
                    ))
                    .style(theme::text::transparent),
                )
            }
            file_transfer::Status::Ready => {
                let direction = match transfer.direction {
                    file_transfer::Direction::Sent => "to",
                    file_transfer::Direction::Received => "from",
                };

                container(
                    text(format!(
                        "Transfer {} {}. Waiting for remote user to connect.",
                        direction, transfer.remote_user
                    ))
                    .style(theme::text::transparent),
                )
            }
            file_transfer::Status::Active {
                transferred,
                elapsed,
            } => {
                let transfer_speed = if elapsed.as_secs() == 0 {
                    String::default()
                } else {
                    let bytes_per_second = *transferred / elapsed.as_secs();
                    let transfer_speed = ByteSize::b(bytes_per_second);

                    format!("({transfer_speed}/s)")
                };

                let transferred = ByteSize::b(*transferred);
                let file_size = ByteSize::b(transfer.size);

                let progress_bar = container(progress_bar(0.0..=1.0, transfer.progress() as f32))
                    .padding([4, 0])
                    .height(11);

                container(
                    column![
                        text(format!("{transferred} of {file_size} {transfer_speed}"))
                            .style(theme::text::transparent),
                        progress_bar
                    ]
                    .spacing(0),
                )
            }
            file_transfer::Status::Completed { elapsed, sha256 } => {
                let mut formatter = timeago::Formatter::new();
                formatter
                    .ago("")
                    .min_unit(timeago::TimeUnit::Seconds)
                    .too_low("under a second");
                let elapsed = formatter.convert(*elapsed);

                let direction = match transfer.direction {
                    file_transfer::Direction::Sent => "to",
                    file_transfer::Direction::Received => "from",
                };

                container(
                    text(format!(
                        "Completed {} {} in {elapsed}. Checksum: {sha256}",
                        direction, transfer.remote_user,
                    ))
                    .style(theme::text::transparent),
                )
            }
            file_transfer::Status::Failed { error } => {
                container(text(format!("Failed: {error}")).style(theme::text::error))
            }
        };

        let file_size = ByteSize::b(transfer.size);
        let filename = container(text(format!("{} ({file_size})", transfer.filename)));

        let mut buttons = row![].align_items(iced::Alignment::Center).spacing(2);
        let content = column![filename, status].width(Length::Fill).spacing(0);

        match &transfer.status {
            file_transfer::Status::PendingApproval => {
                buttons =
                    buttons.push(row_button(icon::checkmark(), Message::Approve(transfer.id)));
                buttons = buttons.push(row_button(icon::close(), Message::Clear(transfer.id)));
            }
            file_transfer::Status::PendingReverseConfirmation
            | file_transfer::Status::Queued
            | file_transfer::Status::Ready => {
                buttons = buttons.push(row_button(icon::close(), Message::Clear(transfer.id)));
            }
            file_transfer::Status::Active { .. } | file_transfer::Status::Completed { .. } => {
                buttons = buttons.push(row_button(icon::folder(), Message::OpenDirectory));
                buttons = buttons.push(row_button(icon::close(), Message::Clear(transfer.id)));
            }
            file_transfer::Status::Failed { .. } => {
                buttons = buttons.push(row_button(icon::close(), Message::Clear(transfer.id)));
            }
        }

        let row = row![content, buttons]
            .spacing(6)
            .align_items(iced::Alignment::Center);

        container(row)
            .padding([6, 4, 6, 8])
            .width(Length::Fill)
            .align_y(alignment::Vertical::Center)
            .style(move |theme, status| theme::container::table_row(theme, status, idx))
            .into()
    }
}

fn row_button(icon: Text, message: Message) -> Element<Message> {
    button(
        container(icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y(),
    )
    .on_press(message)
    .padding(5)
    .width(22)
    .height(22)
    .style(|theme, status| theme::button::tertiary(theme, status, false))
    .into()
}
