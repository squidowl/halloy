use serde::Deserialize;

use crate::{message::Source, Message};

#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
pub enum Filter {
    ExcludeSources(Vec<Source>),
}

impl Filter {
    fn pass(&self, message: &Message) -> bool {
        match &self {
            Filter::ExcludeSources(target_list) => !target_list.contains(message.target.source()),
        }
    }
}

pub struct FilterChain<'f> {
    filters: &'f Vec<Filter>,
}

impl<'f> FilterChain<'f> {
    pub fn from(filters: &'f Vec<Filter>) -> Self {
        Self { filters }
    }

    pub fn pass(&self, message: &Message) -> bool {
        for filter in self.filters {
            if !filter.pass(message) {
                return false;
            }
        }
        true
    }
}
