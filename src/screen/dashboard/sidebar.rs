use std::iter;
use std::time::Duration;

use data::config::{self, Config, sidebar};
use data::dashboard::{BufferAction, BufferFocusedAction};
use data::{
    Image, Version, buffer, file_transfer, history, isupport, server,
    server_icon, target,
};
use iced::widget::text::{LineHeight, Shaping};
use iced::widget::{
    Column, Row, Scrollable, Space, Svg, button, column, container, pane_grid,
    row, rule, scrollable, space, stack,
};
use iced::{
    Alignment, Border, ContentFit, Length, Padding, Task, mouse, padding,
};
use itertools::Either;
use tokio::time;

use super::{Focus, Panes, Server};
use crate::widget::{
    Element, Text, TextExt, context_menu, double_pass, image, text,
};
use crate::{Theme, font, icon, platform_specific, theme, window};

const CONFIG_RELOAD_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub enum Message {
    New(buffer::Upstream),
    Popout(buffer::Upstream),
    Focus(window::Id, pane_grid::Pane),
    Replace(buffer::Upstream),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane),
    Detach(buffer::Upstream),
    Leave(buffer::Upstream),
    CloseAllQueries(Server, Vec<target::Query>),
    ToggleInternalBuffer(buffer::Internal),
    ToggleCommandBar,
    ToggleThemeEditor,
    ReloadConfigFile,
    ConfigReloaded(Result<Config, config::Error>),
    OpenReleaseWebsite,
    OpenAbout {
        version: String,
        commit: String,
        system_information: Option<iced::system::Information>,
    },
    OpenDocumentation,
    OpenConfigFile,
    ReloadComplete,
    MarkAsRead(buffer::Upstream),
    MarkServerAsRead(Server),
    QuitApplication,
    Connect(Server),
    Remove(Server),
    SystemInformation(iced::system::Information),
}

#[derive(Debug, Clone)]
pub enum Event {
    New(buffer::Upstream),
    Popout(buffer::Upstream),
    Focus(window::Id, pane_grid::Pane),
    Replace(buffer::Upstream),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane),
    Detach(buffer::Upstream),
    Leave(buffer::Upstream),
    CloseAllQueries(Server, Vec<target::Query>),
    ToggleInternalBuffer(buffer::Internal),
    ToggleCommandBar,
    ToggleThemeEditor,
    OpenReleaseWebsite,
    OpenAbout {
        version: String,
        commit: String,
        system_information: Option<iced::system::Information>,
    },
    OpenDocumentation,
    OpenConfigFile,
    ConfigReloaded(Result<Config, config::Error>),
    MarkAsRead(buffer::Upstream),
    MarkServerAsRead(Server),
    QuitApplication,
    Connect(Server),
    Remove(Server),
}

#[derive(Clone)]
pub struct Sidebar {
    pub hidden: bool,
    reloading_config: bool,
    system_information: Option<iced::system::Information>,
}

