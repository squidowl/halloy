use std::collections::HashMap;

use data::Server;
use data::config::Config;
use data::config::server::SidebarVisibility;

use crate::dashboard::sidebar::ConnectionStatus;
use crate::widget::Text;
use crate::{icon, theme};

#[derive(Clone, Default)]
pub struct State {
    visibility: HashMap<Server, SidebarVisibility>,
}

impl State {
    pub fn set(&mut self, server: Server, visibility: SidebarVisibility) {
        self.visibility.insert(server, visibility);
    }

    pub fn is_expanded(&self, config: &Config, server: &Server) -> bool {
        let visibility = self
            .visibility
            .get(server)
            .copied()
            .or_else(|| {
                config
                    .servers
                    .get(server)
                    .map(|server| server.sidebar_visibility)
            })
            .unwrap_or_default();

        matches!(visibility, SidebarVisibility::Expanded)
    }

    pub fn disclosure(
        &self,
        config: &Config,
        server: &Server,
        connection_status: &ConnectionStatus,
        has_members: bool,
        content_height: f32,
    ) -> Option<Disclosure> {
        if !matches!(connection_status, ConnectionStatus::Connected { .. })
            || !has_members
            || !config.sidebar.collapse_button.enabled
        {
            return None;
        }

        let is_expanded = self.is_expanded(config, server);
        let indicator = match (config.sidebar.position, is_expanded) {
            (
                data::config::sidebar::Position::Left
                | data::config::sidebar::Position::Right,
                true,
            ) => icon::chevron_down(),
            (
                data::config::sidebar::Position::Left
                | data::config::sidebar::Position::Right,
                false,
            ) => icon::chevron_right(),
            (
                data::config::sidebar::Position::Top
                | data::config::sidebar::Position::Bottom,
                true,
            ) => icon::chevron_right(),
            (
                data::config::sidebar::Position::Top
                | data::config::sidebar::Position::Bottom,
                false,
            ) => icon::chevron_left(),
        };

        Some(Disclosure {
            indicator,
            next_visibility: if is_expanded {
                SidebarVisibility::Collapsed
            } else {
                SidebarVisibility::Expanded
            },
            size: content_height
                + 1.0
                + 2.0 * f32::from(config.sidebar.padding.buffer[0]),
        })
    }
}

pub struct Disclosure {
    pub indicator: Text<'static>,
    pub next_visibility: SidebarVisibility,
    pub size: f32,
}

impl Disclosure {
    pub fn indicator(self) -> Text<'static> {
        self.indicator.size(8).style(theme::text::secondary)
    }
}
