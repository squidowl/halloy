use std::collections::HashMap;

use data::config::Config;
use data::config::server::SidebarVisibility;
use data::{Server, buffer, client, history, isupport, target};

use super::{IndicatorState, Panes, indicator_state};
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

pub fn indicators(
    config: &Config,
    panes: &Panes,
    clients: &client::Map,
    connection: &client::Client,
    server: &Server,
    queries: &[&target::Query],
    casemapping: isupport::CaseMap,
    history: &history::Manager,
) -> IndicatorState {
    let mut indicators = IndicatorState::default();

    for channel in connection.channels() {
        let buffer = buffer::Upstream::Channel(server.clone(), channel.clone());
        let kind = history::Kind::Channel(server.clone(), channel.clone());
        indicators.merge(indicator_state(
            config,
            panes,
            &buffer,
            &kind,
            casemapping,
            history,
        ));
    }

    for query in queries {
        let query = clients.resolve_query(server, query).unwrap_or(query);
        let buffer = buffer::Upstream::Query(server.clone(), query.clone());
        let kind = history::Kind::Query(server.clone(), query.clone());
        indicators.merge(indicator_state(
            config,
            panes,
            &buffer,
            &kind,
            casemapping,
            history,
        ));
    }

    indicators
}
