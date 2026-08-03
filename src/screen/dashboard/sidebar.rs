use std::time::Duration;
use std::{convert, iter};

use data::buffer::Buffer;
use data::config::server::SidebarVisibility;
use data::config::sidebar::{InternalBuffer, PrimaryIcon};
use data::config::{self, Config, sidebar};
use data::dashboard::{BufferAction, BufferFocusedAction};
use data::{
    Image, Version, buffer, client, file_transfer, history, isupport, server,
    server_icon, target,
};
use iced::Length::Shrink;
use iced::widget::text::{Ellipsis, LineHeight, Shaping, Wrapping};
use iced::widget::{
    Column, Row, Scrollable, Space, button, column, container, pane_grid, row,
    rule, scrollable, space, stack,
};
use iced::{
    Alignment, Border, ContentFit, Length, Padding, Task, keyboard, mouse,
    padding,
};
use itertools::Either;
use tokio::time;

use super::{Focus, Panes, Server};
use crate::widget::text_color_svg::TextColorSvg;
use crate::widget::{
    Element, Text, TextExt, context_menu, double_pass, image, text,
};
use crate::{Theme, font, icon, platform_specific, theme, window};

mod collapse;

const CONFIG_RELOAD_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub enum Message {
    New(data::Buffer),
    Popout(data::Buffer),
    Focus(window::Id, pane_grid::Pane),
    Replace(data::Buffer),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane),
    Detach(buffer::Upstream),
    Leave(buffer::Upstream),
    CloseAllQueries(Server, Vec<target::Query>),
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
    ReloadComplete,
    MarkAsRead(data::Buffer),
    MarkServerAsRead(Server),
    QuitApplication,
    Connect(Server),
    DisableAutoconnect(Server),
    Remove(Server),
    SystemInformation(iced::system::Information),
    ShowMutedBuffers(bool),
    SetServerVisibility(Server, SidebarVisibility),
}

#[derive(Debug, Clone)]
pub enum Event {
    New(data::Buffer),
    Popout(data::Buffer),
    Focus(window::Id, pane_grid::Pane),
    Replace(data::Buffer),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane),
    Detach(buffer::Upstream),
    Leave(buffer::Upstream),
    CloseAllQueries(Server, Vec<target::Query>),
    ToggleCommandBar,
    ToggleThemeEditor,
    OpenReleaseWebsite,
    OpenAbout {
        version: String,
        commit: String,
        system_information: Option<iced::system::Information>,
    },
    OpenDocumentation,
    ConfigReloaded(Result<Config, config::Error>),
    MarkAsRead(data::Buffer),
    MarkServerAsRead(Server),
    QuitApplication,
    Connect(Server),
    DisableAutoconnect(Server),
    Remove(Server),
    ShowMutedBuffers(bool),
}

#[derive(Clone)]
pub struct Sidebar {
    pub hidden: bool,
    collapse: collapse::State,
    reloading_config: bool,
    system_information: Option<iced::system::Information>,
}

impl Sidebar {
    pub fn new(hidden: bool) -> (Self, Task<Message>) {
        (
            Self {
                hidden,
                collapse: collapse::State::default(),
                reloading_config: false,
                system_information: None,
            },
            iced::system::information().map(Message::SystemInformation),
        )
    }

    pub fn visible_buffers(
        &self,
        servers: &server::Map,
        clients: &data::client::Map,
        history: &history::Manager,
        panes: &Panes,
        config: &Config,
        show_muted_buffers: bool,
        include_collapsed_buffers: bool,
    ) -> Vec<Buffer> {
        self.sidebar_buffer_groups(
            servers,
            clients,
            history,
            panes,
            config,
            show_muted_buffers,
            include_collapsed_buffers,
        )
        .into_iter()
        .flat_map(|sidebar_buffer_group| match sidebar_buffer_group {
            SidebarBufferGroup::Upstream {
                visible_buffers, ..
            } => visible_buffers
                .into_iter()
                .map(|buffer_data| Buffer::Upstream(buffer_data.buffer))
                .collect::<Vec<Buffer>>(),
            SidebarBufferGroup::Internal { visible_buffers } => visible_buffers
                .into_iter()
                .map(|buffer_data| Buffer::Internal(buffer_data.buffer))
                .collect::<Vec<Buffer>>(),
        })
        .collect()
    }

    pub fn visible_buffers_with_has_unread(
        &self,
        servers: &server::Map,
        clients: &data::client::Map,
        history: &history::Manager,
        panes: &Panes,
        config: &Config,
        show_muted_buffers: bool,
        include_collapsed_buffers: bool,
    ) -> Vec<(Buffer, bool)> {
        self.sidebar_buffer_groups(
            servers,
            clients,
            history,
            panes,
            config,
            show_muted_buffers,
            include_collapsed_buffers,
        )
        .into_iter()
        .flat_map(|sidebar_buffer_group| match sidebar_buffer_group {
            SidebarBufferGroup::Upstream {
                visible_buffers, ..
            } => visible_buffers
                .into_iter()
                .map(|buffer_data| {
                    (
                        Buffer::Upstream(buffer_data.buffer),
                        buffer_data.has_unread,
                    )
                })
                .collect::<Vec<(Buffer, bool)>>(),
            SidebarBufferGroup::Internal { visible_buffers } => visible_buffers
                .into_iter()
                .map(|buffer_data| {
                    (
                        Buffer::Internal(buffer_data.buffer),
                        buffer_data.has_unread,
                    )
                })
                .collect::<Vec<(Buffer, bool)>>(),
        })
        .collect()
    }

