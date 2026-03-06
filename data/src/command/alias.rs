use crate::Config;

pub(super) fn expand(
    command: &str,
    raw_args: &str,
    config: &Config,
) -> Option<String> {
    config
        .buffer
        .commands
        .aliases
        .get(&command.to_ascii_lowercase())
        .map(|alias| expand_alias(alias, raw_args))
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

        let Some((arg_index, consumes_dash)) = parse_placeholder(rest) else {
            expanded.push('$');
            continue;
        };

        write_placeholder(&mut expanded, args, arg_index, consumes_dash);
        rest = &rest[1 + usize::from(consumes_dash)..];
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
    let consumes_dash = input.as_bytes().get(1) == Some(&b'-');

    Some((arg_index, consumes_dash))
}

fn write_placeholder(
    expanded: &mut String,
    args: &[&str],
    arg_index: usize,
    consumes_dash: bool,
) {
    if consumes_dash {
        if let Some(arg_range) = args.get(arg_index..) {
            append_args(expanded, arg_range);
        }
    } else if let Some(arg) = args.get(arg_index) {
        expanded.push_str(arg);
    }
}

fn append_args(expanded: &mut String, args: &[&str]) {
    let mut args = args.iter();

    if let Some(first) = args.next() {
        expanded.push_str(first);

        for arg in args {
            expanded.push(' ');
            expanded.push_str(arg);
        }
    }
}
