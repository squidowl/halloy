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
    let expanded = substitute_args(alias.trim(), &args);
    let expanded = expanded.trim_start();

    if expanded.starts_with('/') {
        expanded.to_string()
    } else {
        format!("/{expanded}")
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
