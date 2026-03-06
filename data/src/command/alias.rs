use super::Error;
use crate::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    pub name: String,
    pub body: String,
    pub min_args: usize,
}

pub fn list(config: &Config) -> Vec<Alias> {
    let mut aliases = config
        .buffer
        .commands
        .aliases
        .iter()
        .map(|(name, body)| Alias {
            name: name.clone(),
            body: body.clone(),
            min_args: required_args(body),
        })
        .collect::<Vec<_>>();

    aliases.sort_by(|left, right| left.name.cmp(&right.name));

    aliases
}

pub(super) fn expand(
    command: &str,
    raw_args: &str,
    config: &Config,
) -> Result<Option<String>, Error> {
    let Some(alias) = config
        .buffer
        .commands
        .aliases
        .get(&command.to_ascii_lowercase())
    else {
        return Ok(None);
    };

    let min_args = required_args(alias);
    let actual = raw_args.split_ascii_whitespace().count();

    if actual < min_args {
        return Err(Error::IncorrectArgCount {
            min: min_args,
            max: min_args,
            actual,
        });
    }

    Ok(Some(expand_alias(alias, raw_args)))
}

pub fn required_args(alias: &str) -> usize {
    let mut min_args = 0;
    let mut rest = alias;

    while let Some(index) = rest.find('$') {
        rest = &rest[index + 1..];

        if let Some((arg_index, consumes_dash)) = parse_placeholder(rest) {
            min_args = min_args.max(arg_index + 1);
            rest = &rest[1 + usize::from(consumes_dash)..];
        }
    }

    min_args
}

pub fn placeholder_args(min_args: usize) -> Vec<String> {
    (1..=min_args).map(|index| format!("arg{index}")).collect()
}

fn expand_alias(alias: &str, raw_args: &str) -> String {
    let args = raw_args.split_ascii_whitespace().collect::<Vec<_>>();
    let expanded = substitute_args(alias, &args);
    let trimmed = expanded.trim_start();

    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn substitute_args(template: &str, args: &[&str]) -> String {
    let mut expanded = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(index) = rest.find('$') {
        expanded.push_str(&rest[..index]);
        rest = &rest[index + 1..];

        let Some((arg_index, take_rest)) = parse_placeholder(rest) else {
            expanded.push('$');
            continue;
        };

        push_argument(&mut expanded, args, arg_index, take_rest);
        rest = &rest[1 + usize::from(take_rest)..];
    }

    expanded.push_str(rest);
    expanded
}

fn parse_placeholder(input: &str) -> Option<(usize, bool)> {
    let digit = *input.as_bytes().first()?;

    if !(b'1'..=b'9').contains(&digit) {
        return None;
    }

    let arg_index = (digit - b'1') as usize;
    let take_rest = input.as_bytes().get(1) == Some(&b'-');

    Some((arg_index, take_rest))
}

fn push_argument(
    expanded: &mut String,
    args: &[&str],
    arg_index: usize,
    take_rest: bool,
) {
    if take_rest {
        if let Some(args) = args.get(arg_index..) {
            expanded.push_str(&args.join(" "));
        }
    } else if let Some(arg) = args.get(arg_index) {
        expanded.push_str(arg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- required_args ---

    #[test]
    fn required_args_no_placeholders() {
        assert_eq!(required_args("/mode #halloy +o"), 0);
    }

    #[test]
    fn required_args_single() {
        assert_eq!(required_args("/me says hello to $1!"), 1);
    }

    #[test]
    fn required_args_multiple() {
        assert_eq!(required_args("/mode #halloy +ooo $1 $2 $3"), 3);
    }

    #[test]
    fn required_args_non_sequential() {
        // $3 used without $2 — min_args is still 3
        assert_eq!(required_args("/kick $3 $1"), 3);
    }

    #[test]
    fn required_args_rest_placeholder() {
        assert_eq!(required_args("/topic #halloy $1-"), 1);
    }

    #[test]
    fn required_args_ignores_invalid_placeholders() {
        // $0 and $a are not valid
        assert_eq!(required_args("$0 $a $$ text"), 0);
    }

    #[test]
    fn required_args_dollar_ten_treated_as_dollar_one() {
        // $10 is $1 followed by literal '0'
        assert_eq!(required_args("$10"), 1);
    }

    // --- placeholder_args ---

    #[test]
    fn placeholder_args_generates_names() {
        assert_eq!(placeholder_args(3), vec!["arg1", "arg2", "arg3"]);
    }

    #[test]
    fn placeholder_args_zero() {
        assert!(placeholder_args(0).is_empty());
    }

    // --- parse_placeholder ---

    #[test]
    fn parse_placeholder_valid_digits() {
        assert_eq!(parse_placeholder("1"), Some((0, false)));
        assert_eq!(parse_placeholder("9"), Some((8, false)));
    }

    #[test]
    fn parse_placeholder_with_rest() {
        assert_eq!(parse_placeholder("1-"), Some((0, true)));
        assert_eq!(parse_placeholder("3-rest"), Some((2, true)));
    }

    #[test]
    fn parse_placeholder_invalid() {
        assert_eq!(parse_placeholder("0"), None);
        assert_eq!(parse_placeholder("a"), None);
        assert_eq!(parse_placeholder(""), None);
    }

    // --- substitute_args ---

    #[test]
    fn substitute_basic() {
        assert_eq!(
            substitute_args("/mode #halloy +o $1", &["nick"]),
            "/mode #halloy +o nick"
        );
    }

    #[test]
    fn substitute_multiple_args() {
        assert_eq!(
            substitute_args("/mode +oo $1 $2", &["alice", "bob"]),
            "/mode +oo alice bob"
        );
    }

    #[test]
    fn substitute_rest() {
        assert_eq!(
            substitute_args("/topic #halloy $1-", &["hello", "world"]),
            "/topic #halloy hello world"
        );
    }

    #[test]
    fn substitute_rest_single_arg() {
        assert_eq!(
            substitute_args("/topic #ch $1-", &["only"]),
            "/topic #ch only"
        );
    }

    #[test]
    fn substitute_missing_arg_omitted() {
        // $2 with only 1 arg — silently omitted
        assert_eq!(substitute_args("$1 $2", &["hello"]), "hello ");
    }

    #[test]
    fn substitute_preserves_literal_dollar() {
        assert_eq!(substitute_args("costs $$1", &["five"]), "costs $five");
    }

    #[test]
    fn substitute_no_placeholders() {
        assert_eq!(substitute_args("/list", &[]), "/list");
    }

    #[test]
    fn substitute_extra_args_ignored() {
        assert_eq!(substitute_args("/me $1", &["hello", "extra"]), "/me hello");
    }

    // --- expand_alias ---

    #[test]
    fn expand_alias_prepends_slash() {
        assert_eq!(expand_alias("mode +o $1", "nick"), "/mode +o nick");
    }

    #[test]
    fn expand_alias_preserves_existing_slash() {
        assert_eq!(expand_alias("/mode +o $1", "nick"), "/mode +o nick");
    }

    #[test]
    fn expand_alias_with_rest_args() {
        assert_eq!(
            expand_alias("/topic #ch $1-", "hello world"),
            "/topic #ch hello world"
        );
    }

    #[test]
    fn expand_alias_no_args() {
        assert_eq!(expand_alias("/list", ""), "/list");
    }
}
