use std::borrow::Cow;

use super::Error;
use crate::Config;
use crate::buffer::Upstream;
use crate::user::NickRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    pub name: String,
    pub body: String,
    pub min_args: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context<'a> {
    nick: Option<Cow<'a, str>>,
    channel: Option<Cow<'a, str>>,
    server: Option<Cow<'a, str>>,
}

impl<'a> Context<'a> {
    pub fn new(
        buffer: Option<&'a Upstream>,
        nick: Option<NickRef<'a>>,
    ) -> Self {
        let nick = nick.map(|nick| Cow::Owned(nick.as_str().to_string()));
        let channel = buffer
            .and_then(Upstream::channel)
            .map(|channel| Cow::Borrowed(channel.as_str()));
        let server =
            buffer.map(|buffer| Cow::Owned(buffer.server().to_string()));

        Self {
            nick,
            channel,
            server,
        }
    }
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
    context: &Context<'_>,
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

    Ok(Some(expand_alias(alias, raw_args, context)))
}

pub fn required_args(alias: &str) -> usize {
    let mut min_args = 0;
    let mut rest = alias;

    while let Some(index) = rest.find('$') {
        rest = &rest[index + 1..];

        if let Some(placeholder) = parse_placeholder(rest) {
            if let Placeholder::Argument { index, .. } = placeholder {
                min_args = min_args.max(index + 1);
            }

            rest = &rest[placeholder.consumed_len()..];
        }
    }

    min_args
}

pub fn placeholder_args(min_args: usize) -> Vec<String> {
    (1..=min_args).map(|index| format!("arg{index}")).collect()
}