    fn sidebar_buffer_groups(
        &self,
        servers: &server::Map,
        clients: &data::client::Map,
        history: &history::Manager,
        panes: &Panes,
        config: &Config,
        show_muted_buffers: bool,
        include_collapsed_buffers: bool,
    ) -> Vec<SidebarBufferGroup> {
        let upstream_buffer_data =
            |buffer: buffer::Upstream,
             kind: history::Kind,
             muted: bool,
             casemapping: isupport::CaseMap|
             -> Option<UpstreamBufferSidebarData> {
                UpstreamBufferSidebarData::from_upstream_buffer(
                    buffer,
                    kind,
                    muted,
                    casemapping,
                    show_muted_buffers,
                    history,
                    panes,
                    config,
                )
            };

        let upstream_buffer_group = |server: &Server,
                                     state: &client::State|
         -> Option<SidebarBufferGroup> {
            let casemapping = clients.get_server_casemapping_or_default(server);
            let server_config = servers.get(server);
            let server_icon_enabled = server_config
                .as_ref()
                .is_some_and(|config| config.icon.enabled);
            let server_sidebar_visibility = server_config
                .as_ref()
                .map_or_else(SidebarVisibility::default, |config| {
                    config.sidebar_visibility
                });

            let is_collapsed =
                !self.collapse.is_expanded(server, server_sidebar_visibility);

            match state {
                data::client::State::Disconnected {
                    autoconnect,
                    connecting,
                } => {
                    // Hide channels & queries for disconnected servers
                    upstream_buffer_data(
                        buffer::Upstream::Server(server.clone()),
                        history::Kind::Server(server.clone()),
                        false,
                        casemapping,
                    )
                    .map(|buffer_data| {
                        SidebarBufferGroup::Upstream {
                            server: server.clone(),
                            visible_buffers: vec![buffer_data],
                            connection_status: ConnectionStatus::Disconnected {
                                autoconnect: *autoconnect,
                                connecting: *connecting,
                            },
                            has_collapsible_buffers: false,
                            casemapping,
                            server_icon_enabled,
                            server_sidebar_visibility,
                        }
                    })
                }
                data::client::State::Ready(connection) => {
                    // Connected server.
                    upstream_buffer_data(
                        buffer::Upstream::Server(server.clone()),
                        history::Kind::Server(server.clone()),
                        false,
                        casemapping,
                    )
                    .map(|mut buffer_data| {
                        let mut collapsible_buffers = vec![];

                        // Channels from the connected server.
                        for (channel, muted) in connection.channels_with_muted()
                        {
                            if let Some(buffer_data) = upstream_buffer_data(
                                buffer::Upstream::Channel(
                                    server.clone(),
                                    channel.clone(),
                                ),
                                history::Kind::Channel(
                                    server.clone(),
                                    channel.clone(),
                                ),
                                muted,
                                casemapping,
                            ) {
                                collapsible_buffers.push(buffer_data);
                            }
                        }

                        // Queries from the connected server.
                        for query in history.get_unique_queries(server) {
                            let (resolved_query, muted) = connection
                                .resolve_query_with_muted(
                                    query,
                                    show_muted_buffers,
                                );
                            let query = resolved_query.unwrap_or(query);

                            if let Some(buffer_data) = upstream_buffer_data(
                                buffer::Upstream::Query(
                                    server.clone(),
                                    query.clone(),
                                ),
                                history::Kind::Query(
                                    server.clone(),
                                    query.clone(),
                                ),
                                muted,
                                casemapping,
                            ) {
                                collapsible_buffers.push(buffer_data);
                            }
                        }

                        let has_collapsible_buffers =
                            !collapsible_buffers.is_empty();

                        let visible_buffers = if is_collapsed {
                            buffer_data
                                .collapse_indicators(&collapsible_buffers);

                            vec![buffer_data]
                                .into_iter()
                                .chain(collapsible_buffers.into_iter().filter(
                                    |buffer_data| {
                                        buffer_data.is_visible_pane
                                            || include_collapsed_buffers
                                    },
                                ))
                                .collect()
                        } else {
                            vec![buffer_data]
                                .into_iter()
                                .chain(collapsible_buffers)
                                .collect()
                        };

                        SidebarBufferGroup::Upstream {
                            server: server.clone(),
                            visible_buffers,
                            connection_status: ConnectionStatus::Connected {
                                registration_complete: connection
                                    .registration_complete(),
                            },
                            has_collapsible_buffers,
                            casemapping,
                            server_icon_enabled,
                            server_sidebar_visibility,
                        }
                    })
                }
            }
        };

        let upstream_buffers: Vec<SidebarBufferGroup> = servers
            .keys()
            .filter_map(|server| {
                clients
                    .state(server)
                    .and_then(|state| upstream_buffer_group(server, state))
            })
            .collect();

        let internal_buffer_data =
            |buffer: &InternalBuffer| -> Option<InternalBufferSidebarData> {
                let muted =
                    config.sidebar.internal_buffers.mute.contains(buffer);

                InternalBufferSidebarData::from_internal_buffer(
                    buffer.into(),
                    muted,
                    show_muted_buffers,
                    history,
                    panes,
                    config,
                )
            };

        let internal_buffers: Vec<InternalBufferSidebarData> = config
            .sidebar
            .internal_buffers
            .buffers
            .iter()
            .filter_map(internal_buffer_data)
            .collect();

        let internal_buffer_group = if internal_buffers.is_empty() {
            None
        } else {
            Some(SidebarBufferGroup::Internal {
                visible_buffers: internal_buffers,
            })
        };

        if config.sidebar.internal_buffers.is_before_servers() {
            internal_buffer_group
                .into_iter()
                .chain(upstream_buffers)
                .collect()
        } else {
            upstream_buffers
                .into_iter()
                .chain(internal_buffer_group)
                .collect()
        }
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
            Message::Connect(server) => {
                (Task::none(), Some(Event::Connect(server)))
            }
            Message::DisableAutoconnect(server) => {
                (Task::none(), Some(Event::DisableAutoconnect(server)))
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
            Message::ShowMutedBuffers(show_muted_buffers) => (
                Task::none(),
                Some(Event::ShowMutedBuffers(show_muted_buffers)),
            ),
            Message::SetServerVisibility(server, visibility) => {
                self.collapse.set(server, visibility);
                (context_menu::close(convert::identity).discard(), None)
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
        show_muted_buffers: bool,
    ) -> Element<'a, Message> {
        let keyboard = &config.keyboard;

        let dimensions = Dimensions::from(&config::sidebar::Sidebar::default());

        let logs_has_unread = history.has_unread(&history::Kind::Logs);

        // Show notification dot if theres a new version, if there're transfers,
        // or if the logs have unread messages.
        let show_notification_dot = version.is_old()
            || (!file_transfers.is_empty()
                && config.file_transfer.enabled
                && !config
                    .sidebar
                    .internal_buffers
                    .buffers
                    .contains(&InternalBuffer::FileTransfers))
            || (logs_has_unread
                && !config
                    .sidebar
                    .internal_buffers
                    .buffers
                    .contains(&InternalBuffer::Logs));
        let system_information = self.system_information.clone();

        let icon = icon::menu();

        let badge = if show_notification_dot {
            Some((
                icon::circle().style(theme::text::tertiary),
                dimensions.unread_indicator_size,
            ))
        } else {
            None
        };

        let base = button(
            sidebar_icon(
                Some(Icon::Internal(icon)),
                badge,
                dimensions,
                config.sidebar.position.is_horizontal(),
            )
            .into_iter()
            .next(),
        )
        .padding(4)
        .width(Length::Shrink);

        let menu = Menu::list(
            version.is_old(),
            config.file_transfer.enabled,
            &config.sidebar.internal_buffers.buffers,
            show_muted_buffers,
        );

        if menu.is_empty() {
            base.into()
        } else {
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
                         icon: TextColorSvg<'a, Theme>,
                         message: Message| {
                            let title = title
                                .line_height(theme::line_height(&config.font));
                            let keybind =
                                keybinds.and_then(|key_binds| match key_binds
                                    .primary()
                                {
                                    Some(
                                        kb @ data::shortcut::KeyBind::Bind {
                                            ..
                                        },
                                    ) => Some(
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
                            icon::quit(),
                            Message::QuitApplication,
                        ),
                        Menu::ShowMutedBuffers(show_muted_buffers) => {
                            context_button(
                                text(if show_muted_buffers {
                                    "Show muted buffers"
                                } else {
                                    "Hide muted buffers"
                                }),
                                Some(if show_muted_buffers {
                                    &keyboard.show_muted_buffers
                                } else {
                                    &keyboard.hide_muted_buffers
                                }),
                                if show_muted_buffers {
                                    icon::show()
                                } else {
                                    icon::hide()
                                },
                                Message::ShowMutedBuffers(show_muted_buffers),
                            )
                        }
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
                            Message::Replace(
                                buffer::Internal::FileTransfers.into(),
                            ),
                        ),
                        Menu::Highlights => context_button(
                            text("Highlights"),
                            Some(&keyboard.highlights),
                            icon::highlights(),
                            Message::Replace(
                                buffer::Internal::Highlights.into(),
                            ),
                        ),
                        Menu::ChannelDiscovery => context_button(
                            text("Channel Discovery"),
                            None,
                            icon::channel_discovery(),
                            Message::Replace(
                                buffer::Internal::ChannelDiscovery(None).into(),
                            ),
                        ),
                        Menu::ChannelMonitor => context_button(
                            text("Channel Monitor"),
                            None,
                            icon::channel_monitor(),
                            Message::Replace(
                                buffer::Internal::ChannelMonitor.into(),
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
                            Message::Replace(buffer::Internal::Logs.into()),
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
                            _ => Space::new().width(length).height(1).into(),
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
                        Menu::Version => context_button(
                            text("About Halloy"),
                            None,
                            icon::about(),
                            Message::OpenAbout {
                                version: version.current.clone(),
                                commit: data::environment::GIT_HASH
                                    .map(str::trim)
                                    .filter(|hash| !hash.is_empty())
                                    .unwrap_or("Unknown")
                                    .to_string(),
                                system_information: system_information.clone(),
                            },
                        ),
                        Menu::Documentation => context_button(
                            text("Documentation"),
                            None,
                            icon::documentation(),
                            Message::OpenDocumentation,
                        ),
                        Menu::ConfigEditor => context_button(
                            text("Config Editor"),
                            Some(&keyboard.open_config_editor),
                            icon::config(),
                            Message::Replace(
                                buffer::Internal::ConfigEditor.into(),
                            ),
                        ),
                    }
                },
            )
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
        show_muted_buffers: bool,
        modifiers: keyboard::Modifiers,
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
                        show_muted_buffers,
                    )
                });

            let sidebar_buffer_groups = self.sidebar_buffer_groups(
                servers,
                clients,
                history,
                panes,
                config,
                show_muted_buffers,
                false,
            );

            let mut buffers = vec![];

            if config.sidebar.position.is_horizontal() {
                buffers.push(space::horizontal().width(4).into());
            }

            for (index, sidebar_buffer_group) in
                sidebar_buffer_groups.into_iter().enumerate()
            {
                // Separator between servers and between servers and
                // internal buffers
                if index > 0 {
                    if config.sidebar.position.is_horizontal() {
                        buffers.push(
                            space::horizontal()
                                .width(config.sidebar.spacing.server)
                                .into(),
                        );
                    } else {
                        buffers.push(
                            space::vertical()
                                .height(config.sidebar.spacing.server)
                                .into(),
                        );
                    }
                }

                match sidebar_buffer_group {
                    SidebarBufferGroup::Upstream {
                        server,
                        visible_buffers,
                        connection_status,
                        has_collapsible_buffers,
                        casemapping,
                        server_icon_enabled,
                        server_sidebar_visibility,
                    } => {
                        let server_has_unread =
                            history.server_has_unread(&server);
                        let supports_detach =
                            clients.get_server_supports_detach(&server);

                        for buffer_data in visible_buffers {
                            let context = UpstreamButtonContext {
                                config,
                                panes,
                                focus,
                                server_icons,
                                buffer: buffer_data.buffer,
                                kind: buffer_data.kind,
                                indicators: buffer_data.indicators,
                                connection_status,
                                server_has_collapsible_buffers:
                                    has_collapsible_buffers,
                                server_has_unread,
                                supports_detach,
                                casemapping,
                                server_icon_enabled,
                                server_sidebar_visibility,
                                history,
                                width,
                                theme,
                                collapse: &self.collapse,
                                modifiers,
                            };

                            buffers.push(upstream_buffer_button(context));
                        }
                    }

                    SidebarBufferGroup::Internal { visible_buffers } => {
                        for buffer_data in visible_buffers {
                            buffers.push(internal_buffer_button(
                                config,
                                panes,
                                focus,
                                buffer_data.buffer,
                                buffer_data.kind,
                                buffer_data.indicators,
                                history,
                                width,
                                theme,
                                modifiers,
                            ));
                        }
                    }
                }
            }

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
                .width(Shrink.max(
                    config.sidebar.max_width.map_or(f32::INFINITY, f32::from),
                ))
                .padding(padding)
        };

