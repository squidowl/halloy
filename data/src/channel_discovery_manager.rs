use chrono::{DateTime, Utc};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::message;

static MAX_RESULTS: usize = 150;

#[derive(Default, Clone, Debug)]
pub struct Manager {
    pub channels: HashMap<String, (message::Content, usize)>,
    pub last_updated: Option<DateTime<Utc>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            last_updated: None,
        }
    }

    pub fn push(&mut self, channel: String, topic: String, user_count: String) {
        let user_count = user_count.parse().unwrap_or(0);
        let topic_content = message::parse_fragments(topic);
        
        self.channels.insert(channel, (topic_content, user_count));
    }

    fn sort_by_user_count<'a>(
        &self,
        mut results: Vec<(&'a String, &'a message::Content, &'a usize)>,
    ) -> Vec<(&'a String, &'a message::Content, &'a usize)> {
        results.sort_unstable_by(|(_, _, user_count_a), (_, _, user_count_b)| {
            user_count_b.cmp(user_count_a)
        });
        results
    }

    pub fn items(
        &self,
        search_query: &str,
    ) -> Vec<(&'_ String, &'_ message::Content, &'_ usize)> {
        let query = search_query.trim();

        // all channels when no query
        if query.is_empty() {
            let results: Vec<_> = self
                .channels
                .iter()
                .map(|(channel, (topic_content, user_count))| {
                    (channel, topic_content, user_count)
                })
                .collect();
            return self.sort_by_user_count(results);
        }

        // simple substring search
        if query.len() <= 2 {
            let query_lower = query.to_lowercase();
            let results: Vec<_> = self
                .channels
                .iter()
                .filter_map(|(channel, (topic_content, user_count))| {
                    let channel_lower = channel.to_lowercase();
                    let topic_text = topic_content.text().to_lowercase();
                    if channel_lower.contains(&query_lower)
                        || topic_text.contains(&query_lower)
                    {
                        Some((channel, topic_content, user_count))
                    } else {
                        None
                    }
                })
                .take(MAX_RESULTS)
                .collect();
            return self.sort_by_user_count(results);
        }

        // fuzzy search
        self.fuzzy_search(query)
    }

    fn fuzzy_search(
        &self,
        query: &str,
    ) -> Vec<(&'_ String, &'_ message::Content, &'_ usize)> {
        let pattern =
            Pattern::new(query, CaseMatching::Ignore, Normalization::Smart, AtomKind::Fuzzy);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut buffer = Vec::new();
        let mut search_text = String::new();
        let mut scored = Vec::with_capacity(self.channels.len());

        for (channel, (topic_content, user_count)) in self.channels.iter() {
            search_text.clear();
            search_text.push_str(channel);
            search_text.push(' ');
            search_text.push_str(&topic_content.text());

            let hay = Utf32Str::new(&search_text, &mut buffer);
            let score = pattern.score(hay, &mut matcher).unwrap_or(0);

            if score > 0 {
                scored.push((score, channel, topic_content, user_count));
            }
        }

        fn cmp_entries(
            (score_a, channel_a, _, user_count_a): &(u32, &String, &message::Content, &usize),
            (score_b, channel_b, _, user_count_b): &(u32, &String, &message::Content, &usize),
        ) -> Ordering {
            user_count_b
                .cmp(user_count_a)
                .then_with(|| score_b.cmp(score_a))
                .then_with(|| channel_a.cmp(channel_b))
        }

        if scored.len() > MAX_RESULTS {
            scored.select_nth_unstable_by(MAX_RESULTS - 1, cmp_entries);
            scored.truncate(MAX_RESULTS);
        }

        scored.sort_unstable_by(cmp_entries);

        scored.truncate(MAX_RESULTS);

        scored
            .into_iter()
            .map(|(_, channel, topic_content, user_count)| (channel, topic_content, user_count))
            .collect()
    }
}

