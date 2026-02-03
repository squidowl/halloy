use std::time::Duration;

use data::config::{self, Config, sidebar};
use data::dashboard::{BufferAction, BufferFocusedAction};
use data::{Version, buffer, file_transfer, history, isupport, server, target};
use iced::widget::{
    Column, Row, Scrollable, Space, button, column, container, pane_grid, row,
    rule, scrollable, space, stack, text,
};
use iced::{Alignment, Length, Padding, Task, padding};
use tokio::time;

use super::{Focus, Panes, Server};
use crate::buffer::context_menu as buffer_context_menu;
use crate::widget::{Element, Text, context_menu, double_pass};
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
    OpenDocumentation,
    OpenConfigFile,
    ReloadComplete,
    MarkAsRead(buffer::Upstream),
    MarkServerAsRead(Server),
    Nicklist(buffer_context_menu::Message),
    QuitApplication,
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
    OpenDocumentation,
    OpenConfigFile,
    ConfigReloaded(Result<Config, config::Error>),
    MarkAsRead(buffer::Upstream),
    MarkServerAsRead(Server),
    Nicklist(buffer_context_menu::Message),
    QuitApplication,
}

#[derive(Clone)]
pub struct Sidebar {
    pub hidden: bool,
    nicklist_hidden: bool,
    reloading_config: bool,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            hidden: false,
            nicklist_hidden: false,
            reloading_config: false,
        }
    }

    pub fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden;
    }

    pub fn toggle_nicklist(&mut self) {
        self.nicklist_hidden = !self.nicklist_hidden;
    }

    pub fn update(
        &mut self,
        message: Message,
    ) -> (Task<Message>, Option<Event>) {
        match message {
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
            Message::Nicklist(message) => {
                (Task::none(), Some(Event::Nicklist(message)))
            }
            Message::OpenConfigFile => {
                (Task::none(), Some(Event::OpenConfigFile))
            }
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
        let base = button(icon::menu().size(theme::ICON_SIZE + 2.0))
            .padding(5)
            .width(Length::Shrink);

        let menu = Menu::list(
            config.sidebar.user_menu.show_new_version_indicator
                && version.is_old(),
            config.file_transfer.enabled,
        );

        let logs_has_unread = history.has_unread(&history::Kind::Logs);

        // Show notification dot if theres a new version, if there're transfers,
        // or if the logs have unread messages.
        let show_notification_dot =
            (config.sidebar.user_menu.show_new_version_indicator
                && version.is_old())
                || (!file_transfers.is_empty() && config.file_transfer.enabled)
                || logs_has_unread;

        if menu.is_empty() {
            base.into()
        } else {
            stack![
                context_menu(
                    context_menu::MouseButton::Left,
                    context_menu::Anchor::Widget,
                    context_menu::ToggleBehavior::Close,
                    base,
                    menu,
                    move |menu, length| {
                        let context_button =
                            |title: Text<'a>,
                             keybind: Option<&data::shortcut::KeyBind>,
                             icon: Text<'a>,
                             message: Message| {
                                let keybind = keybind.and_then(|kb| match kb {
                                    data::shortcut::KeyBind::Bind {
                                        ..
                                    } => Some(
                                        text(format!("({kb})"))
                                            .shaping(text::Shaping::Advanced)
                                            .size(theme::TEXT_SIZE - 2.0)
                                            .style(theme::text::secondary)
                                            .font_maybe(
                                                theme::font_style::secondary(
                                                    theme,
                                                )
                                                .map(font::get),
                                            ),
                                    ),
                                    data::shortcut::KeyBind::Unbind => None,
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
                                icon::quit(),
                                Message::QuitApplication,
                            ),
                            Menu::RefreshConfig => context_button(
                                text("Reload config file"),
                                Some(&keyboard.reload_configuration),
                                icon::refresh(),
                                Message::ReloadConfigFile,
                            ),
                            Menu::CommandBar => context_button(
                                text("Command Bar"),
                                Some(&keyboard.command_bar),
                                icon::search(),
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
                                        theme::text::primary
                                    } else {
                                        theme::text::tertiary
                                    },
                                ),
                                Message::ToggleInternalBuffer(
                                    buffer::Internal::FileTransfers,
                                ),
                            ),
                            Menu::Highlights => context_button(
                                text("Highlights"),
                                Some(&keyboard.highlights),
                                icon::highlights(),
                                Message::ToggleInternalBuffer(
                                    buffer::Internal::Highlights,
                                ),
                            ),
                            Menu::ChannelDiscovery => context_button(
                                text("Channel Discovery"),
                                None,
                                icon::channel_discovery(),
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
                                    theme::text::tertiary
                                } else {
                                    theme::text::primary
                                }),
                                Message::ToggleInternalBuffer(
                                    buffer::Internal::Logs,
                                ),
                            ),
                            Menu::ThemeEditor => context_button(
                                text("Theme Editor"),
                                Some(&keyboard.theme_editor),
                                icon::theme_editor(),
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
                                icon::megaphone().style(theme::text::tertiary),
                                Message::OpenReleaseWebsite,
                            ),
                            Menu::Version => container(
                                text(format!("Halloy ({})", version.current))
                                    .style(theme::text::secondary)
                                    .font_maybe(
                                        theme::font_style::secondary(theme)
                                            .map(font::get),
                                    ),
                            )
                            .padding(5)
                            .into(),
                            Menu::Documentation => context_button(
                                text("Documentation"),
                                None,
                                icon::documentation(),
                                Message::OpenDocumentation,
                            ),
                            Menu::OpenConfigFile => context_button(
                                text("Open config file"),
                                None,
                                icon::config(),
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
        clients: &'a data::client::Map,
        history: &'a history::Manager,
        panes: &'a Panes,
        focus: Focus,
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
            let mut client_enumeration = 0;

            if config.sidebar.position.is_horizontal() {
                buffers.push(space::horizontal().width(4).into());
            }

            for server in servers.keys() {
                let casemapping = clients.get_casemapping(server);

                let button =
                    |buffer: buffer::Upstream,
                     connected: bool,
                     server_has_unread: bool,
                     supports_detach: bool,
                     has_unread: bool,
                     has_highlight: bool| {
                        upstream_buffer_button(
                            config,
                            panes,
                            focus,
                            buffer,
                            connected,
                            server_has_unread,
                            supports_detach,
                            casemapping,
                            has_unread,
                            has_highlight,
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
                            buffers.push(button(
                                buffer::Upstream::Server(server.clone()),
                                false,
                                history.server_has_unread(server.clone()),
                                clients.get_server_supports_detach(server),
                                history.has_unread(&history::Kind::Server(
                                    server.clone(),
                                )),
                                history.has_highlight(&history::Kind::Server(
                                    server.clone(),
                                )),
                            ));
                        }
                        data::client::State::Ready(connection) => {
                            // Connected server.
                            buffers.push(button(
                                buffer::Upstream::Server(server.clone()),
                                true,
                                history.server_has_unread(server.clone()),
                                clients.get_server_supports_detach(server),
                                history.has_unread(&history::Kind::Server(
                                    server.clone(),
                                )),
                                history.has_highlight(&history::Kind::Server(
                                    server.clone(),
                                )),
                            ));

                            // Channels from the connected server.
                            for channel in connection.channels() {
                                buffers.push(button(
                                    buffer::Upstream::Channel(
                                        server.clone(),
                                        channel.clone(),
                                    ),
                                    true,
                                    history.server_has_unread(server.clone()),
                                    clients.get_server_supports_detach(server),
                                    history.has_unread(
                                        &history::Kind::Channel(
                                            server.clone(),
                                            channel.clone(),
                                        ),
                                    ),
                                    history.has_highlight(
                                        &history::Kind::Channel(
                                            server.clone(),
                                            channel.clone(),
                                        ),
                                    ),
                                ));
                            }

                            // Queries from the connected server.
                            let queries = history.get_unique_queries(server);
                            for query in queries {
                                let query = clients
                                    .resolve_query(server, query)
                                    .unwrap_or(query);

                                buffers.push(button(
                                    buffer::Upstream::Query(
                                        server.clone(),
                                        query.clone(),
                                    ),
                                    true,
                                    history.server_has_unread(server.clone()),
                                    clients.get_server_supports_detach(server),
                                    history.has_unread(&history::Kind::Query(
                                        server.clone(),
                                        query.clone(),
                                    )),
                                    history.has_highlight(
                                        &history::Kind::Query(
                                            server.clone(),
                                            query.clone(),
                                        ),
                                    ),
                                ));
                            }

                            // Separator between servers.
                            if config.sidebar.position.is_horizontal() {
                                if client_enumeration < clients.len() {
                                    buffers.push(
                                        space::horizontal()
                                            .width(
                                                config.sidebar.spacing.server,
                                            )
                                            .into(),
                                    );
                                }
                            } else {
                                buffers.push(
                                    space::vertical()
                                        .height(config.sidebar.spacing.server)
                                        .into(),
                                );
                            }
                        }
                    }
                }
            }

            match config.sidebar.position {
                sidebar::Position::Left | sidebar::Position::Right => {
                    let show_nicklist =
                        config.sidebar.show_nicklist && !self.nicklist_hidden;

                    let direction = scrollable::Direction::Vertical(
                        scrollable::Scrollbar::default()
                            .width(config.sidebar.scrollbar.width)
                            .scroller_width(
                                config.sidebar.scrollbar.scroller_width,
                            ),
                    );

                    let mut nicklist: Option<Element<'a, Message>> = None;
                    let mut buflist_content =
                        Column::with_children(buffers).spacing(0);

                    if show_nicklist
                        && let Some(list) = focused_channel_nicklist(
                            panes, focus, clients, config, theme, width,
                        )
                    {
                        if config.sidebar.split {
                            let scrollable = Scrollable::new(list)
                                .direction(direction)
                                .width(Length::Shrink);

                            nicklist = Some(
                                container(scrollable)
                                    .padding([0, 2])
                                    .height(Length::FillPortion(
                                        config.sidebar.nicklist_space,
                                    ))
                                    .into(),
                            );
                        } else {
                            buflist_content = buflist_content
                                .push(container(list).padding([0, 2]));
                        }
                    }

                    let buflist_height =
                        if config.sidebar.split && nicklist.is_some() {
                            Length::FillPortion(config.sidebar.buflist_space)
                        } else {
                            Length::Fill
                        };

                    let buflist = Scrollable::new(buflist_content)
                        .direction(direction)
                        .height(buflist_height);

                    let content = if show_nicklist && !config.sidebar.split {
                        column![buflist, user_menu_button]
                    } else {
                        column![buflist, nicklist, user_menu_button]
                    };

                    container(content)
                }
                sidebar::Position::Top | sidebar::Position::Bottom => {
                    // Add buffers to a row.
                    let buffers = row![
                        Scrollable::new(Row::with_children(buffers).spacing(2))
                            .direction(scrollable::Direction::Horizontal(
                                scrollable::Scrollbar::default()
                                    .width(config.sidebar.scrollbar.width)
                                    .scroller_width(
                                        config.sidebar.scrollbar.scroller_width
                                    )
                            ))
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

fn focused_channel_nicklist<'a>(
    panes: &'a Panes,
    focus: Focus,
    clients: &'a data::client::Map,
    config: &'a Config,
    theme: &'a Theme,
    width: Length,
) -> Option<Element<'a, Message>> {
    if !matches!(width, Length::Fill) {
        return None;
    }

    let (server, channel) =
        panes.iter().find_map(|(window, pane, state)| {
            if (Focus { window, pane }) != focus {
                return None;
            }
            match state.buffer.upstream() {
                Some(buffer::Upstream::Channel(server, channel)) => {
                    Some((server, channel))
                }
                _ => None,
            }
        })?;

    let users = clients.get_channel_users(server, channel);
    let prefix = clients.get_prefix(server);
    let list = crate::buffer::channel::nick_list::content(
        server, prefix, channel, users, None, config, theme,
    )
    .map(Message::Nicklist);

    Some(list)
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
    Close(window::Id, pane_grid::Pane),
    CloseAllQueries,
    Detach,
    Leave,
    MarkAsRead,
    MarkServerAsRead,
    NewPane,
    Popout,
    Replace,
    Swap(window::Id, pane_grid::Pane),
}

impl Entry {
    fn list(
        buffer: &buffer::Upstream,
        num_panes: usize,
        open: Option<(window::Id, pane_grid::Pane)>,
        focus: Focus,
        supports_detach: bool,
    ) -> Vec<Self> {
        use Entry::*;
        use itertools::Itertools;

        itertools::chain!(
            match buffer {
                buffer::Upstream::Server(_) =>
                    vec![CloseAllQueries, MarkServerAsRead],
                buffer::Upstream::Channel(_, _) => vec![],
                buffer::Upstream::Query(_, _) => vec![],
            },
            match open {
                None => vec![MarkAsRead, NewPane, Popout, Replace]
                    .into_iter()
                    .chain(
                        (matches!(buffer, buffer::Upstream::Channel(_, _))
                            && supports_detach)
                            .then_some(Detach),
                    )
                    .collect_vec(),
                Some((window, pane)) => (num_panes > 1)
                    .then_some(Close(window, pane))
                    .into_iter()
                    .chain(
                        (Focus { window, pane } != focus)
                            .then_some(Swap(window, pane)),
                    )
                    .collect_vec(),
            },
            vec![Leave]
        )
        .sorted()
        .collect_vec()
    }
}

fn upstream_buffer_button<'a>(
    config: &'a Config,
    panes: &'a Panes,
    focus: Focus,
    buffer: buffer::Upstream,
    connected: bool,
    server_has_unread: bool,
    supports_detach: bool,
    casemapping: isupport::CaseMap,
    has_unread: bool,
    has_highlight: bool,
    history: &'a history::Manager,
    width: Length,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let open = panes.iter().find_map(|(window_id, pane, state)| {
        (state.buffer.upstream() == Some(&buffer)).then_some((window_id, pane))
    });
    let is_focused = panes.iter().find_map(|(window_id, pane, state)| {
        (Focus {
            window: window_id,
            pane,
        } == focus
            && state.buffer.upstream() == Some(&buffer))
        .then_some((window_id, pane))
    });

    let should_indicate_unread = buffer.channel().is_none_or(|channel| {
        config.sidebar.unread_indicator.should_indicate_unread(
            channel,
            buffer.server(),
            casemapping,
        )
    });
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

    // check for server icon first (only for server buffers with icon size configured)
    let icon_tuple = if let (
        buffer::Upstream::Server(server),
        data::config::sidebar::ServerIcon::Size(size),
    ) = (&buffer, &config.sidebar.server_icon)
    {
        Some((
            if server.is_bouncer_network() {
                icon::link()
            } else {
                icon::connected()
            }
            .style(if connected {
                if has_highlight {
                    theme::text::highlight_indicator
                } else if has_unread {
                    theme::text::unread_indicator
                } else {
                    theme::text::primary
                }
            } else {
                theme::text::error
            })
            .size(*size),
            *size,
        ))
    }
    // fall through to unread/highlight icons for all buffers (including server)
    else if show_highlight_icon
        && let Some(highlight_icon) =
            icon::from_icon(config.sidebar.unread_indicator.highlight_icon)
    {
        Some((
            highlight_icon
                .style(theme::text::highlight_indicator)
                .size(config.sidebar.unread_indicator.highlight_icon_size),
            config.sidebar.unread_indicator.highlight_icon_size,
        ))
    } else if show_unread_icon
        && let Some(unread_icon) =
            icon::from_icon(config.sidebar.unread_indicator.icon)
    {
        Some((
            unread_icon
                .style(theme::text::unread_indicator)
                .size(config.sidebar.unread_indicator.icon_size),
            config.sidebar.unread_indicator.icon_size,
        ))
    } else {
        None
    };

    let (left_padding, icon) = if config.sidebar.position.is_horizontal() {
        icon_tuple.map_or((0, None), |(icon, icon_size)| {
            (icon_size + 8, Some(container(icon).center_y(Length::Fill)))
        })
    } else {
        let server_icon_size = match config.sidebar.server_icon {
            data::config::sidebar::ServerIcon::Size(size) => size,
            data::config::sidebar::ServerIcon::Hidden => 0,
        };
        let max_icon_size = server_icon_size.max(
            config
                .sidebar
                .unread_indicator
                .has_unread_icon()
                .then_some(config.sidebar.unread_indicator.icon_size)
                .max(
                    config
                        .sidebar
                        .unread_indicator
                        .has_unread_highlight_icon()
                        .then_some(
                            config.sidebar.unread_indicator.highlight_icon_size,
                        ),
                )
                .unwrap_or(0),
        );
        (
            max_icon_size + 8,
            icon_tuple.map(|(icon, _)| {
                container(icon)
                    .center_y(Length::Fill)
                    .center_x(max_icon_size)
            }),
        )
    };

    let content = container(stack![
        container(match &buffer {
            buffer::Upstream::Server(server) => {
                if let Some(network) = &server.network {
                    Element::from(row![
                        text(network.name.to_string())
                            .style(buffer_title_style)
                            .font_maybe(buffer_title_font.clone())
                            .shaping(text::Shaping::Advanced),
                        Space::new().width(6),
                        text(server.name.to_string())
                            .style(theme::text::secondary)
                            .font_maybe(buffer_title_font)
                            .shaping(text::Shaping::Advanced),
                    ])
                } else {
                    text(server.to_string())
                        .style(buffer_title_style)
                        .font_maybe(buffer_title_font)
                        .shaping(text::Shaping::Advanced)
                        .into()
                }
            }
            buffer::Upstream::Channel(_, channel) => text(channel.to_string())
                .style(buffer_title_style)
                .font_maybe(buffer_title_font)
                .shaping(text::Shaping::Advanced)
                .into(),
            buffer::Upstream::Query(_, query) => text(query.to_string())
                .style(buffer_title_style)
                .font_maybe(buffer_title_font)
                .shaping(text::Shaping::Advanced)
                .into(),
        })
        .padding(Padding::default().left(left_padding))
        .align_y(iced::Alignment::Center),
        icon
    ]);

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
                            match config.actions.sidebar.buffer {
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

    let entries =
        Entry::list(&buffer, panes.len(), open, focus, supports_detach);

    if entries.is_empty() || !connected {
        base.into()
    } else {
        context_menu(
            context_menu::MouseButton::default(),
            context_menu::Anchor::Cursor,
            context_menu::ToggleBehavior::KeepOpen,
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
                            buffer::Upstream::Server(_) => "Leave server",
                            buffer::Upstream::Channel(_, _) => "Leave channel",
                            buffer::Upstream::Query(_, _) => "Close query",
                        },
                        Some(Message::Leave(buffer.clone())),
                    ),
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
