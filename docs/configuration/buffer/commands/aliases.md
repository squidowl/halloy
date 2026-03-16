# Aliases

Define custom slash command aliases.

```toml
# Type: map
# Values: map with string key/value pairs
# Default: {}

[buffer.commands.aliases]
op = "/mode #halloy +ooo $1 $2 $3"
halloy = "/me says halloy to $1!"
topic = "/topic #halloy $1-"
deopme = "/mode -o $nick"
```

Use `$1` through `$9` to insert positional arguments.  A hyphen after
the argument number (e.g. `$1-`) means that all following arguments will
also be included in the argument; for example, `/topic our new topic`
for the alias defined as `topic = "/topic #halloy $1-"` will expand to
`/topic #halloy our new topic`.

You can also use context-aware placeholders:

- `$nick` inserts your current nickname.
- `$channel` inserts the active channel name.
- `$server` inserts the active server name.

Aliases must be specified in reference to existing slash commands, so to
send a regular message the `/msg` command (or equivalent, such as
`/plain`/`/format` command) should be used.  For example, `welcome =
"/msg $channel welcome to IRC $1, enjoy your stay!"`.

## Notes

- Aliases take precedence over built-in commands with the same name.
- Alias expansion happens once; aliases do not expand other aliases.
