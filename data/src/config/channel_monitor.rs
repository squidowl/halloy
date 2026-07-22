use serde::Deserialize;

use crate::config::inclusivities::{Inclusivities, is_target_channel_included};
use crate::{Server, isupport, target};

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct ChannelMonitor {
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
}

impl ChannelMonitor {
    pub fn is_channel_included(
        &self,
        server: &Server,
        channel: &target::Channel,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_target_channel_included(
            self.include.as_ref(),
            self.exclude.as_ref(),
            None,
            channel,
            server,
            casemapping,
        )
    }
}
