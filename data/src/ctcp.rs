#[derive(Debug)]
pub struct Query<'a> {
    pub command: &'a str,
    pub params: &'a str,
}

pub fn is_ctcp_query(text: &str) -> bool {
    text.starts_with('\u{1}')
}

pub fn parse_ctcp_query(text: &str) -> Result<Query, &'static str> {
    let query = text
        .strip_suffix('\u{1}')
        .unwrap_or(text)
        .strip_prefix('\u{1}')
        .ok_or("text is not a CTCP query")?;

    if let Some((command, params)) = query.split_once(char::is_whitespace) {
        Ok(Query { command, params })
    } else {
        Ok(Query {
            command: query,
            params: "",
        })
    }
}