        Some(content.into())
    }
}

enum SidebarBufferGroup {
    Upstream {
        server: Server,
        visible_buffers: Vec<UpstreamBufferSidebarData>,
        connection_status: ConnectionStatus,
        casemapping: isupport::CaseMap,
        has_collapsible_buffers: bool,
        server_icon_enabled: bool,
        server_sidebar_visibility: SidebarVisibility,
    },
    Internal {
        visible_buffers: Vec<InternalBufferSidebarData>,
    },
}

struct UpstreamBufferSidebarData {
    buffer: buffer::Upstream,
    kind: history::Kind,
    is_visible_pane: bool,
    has_unread: bool,
    #[expect(dead_code)] // TODO: Cycle highlights
    has_highlight: bool,
    indicators: IndicatorState,
}

impl UpstreamBufferSidebarData {
    fn from_upstream_buffer(
        buffer: buffer::Upstream,
        kind: history::Kind,
        muted: bool,
        casemapping: isupport::CaseMap,
        show_muted_buffers: bool,
        history: &history::Manager,
        panes: &Panes,
        config: &Config,
    ) -> Option<Self> {
        let is_visible_pane = panes
            .iter_visible()
            .any(|(_, _, state)| state.buffer.upstream() == Some(&buffer));

        let has_unread = history.has_unread(&kind);

        let is_unread_query =
            matches!(buffer, buffer::Upstream::Query(_, _)) && has_unread;

        let has_highlight = history.has_highlight(&kind);

        let indicators = IndicatorState {
            unread: has_unread
                && !(is_unread_query
                    && config.sidebar.unread_indicator.query_as_highlight)
                && (config.sidebar.unread_indicator.show_on_open_buffers
                    || !is_visible_pane)
                && config.sidebar.unread_indicator.should_indicate(
                    buffer.target().as_ref(),
                    buffer.server(),
                    casemapping,
                ),
            highlight: (has_highlight
                || (is_unread_query
                    && config.sidebar.unread_indicator.query_as_highlight))
                && (config.sidebar.highlight_indicator.show_on_open_buffers
                    || !is_visible_pane)
                && config.sidebar.highlight_indicator.should_indicate(
                    buffer.target().as_ref(),
                    buffer.server(),
                    casemapping,
                ),
        };

        if muted
            && !is_visible_pane
            && !indicators.unread
            && !indicators.highlight
            && !show_muted_buffers
        {
            return None;
        }

        Some(Self {
            buffer,
            kind,
            is_visible_pane,
            has_unread,
            has_highlight,
            indicators,
        })
    }