impl Sidebar {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                hidden: false,
                reloading_config: false,
                system_information: None,
            },
            iced::system::information().map(Message::SystemInformation),
        )
    }

    pub fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden;
    }

    pub fn update(
        &mut self,
        message: Message,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::SystemInformation(information) => {
                self.system_information = Some(information);
                (Task::none(), None)
            }
            Message::CloseAllQueries(server, queries) => {
                (Task::none(), Some(Event::CloseAllQueries(server, queries)))
            }
            Message::QuitApplication => {
                (Task::none(), Some(Event::QuitApplication))
            }
            Message::New(source) => (Task::none(), Some(Event::New(source))),
            Message::Popout(source) => {
                (Task::none(), Some(Event::Popout(source)))
            }
            Message::Focus(window, pane) => {
                (Task::none(), Some(Event::Focus(window, pane)))
            }
            Message::Replace(source) => {
                (Task::none(), Some(Event::Replace(source)))
            }
            Message::Close(window, pane) => {
                (Task::none(), Some(Event::Close(window, pane)))
            }
            Message::Swap(window, pane) => {
                (Task::none(), Some(Event::Swap(window, pane)))
            }
            Message::Detach(buffer) => {
                (Task::none(), Some(Event::Detach(buffer)))
            }
            Message::Leave(buffer) => {
                (Task::none(), Some(Event::Leave(buffer)))
            }
            Message::ToggleInternalBuffer(buffer) => {
                (Task::none(), Some(Event::ToggleInternalBuffer(buffer)))
            }
            Message::ToggleCommandBar => {
                (Task::none(), Some(Event::ToggleCommandBar))
            }
            Message::ToggleThemeEditor => {
                (Task::none(), Some(Event::ToggleThemeEditor))
            }
            Message::ReloadConfigFile => {
                self.reloading_config = true;
                (Task::perform(Config::load(), Message::ConfigReloaded), None)
            }
            Message::ConfigReloaded(config) => (
                Task::perform(time::sleep(CONFIG_RELOAD_DELAY), |()| {
                    Message::ReloadComplete
                }),
                Some(Event::ConfigReloaded(config)),
            ),
            Message::OpenReleaseWebsite => {
                (Task::none(), Some(Event::OpenReleaseWebsite))
            }
            Message::ReloadComplete => {
                self.reloading_config = false;
                (Task::none(), None)
            }
            Message::OpenDocumentation => {
                (Task::none(), Some(Event::OpenDocumentation))
            }
            Message::MarkAsRead(buffer) => {
                (Task::none(), Some(Event::MarkAsRead(buffer)))
            }
            Message::MarkServerAsRead(server) => {
                (Task::none(), Some(Event::MarkServerAsRead(server)))
            }
            Message::OpenConfigFile => {
                (Task::none(), Some(Event::OpenConfigFile))
            }
            Message::Connect(server) => {
                (Task::none(), Some(Event::Connect(server)))
            }
            Message::Remove(server) => {
                (Task::none(), Some(Event::Remove(server)))
            }
            Message::OpenAbout {
                version,
                commit,
                system_information,
            } => (
                Task::none(),
                Some(Event::OpenAbout {
                    version,
                    commit,
                    system_information,
                }),
            ),
        }
    }

    fn user_menu_button<'a>(
        &self,
        config: &'a Config,
        history: &'a history::Manager,
        file_transfers: &'a file_transfer::Manager,
        version: &'a Version,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let keyboard = &config.keyboard;
        let menu_icon_size = theme::ICON_SIZE + 2.0;
        let base = button(
            container(
                icon::menu()
                    .style(theme::svg::primary)
                    .width(Length::Shrink)
                    .content_fit(ContentFit::Contain),
            )
            .width(menu_icon_size)
            .height(menu_icon_size),
        )
        .padding(5)
        .width(Length::Shrink);

        let menu = Menu::list(version.is_old(), config.file_transfer.enabled);

        let logs_has_unread = history.has_unread(&history::Kind::Logs);

        // Show notification dot if theres a new version, if there're transfers,
        // or if the logs have unread messages.
        let show_notification_dot = version.is_old()
            || (!file_transfers.is_empty() && config.file_transfer.enabled)
            || logs_has_unread;
        let system_information = self.system_information.clone();

        if menu.is_empty() {
            base.into()
        } else {
            stack![
                context_menu(
                    context_menu::MouseButton::Left,
                    context_menu::Anchor::Widget,
                    context_menu::ToggleBehavior::Close,
                    Some(mouse::Interaction::Pointer),
                    base,
                    menu,
                    move |menu, length| {
                        let context_button =
                            |title: Text<'a>,
                             keybinds: Option<&data::shortcut::KeyBinds>,
                             icon: Svg<'a, Theme>,
                             message: Message| {
                                let title = title.line_height(
                                    theme::line_height(&config.font),
                                );
                                let keybind =
                                    keybinds.and_then(|key_binds| match key_binds
                                        .primary()
                                    {
                                        Some(kb @ data::shortcut::KeyBind::Bind {
                                            ..
                                        }) => Some(
                                            text(format!("({kb})"))
                                                .shaping(Shaping::Advanced)
                                                .size(theme::TEXT_SIZE - 2.0)
                                                .style(theme::text::secondary)
                                                .font_maybe(
                                                    theme::font_style::secondary(
                                                        theme,
                                                    )
                                                    .map(font::get),
                                                ),
                                        ),
                                        _ => None,
                                    });

                                button(
                                    row![
                                        icon.width(Length::Fixed(12.0)),
                                        title,
                                        keybind
                                    ]
                                    .spacing(8)
                                    .align_y(iced::Alignment::Center),
                                )
                                .width(length)
                                .padding(config.context_menu.padding.entry)
                                .on_press(message)
                                .into()
                            };

                        match menu {
                            Menu::QuitApplication => context_button(
                                text("Quit Halloy"),
                                Some(&keyboard.quit_application),
                                icon::quit().style(theme::svg::primary),
                                Message::QuitApplication,
                            ),
                            Menu::RefreshConfig => context_button(
                                text("Reload config file"),
                                Some(&keyboard.reload_configuration),
                                icon::refresh().style(theme::svg::primary),
                                Message::ReloadConfigFile,
                            ),
                            Menu::CommandBar => context_button(
                                text("Command Bar"),
                                Some(&keyboard.command_bar),
                                icon::search().style(theme::svg::primary),
                                Message::ToggleCommandBar,
                            ),
                            Menu::FileTransfers => context_button(
                                text("File Transfers")
                                    .style(if file_transfers.is_empty() {
                                        theme::text::primary
                                    } else {
                                        theme::text::tertiary
                                    })
                                    .font_maybe(if file_transfers.is_empty() {
                                        theme::font_style::primary(theme)
                                            .map(font::get)
                                    } else {
                                        theme::font_style::tertiary(theme)
                                            .map(font::get)
                                    }),
                                Some(&keyboard.file_transfers),
                                icon::file_transfer().style(
                                    if file_transfers.is_empty() {
                                        theme::svg::primary
                                    } else {
                                        theme::svg::tertiary
                                    },
                                ),
                                Message::ToggleInternalBuffer(
                                    buffer::Internal::FileTransfers,
                                ),
                            ),
                            Menu::Highlights => context_button(
                                text("Highlights"),
                                Some(&keyboard.highlights),
                                icon::highlights().style(theme::svg::primary),
                                Message::ToggleInternalBuffer(
                                    buffer::Internal::Highlights,
                                ),
                            ),
                            Menu::ChannelDiscovery => context_button(
                                text("Channel Discovery"),
                                None,
                                icon::channel_discovery().style(theme::svg::primary),
                                Message::ToggleInternalBuffer(
                                    buffer::Internal::ChannelDiscovery(None),
                                ),
                            ),
                            Menu::Logs => context_button(
                                text("Logs")
                                    .style(if logs_has_unread {
                                        theme::text::tertiary
                                    } else {
                                        theme::text::primary
                                    })
                                    .font_maybe(if logs_has_unread {
                                        theme::font_style::tertiary(theme)
                                            .map(font::get)
                                    } else {
                                        theme::font_style::primary(theme)
                                            .map(font::get)
                                    }),
                                Some(&keyboard.logs),
                                icon::logs().style(if logs_has_unread {
                                    theme::svg::tertiary
                                } else {
                                    theme::svg::primary
                                }),
                                Message::ToggleInternalBuffer(
                                    buffer::Internal::Logs,
                                ),
                            ),
                            Menu::ThemeEditor => context_button(
                                text("Theme Editor"),
                                Some(&keyboard.theme_editor),
                                icon::theme_editor().style(theme::svg::primary),
                                Message::ToggleThemeEditor,
                            ),
                            Menu::HorizontalRule => match length {
                                Length::Fill => container(rule::horizontal(1))
                                    .padding([0, 6])
                                    .into(),
                                _ => {
                                    Space::new().width(length).height(1).into()
                                }
                            },
                            Menu::Update => context_button(
                                text("New version available")
                                    .style(theme::text::tertiary)
                                    .font_maybe(
                                        theme::font_style::tertiary(theme)
                                            .map(font::get),
                                    ),
                                None,
                                icon::megaphone().style(theme::svg::tertiary),
                                Message::OpenReleaseWebsite,
                            ),
                            Menu::Version => {
                                context_button(
                                    text("About Halloy"),
                                    None,
                                    icon::documentation().style(theme::svg::primary),
                                    Message::OpenAbout {
                                        version: version.current.clone(),
                                        commit: data::environment::GIT_HASH
                                            .map(str::trim)
                                            .filter(|hash| !hash.is_empty())
                                            .unwrap_or("Unknown")
                                            .to_string(),
                                        system_information: system_information
                                            .clone(),
                                    },
                                )
                            }
                            Menu::Documentation => context_button(
                                text("Documentation"),
                                None,
                                icon::documentation().style(theme::svg::primary),
                                Message::OpenDocumentation,
                            ),
                            Menu::OpenConfigFile => context_button(
                                text("Open config file"),
                                Some(&keyboard.open_config_file),
                                icon::config().style(theme::svg::primary),
                                Message::OpenConfigFile,
                            ),
                        }
                    },
                ),
                if show_notification_dot {
                    Some(
                        container(
                            icon::dot().style(theme::text::tertiary).size(8),
                        )
                        .padding(padding::left(13).top(2)),
                    )
                } else {
                    None
                },
            ]
            .into()
        }
    }

    pub fn view<'a>(
        &'a self,
        servers: &server::Map,
        clients: &data::client::Map,
        history: &'a history::Manager,
        panes: &'a Panes,
        focus: Focus,
        server_icons: &'a server_icon::Manager,
        config: &'a Config,
        file_transfers: &'a file_transfer::Manager,
        version: &'a Version,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        if self.hidden {
            return None;
        }

        let content = |width| {
            let user_menu_button =
                config.sidebar.user_menu.enabled.then(|| {
                    self.user_menu_button(
                        config,
                        history,
                        file_transfers,
                        version,
                        theme,
                    )
                });

            let mut buffers = vec![];

            if config.sidebar.position.is_horizontal() {
                buffers.push(space::horizontal().width(4).into());
            }

            let mut upstream_buffers = vec![];
            let mut client_enumeration = 0;

            for server in servers.keys() {
                let server_has_unread = history.server_has_unread(server);
                let supports_detach =
                    clients.get_server_supports_detach(server);
                let casemapping =
                    clients.get_server_casemapping_or_default(server);

                let button = |buffer: buffer::Upstream,
                              kind: history::Kind,
                              connected: bool| {
                    upstream_buffer_button(
                        config,
                        panes,
                        focus,
                        server_icons,
                        buffer,
                        kind,
                        connected,
                        server_has_unread,
                        supports_detach,
                        casemapping,
                        history,
                        width,
                        theme,
                    )
                };

                if let Some(state) = clients.state(server) {
                    client_enumeration += 1;

                    match state {
                        data::client::State::Disconnected => {
                            // Disconnected server.
                            upstream_buffers.push(button(
                                buffer::Upstream::Server(server.clone()),
                                history::Kind::Server(server.clone()),
                                false,
                            ));
                        }
                        data::client::State::Ready(connection) => {
                            // Connected server.
                            upstream_buffers.push(button(
                                buffer::Upstream::Server(server.clone()),
                                history::Kind::Server(server.clone()),
                                true,
                            ));

                            // Channels from the connected server.
                            for channel in connection.channels() {
                                upstream_buffers.push(button(
                                    buffer::Upstream::Channel(
                                        server.clone(),
                                        channel.clone(),
                                    ),
                                    history::Kind::Channel(
                                        server.clone(),
                                        channel.clone(),
                                    ),
                                    true,
                                ));
                            }

                            // Queries from the connected server.
                            let queries = history.get_unique_queries(server);
                            for query in queries {
                                let query = clients
                                    .resolve_query(server, query)
                                    .unwrap_or(query);

                                upstream_buffers.push(button(
                                    buffer::Upstream::Query(
                                        server.clone(),
                                        query.clone(),
                                    ),
                                    history::Kind::Query(
                                        server.clone(),
                                        query.clone(),
                                    ),
                                    true,
                                ));
                            }

                            // Separator between servers.
                            if client_enumeration < clients.len() {
                                if config.sidebar.position.is_horizontal() {
                                    upstream_buffers.push(
                                        space::horizontal()
                                            .width(
                                                config.sidebar.spacing.server,
                                            )
                                            .into(),
                                    );
                                } else {
                                    upstream_buffers.push(
                                        space::vertical()
                                            .height(
                                                config.sidebar.spacing.server,
                                            )
                                            .into(),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            let mut internal_buffers = vec![];

            for internal_buffer in
                config.sidebar.internal_buffers.buffers.iter()
            {
                let button = |buffer: buffer::Internal, title: &'static str| {
                    internal_buffer_button(
                        config, panes, focus, buffer, title, history, width,
                        theme,
                    )
                };

                match internal_buffer {
                    data::config::sidebar::InternalBuffer::FileTransfer => {
                        config.file_transfer.enabled.then(|| {
                            internal_buffers.push(button(
                                buffer::Internal::FileTransfers,
                                "File Transfers",
                            ));
                        });
                    }
                    data::config::sidebar::InternalBuffer::ChannelDiscovery => {
                        internal_buffers.push(button(
                            buffer::Internal::ChannelDiscovery(None),
                            "Channel Discovery",
                        ));
                    }
                    data::config::sidebar::InternalBuffer::Highlights => {
                        internal_buffers.push(button(
                            buffer::Internal::Highlights,
                            "Highlights",
                        ));
                    }
                    data::config::sidebar::InternalBuffer::Logs => {
                        internal_buffers
                            .push(button(buffer::Internal::Logs, "Logs"));
                    }
                }
            }

            let spacer = if config.sidebar.position.is_horizontal() {
                space::horizontal()
                    .width(config.sidebar.spacing.server)
                    .into()
            } else {
                space::vertical()
                    .height(config.sidebar.spacing.server)
                    .into()
            };

            let (left, right) =
                if config.sidebar.internal_buffers.is_before_servers() {
                    (internal_buffers, upstream_buffers)
                } else {
                    (upstream_buffers, internal_buffers)
                };

            buffers.extend(left);
            if !buffers.is_empty() && !right.is_empty() {
                buffers.push(spacer);
            }
            buffers.extend(right);

            match config.sidebar.position {
                sidebar::Position::Left | sidebar::Position::Right => {
                    let column_padding = if matches!(
                        config.sidebar.position,
                        sidebar::Position::Left
                    ) {
                        padding::right(2)
                    } else {
                        padding::left(2)
                    };

                    // Add buffers to a column.
                    let buffers = column![
                        Scrollable::new(
                            Column::with_children(buffers)
                                .spacing(1)
                                .padding(column_padding)
                        )
                        .direction(
                            scrollable::Direction::Vertical(
                                scrollable::Scrollbar::default()
                                    .width(config.sidebar.scrollbar.width)
                                    .scroller_width(
                                        config.sidebar.scrollbar.scroller_width
                                    )
                                    .spacing(4)
                            )
                        )
                    ];

                    // Wrap buffers in a column with user_menu_button
                    let content = column![
                        container(buffers).height(Length::Fill),
                        user_menu_button,
                    ];

                    container(content)
                }
                sidebar::Position::Top | sidebar::Position::Bottom => {
                    // Add buffers to a row.
                    let buffers = row![
                        Scrollable::new(
                            Row::with_children(buffers)
                                .spacing(2)
                                .align_y(Alignment::Center)
                        )
                        .direction(
                            scrollable::Direction::Horizontal(
                                scrollable::Scrollbar::default()
                                    .width(config.sidebar.scrollbar.width)
                                    .scroller_width(
                                        config.sidebar.scrollbar.scroller_width
                                    )
                                    .spacing(4)
                            )
                        )
                    ];

                    // Wrap buffers in a row with user_menu_button
                    let content = row![
                        container(buffers).width(Length::Fill),
                        user_menu_button,
                    ]
                    .align_y(Alignment::Center);

                    container(content)
                }
            }
        };

        let platform_specific_padding =
            platform_specific::sidebar_padding(config);

        let padding = match config.sidebar.position {
            sidebar::Position::Left => {
                padding::top(8 + platform_specific_padding)
                    .bottom(6)
                    .left(6)
            }
            sidebar::Position::Right => {
                padding::top(8 + platform_specific_padding)
                    .bottom(6)
                    .right(6)
            }
            sidebar::Position::Top => {
                padding::top(8 + platform_specific_padding).right(6)
            }
            sidebar::Position::Bottom => padding::bottom(8)
                .left(6)
                .right(6)
                .top(platform_specific_padding),
        };

        let content = if config.sidebar.position.is_horizontal() {
            container(
                content(Length::Shrink).width(Length::Fill).padding(padding),
            )
        } else {
            let first_pass = content(Length::Shrink);
            let second_pass = content(Length::Fill);

            container(double_pass(first_pass, second_pass))
                .max_width(
                    config.sidebar.max_width.map_or(f32::INFINITY, f32::from),
                )
                .width(Length::Shrink)
                .padding(padding)
        };

        Some(content.into())
    }
}

#[derive(Debug, Clone, Copy)]
enum Menu {
    RefreshConfig,
    CommandBar,
    ThemeEditor,
    Highlights,
    ChannelDiscovery,
    Logs,
    FileTransfers,
    Version,
    Update,
    HorizontalRule,
    Documentation,
    OpenConfigFile,
    QuitApplication,
}

impl Menu {
    fn list(has_new_version: bool, file_transfer_enabled: bool) -> Vec<Self> {
        let mut list = vec![Self::Version];

        if has_new_version {
            list.push(Self::Update);
        }

        list.extend([
            Self::HorizontalRule,
            Self::CommandBar,
            Self::Documentation,
        ]);

        if file_transfer_enabled {
            list.push(Self::FileTransfers);
        }

        list.extend([
            Self::ChannelDiscovery,
            Self::Highlights,
            Self::Logs,
            Self::OpenConfigFile,
            Self::RefreshConfig,
            Self::ThemeEditor,
            Self::QuitApplication,
        ]);

        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Entry {
    Context,
    HorizontalRule,
    Connect,
    Close(window::Id, pane_grid::Pane),
    CloseAllQueries,
    MarkAsRead,
    MarkServerAsRead,
    NewPane,
    Popout,
    Replace,
    Swap(window::Id, pane_grid::Pane),
    Detach,
    Leave,
    Remove,
}

impl Entry {
    fn list(
        buffer: &buffer::Upstream,
        num_panes: usize,
        open: Option<(window::Id, pane_grid::Pane)>,
        focus: Focus,
        connected: bool,
        supports_detach: bool,
    ) -> Vec<Self> {
        use Entry::*;
        use itertools::Itertools;

        itertools::chain!(
            match buffer {
                buffer::Upstream::Server(_) => if connected {
                    vec![CloseAllQueries, MarkServerAsRead]
                } else {
                    vec![Connect, Remove]
                }
                .into_iter()
                .chain(vec![Context, HorizontalRule])
                .collect_vec(),
                buffer::Upstream::Channel(_, _) => vec![],
                buffer::Upstream::Query(_, _) => vec![],
            },
            match open {
                None => vec![MarkAsRead, NewPane, Popout, Replace],
                Some((window, pane)) => (num_panes > 1)
                    .then_some(Close(window, pane))
                    .into_iter()
                    .chain(
                        (Focus { window, pane } != focus)
                            .then_some(Swap(window, pane)),
                    )
                    .collect_vec(),
            },
            if connected {
                (matches!(buffer, buffer::Upstream::Channel(_, _))
                    && supports_detach)
                    .then_some(Detach)
                    .into_iter()
                    .chain(iter::once(Leave))
                    .collect_vec()
            } else {
                vec![]
            },
        )
        .sorted()
        .collect_vec()
    }
}

fn upstream_buffer_button<'a>(
    config: &'a Config,
    panes: &'a Panes,
    focus: Focus,
    server_icons: &'a server_icon::Manager,
    buffer: buffer::Upstream,
    kind: history::Kind,
    connected: bool,
    server_has_unread: bool,
    supports_detach: bool,
    casemapping: isupport::CaseMap,
    history: &'a history::Manager,
    width: Length,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let open = panes.iter().find_map(|(window_id, pane, state)| {
        (state.buffer.upstream() == Some(&buffer)).then_some((window_id, pane))
    });

    let has_unread = if config.sidebar.unread_indicator.show_on_open_buffers
        || open.is_none()
    {
        history.has_unread(&kind)
    } else {
        false
    };

    let has_highlight = if config.sidebar.unread_indicator.show_on_open_buffers
        || open.is_none()
    {
        history.has_highlight(&kind)
    } else {
        false
    };

    let is_focused = panes.iter().find_map(|(window_id, pane, state)| {
        (Focus {
            window: window_id,
            pane,
        } == focus
            && state.buffer.upstream() == Some(&buffer))
        .then_some((window_id, pane))
    });

    let should_indicate_unread =
        config.sidebar.unread_indicator.should_indicate_unread(
            buffer.target().as_ref(),
            buffer.server(),
            casemapping,
        );
    let is_unread_query =
        matches!(buffer, buffer::Upstream::Query(_, _)) && has_unread;
    let has_highlight = has_highlight
        || (is_unread_query
            && config.sidebar.unread_indicator.query_as_highlight);

    let show_highlight_icon = has_highlight
        && config.sidebar.unread_indicator.has_unread_highlight_icon()
        && should_indicate_unread;
    let show_unread_icon = has_unread
        && config.sidebar.unread_indicator.has_unread_icon()
        && should_indicate_unread;
    let show_unread_title = has_unread
        && config.sidebar.unread_indicator.title
        && should_indicate_unread;
    let show_highlight_unread_title = has_highlight
        && config.sidebar.unread_indicator.title
        && should_indicate_unread;

    let buffer_title_style = if show_highlight_unread_title {
        theme::text::highlight_indicator
    } else if show_unread_title {
        theme::text::unread_indicator
    } else if !connected {
        if matches!(&buffer, buffer::Upstream::Server(_)) {
            theme::text::error
        } else {
            theme::text::secondary
        }
    } else {
        theme::text::primary
    };

    let buffer_title_font = theme::font_style::primary(theme).map(font::get);

    let dimensions = Dimensions::from(config);

    let icon = if dimensions.icon_size > 0
        && let buffer::Upstream::Server(server) = &buffer
    {
        if config
            .servers
            .get(server)
            .is_some_and(|server_config| server_config.icon.enabled)
            && let Some(server_icon) = server_icons.get(server)
        {
            Some(Icon::Upstream(server_icon))
        } else {
            Some(Icon::Internal(
                if server.is_bouncer_network() {
                    icon::link()
                } else {
                    icon::connected()
                }
                .style(theme::svg::primary),
            ))
        }
    } else {
        None
    };

    let indicator = if let buffer::Upstream::Server(_) = &buffer
        && !connected
    {
        Some((
            icon::disconnected().style(theme::svg::error),
            dimensions.icon_badge_size,
        ))
    } else if show_highlight_icon
        && let Some(highlight_icon) =
            icon::from_icon(config.sidebar.unread_indicator.highlight_icon)
    {
        Some((
            highlight_icon.style(theme::svg::highlight_indicator),
            dimensions.highlight_indicator_size,
        ))
    } else if show_unread_icon
        && let Some(unread_icon) =
            icon::from_icon(config.sidebar.unread_indicator.icon)
    {
        Some((
            unread_icon.style(theme::svg::unread_indicator),
            dimensions.unread_indicator_size,
        ))
    } else {
        None
    };

    let mut content = row![].align_y(iced::Alignment::Center);

    content = content.extend(sidebar_icon(
        icon,
        indicator,
        dimensions,
        config.sidebar.position.is_horizontal(),
    ));

    match &buffer {
        buffer::Upstream::Server(server) => {
            let font_size = config
                .sidebar
                .server_font_size
                .or(config.sidebar.font_size)
                .or(config.font.size)
                .map_or(theme::TEXT_SIZE, f32::from);

            if let Some(network) = &server.network {
                content = content.push(
                    text(network.name.to_string())
                        .line_height(LineHeight::Relative(1.0))
                        .size(font_size)
                        .style(buffer_title_style)
                        .font_maybe(buffer_title_font.clone())
                        .shaping(Shaping::Advanced),
                );
                content = content.push(Space::new().width(6));
                content = content.push(
                    text(server.name.to_string())
                        .line_height(LineHeight::Relative(1.0))
                        .size(font_size)
                        .style(theme::text::secondary)
                        .font_maybe(buffer_title_font)
                        .shaping(Shaping::Advanced),
                );
            } else {
                content = content.push(
                    text(server.to_string())
                        .line_height(LineHeight::Relative(1.0))
                        .size(font_size)
                        .style(buffer_title_style)
                        .font_maybe(buffer_title_font)
                        .shaping(Shaping::Advanced),
                );
            }
        }
        buffer::Upstream::Channel(_, channel) => {
            let font_size =
                config.sidebar.font_size.or(config.font.size).map(f32::from);
            let raw_channel = channel.as_str();
            let display_channel =
                if let Some(casing) = config.sidebar.channel_name_casing {
                    casing.apply(raw_channel, casemapping)
                } else {
                    raw_channel.to_owned()
                };

            content = content.push(
                text(display_channel)
                    .line_height(LineHeight::Relative(1.0))
                    .size_maybe(font_size)
                    .style(buffer_title_style)
                    .font_maybe(buffer_title_font)
                    .shaping(Shaping::Advanced),
            );
        }
        buffer::Upstream::Query(_, query) => {
            let font_size =
                config.sidebar.font_size.or(config.font.size).map(f32::from);

            content = content.push(
                text(query.to_string())
                    .line_height(LineHeight::Relative(1.0))
                    .size_maybe(font_size)
                    .style(buffer_title_style)
                    .font_maybe(buffer_title_font)
                    .shaping(Shaping::Advanced),
            );
        }
    }

    let base =
        button(content.width(width).padding(Padding::default().bottom(1)))
            .style(move |theme, status| {
                theme::button::sidebar_buffer(
                    theme,
                    status,
                    is_focused.is_some(),
                    open.is_some(),
                )
            })
            .padding(config.sidebar.padding.buffer)
            .on_press({
                match is_focused {
                    Some((window, pane)) => {
                        if let Some(focus_action) =
                            config.actions.sidebar.focused_buffer
                        {
                            match focus_action {
                                BufferFocusedAction::ClosePane => {
                                    Message::Close(window, pane)
                                }
                            }
                        } else {
                            // Re-focus pane on press instead of disabling the button in order
                            // to have hover status of the button for styling
                            Message::Focus(window, pane)
                        }
                    }
                    None => {
                        if let Some((window, pane)) = open {
                            Message::Focus(window, pane)
                        } else {
                            let action = match &buffer {
                                buffer::Upstream::Channel(_, _) => {
                                    config.actions.sidebar.channel.unwrap_or(
                                        config.actions.sidebar.buffer,
                                    )
                                }
                                buffer::Upstream::Query(_, _) => {
                                    config.actions.sidebar.query.unwrap_or(
                                        config.actions.sidebar.buffer,
                                    )
                                }
                                _ => config.actions.sidebar.buffer,
                            };

                            match action {
                                BufferAction::NewPane => {
                                    Message::New(buffer.clone())
                                }
                                BufferAction::ReplacePane => {
                                    Message::Replace(buffer.clone())
                                }
                                BufferAction::NewWindow => {
                                    Message::Popout(buffer.clone())
                                }
                            }
                        }
                    }
                }
            });

    let entries = Entry::list(
        &buffer,
        panes.len(),
        open,
        focus,
        connected,
        supports_detach,
    );

    if entries.is_empty() {
        base.into()
    } else {
        context_menu(
            context_menu::MouseButton::default(),
            context_menu::Anchor::Cursor,
            context_menu::ToggleBehavior::KeepOpen,
            Some(mouse::Interaction::Pointer),
            base,
            entries,
            move |entry, length| {
                let (content, message) = match entry {
                    Entry::CloseAllQueries => {
                        let queries = history
                            .get_unique_queries(buffer.server())
                            .into_iter()
                            .cloned()
                            .collect::<Vec<_>>();

                        (
                            "Close all queries",
                            if queries.is_empty() {
                                None
                            } else {
                                Some(Message::CloseAllQueries(
                                    buffer.server().clone(),
                                    queries,
                                ))
                            },
                        )
                    }
                    Entry::MarkServerAsRead => (
                        "Mark entire server as read",
                        if server_has_unread {
                            Some(Message::MarkServerAsRead(
                                buffer.server().clone(),
                            ))
                        } else {
                            None
                        },
                    ),
                    Entry::MarkAsRead => (
                        if matches!(&buffer, buffer::Upstream::Server(_)) {
                            "Mark server buffer as read"
                        } else {
                            "Mark as read"
                        },
                        if has_unread {
                            Some(Message::MarkAsRead(buffer.clone()))
                        } else {
                            None
                        },
                    ),
                    Entry::NewPane => {
                        ("Open in new pane", Some(Message::New(buffer.clone())))
                    }
                    Entry::Popout => (
                        "Open in new window",
                        Some(Message::Popout(buffer.clone())),
                    ),
                    Entry::Replace => (
                        "Replace current pane",
                        Some(Message::Replace(buffer.clone())),
                    ),
                    Entry::Close(window, pane) => {
                        ("Close pane", Some(Message::Close(window, pane)))
                    }
                    Entry::Swap(window, pane) => (
                        "Swap with current pane",
                        Some(Message::Swap(window, pane)),
                    ),
                    Entry::Detach => (
                        "Detach from channel",
                        Some(Message::Detach(buffer.clone())),
                    ),
                    Entry::Leave => (
                        match &buffer {
                            buffer::Upstream::Server(_) => {
                                "Disconnect from server"
                            }
                            buffer::Upstream::Channel(_, _) => "Leave channel",
                            buffer::Upstream::Query(_, _) => "Close query",
                        },
                        Some(Message::Leave(buffer.clone())),
                    ),
                    Entry::Connect => (
                        "Connect to server",
                        Some(Message::Connect(buffer.server().clone())),
                    ),
                    Entry::Remove => (
                        "Remove server from sidebar",
                        Some(Message::Remove(buffer.server().clone())),
                    ),
                    Entry::Context => {
                        return container(
                            row![
                                text(match &buffer {
                                    buffer::Upstream::Server(server) => {
                                        if let Some(network) = &server.network {
                                            network.name.to_string()
                                        } else {
                                            format!("{server}")
                                        }
                                    }
                                    buffer::Upstream::Channel(_, channel) => {
                                        format!("{channel}")
                                    }
                                    buffer::Upstream::Query(_, query) => {
                                        format!("{query}")
                                    }
                                })
                                .style(theme::text::primary)
                                .font_maybe(
                                    theme::font_style::primary(theme)
                                        .map(font::get),
                                ),
                                Space::new().width(6),
                                match &buffer {
                                    buffer::Upstream::Server(server) => {
                                        if server.network.is_some() {
                                            Some(server.name.to_string())
                                        } else {
                                            None
                                        }
                                    }
                                    buffer::Upstream::Channel(server, _) => {
                                        Some(format!("{server}"))
                                    }
                                    buffer::Upstream::Query(server, _) => {
                                        Some(format!("{server}"))
                                    }
                                }
                                .map(
                                    |secondary_name| text(secondary_name)
                                        .style(theme::text::secondary)
                                        .font_maybe(
                                            theme::font_style::secondary(theme)
                                                .map(font::get)
                                        ),
                                )
                            ]
                            .width(length),
                        )
                        .padding(config.context_menu.padding.entry)
                        .into();
                    }
                    Entry::HorizontalRule => match length {
                        Length::Fill => {
                            return container(rule::horizontal(1))
                                .padding([0, 6])
                                .into();
                        }
                        _ => {
                            return Space::new().width(length).height(1).into();
                        }
                    },
                };

                button(text(content))
                    .width(length)
                    .padding(config.context_menu.padding.entry)
                    .style(|theme, status| {
                        theme::button::primary(theme, status, false)
                    })
                    .on_press_maybe(message)
                    .into()
            },
        )
        .into()
    }
}

fn internal_buffer_button<'a>(
    config: &'a Config,
    panes: &'a Panes,
    focus: Focus,
    buffer: buffer::Internal,
    title: &'a str,
    history: &'a history::Manager,
    width: Length,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let open = panes.iter().find_map(|(window_id, pane, state)| {
        (state.buffer.internal() == Some(buffer.clone()))
            .then_some((window_id, pane))
    });

    let is_focused = panes.iter().find_map(|(window_id, pane, state)| {
        (Focus {
            window: window_id,
            pane,
        } == focus
            && state.buffer.internal() == Some(buffer.clone()))
        .then_some((window_id, pane))
    });

    let dimensions = Dimensions::from(config);

    let show_icon = dimensions.icon_size > 0;

    let (icon, badge) = match buffer {
        buffer::Internal::ChannelDiscovery(_) => {
            (show_icon.then_some(icon::channel_discovery()), None)
        }
        buffer::Internal::FileTransfers => {
            (show_icon.then_some(icon::file_transfer()), None)
        }
        buffer::Internal::Highlights => {
            let has_unread =
                if config.sidebar.unread_indicator.show_on_open_buffers
                    || open.is_none()
                {
                    history.has_unread(&history::Kind::Highlights)
                } else {
                    false
                };

            let badge = if has_unread
                && let Some(highlight_icon) = icon::from_icon(
                    config.sidebar.unread_indicator.highlight_icon,
                ) {
                Some((
                    highlight_icon.style(theme::svg::highlight_indicator),
                    dimensions.highlight_indicator_size,
                ))
            } else {
                None
            };

            (show_icon.then_some(icon::highlights()), badge)
        }
        buffer::Internal::Logs => {
            let has_unread =
                if config.sidebar.unread_indicator.show_on_open_buffers
                    || open.is_none()
                {
                    history.has_unread(&history::Kind::Logs)
                } else {
                    false
                };

            let badge = if has_unread
                && let Some(unread_icon) =
                    icon::from_icon(config.sidebar.unread_indicator.icon)
            {
                Some((
                    unread_icon.style(theme::svg::unread_indicator),
                    dimensions.unread_indicator_size,
                ))
            } else {
                None
            };

            (show_icon.then_some(icon::logs()), badge)
        }
    };

    let mut content = row![].align_y(iced::Alignment::Center);

    content = content.extend(sidebar_icon(
        icon.map(Icon::Internal),
        badge,
        dimensions,
        config.sidebar.position.is_horizontal(),
    ));

    content = content.push(
        text(title)
            .line_height(LineHeight::Relative(1.0))
            .size_maybe(
                config.sidebar.font_size.or(config.font.size).map(f32::from),
            )
            .style(theme::text::primary)
            .font_maybe(theme::font_style::primary(theme).map(font::get))
            .shaping(Shaping::Advanced),
    );

    button(content.width(width).padding(Padding::default().bottom(1)))
        .style(move |theme, status| {
            theme::button::sidebar_buffer(
                theme,
                status,
                is_focused.is_some(),
                open.is_some(),
            )
        })
        .padding(config.sidebar.padding.buffer)
        .on_press(match is_focused {
            Some((window, pane)) => {
                if let Some(focus_action) =
                    config.actions.sidebar.focused_buffer
                {
                    match focus_action {
                        BufferFocusedAction::ClosePane => {
                            Message::Close(window, pane)
                        }
                    }
                } else {
                    Message::Focus(window, pane)
                }
            }
            None => {
                if let Some((window, pane)) = open {
                    Message::Focus(window, pane)
                } else {
                    Message::ToggleInternalBuffer(buffer)
                }
            }
        })
        .into()
}

enum Icon<'a> {
    Upstream(&'a Image),
    Internal(Svg<'a, Theme>),
}

fn sidebar_icon<'a>(
    icon: Option<Icon<'a>>,
    indicator: Option<(Svg<'a, Theme>, u32)>,
    dimensions: Dimensions,
    sidebar_is_horizontal: bool,
) -> impl IntoIterator<Item = Element<'a, Message>> {
    let (icon, icon_height, icon_left_spacing): (
        Option<Element<'a, Message>>,
        u32,
        f32,
    ) = if let Some(icon) = icon {
        let icon: Element<'a, Message> = container(match icon {
            Icon::Upstream(server_icon) => {
                image::from_data(server_icon, true, ContentFit::Contain)
            }
            Icon::Internal(icon) => icon
                .style(theme::svg::primary)
                .width(Length::Shrink)
                .content_fit(ContentFit::Contain)
                .into(),
        })
        .width(dimensions.icon_size)
        .height(dimensions.icon_size)
        .into();

        let badge: Option<Element<'a, Message>> =
            indicator.map(move |(indicator, _)| {
                container(
                    indicator
                        .width(Length::Shrink)
                        .content_fit(ContentFit::Contain),
                )
                .style(move |theme: &Theme| container::Style {
                    text_color: None,
                    background: Some(
                        theme.styles().buttons.primary.background.into(),
                    ),
                    border: Border {
                        radius: dimensions.icon_badge_size.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .width(dimensions.icon_badge_size)
                .height(dimensions.icon_badge_size)
                .padding(dimensions.icon_badge_padding as f32)
                .into()
            });

        (
            Some(
                stack![
                    row![
                        Space::new().width(dimensions.icon_badge_padding),
                        column![
                            Space::new().height(dimensions.icon_badge_padding),
                            icon
                        ]
                    ]
                    .align_y(iced::Alignment::Center),
                    badge,
                ]
                .into(),
            ),
            dimensions.icon_size,
            dimensions
                .max_indicator_size()
                .saturating_sub(dimensions.icon_badge_size) as f32
                / 2.0,
        )
    } else if let Some((indicator, indicator_size)) = indicator {
        (
            Some(
                container(
                    indicator
                        .width(Length::Shrink)
                        .content_fit(ContentFit::Contain),
                )
                .width(indicator_size)
                .height(indicator_size)
                .into(),
            ),
            indicator_size,
            dimensions
                .max_indicator_size()
                .saturating_sub(indicator_size) as f32
                / 2.0,
        )
    } else {
        (None, 1, 0.0)
    };

    if sidebar_is_horizontal {
        if let Some(icon) = icon {
            Either::Left(vec![icon, Space::new().width(8).into()].into_iter())
        } else {
            Either::Right(iter::empty())
        }
    } else {
        Either::Left(
            vec![
                stack![
                    Space::new()
                        .width(dimensions.max_icon_size())
                        .height(icon_height),
                    icon.map(|icon| row![
                        Space::new().width(icon_left_spacing),
                        icon
                    ])
                ]
                .into(),
                Space::new().width(8).into(),
            ]
            .into_iter(),
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Dimensions {
    icon_size: u32,
    icon_badge_size: u32,
    icon_badge_padding: u32,
    unread_indicator_size: u32,
    highlight_indicator_size: u32,
}

impl From<&Config> for Dimensions {
    fn from(config: &Config) -> Self {
        let (icon_size, icon_badge_padding, icon_badge_size) =
            match config.sidebar.server_icon {
                data::config::sidebar::ServerIcon::Size(icon_size) => {
                    let icon_badge_padding = 2;
                    let icon_badge_size =
                        (icon_size / 3).max(4) + 2 * icon_badge_padding;

                    (icon_size, icon_badge_padding, icon_badge_size)
                }
                data::config::sidebar::ServerIcon::Hidden => (0, 0, 0),
            };

        let unread_indicator_size =
            if config.sidebar.unread_indicator.has_unread_icon() {
                config.sidebar.unread_indicator.icon_size
            } else {
                0
            };

        let highlight_indicator_size =
            if config.sidebar.unread_indicator.has_unread_highlight_icon() {
                config.sidebar.unread_indicator.highlight_icon_size
            } else {
                0
            };

        Self {
            icon_size,
            icon_badge_size,
            icon_badge_padding,
            unread_indicator_size,
            highlight_indicator_size,
        }
    }
}

impl Dimensions {
    fn max_indicator_size(&self) -> u32 {
        self.icon_badge_size
            .max(self.unread_indicator_size)
            .max(self.highlight_indicator_size)
    }

    fn max_icon_size(&self) -> u32 {
        self.icon_size
            .max(self.unread_indicator_size)
            .max(self.highlight_indicator_size)
    }
}
