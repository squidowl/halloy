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

Use `$1` through `$9` to insert positional arguments.

You can also use context-aware placeholders:

- `$nick` inserts your current nickname.
- `$channel` inserts the active channel name.
- `$server` inserts the active server name.

## Notes

- Aliases take precedence over built-in commands with the same name.
- Alias expansion happens once; aliases do not expand other aliases.