    fn collapse_indicators(
        &mut self,
        collapsed_buffers: &[UpstreamBufferSidebarData],
    ) {
        for collapsed_buffer_data in collapsed_buffers {
            self.indicators.merge(collapsed_buffer_data.indicators);
        }
    }
}

struct InternalBufferSidebarData {
    buffer: buffer::Internal,
    kind: Option<history::Kind>,
    has_unread: bool,
    #[expect(dead_code)] // TODO: Cycle highlights
    has_highlight: bool,
    indicators: IndicatorState,
}

impl InternalBufferSidebarData {
    fn from_internal_buffer(
        buffer: buffer::Internal,
        muted: bool,
        show_muted_buffers: bool,
        history: &history::Manager,
        panes: &Panes,
        config: &Config,
    ) -> Option<Self> {
        let kind =
            history::Kind::from_buffer(data::Buffer::Internal(buffer.clone()));

        let is_visible_pane = panes.iter_visible().any(|(_, _, state)| {
            state.buffer.internal().as_ref() == Some(&buffer)
        });

        let has_unread =
            kind.as_ref().is_some_and(|kind| history.has_unread(kind));

        let has_highlight = kind
            .as_ref()
            .is_some_and(|kind| history.has_highlight(kind));

        let indicators = IndicatorState {
            unread: has_unread
                && (config.sidebar.unread_indicator.show_on_open_buffers
                    || !is_visible_pane),
            highlight: has_highlight
                && (config.sidebar.highlight_indicator.show_on_open_buffers
                    || !is_visible_pane),
        };

        if muted
            && !is_visible_pane
            && !indicators.unread
            && !indicators.highlight
            && !show_muted_buffers
        {
            return None;
        }

        Some(Self {
            buffer,
            kind,
            has_unread,
            has_highlight,
            indicators,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum Menu {
    RefreshConfig,
    ConfigEditor,
    CommandBar,
    ThemeEditor,
    Highlights,
    ChannelDiscovery,
    ChannelMonitor,
    Logs,
    FileTransfers,
    Version,
    Update,
    HorizontalRule,
    Documentation,
    QuitApplication,
    ShowMutedBuffers(bool),
}

impl Menu {
    fn list(
        has_new_version: bool,
        file_transfer_enabled: bool,
        internal_buffers_in_sidebar: &[InternalBuffer],
        show_muted_buffers: bool,
    ) -> Vec<Self> {
        let mut list = vec![Self::Version];

        if has_new_version {
            list.push(Self::Update);
        }

        list.extend([
            Self::HorizontalRule,
            Self::CommandBar,
            Self::Documentation,
        ]);

        if file_transfer_enabled
            && !internal_buffers_in_sidebar
                .contains(&InternalBuffer::FileTransfers)
        {
            list.push(Self::FileTransfers);
        }

        if !internal_buffers_in_sidebar
            .contains(&InternalBuffer::ChannelDiscovery)
        {
            list.push(Self::ChannelDiscovery);
        }

        if !internal_buffers_in_sidebar
            .contains(&InternalBuffer::ChannelMonitor)
        {
            list.push(Self::ChannelMonitor);
        }

        if !internal_buffers_in_sidebar.contains(&InternalBuffer::Highlights) {
            list.push(Self::Highlights);
        }

        if !internal_buffers_in_sidebar.contains(&InternalBuffer::Logs) {
            list.push(Self::Logs);
        }

        list.extend([
            Self::ConfigEditor,
            Self::RefreshConfig,
            Self::ThemeEditor,
            Self::ShowMutedBuffers(!show_muted_buffers),
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
    DisableAutoconnect,
    MarkAsRead,
    MarkServerAsRead,
    Close(window::Id, pane_grid::Pane),
    CloseAllQueries,
    NewPane,
    Popout,
    Replace,
    Swap(window::Id, pane_grid::Pane),
    Detach,
    Leave,
    Remove,
    ToggleCollapse,
}

impl Entry {
    fn list(
        buffer: &buffer::Buffer,
        num_panes: usize,
        open_as_window_pane: Option<(window::Id, pane_grid::Pane)>,
        focus: Focus,
        connection_status: Option<ConnectionStatus>,
        supports_detach: bool,
        has_history: bool,
    ) -> Vec<Self> {
        use Entry::*;

        let mut entries = vec![Context, HorizontalRule];

        if let buffer::Buffer::Upstream(buffer::Upstream::Server(_)) = buffer
            && let Some(connection_status) = &connection_status
        {
            match connection_status {
                ConnectionStatus::Connected { .. } => {
                    entries.extend([CloseAllQueries, MarkServerAsRead]);
                }
                ConnectionStatus::Disconnected {
                    autoconnect,
                    connecting,
                } => {
                    if !*connecting {
                        entries.push(Connect);
                    }
                    if *autoconnect {
                        entries.push(DisableAutoconnect);
                    }
                    entries.push(Remove);
                }
            }
        }

        if has_history {
            entries.push(MarkAsRead);
        }

        match open_as_window_pane {
            None => {
                entries.extend([NewPane, Popout, Replace]);
            }
            Some((window, pane)) => {
                if num_panes > 1 {
                    entries.push(Close(window, pane));
                }
                if (Focus { window, pane }) != focus {
                    entries.push(Swap(window, pane));
                }
            }
        }

        let connected = connection_status.is_some_and(|connection_status| {
            matches!(connection_status, ConnectionStatus::Connected { .. })
        });

        if connected {
            if matches!(
                buffer,
                buffer::Buffer::Upstream(buffer::Upstream::Channel(_, _))
            ) && supports_detach
            {
                entries.push(Detach);
            }
            entries.push(Leave);
        }

        // TODO: Use sort or insert order to arrange context menu
        // entries, not both
        entries.sort();

        if let buffer::Buffer::Upstream(buffer::Upstream::Server(_)) = buffer
            && connected
        {
            entries.extend([HorizontalRule, ToggleCollapse]);
        }
        entries
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct IndicatorState {
    unread: bool,
    highlight: bool,
}

impl IndicatorState {
    fn merge(&mut self, other: Self) {
        self.unread |= other.unread;
        self.highlight |= other.highlight;
    }
}

#[derive(Clone)]
struct UpstreamButtonContext<'a> {
    config: &'a Config,
    panes: &'a Panes,
    focus: Focus,
    server_icons: &'a server_icon::Manager,
    buffer: buffer::Upstream,
    kind: history::Kind,
    indicators: IndicatorState,
    connection_status: ConnectionStatus,
    server_has_collapsible_buffers: bool,
    server_has_unread: bool,
    supports_detach: bool,
    casemapping: isupport::CaseMap,
    server_icon_enabled: bool,
    server_sidebar_visibility: SidebarVisibility,
    history: &'a history::Manager,
    width: Length,
    theme: &'a Theme,
    collapse: &'a collapse::State,
    modifiers: keyboard::Modifiers,
}

fn upstream_buffer_title<'a>(
    config: &Config,
    buffer: &buffer::Upstream,
    casemapping: isupport::CaseMap,
    title_style: fn(&Theme) -> iced::widget::text::Style,
    title_font: Option<font::Font>,
) -> Vec<Element<'a, Message>> {
    match buffer {
        buffer::Upstream::Server(server) => {
            let font_size = config
                .sidebar
                .primary_font_size
                .or(config.sidebar.secondary_font_size)
                .or(config.font.size)
                .map_or(theme::TEXT_SIZE, f32::from);

            if let Some(network) = &server.network {
                vec![
                    text(network.name.to_string())
                        .line_height(LineHeight::Relative(1.0))
                        .size(font_size)
                        .style(title_style)
                        .font_maybe(title_font.clone())
                        .shaping(Shaping::Advanced)
                        .wrapping(Wrapping::None)
                        .ellipsis(Ellipsis::End)
                        .into(),
                    Space::new().width(6).into(),
                    text(server.name.to_string())
                        .line_height(LineHeight::Relative(1.0))
                        .size(font_size)
                        .style(theme::text::secondary)
                        .font_maybe(title_font)
                        .shaping(Shaping::Advanced)
                        .wrapping(Wrapping::None)
                        .ellipsis(Ellipsis::End)
                        .into(),
                ]
            } else {
                vec![
                    text(server.to_string())
                        .line_height(LineHeight::Relative(1.0))
                        .size(font_size)
                        .style(title_style)
                        .font_maybe(title_font)
                        .shaping(Shaping::Advanced)
                        .wrapping(Wrapping::None)
                        .ellipsis(Ellipsis::End)
                        .into(),
                ]
            }
        }
        buffer::Upstream::Channel(_, channel) => {
            let raw_channel = channel.as_str();
            let display_channel =
                config.sidebar.channel_name_casing.map_or_else(
                    || raw_channel.to_owned(),
                    |casing| casing.apply(raw_channel, casemapping),
                );

            vec![
                text(display_channel)
                    .line_height(LineHeight::Relative(1.0))
                    .size_maybe(
                        config
                            .sidebar
                            .secondary_font_size
                            .or(config.font.size)
                            .map(f32::from),
                    )
                    .style(title_style)
                    .font_maybe(title_font)
                    .shaping(Shaping::Advanced)
                    .wrapping(Wrapping::None)
                    .ellipsis(Ellipsis::End)
                    .into(),
            ]
        }
        buffer::Upstream::Query(_, query) => {
            vec![
                text(query.to_string())
                    .line_height(LineHeight::Relative(1.0))
                    .size_maybe(
                        config
                            .sidebar
                            .secondary_font_size
                            .or(config.font.size)
                            .map(f32::from),
                    )
                    .style(title_style)
                    .font_maybe(title_font)
                    .shaping(Shaping::Advanced)
                    .wrapping(Wrapping::None)
                    .ellipsis(Ellipsis::End)
                    .into(),
            ]
        }
    }
}

fn upstream_buffer_button<'a>(
    context: UpstreamButtonContext<'a>,
) -> Element<'a, Message> {
    let UpstreamButtonContext {
        config,
        panes,
        focus,
        server_icons,
        buffer,
        kind,
        indicators,
        connection_status,
        server_has_collapsible_buffers,
        casemapping,
        server_icon_enabled,
        server_sidebar_visibility,
        history,
        width,
        theme,
        collapse,
        modifiers,
        ..
    } = &context;

    let open_as_window_pane =
        panes.iter().find_map(|(window_id, pane, state)| {
            (state.buffer.upstream() == Some(buffer))
                .then_some((window_id, pane))
        });

    let focused_as_window_pane =
        panes.iter().find_map(|(window_id, pane, state)| {
            (Focus {
                window: window_id,
                pane,
            } == *focus
                && state.buffer.upstream() == Some(buffer))
            .then_some((window_id, pane))
        });

    let can_mark_as_read = history.can_mark_as_read(kind);

    let show_unread_icon =
        indicators.unread && config.sidebar.unread_indicator.has_icon();
    let show_unread_title =
        indicators.unread && config.sidebar.unread_indicator.title;

    let show_highlight_icon =
        indicators.highlight && config.sidebar.highlight_indicator.has_icon();
    let show_highlight_title =
        indicators.highlight && config.sidebar.highlight_indicator.title;

    let buffer_title_style = if show_highlight_title {
        theme::text::highlight_indicator
    } else if show_unread_title {
        theme::text::unread_indicator
    } else if let ConnectionStatus::Disconnected { connecting, .. } =
        &connection_status
        && !*connecting
    {
        if matches!(&buffer, buffer::Upstream::Server(_)) {
            theme::text::error
        } else {
            theme::text::secondary
        }
    } else {
        theme::text::primary
    };

    let buffer_title_font = theme::font_style::primary(theme).map(font::get);

    let dimensions = Dimensions::from(&config.sidebar);

    let icon = if dimensions.icon_size > 0
        && let buffer::Upstream::Server(server) = buffer
    {
        if *server_icon_enabled
            && let Some(server_icon) = server_icons.get(server)
        {
            Some(Icon::Upstream(server_icon))
        } else {
            Some(Icon::Internal(if server.is_bouncer_network() {
                icon::link()
            } else {
                icon::connected()
            }))
        }
    } else {
        None
    };

    let indicator = if matches!(buffer, buffer::Upstream::Server(_))
        && let ConnectionStatus::Disconnected {
            autoconnect,
            connecting,
        } = connection_status
    {
        Some((
            if *connecting {
                icon::connecting().style(theme::text::success)
            } else if *autoconnect {
                icon::disconnected().style(theme::text::error)
            } else {
                icon::not_connected().style(theme::text::error)
            },
            dimensions.icon_badge_size,
        ))
    } else if matches!(buffer, buffer::Upstream::Server(_))
        && let ConnectionStatus::Connected {
            registration_complete,
        } = connection_status
        && !registration_complete
    {
        Some((
            icon::connecting().style(theme::text::success),
            dimensions.icon_badge_size,
        ))
    } else if show_highlight_icon
        && let Some(highlight_icon) =
            icon::from_icon(config.sidebar.highlight_indicator.icon)
    {
        Some((
            highlight_icon.style(theme::text::highlight_indicator),
            dimensions.highlight_indicator_size,
        ))
    } else if show_unread_icon
        && let Some(unread_icon) =
            icon::from_icon(config.sidebar.unread_indicator.icon)
    {
        Some((
            unread_icon.style(theme::text::unread_indicator),
            dimensions.unread_indicator_size,
        ))
    } else {
        None
    };

    let sidebar_icon_height = if icon.is_some() {
        dimensions
            .icon_size
            .max(indicator.as_ref().map_or(0, |_| dimensions.icon_badge_size))
    } else {
        indicator.as_ref().map_or(1, |(_, size)| *size)
    };

    let mut content = row![].align_y(iced::Alignment::Center);

    content = content.extend(sidebar_icon(
        icon,
        indicator,
        dimensions,
        config.sidebar.position.is_horizontal(),
    ));

    content = content.extend(upstream_buffer_title(
        config,
        buffer,
        *casemapping,
        buffer_title_style,
        buffer_title_font,
    ));

    let disclosure = if let buffer::Upstream::Server(server) = buffer {
        let font_size = config
            .sidebar
            .primary_font_size
            .or(config.sidebar.secondary_font_size)
            .or(config.font.size)
            .map_or(theme::TEXT_SIZE, f32::from);

        collapse.disclosure(
            config,
            server,
            *server_sidebar_visibility,
            connection_status,
            *server_has_collapsible_buffers,
            font_size.max(sidebar_icon_height as f32),
        )
    } else {
        None
    };

    let content_width =
        if disclosure.is_some() && !config.sidebar.position.is_horizontal() {
            Length::Fill
        } else {
            *width
        };

    let button_size = disclosure.as_ref().map(|disclosure| disclosure.size);
    let mut base = button(
        content
            .width(content_width)
            .padding(Padding::default().bottom(1)),
    )
    .style(move |theme, status| {
        theme::button::sidebar_buffer(
            theme,
            status,
            focused_as_window_pane.is_some(),
            open_as_window_pane.is_some(),
        )
    })
    .padding(config.sidebar.padding.buffer)
    .on_press({
        if modifiers.command() {
            if let Some((window, pane)) = open_as_window_pane {
                Message::Focus(window, pane)
            } else {
                Message::Popout(buffer.clone().into())
            }
        } else {
            match focused_as_window_pane {
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
                    if let Some((window, pane)) = open_as_window_pane {
                        Message::Focus(window, pane)
                    } else {
                        let action = match &buffer {
                            buffer::Upstream::Channel(_, _) => config
                                .actions
                                .sidebar
                                .channel
                                .unwrap_or(config.actions.sidebar.buffer),
                            buffer::Upstream::Query(_, _) => config
                                .actions
                                .sidebar
                                .query
                                .unwrap_or(config.actions.sidebar.buffer),
                            _ => config.actions.sidebar.buffer,
                        };

                        match action {
                            BufferAction::NewPane => {
                                Message::New(buffer.clone().into())
                            }
                            BufferAction::ReplacePane => {
                                Message::Replace(buffer.clone().into())
                            }
                            BufferAction::NewWindow => {
                                Message::Popout(buffer.clone().into())
                            }
                        }
                    }
                }
            }
        }
    });

    if let Some(button_size) = button_size {
        base = base.height(button_size);
    }

    let base: Element<'a, Message> = if let Some(disclosure) = disclosure {
        let button_size = disclosure.size;
        let message = Message::SetServerVisibility(
            buffer.server().clone(),
            disclosure.next_visibility,
        );
        let disclosure_button = button(
            container(disclosure.indicator())
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(button_size)
        .height(button_size)
        .style(|theme, status| {
            theme::button::sidebar_buffer(theme, status, false, false)
        })
        .padding(0)
        .on_press(message);

        row![base, disclosure_button]
            .width(*width)
            .align_y(iced::Alignment::Center)
            .into()
    } else {
        base.into()
    };

    upstream_buffer_context_menu(
        context,
        base,
        open_as_window_pane,
        can_mark_as_read,
    )
}

fn upstream_buffer_context_menu<'a>(
    context: UpstreamButtonContext<'a>,
    base: Element<'a, Message>,
    open_as_window_pane: Option<(window::Id, pane_grid::Pane)>,
    can_mark_as_read: bool,
) -> Element<'a, Message> {
    let UpstreamButtonContext {
        config,
        panes,
        focus,
        buffer,
        connection_status,
        server_has_unread,
        supports_detach,
        history,
        theme,
        collapse,
        server_sidebar_visibility,
        ..
    } = context;

    let entries = Entry::list(
        &buffer.clone().into(),
        panes.len(),
        open_as_window_pane,
        focus,
        Some(connection_status),
        supports_detach,
        true,
    );

    if entries.is_empty() {
        return base;
    }

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
                Entry::MarkAsRead => (
                    if matches!(&buffer, buffer::Upstream::Server(_)) {
                        "Mark server buffer as read"
                    } else {
                        "Mark as read"
                    },
                    can_mark_as_read
                        .then(|| Message::MarkAsRead(buffer.clone().into())),
                ),
                Entry::MarkServerAsRead => (
                    "Mark entire server as read",
                    server_has_unread.then(|| {
                        Message::MarkServerAsRead(buffer.server().clone())
                    }),
                ),
                Entry::NewPane => (
                    "Open in new pane",
                    Some(Message::New(buffer.clone().into())),
                ),
                Entry::Popout => (
                    "Open in new window",
                    Some(Message::Popout(buffer.clone().into())),
                ),
                Entry::Replace => (
                    "Replace current pane",
                    Some(Message::Replace(buffer.clone().into())),
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
                        buffer::Upstream::Server(_) => "Disconnect from server",
                        buffer::Upstream::Channel(_, _) => "Leave channel",
                        buffer::Upstream::Query(_, _) => "Close query",
                    },
                    Some(Message::Leave(buffer.clone())),
                ),
                Entry::Connect => (
                    "Connect to server",
                    Some(Message::Connect(buffer.server().clone())),
                ),
                Entry::DisableAutoconnect => (
                    "Disable autoconnect",
                    Some(Message::DisableAutoconnect(buffer.server().clone())),
                ),
                Entry::Remove => (
                    "Remove server from sidebar",
                    Some(Message::Remove(buffer.server().clone())),
                ),
                Entry::Context => {
                    return container(
                        row![
                            text(match &buffer {
                                buffer::Upstream::Server(server) =>
                                    server.network.as_ref().map_or_else(
                                        || format!("{server}"),
                                        |network| network.name.to_string(),
                                    ),
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
                                buffer::Upstream::Server(server) => server
                                    .network
                                    .is_some()
                                    .then(|| server.name.to_string()),
                                buffer::Upstream::Channel(server, _)
                                | buffer::Upstream::Query(server, _) => {
                                    Some(format!("{server}"))
                                }
                            }
                            .map(
                                |secondary_name| text(secondary_name)
                                    .style(theme::text::secondary)
                                    .font_maybe(
                                        theme::font_style::secondary(theme)
                                            .map(font::get)
                                    )
                            ),
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
                Entry::ToggleCollapse => {
                    let server = buffer.server();
                    let is_expanded =
                        collapse.is_expanded(server, server_sidebar_visibility);
                    (
                        if is_expanded {
                            "Collapse server"
                        } else {
                            "Expand server"
                        },
                        Some(Message::SetServerVisibility(
                            server.clone(),
                            if is_expanded {
                                SidebarVisibility::Collapsed
                            } else {
                                SidebarVisibility::Expanded
                            },
                        )),
                    )
                }
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

fn internal_buffer_button<'a>(
    config: &'a Config,
    panes: &'a Panes,
    focus: Focus,
    buffer: buffer::Internal,
    kind: Option<history::Kind>,
    indicators: IndicatorState,
    history: &'a history::Manager,
    width: Length,
    theme: &'a Theme,
    modifiers: keyboard::Modifiers,
) -> Element<'a, Message> {
    let open_as_window_pane =
        panes.iter().find_map(|(window_id, pane, state)| {
            (state.buffer.internal().as_ref() == Some(&buffer))
                .then_some((window_id, pane))
        });

    let focused_as_window_pane =
        panes.iter().find_map(|(window_id, pane, state)| {
            (Focus {
                window: window_id,
                pane,
            } == focus
                && state.buffer.internal().as_ref() == Some(&buffer))
            .then_some((window_id, pane))
        });

    let can_mark_as_read = kind
        .as_ref()
        .is_some_and(|kind| history.can_mark_as_read(kind));

    let dimensions = Dimensions::from(&config.sidebar);

    let show_icon = dimensions.icon_size > 0;

    let (icon, badge) = match buffer {
        buffer::Internal::ChannelDiscovery(_) => {
            (show_icon.then_some(icon::channel_discovery()), None)
        }
        buffer::Internal::ConfigEditor => {
            (show_icon.then_some(icon::config()), None)
        }
        buffer::Internal::FileTransfers => {
            (show_icon.then_some(icon::file_transfer()), None)
        }
        buffer::Internal::Highlights => {
            let badge = if indicators.highlight
                && let Some(highlight_icon) =
                    icon::from_icon(config.sidebar.highlight_indicator.icon)
            {
                Some((
                    highlight_icon.style(theme::text::highlight_indicator),
                    dimensions.highlight_indicator_size,
                ))
            } else {
                None
            };

            (show_icon.then_some(icon::highlights()), badge)
        }
        buffer::Internal::Logs => {
            let badge = if indicators.unread {
                Some((
                    icon::log_indicator().style(if indicators.highlight {
                        theme::text::error
                    } else {
                        theme::text::warning
                    }),
                    dimensions.unread_indicator_size,
                ))
            } else {
                None
            };

            (show_icon.then_some(icon::logs()), badge)
        }
        buffer::Internal::ChannelMonitor => {
            let badge = if indicators.highlight
                && let Some(highlight_icon) =
                    icon::from_icon(config.sidebar.highlight_indicator.icon)
            {
                Some((
                    highlight_icon.style(theme::text::highlight_indicator),
                    dimensions.highlight_indicator_size,
                ))
            } else if indicators.unread
                && let Some(unread_icon) =
                    icon::from_icon(config.sidebar.unread_indicator.icon)
            {
                Some((
                    unread_icon.style(theme::text::unread_indicator),
                    dimensions.unread_indicator_size,
                ))
            } else {
                None
            };

            (show_icon.then_some(icon::channel_monitor()), badge)
        }
    };

    let title: &'static str = (&buffer).into();

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
                config
                    .sidebar
                    .primary_font_size
                    .or(config.sidebar.secondary_font_size)
                    .or(config.font.size)
                    .map(f32::from),
            )
            .style(theme::text::primary)
            .font_maybe(theme::font_style::primary(theme).map(font::get))
            .shaping(Shaping::Advanced)
            .wrapping(Wrapping::None)
            .ellipsis(Ellipsis::End),
    );

    let base =
        button(content.width(width).padding(Padding::default().bottom(1)))
            .style(move |theme, status| {
                theme::button::sidebar_buffer(
                    theme,
                    status,
                    focused_as_window_pane.is_some(),
                    open_as_window_pane.is_some(),
                )
            })
            .padding(config.sidebar.padding.buffer)
            .on_press(if modifiers.command() {
                if let Some((window, pane)) = open_as_window_pane {
                    Message::Focus(window, pane)
                } else {
                    Message::Popout(buffer.clone().into())
                }
            } else {
                match focused_as_window_pane {
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
                        if let Some((window, pane)) = open_as_window_pane {
                            Message::Focus(window, pane)
                        } else {
                            match config.actions.sidebar.buffer {
                                BufferAction::NewPane => {
                                    Message::New(buffer.clone().into())
                                }
                                BufferAction::ReplacePane => {
                                    Message::Replace(buffer.clone().into())
                                }
                                BufferAction::NewWindow => {
                                    Message::Popout(buffer.clone().into())
                                }
                            }
                        }
                    }
                }
            });