fn expand_alias(alias: &str, raw_args: &str, context: &Context<'_>) -> String {
    let args = raw_args.split_ascii_whitespace().collect::<Vec<_>>();
    let expanded = substitute_args(alias, &args, context);
    let trimmed = expanded.trim_start();

    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn substitute_args(
    template: &str,
    args: &[&str],
    context: &Context<'_>,
) -> String {
    let mut expanded = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(index) = rest.find('$') {
        expanded.push_str(&rest[..index]);
        rest = &rest[index + 1..];

        let Some(placeholder) = parse_placeholder(rest) else {
            expanded.push('$');
            continue;
        };

        push_placeholder(&mut expanded, args, context, &placeholder);
        rest = &rest[placeholder.consumed_len()..];
    }

    expanded.push_str(rest);
    expanded
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Placeholder {
    Argument { index: usize, take_rest: bool },
    Variable(Variable),
}

impl Placeholder {
    fn consumed_len(&self) -> usize {
        match self {
            Self::Argument { take_rest, .. } => 1 + usize::from(*take_rest),
            Self::Variable(variable) => variable.as_str().len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variable {
    Nick,
    Channel,
    Server,
}

impl Variable {
    fn as_str(self) -> &'static str {
        match self {
            Self::Nick => "nick",
            Self::Channel => "channel",
            Self::Server => "server",
        }
    }
}

fn parse_placeholder(input: &str) -> Option<Placeholder> {
    let digit = *input.as_bytes().first()?;

    if (b'1'..=b'9').contains(&digit) {
        let arg_index = (digit - b'1') as usize;
        let take_rest = input.as_bytes().get(1) == Some(&b'-');

        return Some(Placeholder::Argument {
            index: arg_index,
            take_rest,
        });
    }

    for variable in [Variable::Nick, Variable::Channel, Variable::Server] {
        if input.starts_with(variable.as_str()) {
            return Some(Placeholder::Variable(variable));
        }
    }

    None
}

fn push_placeholder(
    expanded: &mut String,
    args: &[&str],
    context: &Context<'_>,
    placeholder: &Placeholder,
) {
    match placeholder {
        Placeholder::Argument { index, take_rest } => {
            if *take_rest {
                if let Some(args) = args.get(*index..) {
                    expanded.push_str(&args.join(" "));
                }
            } else if let Some(arg) = args.get(*index) {
                expanded.push_str(arg);
            }
        }
        Placeholder::Variable(variable) => {
            if let Some(value) = match variable {
                Variable::Nick => context.nick.as_deref(),
                Variable::Channel => context.channel.as_deref(),
                Variable::Server => context.server.as_deref(),
            } {
                expanded.push_str(value);
            }
        }
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
    fn required_args_ignores_named_placeholders() {
        assert_eq!(required_args("/msg $nick on $server"), 0);
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
        assert_eq!(
            parse_placeholder("1"),
            Some(Placeholder::Argument {
                index: 0,
                take_rest: false,
            })
        );
        assert_eq!(
            parse_placeholder("9"),
            Some(Placeholder::Argument {
                index: 8,
                take_rest: false,
            })
        );
    }

    #[test]
    fn parse_placeholder_with_rest() {
        assert_eq!(
            parse_placeholder("1-"),
            Some(Placeholder::Argument {
                index: 0,
                take_rest: true,
            })
        );
        assert_eq!(
            parse_placeholder("3-rest"),
            Some(Placeholder::Argument {
                index: 2,
                take_rest: true,
            })
        );
    }

    #[test]
    fn parse_placeholder_named() {
        assert_eq!(
            parse_placeholder("nick"),
            Some(Placeholder::Variable(Variable::Nick))
        );
        assert_eq!(
            parse_placeholder("channel-rest"),
            Some(Placeholder::Variable(Variable::Channel))
        );
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
            substitute_args(
                "/mode #halloy +o $1",
                &["nick"],
                &Context::default()
            ),
            "/mode #halloy +o nick"
        );
    }

    #[test]
    fn substitute_multiple_args() {
        assert_eq!(
            substitute_args(
                "/mode +oo $1 $2",
                &["alice", "bob"],
                &Context::default()
            ),
            "/mode +oo alice bob"
        );
    }

    #[test]
    fn substitute_rest() {
        assert_eq!(
            substitute_args(
                "/topic #halloy $1-",
                &["hello", "world"],
                &Context::default()
            ),
            "/topic #halloy hello world"
        );
    }

    #[test]
    fn substitute_rest_single_arg() {
        assert_eq!(
            substitute_args("/topic #ch $1-", &["only"], &Context::default()),
            "/topic #ch only"
        );
    }

    #[test]
    fn substitute_missing_arg_omitted() {
        // $2 with only 1 arg — silently omitted
        assert_eq!(
            substitute_args("$1 $2", &["hello"], &Context::default()),
            "hello "
        );
    }

    #[test]
    fn substitute_preserves_literal_dollar() {
        assert_eq!(
            substitute_args("costs $$1", &["five"], &Context::default()),
            "costs $five"
        );
    }

    #[test]
    fn substitute_no_placeholders() {
        assert_eq!(substitute_args("/list", &[], &Context::default()), "/list");
    }

    #[test]
    fn substitute_extra_args_ignored() {
        assert_eq!(
            substitute_args("/me $1", &["hello", "extra"], &Context::default()),
            "/me hello"
        );
    }

    #[test]
    fn substitute_named_placeholders() {
        let context = Context {
            nick: Some(Cow::Borrowed("casperstorm")),
            channel: Some(Cow::Borrowed("#halloy")),
            server: Some(Cow::Borrowed("libera")),
        };

        assert_eq!(
            substitute_args(
                "/msg $nick from $channel on $server",
                &[],
                &context
            ),
            "/msg casperstorm from #halloy on libera"
        );
    }

    #[test]
    fn substitute_repeated_named_placeholders() {
        let context = Context {
            nick: Some(Cow::Borrowed("casperstorm")),
            channel: Some(Cow::Borrowed("#halloy")),
            server: Some(Cow::Borrowed("libera")),
        };

        assert_eq!(
            substitute_args(
                "/me $nick waves at $nick on $server",
                &[],
                &context
            ),
            "/me casperstorm waves at casperstorm on libera"
        );
    }

    #[test]
    fn substitute_missing_named_placeholders_omitted() {
        assert_eq!(
            substitute_args(
                "/msg $channel$server$nick",
                &[],
                &Context::default()
            ),
            "/msg "
        );
    }

    // --- expand_alias ---

    #[test]
    fn expand_alias_prepends_slash() {
        assert_eq!(
            expand_alias("mode +o $1", "nick", &Context::default()),
            "/mode +o nick"
        );
    }

    #[test]
    fn expand_alias_preserves_existing_slash() {
        assert_eq!(
            expand_alias("/mode +o $1", "nick", &Context::default()),
            "/mode +o nick"
        );
    }

    #[test]
    fn expand_alias_with_rest_args() {
        assert_eq!(
            expand_alias("/topic #ch $1-", "hello world", &Context::default()),
            "/topic #ch hello world"
        );
    }

    #[test]
    fn expand_alias_no_args() {
        assert_eq!(expand_alias("/list", "", &Context::default()), "/list");
    }

    #[test]
    fn context_from_server_buffer() {
        let nick = crate::user::Nick::from_str(
            "casperstorm",
            crate::isupport::CaseMap::default(),
        );
        let server = crate::Server::from(std::sync::Arc::<str>::from("libera"));
        let buffer = Upstream::Server(server);
        let context = Context::new(Some(&buffer), Some(nick.as_nickref()));

        assert_eq!(context.nick.as_deref(), Some("casperstorm"));
        assert_eq!(context.channel.as_deref(), None);
        assert_eq!(context.server.as_deref(), Some("libera"));
    }
}
