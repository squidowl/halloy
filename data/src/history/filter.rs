use chrono::{DateTime, Utc};

use crate::{message::Source, Message};

#[derive(Debug)]
pub enum Filter {
    ExcludeSources(Vec<Source>),
}

impl Filter {
    fn filter(&self, message: &Message) -> bool {
        match &self {
            Filter::ExcludeSources(target_list) => !target_list.contains(&message.target.source()),
        }
    }
}

#[derive(Debug)]
pub struct FilterChain {
    filters: Vec<Filter>,
}

impl FilterChain {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }
    pub fn add_source_list_filter<I, T>(mut self, values: &I) -> Self
    where
        I: IntoIterator<Item = T> + Clone,
        T: Into<Source> + Clone,
    {
        let target_list: Vec<Source> = values
            .clone()
            .into_iter()
            .map(|targ| targ.clone().into())
            .collect();
        let filter = Filter::ExcludeSources(target_list);
        self.filters.push(filter);
        self
    }

    pub fn build(self) -> FilterChain {
        FilterChain {
            filters: self.filters,
        }
    }

    pub fn test(&self, message: &Message) -> bool {
        for f in &self.filters {
            if f.filter(&message) == false {
                return false;
            }
        }
        return true;
    }
}