    let entries = Entry::list(
        &buffer.clone().into(),
        panes.len(),
        open_as_window_pane,
        focus,
        None,
        false,
        kind.is_some(),
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
                    Entry::MarkAsRead => (
                        "Mark as read",
                        if can_mark_as_read {
                            Some(Message::MarkAsRead(buffer.clone().into()))
                        } else {
                            None
                        },
                    ),
                    Entry::NewPane => (
                        "Open in new pane",
                        Some(Message::New(buffer.clone().into())),
                    ),
                    Entry::Popout => (
                        "Open in new window",
                        Some(Message::Popout(buffer.clone().into())),
                    ),
                    Entry::Replace => (
                        "Replace current pane",
                        Some(Message::Replace(buffer.clone().into())),
                    ),
                    Entry::Close(window, pane) => {
                        ("Close pane", Some(Message::Close(window, pane)))
                    }
                    Entry::Swap(window, pane) => (
                        "Swap with current pane",
                        Some(Message::Swap(window, pane)),
                    ),
                    Entry::Context => {
                        return container(
                            text(title)
                                .style(theme::text::primary)
                                .font_maybe(
                                    theme::font_style::primary(theme)
                                        .map(font::get),
                                )
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
                    _ => {
                        return row![].into();
                    }
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

enum Icon<'a> {
    Upstream(&'a Image),
    Internal(TextColorSvg<'a, Theme>),
}

fn sidebar_icon<'a>(
    icon: Option<Icon<'a>>,
    indicator: Option<(TextColorSvg<'a, Theme>, u32)>,
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
            Icon::Internal(icon) => icon.into(),
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

impl From<&config::sidebar::Sidebar> for Dimensions {
    fn from(config: &config::sidebar::Sidebar) -> Self {
        let (icon_size, icon_badge_padding, icon_badge_size) =
            match config.primary_icon {
                PrimaryIcon::Size(icon_size) => {
                    let icon_badge_padding = 2;
                    let icon_badge_size =
                        (icon_size / 3).max(4) + 2 * icon_badge_padding;

                    (icon_size, icon_badge_padding, icon_badge_size)
                }
                PrimaryIcon::Hidden => (0, 0, 0),
            };

        let unread_indicator_size = if config.unread_indicator.has_icon() {
            config.unread_indicator.icon_size
        } else {
            0
        };

        let highlight_indicator_size = if config.highlight_indicator.has_icon()
        {
            config.highlight_indicator.icon_size
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

#[derive(Debug, Clone, Copy)]
enum ConnectionStatus {
    Connected { registration_complete: bool },
    Disconnected { autoconnect: bool, connecting: bool },
}
