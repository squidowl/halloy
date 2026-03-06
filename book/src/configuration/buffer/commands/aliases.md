# Aliases

Define custom slash command aliases.

```toml
# Type: map
# Values: map with string key/value pairs
# Default: {}

[buffer.commands.aliases]
op = "/mode #noc +ooo $1 $2 $3"
halloy = "/me says halloy to $1!"
```

Use `$1` through `$9` to insert corresponding arguments.

## Notes

- Aliases take precedence over built-in commands with the same name.
- Alias expansion happens once; aliases do not expand other aliases.
