#[derive(Debug)]
pub struct Query<'a> {
    pub command: &'a str,
    pub params: &'a str,
}

pub fn is_query(text: &str) -> bool {
    text.starts_with('\u{1}')
}

pub fn parse_query(text: &str) -> Option<Query> {
    let query = text
        .strip_suffix('\u{1}')
        .unwrap_or(text)
        .strip_prefix('\u{1}')?;

    if let Some((command, params)) = query.split_once(char::is_whitespace) {
        Some(Query { command, params })
    } else {
        Some(Query {
            command: query,
            params: "",
        })
    }
}
