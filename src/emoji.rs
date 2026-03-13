use std::collections::HashSet;

use strsim::jaro_winkler;

struct SearchMatch {
    emoji: &'static emojis::Emoji,
    shortcode: &'static str,
    similarity: f64,
}

pub fn matching_emojis(query: &str) -> Vec<&'static emojis::Emoji> {
    if query.is_empty() {
        return emojis::iter().collect();
    }

    let mut seen = HashSet::new();

    search_matches(query)
        .into_iter()
        .filter_map(|matched| {
            seen.insert(matched.emoji.as_str()).then_some(matched.emoji)
        })
        .collect()
}

pub fn matching_shortcodes(query: &str) -> Vec<&'static str> {
    search_matches(query)
        .into_iter()
        .map(|matched| matched.shortcode)
        .collect()
}

fn search_matches(query: &str) -> Vec<SearchMatch> {
    let mut filtered = emojis::iter()
        .flat_map(|emoji| {
            emoji.shortcodes().filter_map(move |shortcode| {
                if shortcode.contains(query) {
                    Some(SearchMatch {
                        emoji,
                        shortcode,
                        similarity: jaro_winkler(query, shortcode),
                    })
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();

    filtered.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));
    filtered
}
